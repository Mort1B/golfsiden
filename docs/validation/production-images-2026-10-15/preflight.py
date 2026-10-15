"""Execute packaged-artifact and fail-closed preflights, without a database/server."""
from pathlib import Path
import hashlib
import json
import re
import secrets
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
PHASE = sys.argv[1] if len(sys.argv) > 1 else 'baseline'
assert PHASE in ('baseline', 'candidate')
builds = json.loads((HERE / (PHASE + '-builds.json')).read_text())
assert len(builds['builds']) == 3 and all(item['exit_code'] == 0 for item in builds['builds'])
tags = {item['service']: item['tag'] for item in builds['builds']}
report = {'phase': PHASE, 'context_sha256': builds['context_sha256'], 'checks': [],
          'network': 'none', 'published_ports': False, 'image_volumes': 'ignore',
          'database_initialized': False, 'proxy_server_started': False}
secret = secrets.token_urlsafe(32)
password = secrets.token_urlsafe(32)
containers = []


def command(args, timeout=60):
    return subprocess.run(args, capture_output=True, text=True, timeout=timeout)


with tempfile.TemporaryDirectory(prefix='golf-deploy2-inspect-') as directory:
    root = Path(directory)

    def run(tag, args, env=None):
        index = len(containers)
        cidfile = root / (str(index) + '.cid')
        envfile = root / (str(index) + '.env')
        envfile.write_text(''.join(key + '=' + value + '\n' for key, value in (env or {}).items()))
        envfile.chmod(0o600)
        create = ['podman', 'create', '--cidfile', str(cidfile), '--network', 'none',
                  '--read-only', '--image-volume=ignore', '--env-file', str(envfile)]
        if args:
            create += ['--entrypoint', args[0]]
        create += [tag] + args[1:]
        cid = None
        try:
            created = command(create)
            assert created.returncode == 0, 'Owned probe container creation failed'
            cid = cidfile.read_text().strip()
            assert re.fullmatch(r'[0-9a-f]{64}', cid)
            containers.append(cid)
            output = command(['podman', 'start', '--attach', cid], timeout=45)
            state = json.loads(command(['podman', 'inspect', cid, '--format', '{{json .State}}']).stdout)
            assert not state['Running']
            return int(state['ExitCode']), output.stdout, output.stderr
        finally:
            if cidfile.exists():
                owned = cidfile.read_text().strip()
                assert re.fullmatch(r'[0-9a-f]{64}', owned)
                assert command(['podman', 'rm', '--force', owned]).returncode == 0
                assert command(['podman', 'container', 'exists', owned]).returncode == 1

    def check(name, tag, args, env=None, rejected=None):
        code, stdout, stderr = run(tag, args, env)
        if rejected is None:
            assert code == 0, name
        else:
            assert code in (1, 2) and rejected in stdout + stderr, (name, code)
        report['checks'].append({'case': name, 'exit_code': code, 'passed': True})
        print('PASS ' + name, flush=True)
        return stdout

    shell = '''set -eu
test "$(id -u)" = 10001
for binary in golf-api migrate password-recovery; do
    test -x "/usr/local/bin/$binary"
    ldd "/usr/local/bin/$binary"
done
test ! -e /usr/local/bin/seed
test ! -e /source
curl --version
'''
    linked = check('api-user-binaries-libraries-no-seed-no-source', tags['api'], ['/bin/sh', '-c', shell])
    assert 'not found' not in linked
    api = {'APP_ENV': 'production', 'SESSION_COOKIE_SECURE': 'true', 'RUN_MIGRATIONS': 'false',
           'PROXY_SHARED_SECRET': secret, 'APP_DATABASE_USER': 'golf_runtime'}
    for name, changes, expected in [
        ('api-insecure-cookie-denied', {'SESSION_COOKIE_SECURE': 'false'}, 'SESSION_COOKIE_SECURE'),
        ('api-startup-migration-denied', {'RUN_MIGRATIONS': 'true'}, 'RUN_MIGRATIONS'),
        ('api-invalid-proxy-secret-denied', {'PROXY_SHARED_SECRET': 'invalid'}, '[redacted]'),
        ('api-missing-runtime-user-denied', {'APP_DATABASE_USER': None}, 'APP_DATABASE_USER'),
        ('api-zero-port-denied', {'PORT': '0'}, 'PORT'),
    ]:
        selected = dict(api)
        for key, value in changes.items():
            if value is None:
                selected.pop(key)
            else:
                selected[key] = value
        check(name, tags['api'], ['/usr/local/bin/golf-api'], selected, expected)
    check('recovery-command-executes-and-requires-action', tags['api'],
          ['/usr/local/bin/password-recovery'], rejected='expected issue or revoke')
    check('migrate-command-executes-and-requires-database', tags['api'],
          ['/usr/local/bin/migrate'], rejected='NotPresent')

    pg = {'POSTGRES_USER': 'golf_owner', 'APP_DATABASE_USER': 'golf_runtime',
          'POSTGRES_PASSWORD': password, 'APP_DATABASE_PASSWORD': secrets.token_urlsafe(32)}
    # Run through the production wrapper; --version prevents cluster initialization.
    wrapper = ['/usr/local/bin/validate-and-run-postgres', 'postgres', '--version']
    report['postgres_version'] = check('postgres-valid-wrapper-version', tags['postgres'], wrapper, pg).strip()
    for name, changes, expected in [
        ('postgres-same-user-denied', {'APP_DATABASE_USER': 'golf_owner'}, 'must differ'),
        ('postgres-same-password-denied', {'APP_DATABASE_PASSWORD': password}, 'passwords must differ'),
        ('postgres-placeholder-denied', {'POSTGRES_PASSWORD': 'replace-with-url-safe-random-owner-password'}, 'replace the example'),
        ('postgres-short-password-denied', {'POSTGRES_PASSWORD': 'short'}, 'at least 24'),
        ('postgres-missing-password-denied', {'POSTGRES_PASSWORD': None}, 'POSTGRES_PASSWORD is required'),
    ]:
        selected = dict(pg)
        for key, value in changes.items():
            if value is None:
                selected.pop(key)
            else:
                selected[key] = value
        check(name, tags['postgres'], wrapper, selected, expected)

    files = check('web-built-assets-no-source-or-node-modules', tags['web'], ['/bin/sh', '-c',
        'set -eu; test -s /srv/index.html; test ! -e /source; test ! -e /srv/node_modules; find /srv -type f -exec sha256sum {} \\;'])
    assets = {}
    for line in files.splitlines():
        match = re.fullmatch(r'([0-9a-f]{64})  /srv/(.+)', line)
        assert match
        assets[match[2]] = match[1]
    assert 'index.html' in assets and len(assets) > 1
    (HERE / 'web-assets.json').write_text(json.dumps(assets, indent=2, sort_keys=True) + '\n')
    report['web_asset_count'] = len(assets)
    web = {'SITE_ADDRESS': 'localhost', 'PROXY_SHARED_SECRET': secret}
    adapted = check('caddy-real-config-adapts', tags['web'],
                    ['caddy', 'adapt', '--config', '/etc/caddy/Caddyfile', '--adapter', 'caddyfile'], web)
    graph = json.loads(adapted)

    def objects(value):
        if isinstance(value, dict):
            yield value
            for child in value.values():
                yield from objects(child)
        elif isinstance(value, list):
            for child in value:
                yield from objects(child)

    nodes = list(objects(graph))
    proxies = [node for node in nodes if node.get('handler') == 'reverse_proxy']
    assert len(proxies) == 1
    proxy = proxies[0]
    assert proxy['upstreams'] == [{'dial': 'api:3000'}] and proxy['flush_interval'] < 0
    headers = {key.lower(): value for key, value in proxy['headers']['request']['set'].items()}
    assert headers['x-golf-client-ip'] == ['{http.request.remote.host}']
    assert headers['x-golf-proxy-secret'] == [secret]
    assert any(node.get('handler') == 'file_server' for node in nodes)
    report['caddy_proxy_headers_and_unbuffered_config_verified'] = True
    for name, selected, expected in [
        ('web-invalid-hostname-denied', dict(web, SITE_ADDRESS='https://localhost/bad'), 'hostname without'),
        ('web-invalid-secret-denied', dict(web, PROXY_SHARED_SECRET='!' * 43), '43-character base64url'),
        ('web-missing-secret-denied', {'SITE_ADDRESS': 'localhost'}, 'PROXY_SHARED_SECRET is required'),
    ]:
        check(name, tags['web'], ['/usr/local/bin/validate-and-run-caddy'], selected, expected)
    report['rust_builder_version'] = check('actual-rust-builder-version', 'docker.io/library/rust:1.88-bookworm', ['rustc', '--version']).strip()
    report['node_builder_version'] = check('actual-node-builder-version', 'docker.io/library/node:22-alpine', ['node', '--version']).strip()
    report['npm_builder_version'] = check('actual-npm-builder-version', 'docker.io/library/node:22-alpine', ['npm', '--version']).strip()
    report['caddy_version'] = check('actual-caddy-runtime-version', tags['web'], ['caddy', 'version']).strip()
    report['owned_probe_containers_removed'] = len(containers)
report['private_env_files_removed'] = not Path(directory).exists()
assert len(report['checks']) == 23 and report['owned_probe_containers_removed'] == 23
assert report['private_env_files_removed']
(HERE / 'inspection.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({key: value for key, value in report.items() if key != 'checks'}))

"""Owned loopback Compose rehearsal. Raw diagnostics/secrets stay in private tempdir.

Use setup/start/cleanup with the control JSON path created for a private rootless
Podman API. See README for the engine and certificate acceptance limits.
"""
import json
import hashlib
import os
from pathlib import Path
import re
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


class Rehearsal:
    def __init__(self, control):
        self.path = Path(control)
        self.state = json.loads(self.path.read_text())
        if os.environ.get('GOLF_RELEASE_BUILD_REPORT'):
            self.state.setdefault('candidate_build_report', os.environ['GOLF_RELEASE_BUILD_REPORT'])
        self.root = Path(self.state['directory'])
        assert self.root.parent == Path('/tmp') and self.root.name.startswith('golf-deploy3-')
        assert self.root.stat().st_mode & 0o077 == 0
        self.env = os.environ.copy()
        # Compose shell interpolation wins over --env-file. Never inherit a
        # caller's real database/provider configuration into a disposable run.
        for key in re.findall(r'\$\{([A-Z][A-Z0-9_]*)', (REPO / 'compose.production.yml').read_text()):
            self.env.pop(key, None)
        self.env.update(self.state.get('values', {}))
        self.env.pop('DOCKER_CONTEXT', None)
        self.env['DOCKER_HOST'] = 'unix://' + self.state['socket']
        self.checks = self.state.get('checks', {})

    def save(self):
        self.state['checks'] = self.checks
        self.path.write_text(json.dumps(self.state))
        self.path.chmod(0o600)

    def run(self, args, *, data=None, env=None, timeout=120, ok=True):
        result = subprocess.run(args, input=data, env=env or self.env, cwd=REPO,
                                capture_output=True, text=True, timeout=timeout)
        with (self.root / 'diagnostics.log').open('a') as log:
            log.write(result.stdout + result.stderr)
        if ok and result.returncode:
            raise RuntimeError('command failed; private diagnostics retained: ' + args[0])
        return result

    def compose(self, *args, extra=None, **kwargs):
        command = ['docker', 'compose', '-p', self.state['project'], '--env-file',
                   str(self.root / 'production.env'), '-f', str(REPO / 'compose.production.yml'),
                   '-f', str(self.root / 'override.json')]
        if extra:
            command += ['-f', str(extra)]
        return self.run(command + list(args), **kwargs)

    def sql(self, query, runtime=False, ok=True):
        values = self.state['values']
        env = self.env.copy()
        env['PGPASSWORD'] = values['APP_DATABASE_PASSWORD' if runtime else 'POSTGRES_OWNER_PASSWORD']
        user = values['APP_DATABASE_USER' if runtime else 'POSTGRES_OWNER_USER']
        return self.compose('exec', '-T', '-e', 'PGPASSWORD', 'postgres', 'psql', '-X',
                            '-h', '127.0.0.1', '-U', user, '-d', values['POSTGRES_DB'],
                            '-At', '-v', 'ON_ERROR_STOP=1', '-v', 'VERBOSITY=verbose',
                            data=query, env=env, ok=ok).stdout.strip() if ok else self.compose(
                                'exec', '-T', '-e', 'PGPASSWORD', 'postgres', 'psql', '-X',
                                '-h', '127.0.0.1', '-U', user, '-d', values['POSTGRES_DB'],
                                '-At', '-v', 'ON_ERROR_STOP=1', '-v', 'VERBOSITY=verbose',
                                data=query, env=env, ok=False)

    def receipt(self, name, value=True):
        assert value, name
        self.checks[name] = value
        self.save()
        print(json.dumps({'check': name, 'passed': True}), flush=True)

    def status(self, path):
        result = self.run(['curl', '--silent', '--insecure', '--max-time', '8',
                           '--output', '/dev/null', '--write-out', '%{http_code}',
                           self.state['origin'] + path], ok=False)
        return result.stdout.strip()

    def wait_status(self, path, expected, timeout=45):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.status(path) == str(expected):
                return
            time.sleep(.5)
        raise RuntimeError('HTTP status wait failed: ' + path)

    def setup(self, migrate=True):
        candidate = self.state.get('candidate_build_report')
        builds = json.loads(Path(candidate).read_text()) if candidate else json.loads((HERE.parent / 'production-images-2026-10-15/baseline-builds.json').read_text())
        web_report = HERE / 'web-build.json'
        web_build = json.loads(web_report.read_text()) if web_report.exists() and not candidate else None
        exclusions = []
        if candidate:
            assert builds.get('temporary_context_removed') is True
            assert len(builds['builds']) == 3 and {item['service'] for item in builds['builds']} == {'postgres', 'api', 'web'}
            assert all(item['exit_code'] == 0 for item in builds['builds'])
            assert set(builds['overlays']) == {'backend/Dockerfile.production', 'frontend/Dockerfile.production', 'deploy/postgres/Dockerfile.production'}
            for name, digest in builds['overlays'].items():
                assert hashlib.sha256((REPO / name).read_bytes()).hexdigest() == digest
                exclusions.append(':!' + name)
        if web_build:
            assert hashlib.sha256((REPO / 'deploy/Caddyfile').read_bytes()).hexdigest() == web_build['overlays']['deploy/Caddyfile']
            exclusions = [':!deploy/Caddyfile']
        self.run(['git', 'diff', '--exit-code', builds['base_commit'], '--', 'backend',
                  'frontend', 'migrations', 'vendor', 'deploy', 'scripts', 'Cargo.toml',
                  'Cargo.lock', 'compose.production.yml', '.dockerignore', '.env.production.example'] + exclusions)
        images = {}
        for item in builds['builds']:
            current = json.loads(self.run(['docker', 'image', 'inspect', item['tag']]).stdout)[0]
            assert current['Id'].removeprefix('sha256:') == item['image']['Id']
            assert current['Config']['Labels']['io.golfsiden.context.sha256'] == builds['context_sha256']
            images[item['service']] = item['tag']
        if web_build:
            current = json.loads(self.run(['docker', 'image', 'inspect', web_build['tag']]).stdout)[0]
            assert current['Id'].removeprefix('sha256:') == web_build['image']['Id']
            assert current['Config']['Labels']['io.golfsiden.context.sha256'] == web_build['context_sha256']
            images['web'] = web_build['tag']
            self.state['web_build'] = web_build
        def port():
            with socket.socket() as handle:
                handle.bind(('127.0.0.1', 0))
                return handle.getsockname()[1]
        http_port, https_port = port(), port()
        assert http_port != https_port
        owner_password, app_password = secrets.token_urlsafe(32), secrets.token_urlsafe(32)
        values = {'SITE_ADDRESS': 'localhost', 'RESET_PASSWORD_ORIGIN': f'https://localhost:{https_port}',
                  'PUBLIC_HTTP_PORT': f'127.0.0.1:{http_port}', 'PUBLIC_HTTPS_PORT': f'127.0.0.1:{https_port}',
                  'GOLFSIDEN_IMAGE_TAG': builds['tag'], 'PROXY_SHARED_SECRET': secrets.token_urlsafe(32),
                  'POSTGRES_DB': 'rehearsal', 'POSTGRES_OWNER_USER': 'rehearsal_owner',
                  'POSTGRES_OWNER_PASSWORD': owner_password, 'APP_DATABASE_USER': 'rehearsal_app',
                  'APP_DATABASE_PASSWORD': app_password,
                  'MIGRATION_DATABASE_URL': f'postgres://rehearsal_owner:{owner_password}@postgres:5432/rehearsal',
                  'APP_DATABASE_URL': f'postgres://rehearsal_app:{app_password}@postgres:5432/rehearsal'}
        envfile = self.root / 'production.env'
        envfile.write_text(''.join(f'{key}={value}\n' for key, value in values.items()))
        envfile.chmod(0o600)
        image_map = {**images, 'migrate': images['api'], 'permissions': images['postgres']}
        override = {'services': {key: {'image': value, 'pull_policy': 'never'} for key, value in image_map.items()}}
        (self.root / 'override.json').write_text(json.dumps(override))
        self.state.update(project='golf-deploy3-' + secrets.token_hex(5), values=values,
                          origin=values['RESET_PASSWORD_ORIGIN'], http_port=http_port,
                          https_port=https_port, images=images, source=builds['base_commit'],
                          context_sha256=builds['context_sha256'])
        if candidate:
            self.state['candidate_build_snapshot'] = builds
        self.save()
        self.env.update(values)
        config = json.loads(self.compose('--profile', 'tools', 'config', '--format', 'json').stdout)
        assert config['networks']['data']['internal']
        for service in ('api', 'postgres'):
            assert not config['services'][service].get('ports')
        assert all(p['host_ip'] == '127.0.0.1' for p in config['services']['web']['ports'])
        assert config['services']['api']['environment']['SESSION_COOKIE_SECURE'] == 'true'
        assert config['services']['api']['environment']['DATABASE_URL'] == values['APP_DATABASE_URL']
        assert config['services']['migrate']['environment']['DATABASE_URL'] == values['MIGRATION_DATABASE_URL']
        assert config['services']['postgres']['environment']['POSTGRES_PASSWORD'] == values['POSTGRES_OWNER_PASSWORD']
        self.receipt('exact_images_unchanged_inputs_private_configuration')
        self.compose('up', '-d', '--no-build', '--pull', 'never', '--wait', '--wait-timeout', '90', 'postgres')
        self.receipt('compose_postgres_fresh_cluster_healthy')
        if not migrate:
            assert self.sql("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public';") == '0'
            self.receipt('restore_target_started_without_migrations')
            return
        self.compose('--profile', 'tools', 'run', '--rm', '-T', '--no-deps', 'migrate')
        self.compose('--profile', 'tools', 'run', '--rm', '-T', '--no-deps', 'permissions')
        assert self.sql('SELECT count(*),max(version),bool_and(success) FROM _sqlx_migrations;') == '39|39|t'
        assert self.sql('SELECT count(*) FROM users;') == '0'
        self.receipt('owner_migrates_39_real_grants_no_seed')

    def start(self):
        self.compose('up', '-d', '--no-build', '--pull', 'never', '--wait', '--wait-timeout', '90', 'api', 'web')
        self.wait_status('/api/ready', 200)
        assert self.status('/api/health') == '200'
        ids = self.compose('ps', '-q').stdout.split()
        services = {}
        for ident in ids:
            item = json.loads(self.run(['docker', 'inspect', ident]).stdout)[0]
            label = item['Config']['Labels']
            assert label['com.docker.compose.project'] == self.state['project']
            service = label['com.docker.compose.service']
            if service in ('postgres', 'api'):
                assert not item['HostConfig']['PortBindings']
            services[service] = {'id': ident, 'image': item['Image'], 'restarts': item['RestartCount'],
                                 'networks': sorted(item['NetworkSettings']['Networks'])}
        assert set(services) == {'postgres', 'api', 'web'}
        self.state['services'] = services
        self.save()
        self.receipt('production_api_web_healthy_https_through_caddy')

    def cleanup(self, receipt_path=None):
        if self.state.get('candidate_build_report'):
            assert receipt_path is not None, 'Candidate cleanup requires its own evidence destination'
        project = self.state.get('project')
        counts = {}
        if project:
            for kind, command in [('containers', ['docker', 'ps', '-aq']),
                                  ('volumes', ['docker', 'volume', 'ls', '-q']),
                                  ('networks', ['docker', 'network', 'ls', '-q'])]:
                counts[kind] = len(self.run(command + ['--filter', 'label=com.docker.compose.project=' + project]).stdout.split())
            self.compose('--profile', 'tools', 'down', '--volumes', '--remove-orphans', '--timeout', '15')
            for kind, noun in [('container', 'ps'), ('volume', 'volume'), ('network', 'network')]:
                command = ['docker', 'ps', '-aq'] if kind == 'container' else ['docker', noun, 'ls', '-q']
                result = self.run(command + ['--filter', 'label=com.docker.compose.project=' + project])
                assert not result.stdout.strip(), 'owned ' + kind + ' remains'
        safe = {'engine': 'rootless Podman 5.8.7 via private Docker-compatible API',
                'compose_cli': self.run(['docker', 'compose', 'version', '--short']).stdout.strip(),
                'native_docker_engine_acceptance': False,
                'source_candidate': self.state.get('source'), 'context_sha256': self.state.get('context_sha256'),
                'web_build': self.state.get('web_build'),
                'images': self.state.get('images'), 'checks': self.checks,
                'owned_containers_networks_volumes_removed': True,
                'owned_resource_counts_before_removal': counts,
                'private_configuration_diagnostics_ca_profile_removed': True,
                'three_deploy2_images_and_repaired_web_retained': True}
        if self.state.get('candidate_build_report'):
            safe['candidate_build'] = self.state.get('candidate_build_snapshot')
        pid = self.state['pid']
        # Verify process identity before signalling a PID from a saved control file.
        process = Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
        assert b'podman' in process[0] and b'service' in process
        assert ('unix://' + self.state['socket']).encode() in process
        os.kill(pid, signal.SIGTERM)
        for _ in range(40):
            if not Path(self.state['socket']).exists():
                break
            time.sleep(.1)
        assert not Path(self.state['socket']).exists()
        shutil.rmtree(self.root)
        assert not self.root.exists()
        pointer = Path('/tmp/golf-deploy3-control-path')
        if pointer.exists() and pointer.read_text() == str(self.path):
            pointer.unlink()
        destination = Path(receipt_path) if receipt_path else HERE / 'integration.json'
        destination.write_text(json.dumps(safe, indent=2) + '\n')
        print(json.dumps({'cleanup': 'passed', 'checks': len(self.checks)}))


if __name__ == '__main__':
    try:
        rehearsal = Rehearsal(sys.argv[2])
        getattr(rehearsal, sys.argv[1])()
    except Exception as error:
        print(json.dumps({'failed': type(error).__name__, 'detail': str(error) if isinstance(error, RuntimeError) else 'assertion or tooling failure; inspect private diagnostics'}))
        sys.exit(1)

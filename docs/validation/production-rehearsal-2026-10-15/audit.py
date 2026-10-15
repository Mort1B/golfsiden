"""Final live identity, network, proxy and log checks before owned cleanup."""
import json
import os
import sys
from urllib.parse import urlparse

from rehearsal import HERE, Rehearsal


def main(rehearsal):
    services = rehearsal.state['services']
    builds = json.loads((HERE.parent / 'production-images-2026-10-15/baseline-builds.json').read_text())
    expected = {item['service']: item['image']['Id'] for item in builds['builds']}
    if rehearsal.state.get('web_build'):
        expected['web'] = rehearsal.state['web_build']['image']['Id']
    for name, service in services.items():
        item = json.loads(rehearsal.run(['docker', 'inspect', service['id']]).stdout)[0]
        assert item['Image'].removeprefix('sha256:') == expected[name]
        assert item['State']['Running'] and item['RestartCount'] == 0
        if name in ('api', 'postgres'):
            assert item['State']['Health']['Status'] == 'healthy'
            assert not item['HostConfig']['PortBindings']
        else:
            assert all(binding['HostIp'] == '127.0.0.1' for values in item['HostConfig']['PortBindings'].values() for binding in values)
        environment = dict(value.split('=', 1) for value in item['Config']['Env'])
        if name == 'api':
            values = rehearsal.state['values']
            assert environment['DATABASE_URL'] == values['APP_DATABASE_URL']
            assert environment['APP_DATABASE_USER'] == values['APP_DATABASE_USER']
            assert environment['PROXY_SHARED_SECRET'] == values['PROXY_SHARED_SECRET']
            assert environment['APP_ENV'] == 'production'
            assert environment['RUN_MIGRATIONS'] == 'false'
            assert environment['SESSION_COOKIE_SECURE'] == 'true'
            assert not environment.get('CORS_ALLOWED_ORIGIN')
            assert rehearsal.compose('exec', '-T', 'api', 'id', '-u').stdout.strip() == '10001'
        logs = rehearsal.run(['docker', 'logs', service['id']])
        for key in ('POSTGRES_OWNER_PASSWORD', 'APP_DATABASE_PASSWORD', 'PROXY_SHARED_SECRET'):
            assert rehearsal.state['values'][key] not in logs.stdout + logs.stderr
    rehearsal.receipt('actual_container_image_ids_production_environment_uid_no_restarts_or_secret_logs')
    app = rehearsal.state['project'] + '_app'
    data = rehearsal.state['project'] + '_data'
    for name, internal in [(app, False), (data, True)]:
        network = json.loads(rehearsal.run(['docker', 'network', 'inspect', name]).stdout)[0]
        assert network['Internal'] == internal
        assert network['Labels']['com.docker.compose.project'] == rehearsal.state['project']
    assert services['postgres']['networks'] == [data]
    assert services['api']['networks'] == sorted([app, data])
    assert services['web']['networks'] == [app]
    rehearsal.receipt('actual_private_database_network_and_only_loopback_web_publications')
    assets = rehearsal.compose('exec', '-T', 'web', 'sh', '-c', 'cd /srv && find . -type f -exec sha256sum {} +').stdout
    actual_assets = {line.split('  ', 1)[1].removeprefix('./'): line.split('  ', 1)[0] for line in assets.splitlines()}
    assert actual_assets == json.loads((HERE.parent / 'production-images-2026-10-15/web-assets.json').read_text())
    adapted = json.loads(rehearsal.compose('exec', '-T', 'web', 'caddy', 'adapt', '--config', '/etc/caddy/Caddyfile', '--adapter', 'caddyfile').stdout)
    encoder = adapted['logging']['logs']['default']['encoder']
    assert encoder['format'] == 'filter'
    assert encoder['fields']['request']['filter'] == 'delete'
    assert encoder['fields']['resp_headers']['filter'] == 'delete'
    rehearsal.receipt('rebuilt_web_65_assets_byte_identical_and_actual_default_logger_filter_adapted')
    sentinels = {'MIGRATION_DATABASE_URL': 'postgres://invalid.invalid/forbidden',
                 'APP_DATABASE_URL': 'postgres://invalid.invalid/forbidden',
                 'GOLF_COURSE_API_KEY': 'private-host-sentinel', 'PUBLIC_HTTPS_PORT': '0.0.0.0:443'}
    old = {key: os.environ.get(key) for key in sentinels}
    try:
        os.environ.update(sentinels)
        isolated = Rehearsal(rehearsal.path)
        config = json.loads(isolated.compose('--profile', 'tools', 'config', '--format', 'json').stdout)
        assert config['services']['migrate']['environment']['DATABASE_URL'] == rehearsal.state['values']['MIGRATION_DATABASE_URL']
        assert config['services']['api']['environment']['DATABASE_URL'] == rehearsal.state['values']['APP_DATABASE_URL']
        assert config['services']['api']['environment']['GOLF_COURSE_API_KEY'] == ''
        assert all(port['host_ip'] == '127.0.0.1' for port in config['services']['web']['ports'])
    finally:
        for key, value in old.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
    rehearsal.receipt('inherited_database_provider_port_sentinels_cannot_escape_owned_configuration')
    headers = rehearsal.run(['curl', '--silent', '--max-time', '8', '--dump-header', '-',
                             '--output', '/dev/null', f"http://localhost:{rehearsal.state['http_port']}/login"]).stdout
    assert headers.startswith('HTTP/1.1 308')
    location = next(line.split(': ', 1)[1].strip() for line in headers.splitlines() if line.lower().startswith('location: '))
    target = urlparse(location)
    assert target.scheme == 'https' and target.hostname == 'localhost' and target.path == '/login'
    # Container 443 is mapped to a random host port; native Caddy's redirect uses
    # standard 443. We use explicit HTTPS origin above and do not claim following
    # this redirect works with the nonstandard rehearsal mapping.
    assert target.port is None or target.port == 443
    rehearsal.receipt('http_redirect_https_scheme_host_path_only_standard443')
    rehearsal.wait_status('/api/ready', 200)
    assert rehearsal.status('/api/health') == '200'
    rehearsal.receipt('final_live_health_ready200')


if __name__ == '__main__':
    rehearsal = Rehearsal(sys.argv[1])
    try:
        main(rehearsal)
    except Exception as error:
        import traceback
        with (rehearsal.root / 'failure.log').open('a') as log:
            traceback.print_exc(file=log)
        print(json.dumps({'failed': type(error).__name__, 'detail': 'final live audit; private diagnostics only'}))
        sys.exit(1)

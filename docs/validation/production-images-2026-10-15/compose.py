"""Render the real Compose contract privately; never start deployed services."""
from pathlib import Path
import hashlib
import json
import os
import re
import secrets
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
source = (REPO / 'compose.production.yml').read_text()
keys = set(re.findall(r'\$\{([A-Z_]+)', source))
required = sorted(set(re.findall(r'\$\{([A-Z_]+):\?', source)))
environment = {key: value for key, value in os.environ.items() if key not in keys}
owner_password, app_password = secrets.token_urlsafe(32), secrets.token_urlsafe(32)
values = {
    'SITE_ADDRESS': 'localhost', 'RESET_PASSWORD_ORIGIN': 'https://localhost',
    'GOLFSIDEN_IMAGE_TAG': 'deploy2-contract', 'PROXY_SHARED_SECRET': secrets.token_urlsafe(32),
    'POSTGRES_DB': 'golf_acceptance', 'POSTGRES_OWNER_USER': 'golf_owner',
    'APP_DATABASE_USER': 'golf_runtime', 'POSTGRES_OWNER_PASSWORD': owner_password,
    'APP_DATABASE_PASSWORD': app_password,
    'MIGRATION_DATABASE_URL': f'postgres://golf_owner:{owner_password}@postgres:5432/golf_acceptance',
    'APP_DATABASE_URL': f'postgres://golf_runtime:{app_password}@postgres:5432/golf_acceptance',
}
report = {'compose_sha256': hashlib.sha256(source.encode()).hexdigest(), 'checks': [],
          'services_started': False, 'rendered_secrets_published': False}
with tempfile.TemporaryDirectory(prefix='golf-deploy2-compose-') as directory:
    envfile = Path(directory) / 'synthetic.env'

    def render(selected, quiet=False):
        envfile.write_text(''.join(key + '=' + value + '\n' for key, value in selected.items()))
        envfile.chmod(0o600)
        command = ['docker', 'compose', '--env-file', str(envfile), '-f',
                   str(REPO / 'compose.production.yml'), '--profile', 'tools', 'config']
        command += ['--quiet'] if quiet else ['--format', 'json']
        return subprocess.run(command, env=environment, capture_output=True, text=True, timeout=60)

    rendered = render(values)
    assert rendered.returncode == 0, 'Compose configuration must render'
    graph = json.loads(rendered.stdout)
    services = graph['services']
    assert sorted(services) == ['api', 'migrate', 'permissions', 'postgres', 'web']
    assert graph['networks']['data']['internal'] is True
    assert not services['postgres'].get('ports') and not services['api'].get('ports')
    ports = services['web']['ports']
    assert sorted((int(p['target']), str(p['published']), p['protocol']) for p in ports) == [
        (80, '80', 'tcp'), (443, '443', 'tcp'), (443, '443', 'udp')]
    api = services['api']['environment']
    assert (api['APP_ENV'], api['RUN_MIGRATIONS'], api['SESSION_COOKIE_SECURE']) == ('production', 'false', 'true')
    assert api['APP_DATABASE_USER'] == values['APP_DATABASE_USER']
    assert api['DATABASE_URL'] == values['APP_DATABASE_URL']
    assert services['migrate']['environment']['DATABASE_URL'] == values['MIGRATION_DATABASE_URL']
    assert values['POSTGRES_OWNER_USER'] != values['APP_DATABASE_USER']
    assert values['POSTGRES_OWNER_PASSWORD'] != values['APP_DATABASE_PASSWORD']
    assert services['web']['environment']['PROXY_SHARED_SECRET'] == api['PROXY_SHARED_SECRET']
    assert all(services[name]['profiles'] == ['tools'] for name in ('migrate', 'permissions'))
    assert services['api']['healthcheck']['test'][-1] == 'http://127.0.0.1:3000/api/ready'
    assert set(services['postgres']['networks']) == {'data'}
    assert set(services['api']['networks']) == {'app', 'data'}
    assert set(services['web']['networks']) == {'app'}
    report['checks'].append({'case': 'positive-production-graph', 'passed': True})
    for name in required:
        selected = dict(values)
        selected.pop(name)
        result = render(selected, quiet=True)
        assert result.returncode != 0 and name in result.stderr, name
        report['checks'].append({'case': 'missing-' + name, 'passed': True})
report['private_env_removed'] = not Path(directory).exists()
report['compose_version'] = subprocess.check_output(['docker', 'compose', 'version', '--short'], text=True).strip()
assert len(report['checks']) == 1 + len(required) and report['private_env_removed']
(HERE / 'compose.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))

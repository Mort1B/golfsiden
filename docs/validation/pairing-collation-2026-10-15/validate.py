"""Reproduce A1v and validate its test-only repair on two owned PostgreSQL images."""
import argparse
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CASE = ('pairing_team_sessions::contracts::'
        'legacy_individual_groups_keep_identity_schedule_and_distinct_sorting')
IMAGES = [('alpine', 'docker.io/library/postgres:17-alpine'),
          ('debian', 'docker.io/library/postgres:17')]
FOCUSED = ['cargo', 'test', '--locked', '-p', 'golf-api', '--features',
           'database-tests', '--test', 'pairing_team_sessions']


def capture(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL,
                                   timeout=60).strip()


def check(name, command, environment, phase, platform):
    print(f'START {phase}/{platform}/{name}', flush=True)
    started = time.monotonic()
    with tempfile.NamedTemporaryFile(mode='w', prefix=f'golf-a1v-{name}-',
                                     suffix='.log', delete=False) as log:
        completed = subprocess.run(command, cwd=ROOT, env=environment,
                                   stdout=log, stderr=subprocess.STDOUT, timeout=2400)
        logfile = Path(log.name)
    content = logfile.read_text()
    totals = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)
    item = {'exit': completed.returncode, 'seconds': round(time.monotonic() - started, 2)}
    if totals:
        item.update(zip(['passed', 'failed', 'ignored'],
                        [sum(int(row[n]) for row in totals) for n in range(3)]))
    item['failed_tests'] = re.findall(r'^test ([\w:]+) \.\.\. FAILED$', content, re.M)
    if phase == 'baseline' and platform == 'debian':
        item['original_order_assertion_reproduced'] = (
            completed.returncode == 101 and item['failed_tests'] == [CASE]
            and [item.get(key) for key in ('passed', 'failed', 'ignored')] == [0, 1, 0]
            and 'left: String("d1000000-0000-0000-0000-000000000001")' in content
            and 'right: "d1000000-0000-0000-0000-000000000002"' in content)
        accepted = item['original_order_assertion_reproduced']
    else:
        accepted = completed.returncode == 0
        if name in ('focused', 'pairing_suite'):
            expected_count = 1 if name == 'focused' else 14
            accepted = accepted and [item.get(key) for key in ('passed', 'failed', 'ignored')] == [expected_count, 0, 0]
    item['accepted'] = accepted
    # Keep raw local diagnostics private; tracked evidence contains only synthetic summaries.
    print(f'END {phase}/{platform}/{name} ' + json.dumps(item), flush=True)
    return item


def fixture(phase, platform, image):
    result = {'image': image, 'image_id': capture(['podman', 'image', 'inspect', image,
                                                '--format', '{{.Id}}'])}
    with tempfile.TemporaryDirectory(prefix='golf-a1v-') as temp:
        temporary = Path(temp)
        cidfile = temporary / 'container.cid'
        pg_env = temporary / 'pg.env'
        password = secrets.token_urlsafe(24)
        pg_env.write_text('POSTGRES_USER=golf\nPOSTGRES_DB=golf\nPOSTGRES_PASSWORD=' + password + '\n')
        pg_env.chmod(0o600)
        try:
            capture(['podman', 'create', '--cidfile', str(cidfile), '--name', temporary.name,
                     '--env-file', str(pg_env), '--publish', '127.0.0.1::5432',
                     '--tmpfs', '/var/lib/postgresql/data:rw', image])
            cid = cidfile.read_text().strip()
            capture(['podman', 'start', cid])
            deadline = time.monotonic() + 45
            while subprocess.run(['podman', 'exec', cid, 'pg_isready', '-h', '127.0.0.1', '-U', 'golf'],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                if time.monotonic() > deadline:
                    raise RuntimeError('Owned PostgreSQL startup timed out')
                time.sleep(.2)
            address = capture(['podman', 'port', cid, '5432/tcp'])
            environment = dict(os.environ, DATABASE_URL=f'postgres://golf:{password}@{address}/golf')
            query = """SELECT json_build_object(
              'version',version(),'collation',datcollate,'ctype',datctype,'provider',datlocprovider,
              'group_native',(SELECT json_agg(v ORDER BY v) FROM (VALUES ('b-group'),('B-group')) x(v)),
              'player_native',(SELECT json_agg(v ORDER BY v) FROM (VALUES ('b-player'),('B-player')) x(v)),
              'group_c',(SELECT json_agg(v ORDER BY v COLLATE "C") FROM (VALUES ('b-group'),('B-group')) x(v)))
              FROM pg_database WHERE datname=current_database()"""
            result['database'] = json.loads(capture(['podman', 'exec', cid, 'psql', '-X', '-U',
                                                     'golf', '-d', 'golf', '-Atc', query]))
            commands = [('focused', FOCUSED + [CASE, '--', '--exact', '--test-threads=1'])]
            if phase == 'candidate':
                commands += [('pairing_suite', FOCUSED + ['--', '--test-threads=2'])]
                if platform == 'alpine':
                    commands += [
                        ('format', ['cargo', 'fmt', '--all', '--', '--check']),
                        ('rust', ['cargo', 'test', '--locked', '--workspace', '--all-targets']),
                        ('clippy', ['cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings']),
                        ('postgres', ['cargo', 'test', '--locked', '--workspace', '--all-targets', '--features', 'database-tests', '--no-fail-fast', '--', '--test-threads=2']),
                        ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
                        ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
                    ]
            result['checks'] = {}
            for name, command in commands:
                item = check(name, command, environment, phase, platform)
                result['checks'][name] = item
                if not item['accepted']:
                    break
            if 'seed' in result['checks'] and result['checks']['seed']['accepted']:
                result['migrations'] = int(capture(['podman', 'exec', cid, 'psql', '-X', '-U',
                                                   'golf', '-d', 'golf', '-Atc',
                                                   'SELECT count(*) FROM _sqlx_migrations WHERE success']))
        finally:
            if cidfile.exists():
                cid = cidfile.read_text().strip()
                capture(['podman', 'rm', '-f', cid])
                result['owned_container_removed'] = not capture(['podman', 'ps', '-a', '--filter',
                                                                f'id={cid}', '--format', '{{.ID}}'])
            (HERE / f'{phase}-{platform}.json').write_text(json.dumps(result, indent=2) + '\n')
    return (result.get('owned_container_removed') is True
            and all(item['accepted'] for item in result['checks'].values()))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['baseline', 'candidate'])
    args = parser.parse_args()
    accepted = True
    for platform, image in IMAGES:
        accepted = fixture(args.phase, platform, image) and accepted
    if not accepted:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

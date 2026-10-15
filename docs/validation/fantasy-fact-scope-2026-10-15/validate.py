"""Run A1u affected ladders on one exclusively owned disposable database."""
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


def capture(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL, timeout=60).strip()


def main():
    result = {}
    with tempfile.TemporaryDirectory(prefix='golf-a1u-validation-') as temp:
        temporary = Path(temp)
        cidfile = temporary / 'container.cid'
        pg_env = temporary / 'pg.env'
        password = secrets.token_urlsafe(24)
        pg_env.write_text('POSTGRES_USER=golf\nPOSTGRES_DB=golf\nPOSTGRES_PASSWORD=' + password + '\n')
        pg_env.chmod(0o600)
        try:
            capture(['podman', 'create', '--cidfile', str(cidfile), '--name', temporary.name,
                     '--env-file', str(pg_env), '--publish', '127.0.0.1::5432',
                     '--tmpfs', '/var/lib/postgresql/data:rw', 'docker.io/library/postgres:17-alpine'])
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
            commands = [
                ('focused', ['cargo', 'test', '--locked', '-p', 'golf-api', '--features', 'database-tests',
                             '--test', 'fantasy_fact_scope', '--test', 'fantasy_results', '--test', 'fantasy_result_formats',
                             '--test', 'fantasy_result_matches', '--test', 'fantasy_result_teams',
                             '--test', 'fantasy_result_contract', '--test', 'fantasy_sources', '--', '--test-threads=2']),
                ('format', ['cargo', 'fmt', '--all', '--', '--check']),
                ('rust', ['cargo', 'test', '--locked', '--workspace', '--all-targets']),
                ('clippy', ['cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings']),
                ('postgres', ['cargo', 'test', '--locked', '--workspace', '--all-targets', '--features', 'database-tests', '--no-fail-fast', '--', '--test-threads=2']),
                ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
                ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
                ('build', ['cargo', 'build', '--locked', '-p', 'golf-api', '--bins']),
            ]
            for name, command in commands:
                print('START ' + name, flush=True)
                started = time.monotonic()
                logfile = Path('/tmp') / ('golf-a1u-' + name + '.log')
                with logfile.open('w') as log:
                    completed = subprocess.run(command, cwd=ROOT, env=environment, stdout=log,
                                               stderr=subprocess.STDOUT, timeout=2400)
                content = logfile.read_text()
                totals = re.findall(r'test result: (?:ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;', content)
                item = {'exit': completed.returncode, 'seconds': round(time.monotonic() - started, 2)}
                if totals:
                    item.update(zip(['passed', 'failed', 'ignored'], [sum(int(row[n]) for row in totals) for n in range(3)]))
                item['failed_tests'] = re.findall(r'^test ([\w:]+) \.\.\. FAILED$', content, re.M)
                result[name] = item
                print('END ' + name + ' ' + json.dumps(item), flush=True)
                if name == 'focused' and completed.returncode:
                    raise RuntimeError('Focused checks failed; inspect local log')
            result['migrations'] = int(capture(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-d', 'golf', '-Atc', 'SELECT count(*) FROM _sqlx_migrations WHERE success']))
        finally:
            if cidfile.exists():
                cid = cidfile.read_text().strip()
                capture(['podman', 'rm', '-f', cid])
                result['owned_container_removed'] = not capture(['podman', 'ps', '-a', '--filter', f'id={cid}', '--format', '{{.ID}}'])
            (HERE / 'validation.json').write_text(json.dumps(result, indent=2) + '\n')
            print('Owned validation fixture removed', flush=True)
    if any(isinstance(value, dict) and value.get('exit') for value in result.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()

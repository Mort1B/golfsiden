"""A1w affected Rust/PostgreSQL ladders on one exclusively owned disposable DB."""
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
from fixtures import Database, capture

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    result = {'checks': {}}
    with tempfile.TemporaryDirectory(prefix='golf-a1w-validation-') as temp:
        root = Path(temp)
        database = Database(root, 'ladder', secrets.token_urlsafe(24))
        result['fixture'] = database.evidence
        try:
            with database:
                environment = dict(os.environ, DATABASE_URL=f'postgres://golf:{database.password}@{database.address}/golf')
                commands = [
                    ('cancellation', ['cargo', 'test', '--locked', '-p', 'golf-api', '--features', 'database-tests', '--test', 'transaction_cancellation', '--', '--test-threads=2']),
                    ('format', ['cargo', 'fmt', '--all', '--', '--check']),
                    ('rust', ['cargo', 'test', '--locked', '--workspace', '--all-targets']),
                    ('clippy', ['cargo', 'clippy', '--locked', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings']),
                    ('postgres', ['cargo', 'test', '--locked', '--workspace', '--all-targets', '--features', 'database-tests', '--no-fail-fast', '--', '--test-threads=2']),
                    ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
                    ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
                    ('release', ['cargo', 'build', '--locked', '--release', '-p', 'golf-api', '--bins']),
                ]
                for name, command in commands:
                    print('START ' + name, flush=True)
                    started = time.monotonic()
                    with tempfile.NamedTemporaryFile(mode='w', prefix=f'golf-a1w-{name}-', suffix='.log', delete=False) as log:
                        completed = subprocess.run(command, cwd=ROOT, env=environment, stdout=log,
                                                   stderr=subprocess.STDOUT, timeout=2400)
                        logfile = Path(log.name)
                    content = logfile.read_text()
                    totals = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)
                    item = {'exit': completed.returncode, 'seconds': round(time.monotonic() - started, 2)}
                    if totals:
                        item.update(zip(['passed', 'failed', 'ignored'],
                                        [sum(int(row[n]) for row in totals) for n in range(3)]))
                    item['failed_tests'] = re.findall(r'^test ([\w:]+) \.\.\. FAILED$', content, re.M)
                    item['accepted'] = completed.returncode == 0
                    if name == 'cancellation':
                        item['accepted'] &= [item.get(key) for key in ('passed', 'failed', 'ignored')] == [5, 0, 0]
                    elif name in ('rust', 'postgres'):
                        item['accepted'] &= item.get('passed', 0) > 0 and item.get('failed') == 0
                    result['checks'][name] = item
                    print('END ' + name + ' ' + json.dumps(item), flush=True)
                    if not item['accepted']:
                        raise RuntimeError('Affected ladder failed; inspect private local diagnostic log')
                result['migrations'] = int(capture(['podman', 'exec', database.cid, 'psql', '-X', '-U',
                                                   'golf', '-d', 'golf', '-Atc',
                                                   'SELECT count(*) FROM _sqlx_migrations WHERE success']))
        finally:
            (HERE / 'validation.json').write_text(json.dumps(result, indent=2) + '\n')
    result['temporary_credentials_removed'] = not root.exists()
    assert result['fixture']['removed'] and result['temporary_credentials_removed']
    (HERE / 'validation.json').write_text(json.dumps(result, indent=2) + '\n')
    print('PASS ladders and owned fixture/credential cleanup', flush=True)


if __name__ == '__main__':
    main()

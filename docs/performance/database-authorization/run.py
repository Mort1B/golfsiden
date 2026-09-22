"""Run only on an exclusive disposable server: pg_stat_statements_reset is global."""
import os
from pathlib import Path
import subprocess
import sys

if os.environ.get('GOLF_AUTH_MEASURE') != '1' or not os.environ.get('DATABASE_URL'):
    raise SystemExit('Set the explicit measurement opt-in and disposable database connection.')
output = Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=False)
for run in range(1, 4):
    target = output / f'run-{run}'
    target.mkdir()
    env = dict(os.environ, GOLF_AUTH_MEASURE_OUT=str(target), CARGO_BUILD_JOBS='2')
    with (output / f'run-{run}.log').open('w') as log:
        subprocess.run(['cargo', 'test', '--release', '-p', 'golf-api', '--features', 'database-tests',
                        '--test', 'match_authorization_measurement', '--', '--ignored', '--test-threads=1'],
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    print(f'Release measurement run {run}/3 passed', flush=True)

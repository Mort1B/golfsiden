"""A1z: bounded unchanged-case diagnosis on an exclusively owned PostgreSQL."""
from pathlib import Path
import hashlib
import json
import os
import re
import secrets
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path.insert(0, str(HERE.parent / 'sqlx-verify-ca-2026-10-15'))
from fixtures import Database

TARGET = 'lifecycle_expiry::scenarios::match_late_expiry_rolls_back_all_actions'
CARGO = ['cargo', 'test', '--locked', '-p', 'golf-api', '--features',
         'database-tests', '--test', 'lifecycle_expiry']
PHASE = sys.argv[1]
if PHASE == 'baseline':
    commands = [('default-module-1', CARGO + ['--', '--nocapture']),
                ('default-module-2', CARGO + ['--', '--nocapture']),
                ('default-exact-target', CARGO + [TARGET, '--', '--exact', '--nocapture']),
                ('two-thread-module', CARGO + ['--', '--nocapture', '--test-threads=2'])]
else:
    raise SystemExit('Unknown phase')

report = {
    'phase': PHASE,
    'base_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
    'source_sha256': {name: hashlib.sha256((REPO / name).read_bytes()).hexdigest() for name in
                      ('backend/tests/lifecycle_expiry/scenarios.rs',
                       'backend/tests/lifecycle_expiry/support.rs',
                       'backend/tests/lifecycle_expiry/state.sql')},
    'scheduling': {'cpu_count': os.cpu_count(), 'affinity_count': len(os.sched_getaffinity(0)),
                   'inherited_thread_setting_removed': 'RUST_TEST_THREADS' in os.environ,
                   'default': 'libtest default; RUST_TEST_THREADS unset'},
    'runs': [],
}
try:
    with tempfile.TemporaryDirectory(prefix='golf-a1z-') as directory:
        with Database(directory, 'lifecycle', secrets.token_hex(24)) as database:
            report['database'] = database.evidence
            env = dict(os.environ)
            env.pop('RUST_TEST_THREADS', None)
            env['DATABASE_URL'] = f'postgres://golf:{database.password}@127.0.0.1:{database.port}/golf'
            for name, command in commands:
                print('START ' + name, flush=True)
                started = time.monotonic()
                load = os.getloadavg()
                with tempfile.TemporaryFile(mode='w+', encoding='utf-8') as log:
                    result = subprocess.run(command, cwd=REPO, env=env, stdout=log,
                                            stderr=subprocess.STDOUT, timeout=2400)
                    log.seek(0)
                    output = log.read()
                counts = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', output)
                # Never retain panic payloads, SQL bind values, session identities or raw JSON.
                panic_locations = re.findall(r"panicked at ([a-zA-Z0-9_./-]+:\d+:\d+)", output)
                early_codes = re.findall(r'returned before wait: \{.*?"code"\s*:\s*"([a-z_]+)"', output)
                item = {
                    'name': name, 'command': command, 'exit_code': result.returncode,
                    'elapsed_seconds': round(time.monotonic() - started, 3),
                    'load_before': load, 'load_after': os.getloadavg(),
                    'totals': {label: sum(int(c[i]) for c in counts)
                               for i, label in enumerate(('passed', 'failed', 'ignored'))},
                    'failed_tests': sorted(set(re.findall(r'^test ([\w:]+) \.\.\. FAILED$', output, re.M))),
                    'target_outcomes': re.findall(r'^test ' + re.escape(TARGET) + r' \.\.\. (\w+)$', output, re.M),
                    'panic_locations': panic_locations,
                    'early_response_codes': early_codes,
                    'controlled_wait_timeout': 'lifecycle request must reach its controlled wait: Elapsed' in output,
                    'rollback_assertion_failed': 'all lifecycle effects must roll back' in output,
                    'late_snapshot_precondition_failed': any(message in output for message in
                        ('expected player snapshots before late wait', 'expected team snapshots before late wait',
                         'expected tournament handicap lock before late wait')),
                }
                report['runs'].append(item)
                (HERE / (PHASE + '.json')).write_text(json.dumps(report, indent=2) + '\n')
                print(json.dumps(item), flush=True)
        report['database_removed'] = database.evidence['removed']
    report['private_fixture_files_removed'] = not Path(directory).exists()
finally:
    (HERE / (PHASE + '.json')).write_text(json.dumps(report, indent=2) + '\n')

if len(report['runs']) != len(commands):
    raise SystemExit('Incomplete evidence')
for run in report['runs']:
    expected = 1 if run['name'] == 'default-exact-target' else 6
    if run['exit_code'] == 0:
        assert run['totals'] == dict(passed=expected, failed=0, ignored=0)
        assert run['target_outcomes'] == ['ok']
        assert not run['failed_tests'] and not run['panic_locations']
assert report['database_removed'] and report['private_fixture_files_removed']
if any(run['exit_code'] for run in report['runs']):
    raise SystemExit('Failure retained for bounded diagnosis; no passing baseline claimed')

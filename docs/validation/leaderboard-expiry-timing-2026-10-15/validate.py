"""A1y owned-PostgreSQL runs; publish only sanitized test and timing evidence."""
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
from evidence import check_report

PHASE = sys.argv[1]
TARGET = 'leaderboard_sessions::expiry::expiry_during_score_loading_denies_all_boards'
CARGO = ['cargo', 'test', '--locked', '-p', 'golf-api', '--features',
         'database-tests', '--test', 'leaderboard_sessions']
MODULE = CARGO + ['--', '--nocapture']
if PHASE == 'baseline':
    COMMANDS = [(f'default-{i}', MODULE) for i in range(1, 4)]
    COMMANDS += [('two-thread-control', MODULE + ['--test-threads=2'])]
elif PHASE in ('controlled-original', 'controlled-before', 'controlled-after'):
    COMMANDS = [('controlled-setup-delay', CARGO + [TARGET, '--', '--exact', '--nocapture'])]
elif PHASE == 'repaired':
    COMMANDS = [(f'default-{i}', MODULE) for i in range(1, 4)]
    COMMANDS += [('two-thread-control', MODULE + ['--test-threads=2'])]
elif PHASE == 'ladder':
    COMMANDS = [
        ('format', ['cargo', 'fmt', '--all', '--', '--check']),
        ('rust-tests', ['cargo', 'test', '--locked', '--workspace', '--all-targets']),
        ('clippy', ['cargo', 'clippy', '--locked', '--workspace', '--all-targets',
                    '--all-features', '--', '-D', 'warnings']),
        ('database-default', ['cargo', 'test', '--locked', '--workspace', '--all-targets',
                              '--features', 'database-tests']),
        ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
        ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
    ]
elif PHASE == 'continuation':
    COMMANDS = [
        ('database-two-thread', ['cargo', 'test', '--locked', '--workspace', '--all-targets',
                                 '--features', 'database-tests', '--no-fail-fast', '--', '--test-threads=2']),
        ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
        ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
    ]
else:
    raise SystemExit('Unknown evidence phase')

source = REPO / 'backend/tests/leaderboard_sessions/expiry.rs'
report = {
    'phase': PHASE,
    'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
    'base_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
    'scheduling': {'cpu_count': os.cpu_count(), 'affinity_count': len(os.sched_getaffinity(0)),
                   'inherited_thread_setting_removed': 'RUST_TEST_THREADS' in os.environ,
                   'default_thread_setting': 'libtest default; RUST_TEST_THREADS unset'},
    'runs': [],
}

try:
    with tempfile.TemporaryDirectory(prefix='golf-a1y-') as directory:
        with Database(directory, 'timing', secrets.token_hex(24)) as database:
            report['database'] = database.evidence
            env = dict(os.environ)
            env.pop('RUST_TEST_THREADS', None)
            env['DATABASE_URL'] = f'postgres://golf:{database.password}@127.0.0.1:{database.port}/golf'
            for name, command in COMMANDS:
                started = time.monotonic()
                load_before = os.getloadavg()
                # Raw output is private and deleted with the fixture credentials.
                with tempfile.TemporaryFile(mode='w+', encoding='utf-8') as log:
                    result = subprocess.run(command, cwd=REPO, env=env, stdout=log,
                                            stderr=subprocess.STDOUT, timeout=2400)
                    log.seek(0)
                    output = log.read()
                counts = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', output)
                timing = [line.strip() for line in output.splitlines() if line.startswith('A1Y ')]
                # These diagnostics contain only ordinals, status codes and relative times.
                if any(not re.fullmatch(r'A1Y [a-zA-Z0-9_ =.\-]+', line) for line in timing):
                    raise RuntimeError('Unexpected diagnostic contents; raw output remains private')
                run = {
                    'name': name, 'command': command, 'exit_code': result.returncode,
                    'elapsed_seconds': round(time.monotonic() - started, 3),
                    'load_before': load_before, 'load_after': os.getloadavg(),
                    'totals': {label: sum(int(c[i]) for c in counts)
                               for i, label in enumerate(('passed', 'failed', 'ignored'))},
                    'failed_tests': sorted(set(re.findall(r'^test ([\w:]+) \.\.\. FAILED$', output, re.M))),
                    'target_outcomes': re.findall(r'^test ' + re.escape(TARGET) + r' \.\.\. (\w+)$', output, re.M),
                    'diagnostics': timing,
                    'controlled_wait_timeout': 'read must reach the controlled database wait: Elapsed' in output,
                    'active_gate_assertion_failed': 'scores wait must be observed before expiry' in output,
                    'early_response': 'request completed before the scores wait' in output,
                }
                report['runs'].append(run)
                (HERE / f'{PHASE}.json').write_text(json.dumps(report, indent=2) + '\n')
                print(json.dumps({'phase': PHASE, 'run': name, 'exit_code': result.returncode,
                                  'elapsed_seconds': run['elapsed_seconds'], 'totals': run['totals'],
                                  'failed_tests': run['failed_tests'], 'diagnostics': timing}), flush=True)
                if result.returncode and PHASE not in ('controlled-original', 'controlled-before'):
                    break
    report['private_fixture_files_removed'] = not Path(directory).exists()
finally:
    (HERE / f'{PHASE}.json').write_text(json.dumps(report, indent=2) + '\n')

if PHASE in ('controlled-original', 'controlled-before'):
    run = report['runs'][0]
    expected_observation = run['controlled_wait_timeout'] if PHASE == 'controlled-original' else run['early_response']
    if run['exit_code'] != 101 or run['failed_tests'] != [TARGET] or not expected_observation:
        raise SystemExit('Expected controlled fixture failure was not observed')
elif any(run['exit_code'] for run in report['runs']):
    raise SystemExit(1)
check_report(report)

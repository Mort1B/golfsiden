"""Fail closed on incomplete A1y receipts; also audit retained receipts offline."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
TARGET = 'leaderboard_sessions::expiry::expiry_during_score_loading_denies_all_boards'
PHASES = ('baseline', 'controlled-original', 'controlled-before', 'controlled-after',
          'repaired', 'ladder', 'continuation')
SEPARATE_FAILURE = 'lifecycle_expiry::scenarios::match_late_expiry_rolls_back_all_actions'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def totals(run, passed, failed, ignored):
    require(run['totals'] == dict(passed=passed, failed=failed, ignored=ignored),
            'Unexpected exact test totals: ' + run['name'])


def timings(run):
    return [dict(field.split('=', 1) for field in line.split()[1:])
            for line in run['diagnostics']]


def four_boards(run):
    rows = timings(run)
    require(len(rows) == 16, 'Need four complete board timing sequences')
    for board in range(4):
        sequence = rows[board * 4:board * 4 + 4]
        require(all(row['board'] == str(board) for row in sequence), 'Unexpected board order')
        require([row['event'] for row in sequence] == ['armed', 'gate', 'release', 'response'],
                'Unexpected gate/expiry/response order')
        require(float(sequence[0]['remaining_ms']) > 0, 'Armed session must be active')
        require(float(sequence[1]['remaining_ms']) > 0, 'Scores gate must precede expiry')
        require(float(sequence[1]['query_wait_ms']) >= 0, 'Query age must be nonnegative')
        require(float(sequence[2]['remaining_ms']) <= 0, 'Gate release must follow expiry')
        require(sequence[3]['status'] == '401', 'Expired read must deny payload')


def check_report(report):
    phase = report['phase']
    require(phase in PHASES, 'Unexpected evidence phase')
    require(report['database']['removed'], 'Owned database cleanup unconfirmed')
    require(report.get('private_fixture_files_removed'), 'Private fixture cleanup unconfirmed')
    require(report['scheduling']['default_thread_setting'] ==
            'libtest default; RUST_TEST_THREADS unset', 'Unexpected default scheduling')
    runs = report['runs']
    blocked_default = (phase == 'ladder' and len(runs) == 4 and
                       runs[-1]['name'] == 'database-default' and runs[-1]['exit_code'] == 101)
    if phase in ('baseline', 'repaired'):
        require([run['name'] for run in runs] ==
                ['default-1', 'default-2', 'default-3', 'two-thread-control'], 'Incomplete repetitions')
    elif phase.startswith('controlled-'):
        require(len(runs) == 1 and runs[0]['name'] == 'controlled-setup-delay', 'Incomplete control')
    elif phase == 'continuation':
        require([run['name'] for run in runs] == ['database-two-thread', 'migrate', 'seed'],
                'Incomplete continuation ladder')
    else:
        require([run['name'] for run in runs] ==
                ['format', 'rust-tests', 'clippy', 'database-default'] +
                ([] if blocked_default else ['migrate', 'seed']),
                'Incomplete affected validation ladder')
    negative = phase in ('controlled-original', 'controlled-before')
    for run in runs:
        if blocked_default and run['name'] == 'database-default':
            require(run['failed_tests'] == [SEPARATE_FAILURE], 'Unexpected default-ladder failure')
            totals(run, 477, 1, 1)
            require(run['target_outcomes'] == ['ok'], 'Target did not pass in the partial default ladder')
            require(not any(arg.startswith('--test-threads') for arg in run['command']),
                    'Recorded failed ladder must retain default concurrency')
            continue
        require(run['exit_code'] == (101 if negative else 0), 'Unexpected command outcome')
        require(run['failed_tests'] == ([TARGET] if negative else []), 'Unexpected failed tests')
        if negative:
            totals(run, 0, 1, 0)
            require(run['target_outcomes'] == ['FAILED'], 'Wrong negative test executed')
        elif phase in ('baseline', 'repaired', 'controlled-after'):
            totals(run, 1 if phase == 'controlled-after' else 10, 0, 0)
            require(run['target_outcomes'] == ['ok'], 'Target test was not executed')
            require(('--test-threads=2' in run['command']) ==
                    (run['name'] == 'two-thread-control'), 'Unexpected thread control')
            if phase != 'baseline':
                four_boards(run)
        elif run['name'] == 'rust-tests':
            totals(run, 264, 0, 1)
        elif run['name'] == 'database-default':
            totals(run, 953, 0, 5)
            require(run['target_outcomes'] == ['ok'], 'Full ladder omitted the target test')
            require(not any(arg.startswith('--test-threads') for arg in run['command']),
                    'Full database ladder must retain default concurrency')
        elif run['name'] == 'database-two-thread':
            totals(run, 953, 0, 5)
            require(run['target_outcomes'] == ['ok'], 'Control ladder omitted the target test')
            require('--test-threads=2' in run['command'], 'Missing explicit two-thread control')
        if negative and phase == 'controlled-original':
            require(run['controlled_wait_timeout'], 'Original wait timeout did not reproduce')
        elif negative:
            require(run['early_response'], 'Early-response setup failure did not reproduce')
            rows = timings(run)
            require(len(rows) == 2 and [row['event'] for row in rows] == ['armed', 'early_response'],
                    'Expected pre-gate expiry ordering was not recorded')
            require(all(row['board'] == '0' and float(row['remaining_ms']) < 0 for row in rows),
                    'Expected expired pre-gate session')
            require(rows[1]['status'] == '401', 'Expected an early unauthenticated response')
        else:
            require(not any(run[key] for key in ('controlled_wait_timeout',
                                                'active_gate_assertion_failed', 'early_response')),
                    'Unexpected setup diagnostic failure')
    return 'blocked by separate unchanged test' if blocked_default else (
        'expected negative control' if negative else 'passed')


if __name__ == '__main__':
    final_sha = hashlib.sha256((HERE.parents[2] /
                               'backend/tests/leaderboard_sessions/expiry.rs').read_bytes()).hexdigest()
    for phase in PHASES:
        report = json.loads((HERE / f'{phase}.json').read_text())
        outcome = check_report(report)
        if phase in ('repaired', 'ladder', 'continuation'):
            require(report['source_sha256'] == final_sha, 'Final source does not match validation')
        print('VERIFIED receipt: ' + phase + ' — ' + outcome)

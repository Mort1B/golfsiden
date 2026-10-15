"""A8 acceptance on a uniquely owned PostgreSQL; never connects to hosted data.
Baseline captures the original loader and requires the query-budget regression.
Current runs measurement, session regressions, full database suite, migrate/seed.
Raw logs remain local. Only this invocation's CID is removed, even on failure.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--phase', choices=['baseline', 'current'], required=True)
    parser.add_argument('--measurement-only', action='store_true', help='Skip contracts/full acceptance for a serial timing comparison')
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix='golf-a8-postgres-'))
    cidfile = temporary / 'container.cid'
    results = {'phase': args.phase}
    try:
        subprocess.run([
            'podman', 'create', '--cidfile', str(cidfile), '--name', temporary.name,
            '--publish', '127.0.0.1::5432', '-e', 'POSTGRES_USER=golf',
            '-e', 'POSTGRES_PASSWORD=golf', '-e', 'POSTGRES_DB=golf',
            '--tmpfs', '/var/lib/postgresql/data:rw',
            'docker.io/library/postgres:17-alpine',
            '-c', 'shared_preload_libraries=pg_stat_statements',
        ], check=True, capture_output=True, timeout=60)
        cid = cidfile.read_text().strip()
        subprocess.run(['podman', 'start', cid], check=True, capture_output=True, timeout=60)
        deadline = time.monotonic() + 30
        while subprocess.run(['podman', 'exec', cid, 'pg_isready', '-h', '127.0.0.1', '-U', 'golf'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
            if time.monotonic() > deadline:
                raise RuntimeError('synthetic PostgreSQL startup timed out')
            time.sleep(0.1)
        address = subprocess.check_output(['podman', 'port', cid, '5432/tcp'], text=True).strip()
        assert re.fullmatch(r'127\.0\.0\.1:[0-9]+', address)
        environment = dict(os.environ, DATABASE_URL='postgres://golf:golf@' + address + '/golf',
                           GOLF_TEAM_MEASURE='1', GOLF_TEAM_MEASURE_OUT=str(args.output / 'measurement.json'))
        results['version'] = subprocess.check_output(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-Atc', 'SELECT version()'], text=True).strip()
        results['database_collation'] = subprocess.check_output(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-Atc', 'SELECT datcollate FROM pg_database WHERE datname=current_database()'], text=True).strip()
        commands = [
            ('measurement', ['cargo', 'test', '--locked', '-p', 'golf-api', '--features', 'database-tests', '--test', 'team_loading', 'counts_and_complete_local_read_cost', '--', '--ignored', '--test-threads=1']),
        ]
        if not args.measurement_only:
            commands.append(('contracts', ['cargo', 'test', '--locked', '-p', 'golf-api', '--features', 'database-tests', '--test', 'team_loading', '--', '--test-threads=1']))
        if args.phase == 'current' and not args.measurement_only:
            commands += [
                ('session-regressions', ['cargo', 'test', '--locked', '-p', 'golf-api', '--features', 'database-tests', '--test', 'pairing_team_sessions', '--', '--test-threads=2']),
                ('database-suite', ['cargo', 'test', '--locked', '--workspace', '--all-targets', '--features', 'database-tests', '--', '--test-threads=2']),
                ('migrate', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'migrate']),
                ('seed', ['cargo', 'run', '--locked', '-p', 'golf-api', '--bin', 'seed']),
            ]
        for label, command in commands:
            if label == 'measurement':
                # A stale baseline must never disguise an unrelated exit 101.
                (args.output / 'measurement.json').unlink(missing_ok=True)
            started = time.monotonic()
            print('START ' + label, flush=True)
            log = args.output / (label + '.log')
            with log.open('w') as output:
                result = subprocess.run(command, cwd=ROOT, env=environment, stdout=output, stderr=subprocess.STDOUT, timeout=2400)
            entry = {'command': command, 'exit': result.returncode, 'seconds': round(time.monotonic()-started, 2)}
            totals = re.findall(r'test result: (?:ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;', log.read_text())
            if totals:
                entry.update(zip(['passed', 'failed', 'ignored'], [sum(int(row[n]) for row in totals) for n in range(3)]))
            results[label] = entry
            print('END ' + label + ' ' + json.dumps(entry), flush=True)
            if args.phase == 'baseline' and label == 'measurement':
                evidence = json.loads((args.output / 'measurement.json').read_text())
                assert result.returncode == 101 and len(evidence['regressions']) == 6
                assert len(evidence['samples']) == 12
                entry['expected_original_budget_failure'] = True
            elif result.returncode:
                raise RuntimeError(label + ' failed; inspect its local log')
        if args.phase == 'current' and not args.measurement_only:
            results['migrations_current'] = subprocess.check_output(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-Atc', 'SELECT count(*) FROM _sqlx_migrations WHERE success'], text=True).strip()
    finally:
        try:
            if cidfile.is_file():
                cid = cidfile.read_text().strip()
                assert re.fullmatch('[0-9a-f]{64}', cid)
                results['owned_container_removed'] = False
                results['retained_container_id'] = cid
                subprocess.run(['podman', 'rm', '-f', cid], check=True, capture_output=True, timeout=60)
                results['owned_container_removed'] = True
                results.pop('retained_container_id')
        finally:
            shutil.rmtree(temporary)
            (args.output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
            print('CLEANUP owned fixture removed' if results.get('owned_container_removed') else 'CLEANUP unconfirmed; inspect results.json', flush=True)


if __name__ == '__main__':
    main()

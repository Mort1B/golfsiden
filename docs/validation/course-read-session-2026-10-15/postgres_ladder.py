"""Run A1n database acceptance in a uniquely owned, disposable local PostgreSQL.
Only synthetic data; binds an ephemeral loopback port. Deletes only its own CID.
Raw command logs stay in the supplied output directory; JSON contains no URLs.
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
    parser.add_argument('--phase', choices=['baseline','current'], required=True)
    parser.add_argument('--focused-only', action='store_true', help='Run current focused checks without repeating the full ladder')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix='golf-a1n-postgres-'))
    cidfile = temporary / 'container.cid'
    results = {'phase':args.phase, 'focused_only':args.focused_only}
    try:
        subprocess.run([
            'podman', 'create', '--cidfile', str(cidfile), '--name', temporary.name,
            '--publish', '127.0.0.1::5432', '-e', 'POSTGRES_USER=golf',
            '-e', 'POSTGRES_PASSWORD=golf', '-e', 'POSTGRES_DB=golf',
            '--tmpfs', '/var/lib/postgresql/data:rw',
            'docker.io/library/postgres:17-alpine',
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
        environment = dict(os.environ, DATABASE_URL='postgres://golf:golf@' + address + '/golf')
        results['version'] = subprocess.check_output(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-Atc', 'SELECT version()'], text=True).strip()
        commands = [
            ('course-sessions', ['cargo','test','--locked','-p','golf-api','--features','database-tests','--test','course_read_sessions','--','--test-threads=2']),
        ]
        if args.phase == 'current':
            commands += [
                ('conditional-provider', ['cargo','test','--locked','-p','golf-api','--lib','--features','database-tests','api::course_provider::session_tests::','--','--test-threads=2']),
                ('existing-course-contracts', ['cargo','test','--locked','-p','golf-api','--features','database-tests','--test','course_provider','--test','course_presets','--','--test-threads=2']),
            ]
        if args.phase == 'current' and not args.focused_only:
            commands += [
                ('database-suite', ['cargo','test','--locked','--workspace','--all-targets','--features','database-tests','--','--test-threads=2']),
                ('migrate', ['cargo','run','--locked','-p','golf-api','--bin','migrate']),
                ('seed', ['cargo','run','--locked','-p','golf-api','--bin','seed']),
            ]
        for label, command in commands:
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
            if args.phase == 'baseline':
                failed = re.findall(r'^test (\S+) \.\.\. FAILED$', log.read_text(), re.MULTILINE)
                expected = {
                    'course_read_sessions::authorization::membership_demotion_retries_exact_admin_and_cancellation_releases_session',
                    'course_read_sessions::authorization::revocation_after_extraction_before_authority_denies_all_three_gets',
                    'course_read_sessions::authorization::credential_replacement_after_extraction_denies_all_three_gets',
                    'course_read_sessions::expiry::expiry_during_membership_wait_denies_all_three_gets',
                    'course_read_sessions::expiry::preset_final_loading_expiry_denies_populated_and_empty_results',
                    'course_read_sessions::contracts::exact_payloads_roles_query_errors_and_private_extractor_responses',
                    'course_read_sessions::retry::serialization_failure_retries_once_then_private_503',
                }
                entry['failed_tests'] = failed
                entry['expected_original_regressions'] = set(failed) == expected
                assert result.returncode == 101 and set(failed) == expected
            elif result.returncode:
                raise RuntimeError(label + ' failed; inspect its local log')
        if args.phase == 'current' and not args.focused_only:
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
            if results.get('owned_container_removed'):
                print('CLEANUP owned fixture and temporary directory removed', flush=True)
            else:
                print('CLEANUP container removal unconfirmed; inspect results.json', flush=True)


if __name__ == '__main__':
    main()

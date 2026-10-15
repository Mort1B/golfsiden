"""A1x baseline/candidate browser run on fresh owned PG/API/Vite; no retained data."""
import argparse
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent / 'sqlx-verify-ca-2026-10-15'))
from fixtures import Database


def stop_group(process):
    if process is None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=5)
    except ProcessLookupError:
        pass
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)
    # The test API/Chrome group can outlive its npm leader on failure.
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def ready(url, process):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError('Owned Vite exited before readiness')
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except OSError:
            pass
        time.sleep(.1)
    raise RuntimeError('Owned Vite readiness timed out')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['baseline', 'candidate'])
    args = parser.parse_args()
    # A failed setup must never leave an earlier success receipt for this phase.
    for name in (f'{args.phase}.json', f'{args.phase}-measurements.json'):
        (HERE / name).unlink(missing_ok=True)
    for port in (3000, 5173):
        with socket.socket() as probe:
            probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            probe.bind(('127.0.0.1', port))  # Never stop or reuse an existing service.
    result = {'phase': args.phase, 'checks': {}}
    with tempfile.TemporaryDirectory(prefix='golf-a1x-') as temp:
        directory = Path(temp)
        database = Database(directory, 'browser', secrets.token_urlsafe(24))
        result['fixture'] = database.evidence
        vite = None
        browser = None
        with database:
            environment = dict(os.environ, DATABASE_URL=f'postgres://golf:{database.password}@{database.address}/golf',
                               APP_ENV='development', GOLF_LEADERBOARD_LIVE_BROWSER='1',
                               GOLF_COURSE_API_KEY='',
                               GOLF_LEADERBOARD_MEASUREMENTS=str(HERE / f'{args.phase}-measurements.json'))

            def check(name, command, cwd=ROOT, timeout=180):
                print('START ' + name, flush=True)
                started = time.monotonic()
                with tempfile.NamedTemporaryFile(mode='w', prefix=f'golf-a1x-{name}-', suffix='.log', delete=False) as log:
                    completed = subprocess.run(command, cwd=cwd, env=environment, stdout=log,
                                               stderr=subprocess.STDOUT, timeout=timeout)
                item = {'exit': completed.returncode, 'seconds': round(time.monotonic() - started, 2)}
                result['checks'][name] = item
                print('END ' + name + ' ' + json.dumps(item), flush=True)
                if completed.returncode:
                    raise RuntimeError('Owned fixture/validation command failed; inspect private local log')

            try:
                check('build_api', ['cargo', 'build', '--locked', '-p', 'golf-api', '--bins'])
                check('migrate', [str(ROOT / 'target/debug/migrate')])
                check('seed', [str(ROOT / 'target/debug/seed')])
                with tempfile.NamedTemporaryFile(mode='w', prefix='golf-a1x-vite-', suffix='.log', delete=False) as log:
                    vite = subprocess.Popen(['npm', 'run', 'dev', '--', '--host', '127.0.0.1', '--port', '5173', '--strictPort'],
                                            cwd=ROOT / 'frontend', env=environment, stdout=log,
                                            stderr=subprocess.STDOUT, start_new_session=True)
                    ready('http://127.0.0.1:5173', vite)
                    report = Path('/tmp') / f'golf-a1x-{args.phase}-playwright.json'
                    errorlog = Path('/tmp') / f'golf-a1x-{args.phase}-playwright-errors.log'
                    report.touch(mode=0o600, exist_ok=True)
                    errorlog.touch(mode=0o600, exist_ok=True)
                    report.chmod(0o600)
                    errorlog.chmod(0o600)
                    with report.open('w') as output, errorlog.open('w') as errors:
                        browser = subprocess.Popen(['npx', 'playwright', 'test', '--config', 'playwright.lifecycle.config.ts',
                                                    'leaderboardLive.browser.ts', '--reporter=json',
                                                    f'--output=/tmp/golf-a1x-{args.phase}-browser-results'],
                                                   cwd=ROOT / 'frontend', env=environment, stdout=output,
                                                   stderr=errors, start_new_session=True)
                        code = browser.wait(timeout=300)
                    payload = json.loads(report.read_text())
                    stats = payload['stats']
                    result['browser'] = {'exit': code, **{k: stats[k] for k in ('expected', 'unexpected', 'flaky', 'skipped')}}
                    if args.phase == 'baseline':
                        body = report.read_text()
                        result['browser']['old_label_failure_reproduced'] = (
                            code == 1 and stats['unexpected'] == 1 and stats['expected'] == 0
                            and 'Lagret på serveren' in body and 'Synkronisert|Lagret' in body
                            and 'toHaveText' in body)
                        result['accepted'] = result['browser']['old_label_failure_reproduced']
                    else:
                        result['accepted'] = code == 0 and stats['expected'] == 1 and not any(stats[k] for k in ('unexpected', 'flaky', 'skipped'))
                    print('BROWSER ' + json.dumps(result['browser']), flush=True)
                if args.phase == 'candidate':
                    for name in ('test', 'typecheck', 'lint', 'build'):
                        check('frontend_' + name, ['npm', 'run', name], ROOT / 'frontend')
            finally:
                stop_group(browser)
                stop_group(vite)
        result['owned_services_stopped'] = True
        for port in (3000, 5173):
            with socket.socket() as probe:
                probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                probe.bind(('127.0.0.1', port))
    result['temporary_credentials_removed'] = not directory.exists()
    result['accepted'] = bool(result.get('accepted') and database.evidence['removed'] and result['temporary_credentials_removed'])
    (HERE / f'{args.phase}.json').write_text(json.dumps(result, indent=2) + '\n')
    print('CLEANUP owned PG/API/Vite/browser and credentials removed', flush=True)
    if not result['accepted']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

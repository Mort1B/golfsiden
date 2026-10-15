"""Own a loopback-only A1t fixture; retain only sanitized measurement output."""
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import tempfile
import time
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True,
                          timeout=120, **kwargs).stdout.strip()


def main():
    for filename in ('measurements.json', 'run-results.json', 'summary.json'):
        (HERE / filename).unlink(missing_ok=True)
    for port in (3000, 5173):
        with socket.socket() as check:
            check.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            check.bind(('127.0.0.1', port))
    processes = []
    logs = []
    cid = None
    api_cid = None
    output = {'checks': {}}
    with tempfile.TemporaryDirectory(prefix='golf-a1t-') as temp:
        temporary = Path(temp)
        pg_env = temporary / 'pg.env'
        pg_env.write_text('POSTGRES_USER=golf\nPOSTGRES_DB=golf\nPOSTGRES_PASSWORD='
                          + secrets.token_urlsafe(24) + '\n')
        pg_env.chmod(0o600)
        pg_cidfile = temporary / 'pg.cid'
        api_cidfile = temporary / 'api.cid'
        try:
            run(['podman', 'create', '--cidfile', str(pg_cidfile), '--name', temporary.name,
                       '--env-file', str(pg_env), '--publish', '127.0.0.1::5432',
                       '--publish', '127.0.0.1:3000:3000',
                       '--tmpfs', '/var/lib/postgresql/data:rw',
                       'docker.io/library/postgres:17-alpine',
                       '-c', 'shared_preload_libraries=pg_stat_statements',
                       '-c', 'track_io_timing=on'])
            cid = pg_cidfile.read_text().strip()
            run(['podman', 'start', cid])
            deadline = time.monotonic() + 45
            while subprocess.run(['podman', 'exec', cid, 'pg_isready', '-h', '127.0.0.1', '-U', 'golf'],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                if time.monotonic() > deadline:
                    raise RuntimeError('Owned PostgreSQL was not ready')
                time.sleep(0.2)
            address = run(['podman', 'port', cid, '5432/tcp'])
            password = pg_env.read_text().split('POSTGRES_PASSWORD=', 1)[1].strip()
            environment = dict(os.environ, DATABASE_URL=f'postgres://golf:{password}@{address}/golf',
                               APP_ENV='development', SESSION_COOKIE_SECURE='false',
                               RUN_MIGRATIONS='false', GOLF_COURSE_API_KEY='', RUST_LOG='warn',
                               GOLF_A1T_CONTAINER=cid)
            for name, command in [('build', ['cargo', 'build', '--locked', '-p', 'golf-api', '--bins']),
                                  ('migrate', [str(ROOT / 'target/debug/migrate')]),
                                  ('frontend-build', ['npm', 'run', 'build'])]:
                print('START ' + name, flush=True)
                cwd = ROOT / 'frontend' if name == 'frontend-build' else ROOT
                started = time.monotonic()
                with (Path('/tmp') / f'golf-a1t-{name}.log').open('w') as log:
                    result = subprocess.run(command, cwd=cwd, env=environment, stdout=log,
                                            stderr=subprocess.STDOUT, timeout=1200)
                output['checks'][name] = {'exit': result.returncode,
                                         'seconds': round(time.monotonic() - started, 2)}
                if result.returncode:
                    raise RuntimeError(name + ' failed; inspect local log')
            run(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-d', 'golf',
                 '-v', 'ON_ERROR_STOP=1', '-c', 'CREATE EXTENSION pg_stat_statements'])
            api_env = temporary / 'api.env'
            api_env.write_text('DATABASE_URL=postgres://golf:' + password + '@127.0.0.1:5432/golf\n'
                               'APP_ENV=development\nSESSION_COOKIE_SECURE=false\n'
                               'RUN_MIGRATIONS=false\nGOLF_COURSE_API_KEY=\nRUST_LOG=warn\n')
            api_env.chmod(0o600)
            # The unchanged API binds 0.0.0.0 inside this owned namespace;
            # only the PostgreSQL container's loopback-published port is exposed.
            run(['podman', 'create', '--cidfile', str(api_cidfile),
                 '--network', 'container:' + cid, '--read-only', '--cap-drop=ALL',
                 '--security-opt=no-new-privileges', '--security-opt=label=disable',
                 '--user', f'{os.getuid()}:{os.getgid()}',
                 '--env-file', str(api_env), '--volume', '/usr/lib64:/lib64:ro',
                 '--volume', str(ROOT / 'target/debug/golf-api') + ':/app/golf-api:ro',
                 '--entrypoint', '/app/golf-api', 'docker.io/library/postgres:17-alpine'])
            api_cid = api_cidfile.read_text().strip()
            run(['podman', 'start', api_cid])
            for name, command, cwd in [
                ('vite', ['npm', 'run', 'dev', '--', '--host', '127.0.0.1', '--strictPort'], ROOT / 'frontend'),
            ]:
                log = (Path('/tmp') / f'golf-a1t-{name}.log').open('w')
                logs.append(log)
                processes.append(subprocess.Popen(command, cwd=cwd, env=environment,
                                                  stdout=log, stderr=subprocess.STDOUT,
                                                  start_new_session=True))
            for url in ('http://127.0.0.1:3000/api/health', 'http://127.0.0.1:5173'):
                deadline = time.monotonic() + 30
                while True:
                    try:
                        with urllib.request.urlopen(url, timeout=1) as response:
                            if response.status == 200:
                                break
                    except OSError:
                        pass
                    if time.monotonic() > deadline:
                        raise RuntimeError('Owned local service was not ready')
                    time.sleep(0.2)
            print('START browser/API/PostgreSQL measurement', flush=True)
            result = subprocess.run(['node', str(HERE / 'measure.mjs')], cwd=ROOT,
                                    env=environment, timeout=900)
            output['checks']['measurement'] = {'exit': result.returncode}
            if result.returncode:
                raise RuntimeError('Measurement assertions failed')
        finally:
            cleanup_errors = []
            for process in reversed(processes):
                try:
                    if process.poll() is None:
                        try:
                            os.killpg(process.pid, signal.SIGTERM)
                        except ProcessLookupError:
                            pass
                        try:
                            process.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            try:
                                os.killpg(process.pid, signal.SIGKILL)
                            except ProcessLookupError:
                                pass
                            process.wait(timeout=5)
                except (OSError, subprocess.SubprocessError):
                    cleanup_errors.append('owned process cleanup failed')
            for log in logs:
                log.close()
            output['owned_containers_removed'] = True
            for cidfile in (api_cidfile, pg_cidfile):
                if cidfile.exists():
                    owned_cid = cidfile.read_text().strip()
                    try:
                        run(['podman', 'rm', '-f', owned_cid])
                        output['owned_containers_removed'] &= not run(
                            ['podman', 'ps', '-a', '--filter', f'id={owned_cid}', '--format', '{{.ID}}'])
                    except (OSError, subprocess.SubprocessError):
                        output['owned_containers_removed'] = False
                        cleanup_errors.append('owned container removal unconfirmed')
            output['owned_processes_stopped'] = all(p.poll() is not None for p in processes)
            output['cleanup_errors'] = cleanup_errors
            (HERE / 'run-results.json').write_text(json.dumps(output, indent=2) + '\n')
            print('Owned fixture cleanup complete' if not cleanup_errors else 'Owned fixture cleanup requires attention', flush=True)
            if cleanup_errors:
                raise RuntimeError('Owned cleanup could not be confirmed; inspect run-results.json')


if __name__ == '__main__':
    main()

"""Own A1u comparison services. --api is an internal owned-fixture controller."""
import hashlib
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

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, timeout=120, **kwargs).stdout.decode().strip()


def ready(url):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except OSError:
            pass
        time.sleep(.1)
    raise RuntimeError('Owned service not ready')


def api(variant):
    temporary = Path(os.environ['GOLF_A1U_TEMP'])
    cidfile = temporary / 'api.cid'
    if cidfile.exists():
        run(['podman', 'rm', '-f', cidfile.read_text().strip()])
        cidfile.unlink(missing_ok=True)
    if variant == 'stop':
        return
    if variant not in ('baseline', 'candidate'):
        raise ValueError('Unknown API variant')
    binary = os.environ['GOLF_A1U_' + variant.upper()]
    run(['podman', 'create', '--cidfile', str(cidfile), '--network', 'container:' + os.environ['GOLF_A1U_CONTAINER'],
         '--read-only', '--cap-drop=ALL', '--security-opt=no-new-privileges', '--security-opt=label=disable',
         '--user', f'{os.getuid()}:{os.getgid()}', '--env-file', str(temporary / 'api.env'),
         '--volume', '/usr/lib64:/lib64:ro', '--volume', binary + ':/app/golf-api:ro',
         '--entrypoint', '/app/golf-api', 'docker.io/library/postgres:17-alpine'])
    run(['podman', 'start', cidfile.read_text().strip()])
    ready('http://127.0.0.1:3000/api/health')


def cleanup_browser(temporary):
    pidfile = temporary / 'browser.pid'
    if not pidfile.exists():
        return False
    pid = int(pidfile.read_text())
    if pid <= 0:
        return False
    try:
        os.killpg(pid, 0)
        try:
            if os.getpgid(pid) != pid:
                return False  # Never signal an unrelated process group.
        except ProcessLookupError:
            pass  # A detached owned group can outlive its Chrome leader.
        os.killpg(pid, signal.SIGTERM)
        for _ in range(80):
            time.sleep(.05)
            os.killpg(pid, 0)
        os.killpg(pid, signal.SIGKILL)
        for _ in range(20):
            time.sleep(.05)
            os.killpg(pid, 0)
    except ProcessLookupError:
        return True
    return False


def main():
    baseline = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path('/tmp/golf-a1u-baseline/golf-api')
    candidate = ROOT / 'target/debug/golf-api'
    assert baseline.is_file() and candidate.is_file()
    for name in ('measurements.json', 'run-results.json', 'summary.json'):
        (HERE / name).unlink(missing_ok=True)
    for port in (3000, 5173):
        with socket.socket() as probe:
            probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            probe.bind(('127.0.0.1', port))
    result = {}
    provenance = json.loads((HERE / 'builds.json').read_text())
    for variant, binary in [('baseline', baseline), ('candidate', candidate)]:
        with binary.open('rb') as stream:
            result[variant + '_binary_sha256'] = hashlib.file_digest(stream, 'sha256').hexdigest()
        assert result[variant + '_binary_sha256'] == provenance[variant + '_binary_sha256']
    assert result['baseline_binary_sha256'] != result['candidate_binary_sha256']
    with tempfile.TemporaryDirectory(prefix='golf-a1u-comparison-') as temp:
        temporary = Path(temp)
        cidfile = temporary / 'pg.cid'
        process = None
        node = None
        password = secrets.token_urlsafe(24)
        pg_env = temporary / 'pg.env'
        pg_env.write_text('POSTGRES_USER=golf\nPOSTGRES_DB=golf\nPOSTGRES_PASSWORD=' + password + '\n')
        pg_env.chmod(0o600)
        api_env = temporary / 'api.env'
        api_env.write_text('DATABASE_URL=postgres://golf:' + password + '@127.0.0.1:5432/golf\n'
                           'APP_ENV=development\nSESSION_COOKIE_SECURE=false\nRUN_MIGRATIONS=false\n'
                           'GOLF_COURSE_API_KEY=\nRUST_LOG=warn\n')
        api_env.chmod(0o600)
        try:
            run(['podman', 'create', '--cidfile', str(cidfile), '--name', temporary.name,
                 '--env-file', str(pg_env), '--publish', '127.0.0.1::5432', '--publish', '127.0.0.1:3000:3000',
                 '--tmpfs', '/var/lib/postgresql/data:rw', 'docker.io/library/postgres:17-alpine',
                 '-c', 'shared_preload_libraries=pg_stat_statements', '-c', 'track_io_timing=on'])
            cid = cidfile.read_text().strip()
            run(['podman', 'start', cid])
            deadline = time.monotonic() + 45
            while subprocess.run(['podman', 'exec', cid, 'pg_isready', '-h', '127.0.0.1', '-U', 'golf'],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                if time.monotonic() > deadline:
                    raise RuntimeError('Owned PostgreSQL not ready')
                time.sleep(.2)
            address = run(['podman', 'port', cid, '5432/tcp'])
            environment = dict(os.environ, DATABASE_URL=f'postgres://golf:{password}@{address}/golf',
                               GOLF_A1U_TEMP=temp, GOLF_A1U_CONTAINER=cid,
                               GOLF_A1U_BASELINE=str(baseline), GOLF_A1U_CANDIDATE=str(candidate))
            with Path('/tmp/golf-a1u-comparison-migrate.log').open('w') as log:
                subprocess.run([str(ROOT / 'target/debug/migrate')], cwd=ROOT, env=environment,
                               check=True, stdout=log, stderr=subprocess.STDOUT, timeout=120)
            run(['podman', 'exec', cid, 'psql', '-X', '-U', 'golf', '-d', 'golf', '-v', 'ON_ERROR_STOP=1', '-c', 'CREATE EXTENSION pg_stat_statements'])
            with Path('/tmp/golf-a1u-vite.log').open('w') as log:
                process = subprocess.Popen(['npm', 'run', 'dev', '--', '--host', '127.0.0.1', '--strictPort'],
                                           cwd=ROOT / 'frontend', env=environment, stdout=log,
                                           stderr=subprocess.STDOUT, start_new_session=True)
                ready('http://127.0.0.1:5173')
                node = subprocess.Popen(['node', str(HERE / 'measure.mjs')], cwd=ROOT,
                                        env=environment, start_new_session=True)
                exit_code = node.wait(timeout=900)
                result['measurement_exit'] = exit_code
                if exit_code:
                    raise RuntimeError('Comparison assertions failed')
        finally:
            errors = []
            for owned_process in (node, process):
                try:
                    if not owned_process:
                        continue
                    # Also stop descendants if the controller itself has exited.
                    try:
                        os.killpg(owned_process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        owned_process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(owned_process.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        owned_process.wait(timeout=5)
                except (OSError, subprocess.SubprocessError):
                    errors.append('owned process stop unconfirmed')
            try:
                result['owned_browser_group_removed'] = cleanup_browser(temporary)
                if not result['owned_browser_group_removed']:
                    errors.append('owned browser group cleanup unconfirmed')
            except (OSError, ValueError):
                errors.append('owned browser group cleanup unconfirmed')
            for owned in (temporary / 'api.cid', cidfile):
                if owned.exists():
                    try:
                        cid = owned.read_text().strip()
                        run(['podman', 'rm', '-f', cid])
                        assert not run(['podman', 'ps', '-a', '--filter', f'id={cid}', '--format', '{{.ID}}'])
                    except (OSError, subprocess.SubprocessError, AssertionError):
                        errors.append('owned container cleanup unconfirmed')
            result['cleanup_errors'] = errors
            result['owned_services_removed'] = not errors
            (HERE / 'run-results.json').write_text(json.dumps(result, indent=2) + '\n')
            print('Owned comparison fixture removed' if not errors else 'Cleanup requires attention', flush=True)
            if errors:
                raise RuntimeError('Owned cleanup unconfirmed')


if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == '--api':
        api(sys.argv[2])
    else:
        main()

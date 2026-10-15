"""DEPLOY-4 reuses the exact production rehearsal with a separate receipt."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parent / 'production-rehearsal-2026-10-15'))
from rehearsal import Rehearsal


def setup():
    pointer = Path('/tmp/golf-deploy4-control-path')
    assert not pointer.exists(), 'Refusing to replace another fixture pointer'
    root = Path(tempfile.mkdtemp(prefix='golf-deploy3-critical-'))
    root.chmod(0o700)
    with (root / 'socket.log').open('w') as log:
        process = subprocess.Popen(['podman', 'system', 'service', '--time=0', 'unix://' + str(root / 'podman.sock')],
                                   stdout=log, stderr=log, start_new_session=True)
    control = root / 'control.json'
    control.write_text(json.dumps({'pid': process.pid, 'directory': str(root), 'socket': str(root / 'podman.sock')}))
    control.chmod(0o600)
    pointer.write_text(str(control))
    pointer.chmod(0o600)
    for _ in range(50):
        if (root / 'podman.sock').exists():
            break
        time.sleep(.1)
    rehearsal = Rehearsal(control)
    assert rehearsal.run(['docker', 'version', '--format', '{{.Server.Version}}']).stdout.strip() == '5.8.7'
    print(json.dumps({'private_control': str(control)}), flush=True)
    rehearsal.setup()
    rehearsal.start()


def finish(control):
    rehearsal = Rehearsal(control)
    assert rehearsal.status('/api/health') == rehearsal.status('/api/ready') == '200'
    assert rehearsal.sql('SELECT count(*),max(version),bool_and(success) FROM _sqlx_migrations;', runtime=True) == '39|39|t'
    for service in rehearsal.state['services'].values():
        item = json.loads(rehearsal.run(['docker', 'inspect', service['id']]).stdout)[0]
        assert item['State']['Running'] and item['RestartCount'] == 0
        logs = rehearsal.run(['docker', 'logs', service['id']])
        for key in ('POSTGRES_OWNER_PASSWORD', 'APP_DATABASE_PASSWORD', 'PROXY_SHARED_SECRET'):
            assert rehearsal.state['values'][key] not in logs.stdout + logs.stderr
    rehearsal.receipt('final_ready_schema_restricted_role_no_restarts_or_secret_logs')
    rehearsal.cleanup(HERE / 'integration.json')
    pointer = Path('/tmp/golf-deploy4-control-path')
    if pointer.exists() and pointer.read_text() == str(control):
        pointer.unlink()


if __name__ == '__main__':
    try:
        if sys.argv[1] == 'setup':
            setup()
        elif sys.argv[1] == 'finish':
            finish(sys.argv[2])
        else:
            raise ValueError('Expected setup or finish')
    except Exception as error:
        print(json.dumps({'failure_class': type(error).__name__, 'detail': 'private fixture diagnostics retained'}))
        sys.exit(1)

"""Run one complete fresh DEPLOY-4 acceptance sequence, then owned cleanup."""
import json
from pathlib import Path
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
started = time.monotonic()
phases = []
subprocess.run([sys.executable, str(HERE / 'fixture.py'), 'setup'], cwd=REPO, check=True)
control = Path('/tmp/golf-deploy4-control-path').read_text()
try:
    for phase in ('setup', 'fantasy', 'entry', 'numeric', 'final'):
        print(json.dumps({'phase_start': phase}), flush=True)
        result = subprocess.run(['node', str(HERE / 'browser.cjs'), phase, control], cwd=REPO, timeout=300)
        phases.append({'phase': phase, 'exit_code': result.returncode})
        if result.returncode:
            raise RuntimeError('Acceptance phase failed; private diagnostics remain until cleanup')
    subprocess.run([sys.executable, str(HERE / 'fixture.py'), 'finish', control], cwd=REPO, check=True)
except Exception:
    # Preserve the private fixture for diagnosis; do not claim cleanup/success.
    (HERE / 'sequence.json').write_text(json.dumps({'phases': phases, 'complete': False}, indent=2) + '\n')
    raise
(HERE / 'sequence.json').write_text(json.dumps({'phases': phases, 'complete': True,
                                              'elapsed_seconds': round(time.monotonic() - started, 3)}, indent=2) + '\n')
print(json.dumps({'complete_sequence': True, 'phases': len(phases)}), flush=True)

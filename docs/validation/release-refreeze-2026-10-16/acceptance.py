"""Reuse the accepted full production/recovery workflow with new candidate evidence."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
os.environ['GOLF_RECOVERY_EVIDENCE_DIRECTORY'] = str(HERE)
os.environ['GOLF_RELEASE_BUILD_REPORT'] = str(HERE/'candidate-builds.json')
source = HERE.parent/'recovery-2026-10-15/recovery.py'
spec = importlib.util.spec_from_file_location('refreeze_recovery', source)
assert spec and spec.loader
recovery = importlib.util.module_from_spec(spec); spec.loader.exec_module(recovery)
recovery.POINTER = Path('/tmp/golf-refreeze-recovery-controls.json')
from regressions import run as regression

Original = recovery.Rehearsal
boundary = str(HERE.parent/'production-rehearsal-2026-10-15/browser.cjs')


class AcceptedRehearsal(Original):
    def run(self, args, **kwargs):
        result = super().run(args, **kwargs)
        if len(args) > 1 and args[:2] == ['node', boundary] and not self.state.get('release_regressions_complete'):
            regression(self)
            self.state['release_regressions_complete'] = True
            self.save()
        return result


recovery.Rehearsal = AcceptedRehearsal
try:
    recovery.cleanup() if len(sys.argv) > 1 and sys.argv[1] == 'cleanup' else recovery.main()
except Exception as error:
    ownership = json.loads(recovery.POINTER.read_text()) if recovery.POINTER.exists() else {'controls': {}}
    for control in ownership['controls'].values():
        if Path(control).exists():
            Path(control).with_name('private-refreeze-failure.log').write_text(traceback.format_exc())
    print(json.dumps({'failed': type(error).__name__, 'detail': 'private owned diagnostics retained'}), flush=True)
    sys.exit(1)

"""Run the existing 23 packaged-image preflights into this candidate's evidence."""
import ast
import hashlib
import json
from pathlib import Path
import sys

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
source = HERE.parent/'production-images-2026-10-15/preflight.py'
script = source.read_text()
ast.parse(script)
sys.argv = [str(source), 'candidate']
exec(compile(script, str(source), 'exec'), {'__file__': str(HERE/'inherited-preflight.py'), '__name__': '__main__'})
(HERE/'preflight.json').write_bytes((HERE/'inspection.json').read_bytes())
(HERE/'inspection.json').unlink()
(HERE/'preflight-provenance.json').write_text(json.dumps({'source': str(source.relative_to(HERE.parents[2])),
    'sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'unchanged_runner': True}, indent=2)+'\n')

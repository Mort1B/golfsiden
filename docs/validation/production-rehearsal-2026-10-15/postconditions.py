"""Read-only evidence/image audit and offline post-fix setup boundary control."""
import ast
from contextlib import redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tempfile

from rehearsal import HERE, REPO, Rehearsal


checks = {}
integration = json.loads((HERE / 'integration.json').read_text())
browser = json.loads((HERE / 'browser.json').read_text())
web = json.loads((HERE / 'web-build.json').read_text())
baseline = json.loads((HERE.parent / 'production-images-2026-10-15/baseline-builds.json').read_text())
assert len(integration['checks']) == 39 and all(integration['checks'].values())
assert len(browser['checks']) == 7 and all(browser['checks'].values())
assert browser['images'] == integration['images']
assert integration['web_build'] == web
assert hashlib.sha256((REPO / 'deploy/Caddyfile').read_bytes()).hexdigest() == web['overlays']['deploy/Caddyfile']
checks['39_named_checks_seven_browser_checks_and_exact_logging_overlay'] = True
for record in baseline['builds'] + [{'tag': web['tag'], 'image': web['image']}]:
    image = json.loads(subprocess.check_output(['podman', 'image', 'inspect', record['tag']], text=True))[0]
    assert image['Id'] == record['image']['Id']
checks['four_retained_image_ids_still_exact'] = True
assert integration['owned_containers_networks_volumes_removed']
assert integration['owned_resource_counts_before_removal'] == {'containers': 3, 'volumes': 3, 'networks': 2}
assert not Path('/tmp/golf-deploy3-7fyjsmg4').exists()
assert not Path('/tmp/golf-deploy3-control-path').exists()
checks['recorded_private_fixture_and_socket_tree_absent'] = True
subprocess.run(['git', 'diff', '--exit-code', baseline['base_commit'], '--', 'backend',
                'frontend', 'migrations', 'vendor', 'deploy', 'scripts', 'Cargo.toml',
                'Cargo.lock', 'compose.production.yml', '.dockerignore', '.env.production.example',
                ':!deploy/Caddyfile'], cwd=REPO, check=True)
checks['only_reviewed_caddy_production_input_changed'] = True
for file in HERE.glob('*.py'):
    ast.parse(file.read_text())
subprocess.run(['node', '--check', str(HERE / 'browser.cjs')], check=True)
checks['all_harness_python_and_javascript_syntax'] = True


class BeforeServiceStart(Exception):
    pass


# Check fresh post-fix setup's exact reviewed overlay selection and real Compose
# rendering. Stop before the first service mutation; this is NOT another runtime
# integration pass. Image reads use local Podman because the API was cleaned up.
with tempfile.TemporaryDirectory(prefix='golf-deploy3-offline-') as directory:
    root = Path(directory)
    control = root / 'control.json'
    control.write_text(json.dumps({'directory': directory, 'socket': str(root / 'unused.sock'), 'pid': -1}))
    control.chmod(0o600)
    rehearsal = Rehearsal(control)
    original_run, original_compose = rehearsal.run, rehearsal.compose

    def read_only_run(args, **kwargs):
        if args[:3] == ['docker', 'image', 'inspect']:
            args = ['podman'] + args[1:]
        return original_run(args, **kwargs)

    def before_start(*args, **kwargs):
        if args[0] == 'up':
            assert rehearsal.state['images']['web'] == web['tag']
            assert rehearsal.state['web_build'] == web
            raise BeforeServiceStart()
        assert args[0] == '--profile' and 'config' in args
        return original_compose(*args, **kwargs)

    rehearsal.run, rehearsal.compose = read_only_run, before_start
    with redirect_stdout(io.StringIO()):
        try:
            rehearsal.setup()
        except BeforeServiceStart:
            checks['fresh_postfix_setup_selects_reviewed_web_and_validates_render_before_start'] = True
        else:
            raise AssertionError('Expected stop before service mutation')
assert checks['fresh_postfix_setup_selects_reviewed_web_and_validates_render_before_start']
(HERE / 'postconditions.json').write_text(json.dumps(checks, indent=2) + '\n')
print(json.dumps(checks))

"""Read-only acceptance, unchanged-input, retained-image and cleanup audit."""
import ast
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
checks = {}
integration = json.loads((HERE / 'integration.json').read_text())
sequence = json.loads((HERE / 'sequence.json').read_text())
phases = ('setup', 'fantasy', 'entry', 'numeric', 'final')
assert sequence['complete'] is True
assert sequence['phases'] == [{'phase': phase, 'exit_code': 0} for phase in phases]
assert len(integration['checks']) == 32 and all(integration['checks'].values())
previous = {}
for phase, count in zip(phases, (6, 11, 14, 22, 27)):
    receipt = json.loads((HERE / (phase + '.json')).read_text())
    assert receipt['images'] == integration['images']
    assert receipt['browser'] == '155.0.8059.79'
    current = receipt['checks']
    assert len(current) == count and all(current.values())
    assert previous.items() <= current.items() <= integration['checks'].items()
    assert current[phase + '_no_unexpected_console_network_or_http_body_failures']
    previous = current
checks['fresh_five_phase_sequence_32_named_checks_27_browser_checks'] = True

fantasy = json.loads((HERE / 'fantasy.json').read_text())
entry = json.loads((HERE / 'entry.json').read_text())
final = json.loads((HERE / 'final.json').read_text())
assert fantasy['acknowledged_204_abort_records_by_route'] == {'POST fantasy_entry acknowledged204': 2}
assert entry['acknowledged_204_abort_records_by_route'] == {'POST fantasy_entry acknowledged204': 1}
assert final['acknowledged_204_abort_records_by_route'] == {'POST auth_logout acknowledged204': 1}
assert fantasy['expected_http_counts']['503'] == 1
diagnostic = json.loads((HERE / 'no-content-diagnostic.json').read_text())
for probe in diagnostic['observations'].values():
    assert probe['statuses'] == [204] * 3
    assert probe['events'] == [event for _ in range(3) for event in (
        {'event': 'response', 'status': 204},
        {'event': 'failed', 'error': 'net::ERR_ABORTED', 'acknowledged_status': 204})]
assert diagnostic['original_caddy_configuration_restored']
assert diagnostic['application_or_caddy_defect_established'] is False
checks['exact_acknowledged204_route_counts_and_separate_passive_controls'] = True

baseline = json.loads((HERE.parent / 'production-images-2026-10-15/baseline-builds.json').read_text())
web = json.loads((HERE.parent / 'production-rehearsal-2026-10-15/web-build.json').read_text())
assert integration['web_build'] == web
assert hashlib.sha256((REPO / 'deploy/Caddyfile').read_bytes()).hexdigest() == web['overlays']['deploy/Caddyfile']
for record in baseline['builds'] + [{'tag': web['tag'], 'image': web['image']}]:
    actual = json.loads(subprocess.check_output(['podman', 'image', 'inspect', record['tag']], text=True))[0]
    assert actual['Id'] == record['image']['Id']
checks['four_retained_image_ids_and_reviewed_caddy_overlay_exact'] = True
subprocess.run(['git', 'diff', '--exit-code', 'c06e4886e277d3b4339c31b2fef5f33f9e596642', '--',
                'backend', 'frontend', 'migrations', 'vendor', 'deploy', 'scripts', 'Cargo.toml',
                'Cargo.lock', 'compose.production.yml', '.dockerignore', '.env.production.example'],
               cwd=REPO, check=True)
checks['all_production_inputs_unchanged_since_deploy3'] = True

for name in ('integration', 'exploratory-cleanup', 'monitor-triage-cleanup'):
    receipt = json.loads((HERE / (name + '.json')).read_text())
    assert receipt['owned_containers_networks_volumes_removed']
    assert receipt['private_configuration_diagnostics_ca_profile_removed']
    assert receipt['owned_resource_counts_before_removal'] == {'containers': 3, 'volumes': 3, 'networks': 2}
for name in ('am3oihlx', 'd2gkpg3v', '8g5zig5j'):
    assert not Path('/tmp/golf-deploy3-critical-' + name).exists()
assert not Path('/tmp/golf-deploy4-control-path').exists()
checks['all_three_recorded_fixture_trees_resources_and_pointer_removed'] = True

for file in HERE.glob('*.py'):
    ast.parse(file.read_text())
ast.parse((HERE.parent / 'production-rehearsal-2026-10-15/rehearsal.py').read_text())
for file in HERE.glob('*.cjs'):
    subprocess.run(['node', '--check', str(file)], check=True)
subprocess.run(['git', 'diff', '--check'], cwd=REPO, check=True)
checks['all_changed_harness_syntax_and_diff_whitespace'] = True
(HERE / 'postconditions.json').write_text(json.dumps({'checks': checks,
    'sequence_sha256': hashlib.sha256((HERE / 'sequence.json').read_bytes()).hexdigest()}, indent=2) + '\n')
print(json.dumps(checks))

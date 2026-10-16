"""Independently check frozen delivery, receipt bindings, scope and cleanup."""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
checks = {}


def read(name):
    return json.loads((HERE/(name+'.json')).read_text())


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def run(args):
    result = subprocess.run(args, cwd=REPO, capture_output=True, text=True, timeout=240)
    assert result.returncode == 0, 'Release postcondition command failed: '+args[0]
    return result.stdout


def passed(name, condition):
    assert condition, name
    checks[name] = True


build = read('candidate-builds')
images = {i['service']: i['tag'] for i in build['builds']}
passed('three_complete_builds_and_removed_context', set(images) == {'postgres', 'api', 'web'} and len(build['builds']) == 3 and build['temporary_context_removed'] and all(i['exit_code'] == 0 for i in build['builds']))
passed('no_production_input_changes', not run(['git', 'diff', '--name-only', build['base_commit'], '--', 'backend', 'frontend', 'migrations', 'vendor', 'deploy', 'scripts', 'Cargo.toml', 'Cargo.lock', 'compose.production.yml', '.dockerignore', '.env.production.example']).strip())
with tempfile.TemporaryDirectory(prefix='golf-refreeze-verify-context-') as directory:
    context = Path(directory)
    with tempfile.TemporaryFile() as archive:
        subprocess.run(['git', 'archive', '--format=tar', build['base_commit']], cwd=REPO, stdout=archive, check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as tar:
            tar.extractall(context, filter='data')
    value = hashlib.sha256()
    for path in sorted(context.rglob('*')):
        if path.is_file():
            value.update(str(path.relative_to(context)).encode()+b'\0'+path.read_bytes())
    passed('complete_committed_context_hash_matches', value.hexdigest() == build['context_sha256'])
    for name, expected in build['overlays'].items():
        passed('unchanged_dockerfile_'+name, digest(context/name) == expected == digest(REPO/name))
pre = read('preflight')
passed('23_preflights_actual_pg1711', len(pre['checks']) == 23 and all(i['passed'] for i in pre['checks']) and pre['postgres_version'] == 'postgres (PostgreSQL) 17.11' and pre['owned_probe_containers_removed'] == 23 and pre['private_env_files_removed'])
for name, count in [('source-cleanup', 56), ('target-cleanup', 16)]:
    data = read(name)
    passed(name+'_exact_candidate_checks', len(data['checks']) == count and all(data['checks'].values()) and data['candidate_build'] == build and data['images'] == images)
    passed(name+'_owned_resources_removed', data['owned_containers_networks_volumes_removed'] and data['private_configuration_diagnostics_ca_profile_removed'])
for name, count in [('source-flows', 25), ('production-browser', 7), ('release-regressions', 3), ('browser', 4), ('browser-post-database', 3)]:
    data = read(name)
    passed(name+'_same_images_checks', len(data['checks']) == count and all(data['checks'].values()) and data['images'] == images)
reg = read('release-regressions')
passed('committed_three_regressions_pass_without_skip', reg['passed'] == 3 and reg['failed'] == 0 and reg['assertions_unchanged'] and digest(REPO/reg['source_test']) == reg['source_test_sha256'])
seq = read('sequence')
passed('complete_five_ordered_phases', seq['complete'] and seq['production_scripts_unmodified'] and [i['phase'] for i in seq['source_phases']] == ['setup', 'fantasy', 'entry', 'numeric', 'final'] and all(i['exit_code'] == 0 for i in seq['source_phases']))
parity = read('parity')
passed('all_tables_rows_sequences_exact', parity['all_public_tables_exact'] and parity['sequences_exact'] and parity['table_count'] == 61 and parity['row_count'] == sum(parity['rows_by_table'].values()) == 1122)
content = read('image-content')
passed('content_bound_to_source_context', content['source_commit'] == build['base_commit'] and content['context_sha256'] == build['context_sha256'] and not content['fresh_advisory_scan_performed'])
for item in build['builds']:
    service = item['service']
    actual = json.loads(run(['podman', 'image', 'inspect', item['tag']]))[0]
    passed(service+'_retained_id_platform_labels', actual['Id'] == item['image']['Id'] == content['images'][service]['id'] and actual['Architecture'] == 'amd64' and actual['Os'] == 'linux' and actual['Labels']['io.golfsiden.source.commit'] == build['base_commit'] and actual['Labels']['io.golfsiden.context.sha256'] == build['context_sha256'])
    passed(service+'_runtime_inventory_matches', content['images'][service]['runtime_packages_identical_to_previous'] and digest(HERE/('candidate-'+service+'-inventory.txt')) == content['images'][service]['runtime_inventory_sha256'])
    passed(service+'_previous_image_retained', json.loads(run(['podman', 'image', 'inspect', content['images'][service]['previous_id']]))[0]['Id'] == content['images'][service]['previous_id'])
web = content['images']['web']; api = content['images']['api']
passed('accepted_application_changed_runtime_caddy_unchanged', web['web_assets_changed'] and web['web_asset_count'] == pre['web_asset_count'] == 66 and web['caddy_binary_identical_to_previous'] and api['executables']['/usr/local/bin/golf-api'] != api['previous_executables']['/usr/local/bin/golf-api'])
passed('module_metadata_versions', all(value in (HERE/'candidate-web-build-info.txt').read_text() for value in ('go1.27.2', 'caddy/v2\tv2.11.7', 'golang.org/x/net\tv0.60.0', 'quic-go\tv0.63.0', 'http.handlers.reverse_proxy', 'http.handlers.file_server')))
own = read('ownership'); cleanup = read('cleanup')
passed('owned_private_runtime_and_build_roots_removed', own['private_roots_removed'] and own['owned_aliases_removed'] and all(not Path(i).exists() for i in own['private_roots']) and cleanup['build_staging_removed'] and not Path(cleanup['build_staging']).exists() and not Path('/tmp/golf-refreeze-work-path').exists() and not Path('/tmp/golf-refreeze-recovery-controls.json').exists())
for alias in own['owned_aliases']:
    passed('removed_alias_'+alias, subprocess.run(['podman', 'image', 'exists', alias], capture_output=True).returncode == 1)
bundle = read('bundle'); root = Path(bundle['directory']); manifest = read('manifest')
passed('retained_bundle_checksum', digest(Path(bundle['bundle'])) == bundle['bundle_sha256'])
passed('retained_manifest_matches_repository', digest(root/'manifest.json') == bundle['manifest_sha256'] and (root/'manifest.json').read_bytes() == (HERE/'manifest.json').read_bytes())
passed('manifest_matches_accepted_source_images', manifest['source_base'] == build['base_commit'] and manifest['context_sha256'] == build['context_sha256'] and manifest['source_overlays'] == [] and {i['service']: i['id'] for i in manifest['images']} == {i['service']: i['image']['Id'] for i in build['builds']})
run(['python3', str(root/'verify-release.py')]); passed('final_offline_verifier_passed', True)
for name in ('OPERATOR.md', 'verify-release.py'):
    passed('packaged_'+name+'_matches_repository', (root/name).read_bytes() == (HERE/name).read_bytes())
passed('loader_unchanged_from_previous_freeze', (HERE/'verify-release.py').read_bytes() == (HERE.parent/'release-freeze-2026-10-16/verify-release.py').read_bytes())
transfer = read('transfer')
passed('ten_transfer_controls_bound_to_final_bundle', len(transfer['checks']) == 10 and all(transfer['checks'].values()) and transfer['bundle_sha256'] == bundle['bundle_sha256'] and transfer['manifest_sha256'] == bundle['manifest_sha256'])
passed('owned_transfer_roots_removed', not list(Path('/tmp').glob('golf-refreeze-transfer-*')))
old = json.loads((HERE.parent/'release-freeze-2026-10-16/bundle.json').read_text())
passed('previous_bundle_bytes_preserved', digest(Path(old['bundle'])) == old['bundle_sha256'])
historical = run(['git', 'diff', '--name-only', build['base_commit'], '--', 'docs/validation']).splitlines()
allowed = {'docs/validation/release-freeze-2026-10-16/README.md', 'docs/validation/release-freeze-2026-10-16/OPERATOR.md'}
passed('historical_receipts_unchanged', all(i in allowed or i.startswith(str(HERE.relative_to(REPO))+'/') for i in historical))
for path in HERE.glob('*.py'):
    ast.parse(path.read_text())
for name in ('scripts/backup-production.sh', 'scripts/restore-production.sh'):
    run(['sh', '-n', str(root/name)])
passed('python_and_packaged_shell_syntax', True)
documents = [HERE/'README.md', HERE/'OPERATOR.md']+[REPO/'docs'/name for name in ('ARCHITECTURE.md', 'Documentation.md', 'LatestExplanation.md', 'PLANS.md', 'deployment_guide.md')]
for path in documents:
    for target in re.findall(r'\]\(([^)]+)\)', path.read_text()):
        if '://' in target or target.startswith('#'):
            continue
        destination = (path.parent/target.split('#')[0]).resolve()
        if destination == HERE/'postconditions.json':
            continue
        passed('link_'+str(path.relative_to(REPO))+'_'+target, destination.exists())
run(['git', 'diff', '--check']); passed('diff_whitespace_clean', True)
passed('review_resolved', read('review')['remaining_material_findings'] == [])
(HERE/'postconditions.json').write_text(json.dumps({'checks': checks, 'count': len(checks), 'native_docker_accepted': False, 'hosted_deployment_accepted': False}, indent=2)+'\n')
print(json.dumps({'passed': len(checks), 'bundle_sha256': bundle['bundle_sha256']}))

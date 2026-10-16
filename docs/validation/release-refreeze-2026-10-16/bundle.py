"""Package the exact accepted replacement images without rebuilding them."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
work = Path(Path('/tmp/golf-refreeze-work-path').read_text())


def read(name):
    return json.loads((HERE/(name+'.json')).read_text())


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


assert not (HERE/'bundle.json').exists(), 'Refusing to replace a frozen bundle receipt'
builds = read('candidate-builds')
assert read('sequence')['complete'] is True
images = {item['service']: item['tag'] for item in builds['builds']}
assert len(builds['builds']) == 3 and set(images) == {'postgres', 'api', 'web'}
assert builds['temporary_context_removed'] and all(item['exit_code'] == 0 for item in builds['builds'])
for filename in ('source-cleanup', 'target-cleanup'):
    accepted = read(filename)
    assert accepted['candidate_build'] == builds and accepted['images'] == images
    assert accepted['checks'] and all(accepted['checks'].values())
    assert accepted['owned_containers_networks_volumes_removed'] and accepted['private_configuration_diagnostics_ca_profile_removed']
for filename in ('source-flows', 'production-browser', 'browser', 'browser-post-database', 'release-regressions'):
    accepted = read(filename)
    assert accepted['checks'] and all(accepted['checks'].values()) and accepted['images'] == images
preflight = read('preflight')
assert len(preflight['checks']) == 23 and all(i['passed'] for i in preflight['checks'])
parity = read('parity')
assert parity['all_public_tables_exact'] and parity['sequences_exact'] and parity['table_count'] == 61
content = read('image-content')
assert content['source_commit'] == builds['base_commit'] and content['context_sha256'] == builds['context_sha256']
assert all(i['source_labels_verified'] and i['runtime_packages_identical_to_previous'] for i in content['images'].values())
tag = 'release-20261016-'+builds['context_sha256'][:12]
root = Path(tempfile.mkdtemp(prefix='golfsiden-'+tag+'-')); root.chmod(0o700)
manifest = {'tag': tag, 'platform': 'linux/amd64', 'schema': 39,
    'verdict': 'READY WITH KNOWN LIMITATIONS', 'hosted_deployment_accepted': False,
    'source_base': builds['base_commit'], 'dockerfiles': builds['overlays'], 'source_overlays': [],
    'context_sha256': builds['context_sha256'], 'images': [], 'files': {},
    'acceptance': {'packaged_guards': 23, 'release_regressions': 3, 'source_checks': 56,
        'target_checks': 16, 'boundary_browser_checks': 7, 'critical_phases': 5,
        'table_count': 61, 'synthetic_row_count': parity['row_count'], 'sequences_exact': True,
        'browser_widths': [320, 390, 1280], 'engine': 'rootless Podman Docker-compatible API'},
    'source_validation_limit': 'Prior full PostgreSQL source run retains one unrelated tournament-list collation failure; not repaired or rerun here.',
    'runtime_packages_unchanged': True, 'fresh_advisory_scan_performed': False,
    'operator_gates': ['native Docker/host architecture', 'public DNS/HTTPS/ports',
        'actual playing phone', 'protected off-host backup/restore']}
for item in builds['builds']:
    filename = 'golfsiden-'+item['service']+'.tar'
    os.link(work/('candidate-'+item['service']+'.tar'), root/filename)
    manifest['images'].append({'service': item['service'], 'id': item['image']['Id'],
        'source_tag': item['tag'], 'alias': 'golfsiden-'+item['service']+':'+tag,
        'archive': filename, 'archive_sha256': digest(root/filename)})
for name in ('compose.production.yml', 'scripts/backup-production.sh', 'scripts/restore-production.sh'):
    file = root/name; file.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(REPO/name, file)
example = (REPO/'.env.production.example').read_text()
(root/'.env.production.example').write_text('\n'.join('GOLFSIDEN_IMAGE_TAG='+tag if line.startswith('GOLFSIDEN_IMAGE_TAG=') else line for line in example.splitlines())+'\n')
for name in ('verify-release.py', 'OPERATOR.md'):
    shutil.copy2(HERE/name, root/name)
aliases = {i['service']: i['alias'] for i in manifest['images']}
aliases.update(migrate=aliases['api'], permissions=aliases['postgres'])
(root/'compose.release.json').write_text(json.dumps({'services': {k: {'image': v, 'pull_policy': 'never'} for k, v in aliases.items()}}, indent=2)+'\n')
for file in sorted(root.rglob('*')):
    if file.is_file() and file.suffix != '.tar':
        manifest['files'][str(file.relative_to(root))] = digest(file)
(root/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
(HERE/'manifest.json').write_bytes((root/'manifest.json').read_bytes())
subprocess.run(['python3', str(root/'verify-release.py')], check=True)
bundle = root.parent/(root.name+'.tar.gz')
with tarfile.open(bundle, 'w:gz', compresslevel=3) as tar:
    tar.add(root, arcname='golfsiden-'+tag)
receipt = {'directory': str(root), 'bundle': str(bundle), 'bundle_sha256': digest(bundle),
    'bundle_bytes': bundle.stat().st_size, 'manifest_sha256': digest(root/'manifest.json'),
    'image_archive_config_ids_verified_offline': True}
(HERE/'bundle.json').write_text(json.dumps(receipt, indent=2)+'\n')
print(json.dumps(receipt))

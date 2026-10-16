"""Package exact accepted images and runtime files without rebuilding them."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]
work=Path(Path('/tmp/golf-deploy6-work-path').read_text())
builds=json.loads((HERE/'candidate-builds.json').read_text())
assert json.loads((HERE/'sequence.json').read_text())['complete'] is True
assert json.loads((HERE/'content-equality.json').read_text())['three_rust_executables_byte_identical']
expected_images={item['service']:item['tag'] for item in builds['builds']}
assert len(builds['builds'])==3 and set(expected_images)=={'postgres','api','web'}
assert builds['temporary_context_removed'] and all(item['exit_code']==0 for item in builds['builds'])
for filename in ('source-cleanup','target-cleanup'):
    accepted=json.loads((HERE/(filename+'.json')).read_text())
    assert accepted['candidate_build']==builds and accepted['images']==expected_images
    assert accepted['checks'] and all(accepted['checks'].values())
for filename in ('source-flows','production-browser','browser','browser-post-database'):
    accepted=json.loads((HERE/(filename+'.json')).read_text())
    assert accepted['checks'] and all(accepted['checks'].values())
    if 'images' in accepted:assert accepted['images']==expected_images
parity=json.loads((HERE/'parity.json').read_text())
assert parity['all_public_tables_exact'] and parity['sequences_exact'] and parity['table_count']==61
tag='release-20261016-'+builds['context_sha256'][:12]
root=Path(tempfile.mkdtemp(prefix='golfsiden-'+tag+'-'))
root.chmod(0o700)


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


manifest={'tag':tag,'platform':'linux/amd64','schema':39,'verdict':'READY WITH KNOWN LIMITATIONS',
          'hosted_deployment_accepted':False,'source_base':builds['base_commit'],'overlays':builds['overlays'],
          'context_sha256':builds['context_sha256'],'images':[],'files':{},
          'operator_gates':['native Docker/host architecture','public DNS/HTTPS/ports','actual playing phone','protected off-host backup/restore']}
for item in builds['builds']:
    filename='golfsiden-'+item['service']+'.tar'
    os.link(work/('candidate-'+item['service']+'.tar'),root/filename)
    manifest['images'].append({'service':item['service'],'id':item['image']['Id'],
        'source_tag':item['tag'],'alias':'golfsiden-'+item['service']+':'+tag,
        'archive':filename,'archive_sha256':digest(root/filename)})
for name in ('compose.production.yml','scripts/backup-production.sh','scripts/restore-production.sh'):
    file=root/name;file.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(REPO/name,file)
example=(REPO/'.env.production.example').read_text()
lines=[('GOLFSIDEN_IMAGE_TAG='+tag if line.startswith('GOLFSIDEN_IMAGE_TAG=') else line) for line in example.splitlines()]
(root/'.env.production.example').write_text('\n'.join(lines)+'\n')
shutil.copy2(HERE/'verify-release.py',root/'verify-release.py')
images={i['service']:i['alias'] for i in manifest['images']};images.update(migrate=images['api'],permissions=images['postgres'])
(root/'compose.release.json').write_text(json.dumps({'services':{key:{'image':value,'pull_policy':'never'} for key,value in images.items()}},indent=2)+'\n')
shutil.copy2(HERE/'OPERATOR.md',root/'OPERATOR.md')
for file in sorted(root.rglob('*')):
    if file.is_file() and file.suffix!='.tar':manifest['files'][str(file.relative_to(root))]=digest(file)
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(HERE/'manifest.json').write_text((root/'manifest.json').read_text())
subprocess.run(['python',str(root/'verify-release.py')],check=True)
bundle=root.parent/(root.name+'.tar.gz')
with tarfile.open(bundle,'w:gz',compresslevel=3) as tar:
    tar.add(root,arcname='golfsiden-'+tag)
receipt={'directory':str(root),'bundle':str(bundle),'bundle_sha256':digest(bundle),'bundle_bytes':bundle.stat().st_size,
         'manifest_sha256':digest(root/'manifest.json'),'image_archive_config_ids_verified_offline':True}
(HERE/'bundle.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))

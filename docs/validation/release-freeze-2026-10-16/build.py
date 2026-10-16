"""Build only runtime repairs from a tracked snapshot and three reviewed overlays."""
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BASE = '7cf94bb9b40ada7ebb9b24264048a5935fd85676'
SERVICES = {'postgres':'deploy/postgres/Dockerfile.production',
            'api':'backend/Dockerfile.production','web':'frontend/Dockerfile.production'}
report = {'base_commit':BASE,'overlays':{},'builds':[],'builder':'rootless Podman',
          'native_docker_acceptance':False}
work = Path(Path('/tmp/golf-deploy6-work-path').read_text())
bases = json.loads((HERE/'runtime-bases.json').read_text())
for item in bases:
    repository = item['reference'].split('@')[0].rsplit(':',1)[0]
    result = subprocess.run(['podman','pull',repository+'@'+item['index_digest']],capture_output=True,text=True,timeout=180)
    (work/('pull-index-'+item['service']+'.log')).write_text(result.stdout+result.stderr)
    assert result.returncode == 0

with tempfile.TemporaryDirectory(prefix='golf-deploy6-context-') as directory:
    context = Path(directory)
    with tempfile.TemporaryFile() as archive:
        subprocess.run(['git','archive','--format=tar',BASE],cwd=REPO,stdout=archive,check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as tar:
            tar.extractall(context,filter='data')
    for name in SERVICES.values():
        data = (REPO/name).read_bytes(); (context/name).write_bytes(data)
        report['overlays'][name] = hashlib.sha256(data).hexdigest()
    digest = hashlib.sha256()
    for file in sorted(context.rglob('*')):
        if file.is_file():digest.update(str(file.relative_to(context)).encode()+b'\0'+file.read_bytes())
    report['context_sha256'] = digest.hexdigest()
    report['tag'] = 'deploy6-'+BASE[:12]+'-'+digest.hexdigest()[:12]
    for service,dockerfile in SERVICES.items():
        tag = 'localhost/golfsiden-'+service+':'+report['tag']
        assert subprocess.run(['podman','image','exists',tag]).returncode == 1
        started = time.monotonic()
        with (work/('build-'+service+'.log')).open('w') as log:
            result = subprocess.run(['podman','build','--pull=never','--layers','--force-rm',
                '--tag',tag,'--label','io.golfsiden.source.commit='+BASE,
                '--label','io.golfsiden.context.sha256='+report['context_sha256'],
                '--file',str(context/dockerfile),str(context)],cwd=REPO,stdout=log,stderr=subprocess.STDOUT,timeout=1500)
        assert result.returncode == 0, 'Runtime build failed; private diagnostic retained'
        image = json.loads(subprocess.check_output(['podman','image','inspect',tag],text=True))[0]
        item = {'service':service,'dockerfile':dockerfile,'tag':tag,'exit_code':0,
                'elapsed_seconds':round(time.monotonic()-started,3),
                'image':{key:image.get(key) for key in ('Id','Created','Architecture','RepoDigests')}}
        report['builds'].append(item)
        (HERE/'candidate-builds.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps({'built':service,'image_id':image['Id']}),flush=True)
report['temporary_context_removed'] = not Path(directory).exists()
(HERE/'candidate-builds.json').write_text(json.dumps(report,indent=2)+'\n')

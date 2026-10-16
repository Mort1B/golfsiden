"""Build a new frozen candidate from the accepted, committed application source."""
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import time

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BASE = 'baeeda9d51fd730bffb32892bbcbc25a79a24079'
SERVICES = {'postgres': 'deploy/postgres/Dockerfile.production',
            'api': 'backend/Dockerfile.production', 'web': 'frontend/Dockerfile.production'}
POINTER = Path('/tmp/golf-refreeze-work-path')
assert not POINTER.exists(), 'Refusing to replace an owned build pointer'
work = Path(tempfile.mkdtemp(prefix='golf-refreeze-build-')); work.chmod(0o700)
POINTER.write_text(str(work)); POINTER.chmod(0o600)
report = {'base_commit': BASE, 'overlays': {}, 'builds': [], 'builder': 'rootless Podman',
          'native_docker_acceptance': False, 'dockerfiles_unchanged_from_source': True}
with tempfile.TemporaryDirectory(prefix='golf-refreeze-context-') as directory:
    context = Path(directory)
    with tempfile.TemporaryFile() as archive:
        subprocess.run(['git', 'archive', '--format=tar', BASE], cwd=REPO, stdout=archive, check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as tar:
            tar.extractall(context, filter='data')
    for name in SERVICES.values():
        assert (context/name).read_bytes() == (REPO/name).read_bytes()
        report['overlays'][name] = hashlib.sha256((context/name).read_bytes()).hexdigest()
    digest = hashlib.sha256()
    for file in sorted(context.rglob('*')):
        if file.is_file():
            digest.update(str(file.relative_to(context)).encode()+b'\0'+file.read_bytes())
    report['context_sha256'] = digest.hexdigest()
    report['tag'] = 'refreeze-'+BASE[:12]+'-'+digest.hexdigest()[:12]
    for service, dockerfile in SERVICES.items():
        tag = 'localhost/golfsiden-'+service+':'+report['tag']
        assert subprocess.run(['podman', 'image', 'exists', tag]).returncode == 1
        started = time.monotonic()
        with (work/('build-'+service+'.log')).open('w') as log:
            result = subprocess.run(['podman', 'build', '--pull=never', '--layers', '--force-rm',
                '--tag', tag, '--label', 'io.golfsiden.source.commit='+BASE,
                '--label', 'io.golfsiden.context.sha256='+report['context_sha256'],
                '--file', str(context/dockerfile), str(context)], cwd=REPO,
                stdout=log, stderr=subprocess.STDOUT, timeout=1800)
        assert result.returncode == 0, 'Build failed; private diagnostic retained'
        item = json.loads(subprocess.check_output(['podman', 'image', 'inspect', tag], text=True))[0]
        report['builds'].append({'service': service, 'dockerfile': dockerfile, 'tag': tag,
            'exit_code': 0, 'elapsed_seconds': round(time.monotonic()-started, 3),
            'image': {key: item.get(key) for key in ('Id', 'Created', 'Architecture', 'RepoDigests')}})
        (HERE/'candidate-builds.json').write_text(json.dumps(report, indent=2)+'\n')
        print(json.dumps({'built': service, 'image_id': item['Id']}), flush=True)
report['temporary_context_removed'] = not Path(directory).exists()
(HERE/'candidate-builds.json').write_text(json.dumps(report, indent=2)+'\n')

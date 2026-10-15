"""Build actual Dockerfiles from an immutable tracked snapshot, without host secrets."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys
import tarfile
import tempfile
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BASE = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip()
PHASE = sys.argv[1] if len(sys.argv) > 1 else 'baseline'
assert PHASE in ('baseline', 'candidate')
SERVICES = {'postgres': 'deploy/postgres/Dockerfile.production',
            'api': 'backend/Dockerfile.production', 'web': 'frontend/Dockerfile.production'}
OVERLAYS = sys.argv[2:]
ALLOWED = set(SERVICES.values()) | {'Cargo.toml', 'Cargo.lock',
                                    'frontend/package.json', 'frontend/package-lock.json', '.dockerignore'}
assert all(name in ALLOWED for name in OVERLAYS)
report = {'base_commit': BASE, 'phase': PHASE, 'builder': 'rootless Podman',
          'docker_engine_acceptance': False, 'overlays': {}, 'builds': []}


def capture(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL, timeout=60).strip()


def metadata(tag):
    item = json.loads(capture(['podman', 'image', 'inspect', tag]))[0]
    return {key: item.get(key) for key in ('Id', 'Created', 'Architecture', 'RepoDigests')}


with tempfile.TemporaryDirectory(prefix='golf-deploy2-build-') as directory:
    root = Path(directory)
    context = root / 'context'
    context.mkdir()
    with tempfile.TemporaryFile() as archive:
        subprocess.run(['git', 'archive', '--format=tar', BASE], cwd=REPO, stdout=archive, check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as contents:
            contents.extractall(context, filter='data')
    for name in OVERLAYS:
        data = (REPO / name).read_bytes()
        (context / name).write_bytes(data)
        report['overlays'][name] = hashlib.sha256(data).hexdigest()
    fingerprint = hashlib.sha256()
    for file in sorted(context.rglob('*')):
        if file.is_file():
            fingerprint.update(str(file.relative_to(context)).encode() + b'\0' + file.read_bytes())
    report['context_sha256'] = fingerprint.hexdigest()
    report['context_from_git_archive_only'] = True
    report['tag'] = 'deploy2-' + BASE[:12] + '-' + fingerprint.hexdigest()[:12]
    report['base_images'] = {name: metadata('docker.io/library/' + name) for name in
                             ('rust:1.88-bookworm', 'debian:bookworm-slim', 'node:22-alpine',
                              'caddy:2.10.2-alpine', 'postgres:17-alpine')}
    report['builder_version'] = capture(['podman', '--version'])
    for service, dockerfile in SERVICES.items():
        tag = 'localhost/golfsiden-' + service + ':' + report['tag']
        exists = subprocess.run(['podman', 'image', 'exists', tag], stdout=subprocess.DEVNULL).returncode
        if exists == 0:
            raise RuntimeError('Refusing to replace an existing candidate tag')
        command = ['podman', 'build', '--pull=never', '--layers', '--force-rm',
                   '--iidfile', str(root / (service + '.iid')), '--tag', tag,
                   '--label', 'io.golfsiden.validation.step=DEPLOY-2',
                   '--label', 'io.golfsiden.source.commit=' + BASE,
                   '--label', 'io.golfsiden.context.sha256=' + report['context_sha256'],
                   '--file', str(context / dockerfile), str(context)]
        started = time.monotonic()
        print('START build ' + service, flush=True)
        with tempfile.TemporaryFile(mode='w+', encoding='utf-8') as log:
            process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
            try:
                code = process.wait(timeout=2400)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.wait(timeout=60)
                raise
            log.seek(0)
            output = log.read()
        diagnostics = [line.strip() for line in output.splitlines()
                  if re.match(r'^\s*(?:error(?:\[E\d+\])?:|Error:|--> )', line)]
        assert not any(re.search(r'postgres(?:ql)?://', line) for line in diagnostics)
        item = {'service': service, 'dockerfile': dockerfile, 'tag': tag,
                'exit_code': code, 'elapsed_seconds': round(time.monotonic() - started, 3),
                'dockerfile_sha256': hashlib.sha256((context / dockerfile).read_bytes()).hexdigest(),
                'safe_build_diagnostics': diagnostics[:30]}
        if code == 0:
            item['image'] = metadata(tag)
            item['iid_matches'] = (root / (service + '.iid')).read_text().strip().removeprefix('sha256:') == item['image']['Id'].removeprefix('sha256:')
            assert item['iid_matches']
        report['builds'].append(item)
        (HERE / (PHASE + '-builds.json')).write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(item), flush=True)
report['temporary_context_and_logs_removed'] = not Path(directory).exists()
report['owned_images_retained_for_rehearsal'] = [item['tag'] for item in report['builds'] if item['exit_code'] == 0]
(HERE / (PHASE + '-builds.json')).write_text(json.dumps(report, indent=2) + '\n')
assert len(report['builds']) == 3 and report['temporary_context_and_logs_removed']
if any(item['exit_code'] for item in report['builds']):
    raise SystemExit('Production Dockerfile build failure retained; no all-image acceptance claimed')

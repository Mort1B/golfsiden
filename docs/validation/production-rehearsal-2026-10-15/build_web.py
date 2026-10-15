"""Build the actual web Dockerfile with only the reviewed Caddyfile overlay."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time

from rehearsal import HERE, REPO, Rehearsal


def main(rehearsal):
    base = rehearsal.run(['git', 'rev-parse', 'HEAD']).stdout.strip()
    overlay = 'deploy/Caddyfile'
    # No other production input may differ from the DEPLOY-2 candidate.
    rehearsal.run(['git', 'diff', '--exit-code', rehearsal.state['source'], '--',
                  'backend', 'frontend', 'migrations', 'vendor', 'deploy', 'scripts',
                  'Cargo.toml', 'Cargo.lock', 'compose.production.yml', '.dockerignore',
                  '.env.production.example', ':!deploy/Caddyfile'])
    with tempfile.TemporaryDirectory(prefix='golf-deploy3-build-') as directory:
        root = Path(directory)
        context = root / 'context'
        context.mkdir()
        with tempfile.TemporaryFile() as archive:
            subprocess.run(['git', 'archive', '--format=tar', base], cwd=REPO, stdout=archive, check=True)
            archive.seek(0)
            with tarfile.open(fileobj=archive) as contents:
                contents.extractall(context, filter='data')
        (context / overlay).write_bytes((REPO / overlay).read_bytes())
        fingerprint = hashlib.sha256()
        for file in sorted(context.rglob('*')):
            if file.is_file():
                fingerprint.update(str(file.relative_to(context)).encode() + b'\0' + file.read_bytes())
        digest = fingerprint.hexdigest()
        tag = f'localhost/golfsiden-web:deploy3-{base[:12]}-{digest[:12]}'
        assert rehearsal.run(['podman', 'image', 'exists', tag], ok=False).returncode != 0
        started = time.monotonic()
        rehearsal.run(['podman', 'build', '--pull=never', '--layers', '--force-rm',
                       '--iidfile', str(root / 'web.iid'), '--tag', tag,
                       '--label', 'io.golfsiden.validation.step=DEPLOY-3',
                       '--label', 'io.golfsiden.source.commit=' + base,
                       '--label', 'io.golfsiden.context.sha256=' + digest,
                       '--file', str(context / 'frontend/Dockerfile.production'), str(context)], timeout=600)
        item = json.loads(rehearsal.run(['podman', 'image', 'inspect', tag]).stdout)[0]
        assert (root / 'web.iid').read_text().strip().removeprefix('sha256:') == item['Id']
        metadata = {key: item.get(key) for key in ('Id', 'Created', 'Architecture', 'RepoDigests')}
        assert metadata['Architecture'] == 'amd64'
        report = {'base_commit': base, 'overlays': {overlay: hashlib.sha256((REPO / overlay).read_bytes()).hexdigest()},
                  'context_sha256': digest, 'tag': tag, 'image': metadata,
                  'actual_dockerfile': 'frontend/Dockerfile.production', 'pull': 'never',
                  'elapsed_seconds': round(time.monotonic() - started, 3), 'exit_code': 0,
                  'context_from_git_archive_plus_only_caddyfile_overlay': True}
    report['temporary_context_removed'] = not Path(directory).exists()
    (HERE / 'web-build.json').write_text(json.dumps(report, indent=2) + '\n')
    override = rehearsal.root / 'override.json'
    config = json.loads(override.read_text())
    config['services']['web']['image'] = tag
    override.write_text(json.dumps(config))
    rehearsal.state['images']['web'] = tag
    rehearsal.state['web_build'] = report
    rehearsal.save()
    rehearsal.receipt('actual_web_dockerfile_rebuilt_with_only_logging_repair')


if __name__ == '__main__':
    main(Rehearsal(sys.argv[1]))

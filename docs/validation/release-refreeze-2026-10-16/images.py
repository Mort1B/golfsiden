"""Inspect and export the already accepted images; no rebuild or tag replacement."""
import hashlib
import json
from pathlib import Path
import subprocess

if not __debug__:
    raise RuntimeError('Optimized Python disables required release validation')

HERE = Path(__file__).resolve().parent
OLD = HERE.parent/'release-freeze-2026-10-16'
builds = json.loads((HERE/'candidate-builds.json').read_text())
previous = json.loads((OLD/'manifest.json').read_text())
work = Path(Path('/tmp/golf-refreeze-work-path').read_text())


def run(args):
    return subprocess.check_output(args, text=True, timeout=240)


def probe(tag, args):
    return run(['podman', 'run', '--rm', '--network', 'none', '--image-volume=ignore',
                '--entrypoint', args[0], tag, *args[1:]])


report = {'source_commit': builds['base_commit'], 'context_sha256': builds['context_sha256'],
          'no_runtime_or_dependency_update_selected': True, 'fresh_advisory_scan_performed': False,
          'images': {}}
for item in builds['builds']:
    service, tag = item['service'], item['tag']
    image = json.loads(run(['podman', 'image', 'inspect', tag]))[0]
    assert image['Id'] == item['image']['Id'] and image['Architecture'] == 'amd64' and image['Os'] == 'linux'
    assert image['Labels']['io.golfsiden.source.commit'] == builds['base_commit']
    assert image['Labels']['io.golfsiden.context.sha256'] == builds['context_sha256']
    prior = next(i for i in previous['images'] if i['service'] == service)
    old = json.loads(run(['podman', 'image', 'inspect', prior['id']]))[0]
    inventory_args = ['dpkg-query', '-W'] if service == 'api' else ['apk', 'info', '-v']
    inventory = probe(tag, inventory_args)
    assert inventory == probe(prior['id'], inventory_args), 'Runtime package set changed'
    (HERE/('candidate-'+service+'-inventory.txt')).write_text(inventory)
    snapshot = {key: image.get(key) for key in ('Id', 'Created', 'Architecture', 'Os', 'Labels', 'RootFS')}
    (HERE/('candidate-image-'+service+'.json')).write_text(json.dumps(snapshot, indent=2)+'\n')
    evidence = {'id': image['Id'], 'previous_id': prior['id'], 'source_labels_verified': True,
                'runtime_packages_identical_to_previous': True,
                'runtime_inventory_sha256': hashlib.sha256(inventory.encode()).hexdigest()}
    if service == 'api':
        args = ['sha256sum', *['/usr/local/bin/'+name for name in ('golf-api', 'migrate', 'password-recovery')]]
        current, before = probe(tag, args), probe(prior['id'], args)
        evidence['executables'] = {line.split()[1]: line.split()[0] for line in current.splitlines()}
        evidence['previous_executables'] = {line.split()[1]: line.split()[0] for line in before.splitlines()}
        assert evidence['executables']['/usr/local/bin/golf-api'] != evidence['previous_executables']['/usr/local/bin/golf-api']
    if service == 'web':
        info = probe(tag, ['/usr/bin/caddy', 'build-info'])
        modules = probe(tag, ['/usr/bin/caddy', 'list-modules'])
        assert '\tgo1.27.2' in info and 'golang.org/x/net\tv0.60.0' in info
        assert 'github.com/caddyserver/caddy/v2\tv2.11.7' in info and 'quic-go\tv0.63.0' in info
        assert 'http.handlers.reverse_proxy' in modules and 'http.handlers.file_server' in modules
        (HERE/'candidate-web-build-info.txt').write_text(info+'\n'+modules)
        evidence['caddy_sha256'] = probe(tag, ['sha256sum', '/usr/bin/caddy']).split()[0]
        assert evidence['caddy_sha256'] == probe(prior['id'], ['sha256sum', '/usr/bin/caddy']).split()[0]
        evidence['caddy_binary_identical_to_previous'] = True
        assets = probe(tag, ['sh', '-c', 'find /srv -type f -exec sha256sum {} + | sort -k 2'])
        old_assets = probe(prior['id'], ['sh', '-c', 'find /srv -type f -exec sha256sum {} + | sort -k 2'])
        assert assets != old_assets
        evidence.update(web_asset_count=len(assets.splitlines()), web_assets_changed=True,
                        assets_listing_sha256=hashlib.sha256(assets.encode()).hexdigest())
        (HERE/'asset-hashes.txt').write_text(assets)
    archive = work/('candidate-'+service+'.tar')
    assert not archive.exists(), 'Refusing to replace an image archive'
    subprocess.run(['podman', 'save', '--format', 'docker-archive', '--output', str(archive), tag], check=True, timeout=240)
    report['images'][service] = evidence
    print(json.dumps({'exported': service, 'runtime_packages_unchanged': True}), flush=True)
(HERE/'image-content.json').write_text(json.dumps(report, indent=2)+'\n')

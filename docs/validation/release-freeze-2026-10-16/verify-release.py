"""Verify a frozen release offline; optionally load images or check private config.

No application/service is started. Run inside the extracted trusted bundle.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
from urllib.parse import unquote, urlparse


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def run(args):
    result = subprocess.run(args,capture_output=True,text=True,timeout=180)
    if result.returncode:
        raise RuntimeError('Docker command failed; inspect the selected engine privately')
    return result.stdout


def verify(root):
    manifest = json.loads((root/'manifest.json').read_text())
    assert manifest['platform']=='linux/amd64' and manifest['schema']==39
    assert len(manifest['images'])==3 and {i['service'] for i in manifest['images']}=={'postgres','api','web'}
    for name,expected in manifest['files'].items():
        file=(root/name).resolve()
        assert file.is_relative_to(root.resolve()) and file.is_file()
        assert digest(file)==expected,'Release file checksum mismatch'
    for item in manifest['images']:
        archive=root/item['archive']
        assert archive.resolve().is_relative_to(root.resolve()) and digest(archive)==item['archive_sha256']
        with tarfile.open(archive) as tar:
            records=json.load(tar.extractfile('manifest.json'))
            assert len(records)==1
            assert records[0]['RepoTags']==[item['source_tag']],'Unexpected image archive tags'
            config=tar.extractfile(records[0]['Config']).read()
            assert hashlib.sha256(config).hexdigest()==item['id']
            metadata=json.loads(config)
            assert metadata['os']=='linux' and metadata['architecture']=='amd64'
        assert item['alias']=='golfsiden-'+item['service']+':'+manifest['tag']
    return manifest


def check_environment(root,manifest,environment,project):
    # Capture rather than print the rendered config: it contains private secrets.
    config=json.loads(run(['docker','compose','--env-file',str(environment),'-f',str(root/'compose.production.yml'),
                          '--profile','tools','config','--format','json']))
    expected={item['service']:item['alias'] for item in manifest['images']}
    assert config['name']==project,'Compose project differs from the explicitly expected identity'
    expected.update(migrate=expected['api'],permissions=expected['postgres'])
    for name,alias in expected.items():
        assert config['services'][name]['image']==alias,'Private env selects another image tag'
    pg=config['services']['postgres']['environment']
    assert pg['APP_DATABASE_USER']!=pg['POSTGRES_USER']
    for service,user in [('api',pg['APP_DATABASE_USER']),('migrate',pg['POSTGRES_USER'])]:
        url=urlparse(config['services'][service]['environment']['DATABASE_URL'])
        assert url.scheme in ('postgres','postgresql') and url.hostname=='postgres' and url.port==5432
        assert unquote(url.username or '')==user and unquote(url.path.lstrip('/'))==pg['POSTGRES_DB']
        password=pg['APP_DATABASE_PASSWORD' if service=='api' else 'POSTGRES_PASSWORD']
        assert unquote(url.password or '')==password,'Database URL password differs from the configured role'
    assert config['networks']['data']['internal']
    assert not config['services']['postgres'].get('ports') and not config['services']['api'].get('ports')
    assert config['services']['api']['environment']['RUN_MIGRATIONS']=='false'
    assert config['services']['api']['environment']['SESSION_COOKIE_SECURE']=='true'
    print('Private script-only Compose configuration selects the frozen images and local database roles.')


def load_images(root,manifest):
    info=json.loads(run(['docker','info','--format','{{json .}}']))
    architecture=info.get('Architecture',info.get('arch',''))
    os_type=info.get('OSType',info.get('os',''))
    assert os_type=='linux' and architecture in ('amd64','x86_64'),'This bundle requires a linux/amd64 engine'
    for item in manifest['images']:
        # Loading also restores archive tags; protect both source and final aliases.
        for alias in (item['source_tag'],item['alias']):
            prior=subprocess.run(['docker','image','inspect',alias],capture_output=True,text=True)
            if prior.returncode==0:
                assert json.loads(prior.stdout)[0]['Id'].removeprefix('sha256:')==item['id'],'Refusing to replace a different existing release alias'
    for item in manifest['images']:
        run(['docker','load','--input',str(root/item['archive'])])
        actual=json.loads(run(['docker','image','inspect','sha256:'+item['id']]))[0]
        assert actual['Id'].removeprefix('sha256:')==item['id']
        run(['docker','tag','sha256:'+item['id'],item['alias']])
        assert json.loads(run(['docker','image','inspect',item['alias']]))[0]['Id'].removeprefix('sha256:')==item['id']
    print('Three frozen images loaded and exact unqualified release aliases verified. No services started.')


def main():
    if sys.flags.optimize:
        raise RuntimeError('Optimized Python disables required validation')
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',nargs='?',default=str(Path(__file__).resolve().parent))
    parser.add_argument('--load',action='store_true')
    parser.add_argument('--check-env',type=Path)
    parser.add_argument('--project',default='golfsiden',help='Expected project; set COMPOSE_PROJECT_NAME consistently')
    args=parser.parse_args();root=Path(args.directory).resolve()
    manifest=verify(root)
    print('Release archives, image config IDs, platform and runtime-file checksums verified.')
    if args.check_env:check_environment(root,manifest,args.check_env.resolve(),args.project)
    if args.load:load_images(root,manifest)


if __name__=='__main__':
    try:main()
    except Exception:
        print('Release verification failed; no deployment was attempted.',file=sys.stderr)
        sys.exit(1)

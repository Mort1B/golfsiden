"""Exercise handoff on an owned Podman API; starts no application services."""
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tempfile
import time

HERE=Path(__file__).resolve().parent
receipt=json.loads((HERE/'bundle.json').read_text())
release=Path(receipt['directory'])
manifest=json.loads((release/'manifest.json').read_text())
root=Path(tempfile.mkdtemp(prefix='golf-deploy6-transfer-'));root.chmod(0o700)
socket=root/'podman.sock'
env=os.environ.copy()
for key in re.findall(r'\$\{([A-Z][A-Z0-9_]*)',(release/'compose.production.yml').read_text()):env.pop(key,None)
for key in ('DOCKER_CONTEXT','COMPOSE_PROJECT_NAME'):env.pop(key,None)
env['DOCKER_HOST']='unix://'+str(socket)
checks={};created=[]


def run(args,expected=0):
    result=subprocess.run(args,env=env,capture_output=True,text=True,timeout=180)
    with (root/'diagnostics').open('a') as log:log.write(result.stdout+result.stderr)
    assert result.returncode==expected,'Handoff control failed; inspect private diagnostics'
    return result


process=subprocess.Popen(['podman','system','service','--time=0','unix://'+str(socket)],
                         stdout=(root/'api.log').open('w'),stderr=subprocess.STDOUT)
try:
    for attempt in range(100):
        if socket.exists():break
        assert process.poll() is None
        time.sleep(.1)
    verify=['python',str(release/'verify-release.py')]
    run(['python','-O',str(release/'verify-release.py'),'--load'],1)
    checks['optimized_python_rejected_before_docker_load']=True
    run(verify);checks['offline_archives_config_ids_and_runtime_checksums']=True
    file=release/'compose.release.json';original=file.read_bytes()
    try:
        file.write_bytes(original+b' ');run(verify,1)
    finally:file.write_bytes(original)
    checks['changed_runtime_file_rejected']=True
    for item in manifest['images']:
        existing=subprocess.run(['docker','image','inspect',item['alias']],env=env,capture_output=True)
        assert existing.returncode!=0,'Only absent test aliases may be created'
    first,other=manifest['images'][:2]
    run(['docker','tag','sha256:'+other['id'],first['alias']]);created.append(first['alias'])
    run(verify+['--load'],1)
    actual=json.loads(run(['docker','image','inspect',first['alias']]).stdout)[0]
    assert actual['Id'].removeprefix('sha256:')==other['id']
    run(['docker','image','rm',first['alias']]);created.clear()
    checks['conflicting_existing_alias_rejected_before_load_without_replacement']=True
    created.extend(item['alias'] for item in manifest['images'])
    run(verify+['--load']);checks['real_archive_load_and_three_explicit_alias_ids']=True
    owner,app=secrets.token_urlsafe(32),secrets.token_urlsafe(32)
    values={'GOLFSIDEN_IMAGE_TAG':manifest['tag'],'SITE_ADDRESS':'golf.example.com',
            'RESET_PASSWORD_ORIGIN':'https://golf.example.com','POSTGRES_DB':'transfer',
            'POSTGRES_OWNER_USER':'transfer_owner','POSTGRES_OWNER_PASSWORD':owner,
            'APP_DATABASE_USER':'transfer_app','APP_DATABASE_PASSWORD':app,
            'MIGRATION_DATABASE_URL':f'postgres://transfer_owner:{owner}@postgres:5432/transfer',
            'APP_DATABASE_URL':f'postgres://transfer_app:{app}@postgres:5432/transfer',
            'PROXY_SHARED_SECRET':secrets.token_urlsafe(32)}
    private=root/'private.env'
    def write_env():
        private.write_text(''.join(f'{key}={value}\n' for key,value in values.items()));private.chmod(0o600)
    write_env();run(verify+['--check-env',str(private)])
    checks['script_only_config_all_five_aliases_roles_and_local_db_bound']=True
    env['COMPOSE_PROJECT_NAME']='golf-release-restore-check'
    run(verify+['--check-env',str(private)],1)
    run(verify+['--check-env',str(private),'--project',env['COMPOSE_PROJECT_NAME']])
    env.pop('COMPOSE_PROJECT_NAME')
    checks['explicit_custom_project_bound_and_unexpected_project_rejected']=True
    values['GOLFSIDEN_IMAGE_TAG']='wrong-release';write_env();run(verify+['--check-env',str(private)],1)
    checks['wrong_env_tag_rejected_without_private_config_output']=True
    values['GOLFSIDEN_IMAGE_TAG']=manifest['tag'];values['APP_DATABASE_URL']=f'postgres://transfer_app:{owner}@postgres:5432/transfer'
    write_env();run(verify+['--check-env',str(private)],1)
    checks['inconsistent_database_url_password_rejected']=True
finally:
    for alias in created:run(['docker','image','rm',alias])
    process.terminate();process.wait(timeout=10)
    shutil.rmtree(root)
checks['owned_aliases_api_socket_private_config_and_diagnostics_removed']=True
(HERE/'transfer.json').write_text(json.dumps({'checks':checks,'engine':'rootless Podman Docker-compatible API',
    'native_docker_engine_acceptance':False,'application_services_started':False,
    'bundle_sha256':receipt['bundle_sha256'],'manifest_sha256':receipt['manifest_sha256']},indent=2)+'\n')
print(json.dumps(checks))

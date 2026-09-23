from pathlib import Path
import os, subprocess,hashlib,json
root=Path('/tmp/golf-rehearsal-738ec1d');env=dict(os.environ,DOCKER_HOST=f'unix://{root}/podman.sock',COMPOSE_PROJECT_NAME='golf-rehearsal-restore-738ec1d')
script=['scripts/restore-production.sh',str(root/'restore.env')]
def run(label,argv,expected,environment=env,cwd=None):
 p=subprocess.run(argv,env=environment,cwd=cwd,capture_output=True,text=True)
 print(label+': exit '+str(p.returncode)); print((p.stdout+p.stderr)[-2000:])
 assert p.returncode==expected, label
 return p
run('Documented checksum command from checkout',['sha256sum','--check',str(root/'source.dump.sha256')],1)
run('Checksum from dump directory',['sha256sum','--check','source.dump.sha256'],0,cwd=root)
run('Missing restore confirmation',script+[str(root/'source.dump')],2)
env['CONFIRM_EMPTY_RESTORE']='RESTORE_TO_EMPTY_DATABASE'
data=(root/'source.dump').read_bytes()
(root/'bad-checksum.dump').write_bytes(data+b'corruption')
(root/'bad-checksum.dump.sha256').write_text(hashlib.sha256(data).hexdigest()+'  bad-checksum.dump\n')
run('Mismatched checksum',script+[str(root/'bad-checksum.dump')],1)
(root/'truncated.dump').write_bytes(data[:len(data)//2])
(root/'truncated.dump.sha256').write_text(hashlib.sha256(data[:len(data)//2]).hexdigest()+'  truncated.dump\n')
run('Valid checksum but truncated archive',script+[str(root/'truncated.dump')],1)
q="SELECT (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public');"
p=run('Empty schema after failed transaction',['podman','exec','golf-rehearsal-restore-738ec1d-postgres-1','psql','-U','golfsiden_owner','-d','golfsiden','-Atc',q],0)
assert p.stdout.strip()=='0'
run('Successful restore',script+[str(root/'source.dump')],0)
run('Repeat restore rejects populated schema',script+[str(root/'source.dump')],1)

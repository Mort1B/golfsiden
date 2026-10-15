"""Synthetic restore/rollback acceptance; owns one container with no host ports.
The Docker shim delegates only PostgreSQL commands to that exact owned Podman
container. Service stop and runtime-permissions actions are observed, simulated.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
EMPTY = "SELECT (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public');"
SHIM = '''#!{python}
import json,os,re,subprocess,sys
args=sys.argv[1:]
assert args[:5]==['compose','--env-file',os.environ['A5_ENV_FILE'],'-f',os.environ['A5_COMPOSE_FILE']]
if args[5:11]==['exec','-T','postgres','sh','-eu','-c']:
 command=args[11]
 if 'pg_restore' in command: action='restore'
 elif 'pg_dump' in command: action='backup'
 elif 'psql' in command: action='empty-target'
 else: raise SystemExit(95)
elif args[5:]==['stop','web','api']: action='stop'
elif args[5:]==['--profile','tools','run','--rm','permissions']: action='permissions'
else: raise SystemExit(96)
with open(os.environ['A5_CALLS'],'a') as output: output.write(json.dumps({{'action':action}})+'\\n')
if action in ['stop','permissions']: raise SystemExit(0)
database=os.environ['A5_DATABASE'];assert re.fullmatch('a5_[a-z]+',database)
result=subprocess.run(['podman','exec','-i','-e','POSTGRES_DB='+database,os.environ['A5_CONTAINER_ID'],'sh','-eu','-c',command],stdin=sys.stdin.buffer)
raise SystemExit(result.returncode)
'''


def run(args, expected=0, data=None, **kwargs):
    result = subprocess.run(args, input=data, capture_output=True, timeout=60, **kwargs)
    assert result.returncode == expected, (args[0], result.returncode, result.stderr[-2000:])
    return result


def main():
    temporary = Path(tempfile.mkdtemp(prefix='golf-a5-postgres-'))
    cidfile = temporary / 'container.cid'
    try:
        run(['podman','create','--cidfile',str(cidfile),
             '--name',temporary.name,'-e','POSTGRES_USER=golf',
             '-e','POSTGRES_PASSWORD=golf','-e','POSTGRES_DB=golf',
             '--tmpfs','/var/lib/postgresql/data:rw',
             'docker.io/library/postgres:17-alpine'])
        cid = cidfile.read_text().strip()
        run(['podman','start',cid])
        deadline = time.monotonic() + 30
        while subprocess.run(['podman','exec',cid,'pg_isready','-h','127.0.0.1','-U','golf'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
            if time.monotonic() > deadline:
                raise RuntimeError('synthetic PostgreSQL did not start')
            time.sleep(0.1)
        def sql(database, statement):
            return run(['podman','exec',cid,'psql','-X','-U','golf','-d',database,'-At','-v','ON_ERROR_STOP=1','-c',statement]).stdout.decode().strip()
        for database in ['a5_source','a5_target','a5_broken','a5_verbose']:
            run(['podman','exec',cid,'createdb','-U','golf',database])
        sql('a5_source',"CREATE TABLE a5_first(id integer PRIMARY KEY); INSERT INTO a5_first VALUES(1); CREATE TABLE a5_later(id integer PRIMARY KEY,payload text NOT NULL); INSERT INTO a5_later SELECT n,repeat(md5(n::text),4) FROM generate_series(1,3000)n;")
        binary = temporary / 'bin'
        binary.mkdir()
        docker = binary / 'docker'
        docker.write_text(SHIM.format(python=sys.executable))
        docker.chmod(0o700)
        environment_file = temporary / 'synthetic runtime.env'
        environment_file.write_text('# synthetic fixture; no production configuration\n')
        log = temporary / 'calls.jsonl'
        environment = dict(os.environ, PATH=str(binary)+os.pathsep+os.environ['PATH'],
                           A5_ENV_FILE=str(environment_file),A5_COMPOSE_FILE=str(ROOT/'compose.production.yml'),
                           A5_CALLS=str(log),A5_CONTAINER_ID=cid,
                           CONFIRM_EMPTY_RESTORE='RESTORE_TO_EMPTY_DATABASE')
        def invoke(script, archive, database, expected=0):
            log.unlink(missing_ok=True)
            result=run(['/bin/sh',str(ROOT/'scripts'/script),str(environment_file),str(archive)],
                       expected,env=dict(environment,A5_DATABASE=database))
            actions=[json.loads(line)['action'] for line in log.read_text().splitlines()] if log.exists() else []
            return result, actions
        staging = temporary / 'staging with spaces'
        staging.mkdir()
        archive = staging / 'source snapshot.dump'
        _, actions = invoke('backup-production.sh',archive,'a5_source')
        assert actions == ['backup']
        expected = subprocess.check_output(['sha256sum','--',archive.name],cwd=staging)
        assert Path(str(archive)+'.sha256').read_bytes() == expected
        moved = temporary / 'moved pair'
        moved.mkdir()
        for source in [archive,Path(str(archive)+'.sha256')]:
            shutil.move(source,moved/source.name)
        archive = moved / archive.name
        _, actions = invoke('restore-production.sh',archive,'a5_target')
        assert actions == ['empty-target','stop','restore','permissions']
        assert sql('a5_target','SELECT count(*) FROM a5_first') == '1'
        assert sql('a5_target','SELECT count(*) FROM a5_later') == '3000'
        assert sql('a5_source',"SELECT md5(string_agg(id::text||payload,',' ORDER BY id)) FROM a5_later") == sql('a5_target',"SELECT md5(string_agg(id::text||payload,',' ORDER BY id)) FROM a5_later")
        _, actions = invoke('restore-production.sh',archive,'a5_target',1)
        assert actions == ['empty-target']
        broken = moved / 'late failure.dump'
        broken.write_bytes(archive.read_bytes()[:-1024])
        Path(str(broken)+'.sha256').write_bytes(subprocess.check_output(['sha256sum','--',broken.name],cwd=moved))
        run(['podman','exec','-i',cid,'pg_restore','--list'],data=broken.read_bytes())
        result, actions = invoke('restore-production.sh',broken,'a5_broken',1)
        assert actions == ['empty-target','stop','restore']
        assert b'Restore complete' not in result.stdout
        assert sql('a5_broken',EMPTY) == '0'
        verbose=run(['podman','exec','-i',cid,'pg_restore','--username=golf','--dbname=a5_verbose','--exit-on-error','--single-transaction','--no-owner','--no-privileges','--verbose'],1,data=broken.read_bytes())
        assert b'creating TABLE ' in verbose.stderr and b'processing data for table' in verbose.stderr
        assert sql('a5_verbose',EMPTY) == '0'
        result={'backup_actual_custom_format_and_canonical_sidecar':True,
                'relocated_spaced_pair_restored':True,'rows_and_ordered_data_digest_match':True,
                'populated_target_denied_before_stop':True,
                'late_truncated_archive_has_complete_toc':True,
                'actual_wrapper_failure_rolls_back_to_zero_public_objects':True,
                'failed_restore_does_not_invoke_permissions':True,
                'verbose_control_confirms_table_and_data_work_before_rollback':True,
                'runtime_grants_and_service_stop':'shimmed; only invocation/order checked',
                'version':sql('golf','SELECT version()')}
        print(json.dumps(result,indent=2),flush=True)
    finally:
        try:
            if cidfile.is_file():
                cid=cidfile.read_text().strip()
                assert len(cid)==64 and all(c in '0123456789abcdef' for c in cid)
                run(['podman','rm','-f',cid])
        finally:
            shutil.rmtree(temporary)
        print('CLEANUP owned container and temporary synthetic archives removed',file=sys.stderr,flush=True)


if __name__ == '__main__':
    main()

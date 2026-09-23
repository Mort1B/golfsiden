import subprocess,json
from pathlib import Path
root=Path('/tmp/golf-rehearsal-738ec1d'); container='golf-rehearsal-restore-738ec1d-postgres-1'
def run(args,expected=0,data=None):
 p=subprocess.run(['podman','exec','-i',container]+args,input=data,capture_output=True,text=True)
 print((p.stdout+p.stderr).strip());assert p.returncode==expected
 return p.stdout.strip()
runtime=['sh','-c','export PGPASSWORD="$APP_DATABASE_PASSWORD"; exec psql -h 127.0.0.1 -U "$APP_DATABASE_USER" -d "$POSTGRES_DB" -X -At -v ON_ERROR_STOP=1']
run(runtime,data="SELECT current_user,rolsuper,rolcreatedb,rolcreaterole,has_schema_privilege(current_user,'public','CREATE'),has_table_privilege(current_user,'_sqlx_migrations','SELECT'),has_table_privilege(current_user,'_sqlx_migrations','UPDATE') FROM pg_roles WHERE rolname=current_user;")
run(runtime,3,'BEGIN; CREATE TABLE public.rehearsal_forbidden(id int); ROLLBACK;')
run(runtime,3,'BEGIN; UPDATE _sqlx_migrations SET success=success WHERE false; ROLLBACK;')
print('Runtime schema creation and migration-history mutation denied.')
run(['createdb','-U','golfsiden_owner','recovery_rollback_probe'])
data=(root/'source.dump').read_bytes()[:-1024]
p=subprocess.run(['podman','exec','-i',container,'pg_restore','--list'],input=data,capture_output=True)
assert p.returncode==0;print('Late-truncated archive has a readable complete table of contents.')
p=subprocess.run(['podman','exec','-i',container,'pg_restore','--username=golfsiden_owner','--dbname=recovery_rollback_probe','--exit-on-error','--single-transaction','--no-owner','--no-privileges','--verbose'],input=data,capture_output=True)
assert p.returncode==1
log=p.stderr.decode();(root/'late-rollback-private.log').write_text(log)
assert 'creating TABLE ' in log and 'processing data for table' in log
print('Restore executed CREATE TABLE and table data before the injected late EOF failure.')
q="SELECT (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')+(SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public');"
assert run(['psql','-U','golfsiden_owner','-d','recovery_rollback_probe','-Atc',q])=='0'
run(['dropdb','-U','golfsiden_owner','recovery_rollback_probe'])
print('Late failure rolled back all public objects; task-owned probe database removed.')

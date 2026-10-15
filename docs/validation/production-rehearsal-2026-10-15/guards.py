"""Exercise real production startup, PostgreSQL authority and live readiness."""
import json
from pathlib import Path
import sys

from rehearsal import Rehearsal


def startup(rehearsal, name, expected, environment=None):
    override = rehearsal.root / 'probe.json'
    override.write_text(json.dumps({'services': {'api': {'environment': environment or {}}}}))
    try:
        result = rehearsal.compose('run', '--rm', '-T', '--no-deps', '--name',
                                   rehearsal.state['project'] + '-probe-' + name,
                                   'api', extra=override, timeout=30, ok=False)
        assert result.returncode != 0 and expected in result.stdout + result.stderr, name
        assert 'golf API listening' not in result.stdout + result.stderr, name
        rehearsal.receipt('startup_rejects_' + name)
    finally:
        override.unlink(missing_ok=True)


def main(rehearsal):
    # These credentials are exactly the ones in the running API container.
    assert rehearsal.sql("SELECT current_user,session_user;", runtime=True) == 'rehearsal_app|rehearsal_app'
    flags = rehearsal.sql("""SELECT rolsuper,rolcreatedb,rolcreaterole,rolreplication,rolbypassrls,
        has_schema_privilege(current_user,'public','CREATE'),
        pg_has_role(current_user,'rehearsal_owner','MEMBER') FROM pg_roles WHERE rolname=current_user;""", runtime=True)
    assert flags == 'f|f|f|f|f|f|f'
    assert rehearsal.sql("SELECT count(*) FROM pg_class WHERE relnamespace='public'::regnamespace AND pg_has_role(current_user,relowner,'MEMBER');", runtime=True) == '0'
    for query in ('CREATE TABLE public.forbidden_probe(id int);',
                  'ALTER TABLE public.users ADD COLUMN forbidden_probe int;',
                  'UPDATE public._sqlx_migrations SET success=success WHERE version=39;'):
        result = rehearsal.sql(query, runtime=True, ok=False)
        assert result.returncode != 0 and '42501' in result.stderr
    rehearsal.receipt('actual_runtime_login_restricted_and_ddl_history_writes_denied')
    # Existing API session/fixture writes subsequently prove representative DML.
    rehearsal.compose('--profile', 'tools', 'run', '--rm', '-T', '--no-deps', 'migrate')
    rehearsal.receipt('matching_migration_rerun_idempotent')
    startup(rehearsal, 'wrong_identity', 'UnexpectedRole',
            {'DATABASE_URL': rehearsal.state['values']['MIGRATION_DATABASE_URL']})
    startup(rehearsal, 'owner_authority', 'ExcessPrivileges',
            {'DATABASE_URL': rehearsal.state['values']['MIGRATION_DATABASE_URL'],
             'APP_DATABASE_USER': 'rehearsal_owner'})
    rehearsal.sql('GRANT CREATE ON SCHEMA public TO rehearsal_app;')
    try:
        startup(rehearsal, 'schema_create', 'ExcessPrivileges')
    finally:
        rehearsal.sql('REVOKE CREATE ON SCHEMA public FROM rehearsal_app;')
    rehearsal.sql('GRANT UPDATE ON _sqlx_migrations TO rehearsal_app;')
    try:
        startup(rehearsal, 'history_write', 'MigrationHistoryWritable')
    finally:
        rehearsal.sql('REVOKE UPDATE ON _sqlx_migrations FROM rehearsal_app;')
    # Preserve full history in a private owner-only table for exact restoration.
    rehearsal.sql('CREATE TABLE deploy3_history_backup AS TABLE _sqlx_migrations; REVOKE ALL ON deploy3_history_backup FROM rehearsal_app;')
    cases = [
        ('pending', 'DELETE FROM _sqlx_migrations WHERE version=39;', 'Pending'),
        ('dirty', 'UPDATE _sqlx_migrations SET success=false WHERE version=39;', 'Dirty'),
        ('checksum', "UPDATE _sqlx_migrations SET checksum=decode('00','hex') WHERE version=39;", 'ChecksumMismatch'),
        ('unknown', 'INSERT INTO _sqlx_migrations SELECT 999,description,installed_on,success,checksum,execution_time FROM deploy3_history_backup WHERE version=39;', 'Unknown'),
        ('missing_history', 'ALTER TABLE _sqlx_migrations RENAME TO deploy3_hidden_history;', 'MissingHistory'),
    ]
    try:
        for name, mutation, expected in cases:
            rehearsal.sql(mutation)
            try:
                assert rehearsal.status('/api/health') == '200'
                rehearsal.wait_status('/api/ready', 503)
                startup(rehearsal, name, expected)
                response = rehearsal.run(['curl', '--silent', '--insecure', rehearsal.state['origin'] + '/api/ready']).stdout
                body = json.loads(response)
                assert body == {'error': {'code': 'service_unavailable', 'message': 'service is not ready'}}
                rehearsal.receipt('live_schema_' + name + '_ready503_health200_generic_error')
            finally:
                if name == 'missing_history':
                    rehearsal.sql('ALTER TABLE deploy3_hidden_history RENAME TO _sqlx_migrations;')
                else:
                    rehearsal.sql('TRUNCATE _sqlx_migrations; INSERT INTO _sqlx_migrations SELECT * FROM deploy3_history_backup;')
                rehearsal.wait_status('/api/ready', 200)
    finally:
        rehearsal.sql('DROP TABLE deploy3_history_backup;')
    rehearsal.compose('stop', '--timeout', '10', 'postgres')
    try:
        assert rehearsal.status('/api/health') == '200'
        # Pool acquisition has a bounded 30-second timeout, so allow curl longer.
        result = rehearsal.run(['curl', '--silent', '--insecure', '--max-time', '40',
                                '--write-out', '\n%{http_code}', rehearsal.state['origin'] + '/api/ready'])
        assert result.stdout.endswith('\n503')
        rehearsal.receipt('database_unavailable_ready503_health200')
    finally:
        rehearsal.compose('up', '-d', '--no-build', '--pull', 'never', '--wait', '--wait-timeout', '90', 'postgres')
        rehearsal.wait_status('/api/ready', 200, timeout=45)
    rehearsal.receipt('database_reconnect_restores_readiness')
    assert rehearsal.sql('SELECT count(*),max(version),bool_and(success) FROM _sqlx_migrations;', runtime=True) == '39|39|t'
    rehearsal.receipt('schema_and_restricted_runtime_restored_after_controls')


if __name__ == '__main__':
    rehearsal = Rehearsal(sys.argv[1])
    try:
        main(rehearsal)
    except Exception as error:
        import traceback
        with (rehearsal.root / 'failure.log').open('a') as log:
            traceback.print_exc(file=log)
        print(json.dumps({'failed': type(error).__name__, 'detail': 'guard harness failure; inspect private diagnostics'}))
        sys.exit(1)

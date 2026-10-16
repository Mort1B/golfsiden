"""Real production scripts, two owned Compose projects, current application data.

Secrets, archives, raw responses and diagnostics remain in private /tmp trees.
Failures preserve controls for explicit owned cleanup; no global prune is used.
"""
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import traceback

SCRIPT_DIR = Path(__file__).resolve().parent
HERE = Path(os.environ.get('GOLF_RECOVERY_EVIDENCE_DIRECTORY', SCRIPT_DIR))
REPO = SCRIPT_DIR.parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(SCRIPT_DIR.parent / 'production-rehearsal-2026-10-15'))
from rehearsal import Rehearsal

POINTER = Path('/tmp/golf-deploy5-controls.json')
EMPTY = """SELECT (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')
 + (SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')
 + (SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public');"""
SNAPSHOT = """CREATE TEMP TABLE recovery_fingerprints(name text, rows bigint, sha256 text);
DO $snapshot$ DECLARE item record; BEGIN
 FOR item IN SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename LOOP
  EXECUTE format('INSERT INTO pg_temp.recovery_fingerprints SELECT %L,count(*),encode(sha256(convert_to(coalesce(string_agg(row_to_json(t)::text,E''\\n'' ORDER BY row_to_json(t)::text),''''),''UTF8'')),''hex'') FROM public.%I t',item.tablename,item.tablename);
 END LOOP;
END $snapshot$;
CREATE TEMP TABLE recovery_sequence_state(name text,last_value bigint,is_called boolean);
DO $snapshot$ DECLARE item record; BEGIN
 FOR item IN SELECT sequencename FROM pg_sequences WHERE schemaname='public' ORDER BY sequencename LOOP
  EXECUTE format('INSERT INTO pg_temp.recovery_sequence_state SELECT %L,last_value,is_called FROM public.%I',item.sequencename,item.sequencename);
 END LOOP;
END $snapshot$;
SELECT coalesce(json_object_agg(name,json_build_object('rows',rows,'sha256',sha256) ORDER BY name),'{}') FROM recovery_fingerprints;
SELECT json_build_object('metadata',(SELECT coalesce(json_agg(row_to_json(s) ORDER BY sequencename),'[]') FROM pg_sequences s WHERE schemaname='public'),
 'state',(SELECT coalesce(json_agg(row_to_json(s) ORDER BY name),'[]') FROM recovery_sequence_state s));"""


def private_json(path, value):
    path.write_text(json.dumps(value))
    path.chmod(0o600)


def new_fixture(role, tag):
    root = Path(tempfile.mkdtemp(prefix='golf-deploy3-recovery-' + role + '-'))
    root.chmod(0o700)
    with (root / 'socket.log').open('w') as log:
        process = subprocess.Popen(['podman', 'system', 'service', '--time=0', 'unix://' + str(root / 'podman.sock')],
                                   stdout=log, stderr=log, start_new_session=True)
    control = root / 'control.json'
    private_json(control, {'pid': process.pid, 'directory': str(root), 'socket': str(root / 'podman.sock')})
    ownership = json.loads(POINTER.read_text())
    ownership['controls'][role] = str(control)
    private_json(POINTER, ownership)
    for _ in range(50):
        if (root / 'podman.sock').exists():
            break
        time.sleep(.1)
    r = Rehearsal(control)
    r.setup(migrate=role == 'source')
    # Script-only Compose calls do not use the fixture's image override. Exact
    # owned aliases let the unchanged real scripts resolve the tested artifacts.
    values = r.state['values']
    values['GOLFSIDEN_IMAGE_TAG'] = tag
    r.env.update(values)
    for key in list(r.env):
        if key.startswith('COMPOSE_') or key in ('DOCKER_CONTEXT', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'):
            r.env.pop(key, None)
    r.env['COMPOSE_PROJECT_NAME'] = r.state['project']
    (root / 'production.env').write_text(''.join(f'{key}={value}\n' for key, value in values.items()))
    (root / 'production.env').chmod(0o600)
    r.save()
    return r


def script_config(r):
    args = ['docker', 'compose', '--env-file', str(r.root / 'production.env'), '-f', str(REPO / 'compose.production.yml')]
    config = json.loads(r.run(args + ['--profile', 'tools', 'config', '--format', 'json']).stdout)
    assert config['name'] == r.state['project'] and config['networks']['data']['internal']
    assert config['services']['api']['environment']['DATABASE_URL'] == r.state['values']['APP_DATABASE_URL']
    assert config['services']['migrate']['environment']['DATABASE_URL'] == r.state['values']['MIGRATION_DATABASE_URL']
    for service in ('api', 'postgres'):
        assert not config['services'][service].get('ports')
    assert all(p['host_ip'] == '127.0.0.1' for p in config['services']['web']['ports'])
    for service, candidate in r.state['images'].items():
        actual = json.loads(r.run(['docker', 'image', 'inspect', config['services'][service]['image']]).stdout)[0]
        expected = json.loads(r.run(['docker', 'image', 'inspect', candidate]).stdout)[0]
        assert actual['Id'] == expected['Id']
    ident = r.compose('ps', '-q', 'postgres').stdout.strip()
    actual = json.loads(r.run(['docker', 'inspect', ident]).stdout)[0]
    assert actual['Config']['Labels']['com.docker.compose.project'] == r.state['project']
    r.receipt('script_only_compose_config_bound_to_owned_project_and_exact_images')


def script(r, name, archive, *, confirmed=True, ok=True):
    env = r.env.copy()
    env.pop('CONFIRM_EMPTY_RESTORE', None)
    if confirmed:
        env['CONFIRM_EMPTY_RESTORE'] = 'RESTORE_TO_EMPTY_DATABASE'
    return r.run([str(REPO / 'scripts' / name), str(r.root / 'production.env'), str(archive)], env=env, ok=ok)


def snapshot(r):
    output = r.sql(SNAPSHOT).splitlines()
    return {'tables': json.loads(output[-2]), 'sequences': json.loads(output[-1])}


def binary_archive_command(r, archive, *args):
    ident = r.compose('ps', '-q', 'postgres').stdout.strip()
    with archive.open('rb') as stream:
        result = subprocess.run(['docker','exec','-i',ident,'pg_restore',*args], stdin=stream,
                                capture_output=True, env=r.env, cwd=REPO, timeout=90)
    with (r.root/'private-archive.log').open('ab') as log:
        log.write(result.stdout+result.stderr)
    return result


def check_roles(r):
    assert r.sql('SELECT current_user,session_user;', runtime=True) == 'rehearsal_app|rehearsal_app'
    assert r.sql("""SELECT rolsuper,rolcreatedb,rolcreaterole,rolreplication,rolbypassrls,
      has_schema_privilege(current_user,'public','CREATE'),pg_has_role(current_user,'rehearsal_owner','MEMBER')
      FROM pg_roles WHERE rolname=current_user;""", runtime=True) == 'f|f|f|f|f|f|f'
    assert r.sql("SELECT count(*) FROM pg_class WHERE relnamespace='public'::regnamespace AND pg_has_role(current_user,relowner,'MEMBER');", runtime=True) == '0'
    assert r.sql('SELECT count(*),max(version),bool_and(success) FROM _sqlx_migrations;', runtime=True) == '39|39|t'
    for query in ('CREATE TABLE public.forbidden_recovery_probe(id int);',
                  'UPDATE public._sqlx_migrations SET success=success WHERE version=39;'):
        result = r.sql(query, runtime=True, ok=False)
        assert result.returncode != 0 and '42501' in result.stderr
    r.receipt('restored_actual_runtime_identity_schema39_and_ddl_history_denials')


def cleanup():
    ownership = json.loads(POINTER.read_text())
    destination = Path(ownership.get('evidence_directory', SCRIPT_DIR))
    for role, control in ownership['controls'].items():
        if Path(control).exists():
            Rehearsal(control).cleanup(destination / (role + '-cleanup.json'))
    for alias in ownership['aliases']:
        subprocess.run(['podman', 'image', 'rm', alias], check=True, capture_output=True)
    for alias in ownership['aliases']:
        assert subprocess.run(['podman', 'image', 'exists', alias]).returncode == 1
    (destination / 'ownership.json').write_text(json.dumps({'private_roots': [str(Path(p).parent) for p in ownership['controls'].values()],
        'owned_aliases': ownership['aliases'], 'owned_aliases_removed': True, 'private_roots_removed': True}, indent=2) + '\n')
    POINTER.unlink()


def main():
    assert not POINTER.exists(), 'Refusing to replace an existing recovery fixture'
    tag = 'deploy5-' + secrets.token_hex(5)
    (HERE/'sequence.json').write_text(json.dumps({'complete':False,'run_id':tag})+'\n')
    private_json(POINTER, {'controls': {}, 'aliases': [], 'evidence_directory': str(HERE.resolve())})
    source = new_fixture('source', tag)
    for service, candidate in source.state['images'].items():
        alias = 'localhost/golfsiden-' + service + ':' + tag
        assert source.run(['docker', 'image', 'inspect', alias], ok=False).returncode != 0
        source.run(['docker', 'image', 'tag', candidate, alias])
        ownership = json.loads(POINTER.read_text()); ownership['aliases'].append(alias); private_json(POINTER, ownership)
    script_config(source)
    source.start()
    evidence = source.root / 'source-browser'
    evidence.mkdir(mode=0o700)
    env = source.env.copy(); env['GOLF_CRITICAL_EVIDENCE_DIRECTORY'] = str(evidence)
    env['GOLF_RETAIN_FANTASY_REPLAY'] = '1'
    phases = []
    for phase in ('setup', 'fantasy', 'entry', 'numeric', 'final'):
        result = source.run(['node', str(SCRIPT_DIR.parent / 'critical-flows-2026-10-15/browser.cjs'), phase, str(source.path)], env=env, timeout=300)
        phases.append({'phase': phase, 'exit_code': result.returncode})
        source.state = json.loads(source.path.read_text()); source.checks = source.state['checks']
        print(json.dumps({'source_phase': phase, 'passed': True}), flush=True)
    last = json.loads((evidence / 'final.json').read_text())
    assert len(last['checks']) >= 25 and all(last['checks'].values())
    (HERE / 'source-flows.json').write_text(json.dumps({'phases': phases, 'browser': last['browser'], 'checks': last['checks'], 'images': last['images']}, indent=2) + '\n')
    if source.state.get('candidate_build_report'):
        # Recheck the changed runtime boundary without overwriting prior evidence.
        from guards import main as runtime_guards
        runtime_guards(source)
        boundary = source.root / 'boundary-browser'; boundary.mkdir(mode=0o700)
        boundary_env = source.env.copy(); boundary_env['GOLF_PRODUCTION_EVIDENCE_DIRECTORY'] = str(boundary)
        source.run(['node', str(SCRIPT_DIR.parent / 'production-rehearsal-2026-10-15/browser.cjs'), str(source.path)], env=boundary_env, timeout=180)
        source.state = json.loads(source.path.read_text()); source.checks = source.state['checks']
        (HERE / 'production-browser.json').write_text((boundary / 'browser.json').read_text())
        marker = 'release-private-' + secrets.token_hex(12)
        source.compose('stop', '--timeout', '10', 'api')
        try:
            failed = source.run(['curl','--silent','--insecure','--max-time','8','--output','/dev/null','--write-out','%{http_code}',
                '--header','Cookie: probe='+marker,'--header','X-CSRF-Token: '+marker,
                source.state['origin']+'/api/health?probe='+marker])
            assert failed.stdout == '502'
        finally:
            source.compose('up','-d','--no-build','--pull','never','--wait','--wait-timeout','90','api')
            source.wait_status('/api/ready',200)
        web_log = source.run(['docker','logs',source.state['services']['web']['id']])
        records = []
        for line in (web_log.stdout+web_log.stderr).splitlines():
            try: records.append(json.loads(line))
            except ValueError: pass
        errors = [r for r in records if r.get('logger')=='http.handlers.reverse_proxy' and r.get('level') in ('error','warn')]
        assert errors and any('error' in r and 'duration' in r for r in errors)
        assert marker not in web_log.stdout+web_log.stderr
        assert source.state['values']['PROXY_SHARED_SECRET'] not in web_log.stdout+web_log.stderr
        assert all('request' not in r and 'resp_headers' not in r for r in errors)
        source.receipt('candidate_actual_proxy502_error_logs_drop_request_canaries_and_proxy_secret')
    source.compose('stop', '--timeout', '10', 'api', 'web')
    source.receipt('source_writes_quiesced_before_snapshot_and_actual_backup')
    before = snapshot(source)
    archive = source.root / 'golfsiden-current.dump'
    script(source, 'backup-production.sh', archive)
    assert archive.stat().st_size > 0
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    assert Path(str(archive) + '.sha256').read_text() == digest + '  ' + archive.name + '\n'
    source.receipt('actual_backup_custom_archive_and_canonical_checksum')

    target = new_fixture('target', tag)
    script_config(target)
    assert target.state['project'] != source.state['project']
    for key in ('POSTGRES_OWNER_PASSWORD', 'APP_DATABASE_PASSWORD', 'PROXY_SHARED_SECRET'):
        assert target.state['values'][key] != source.state['values'][key]
    source_volume = json.loads(source.run(['docker', 'inspect', source.state['services']['postgres']['id']]).stdout)[0]['Mounts']
    target_id = target.compose('ps', '-q', 'postgres').stdout.strip()
    target_volume = json.loads(target.run(['docker', 'inspect', target_id]).stdout)[0]['Mounts']
    assert {m['Name'] for m in source_volume if m['Type']=='volume'}.isdisjoint({m['Name'] for m in target_volume if m['Type']=='volume'})
    assert target.sql(EMPTY) == '0'
    target.receipt('independent_fresh_target_volume_credentials_and_zero_public_objects')
    moved = target.root / 'moved backup pair'; moved.mkdir(mode=0o700)
    restored_archive = moved / archive.name
    shutil.copy2(archive, restored_archive); shutil.copy2(str(archive)+'.sha256', str(restored_archive)+'.sha256')
    assert hashlib.sha256(restored_archive.read_bytes()).hexdigest() == digest
    target.receipt('archive_pair_relocated_and_sha256_verified')
    result = script(target, 'restore-production.sh', restored_archive, confirmed=False, ok=False)
    assert result.returncode == 2 and 'CONFIRM_EMPTY_RESTORE' in result.stderr and target.sql(EMPTY) == '0'
    target.receipt('actual_restore_requires_explicit_confirmation')
    broken = moved / 'tampered.dump'; broken.write_bytes(restored_archive.read_bytes() + b'changed')
    Path(str(broken)+'.sha256').write_text(digest + '  ' + broken.name + '\n')
    result = script(target, 'restore-production.sh', broken, ok=False)
    assert result.returncode != 0 and 'canonical SHA-256' in result.stderr and target.sql(EMPTY) == '0'
    target.receipt('actual_restore_rejects_tampered_pair_without_public_objects')

    # Known trusted synthetic archive, intentionally truncated for a rollback
    # control; this sidecar is never proposed as recovery from untrusted bytes.
    late = moved / 'late-failure.dump'; late.write_bytes(restored_archive.read_bytes()[:-1024])
    Path(str(late)+'.sha256').write_text(hashlib.sha256(late.read_bytes()).hexdigest() + '  ' + late.name + '\n')
    toc = binary_archive_command(target,late,'--list')
    assert toc.returncode == 0 and b'TABLE DATA public fantasy_lineups' in toc.stdout
    result = script(target, 'restore-production.sh', late, ok=False)
    assert result.returncode != 0 and 'pg_restore:' in result.stderr and target.sql(EMPTY) == '0'
    assert 'Restore complete' not in result.stdout and 'GRANT' not in result.stdout
    target.compose('exec','-T','postgres','createdb','--username=rehearsal_owner','recovery_rollback_probe')
    try:
        verbose = binary_archive_command(target,late,'--username=rehearsal_owner','--dbname=recovery_rollback_probe',
            '--exit-on-error','--single-transaction','--no-owner','--no-privileges','--verbose')
        assert verbose.returncode != 0 and b'creating TABLE ' in verbose.stderr and b'processing data for table' in verbose.stderr
        assert target.compose('exec','-T','postgres','psql','-X','-At','--username=rehearsal_owner',
            '--dbname=recovery_rollback_probe','--command',EMPTY).stdout.strip() == '0'
    finally:
        target.compose('exec','-T','postgres','dropdb','--username=rehearsal_owner','recovery_rollback_probe')
    target.receipt('actual_current_archive_failure_and_verbose_late_work_witness_roll_back_to_zero_public_objects')
    script(target, 'restore-production.sh', restored_archive)
    check_roles(target)
    after = snapshot(target)
    assert after == before
    (HERE / 'parity.json').write_text(json.dumps({'all_public_tables_exact': True, 'sequences_exact': True,
        'table_count': len(before['tables']), 'row_count': sum(t['rows'] for t in before['tables'].values()),
        'rows_by_table': {name: row['rows'] for name,row in before['tables'].items()},
        'archive_sha256': digest, 'archive_bytes': archive.stat().st_size}, indent=2) + '\n')
    target.receipt('all_public_table_counts_row_sha256_and_sequences_match_before_login')
    result = script(target, 'restore-production.sh', restored_archive, ok=False)
    assert result.returncode != 0 and 'target public schema contains' in result.stderr
    assert snapshot(target) == before
    target.receipt('actual_repeat_restore_rejects_populated_target_preserving_exact_state')
    private_json(target.root / 'critical-private.json', json.loads((source.root / 'critical-private.json').read_text()))
    shutil.copy2(source.root / 'recovery-lineup-replay.json', target.root / 'recovery-lineup-replay.json')
    target.start()
    with (target.root / 'browser.log').open('w') as log:
        browser = subprocess.Popen(['node', str(SCRIPT_DIR / 'browser.cjs'), str(target.path)], cwd=REPO, env=target.env, stdout=log, stderr=log)
    for _ in range(600):
        if (target.root / 'browser-ready.json').exists():
            break
        assert browser.poll() is None, 'Browser exited before restart control'
        time.sleep(.1)
    assert (target.root / 'browser-ready.json').exists()
    target.compose('restart', 'api', 'web')
    target.wait_status('/api/ready', 200)
    private_json(target.root / 'restart-complete.json', {'ready':True})
    assert browser.wait(timeout=90) == 0, 'Restored browser validation failed'
    target.receipt('restored_login_reads_and_native_live_stream_survive_api_web_restart')
    target.compose('stop', '--timeout', '10', 'postgres')
    assert target.status('/api/health') == '200'
    outage = target.run(['curl', '--silent', '--insecure', '--max-time', '40', '--write-out', '\n%{http_code}', target.state['origin']+'/api/ready'])
    assert outage.stdout.endswith('\n503')
    target.compose('up','-d','--no-build','--pull','never','--wait','--wait-timeout','90','postgres')
    target.wait_status('/api/ready', 200)
    target.run(['node', str(SCRIPT_DIR / 'browser.cjs'), str(target.path), 'post-database'], timeout=90)
    target.receipt('restored_database_outage_health200_ready503_then_reconnect200')
    check_roles(target)
    for r in (source, target):
        for ident in r.compose('ps', '-aq').stdout.split():
            log = r.run(['docker','logs',ident])
            for key in ('POSTGRES_OWNER_PASSWORD','APP_DATABASE_PASSWORD','PROXY_SHARED_SECRET'):
                assert r.state['values'][key] not in log.stdout+log.stderr
        r.receipt('complete_container_logs_omit_generated_passwords_and_proxy_secret')
    cleanup()
    (HERE / 'sequence.json').write_text(json.dumps({'complete':True,'run_id':tag,'source_phases':phases,'production_scripts_unmodified':True},indent=2)+'\n')
    print(json.dumps({'recovery_sequence': 'passed'}), flush=True)


if __name__ == '__main__':
    try:
        cleanup() if len(sys.argv)>1 and sys.argv[1]=='cleanup' else main()
    except Exception as error:
        ownership = json.loads(POINTER.read_text()) if POINTER.exists() else {'controls':{}}
        for control in ownership['controls'].values():
            if Path(control).exists():
                Path(control).with_name('private-recovery-failure.log').write_text(traceback.format_exc())
        print(json.dumps({'failed':type(error).__name__,'detail':'private recovery diagnostics retained'}),flush=True)
        sys.exit(1)

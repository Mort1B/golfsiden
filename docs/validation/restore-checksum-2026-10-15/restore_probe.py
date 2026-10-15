"""Synthetic restore contract checks. Docker is always replaced, never delegated.
Run with --baseline before production edits to reproduce missing/binding defects.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
FAKE_DOCKER = '''#!{python}
import hashlib,json,os,sys
args=sys.argv[1:]
assert args[0]=='compose'
if 'pg_restore' in args[-1]: action='restore'
elif 'pg_dump' in args[-1]: action='backup'
elif 'psql' in args[-1]: action='empty-target'
elif args[-3:]==['stop','web','api']: action='stop'
elif args[-1]=='permissions' and '--profile' in args: action='permissions'
else: raise SystemExit(96)
record={{'action':action,'args':args}}
if action=='restore':
 data=sys.stdin.buffer.read();record.update(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
with open(os.environ['FAKE_DOCKER_LOG'],'a') as output: output.write(json.dumps(record)+'\\n')
if action=='empty-target':
 if os.environ.get('FAKE_PSQL_FAILURE'): raise SystemExit(17)
 print(os.environ.get('FAKE_OBJECT_COUNT','0'))
elif action=='restore' and os.environ.get('FAKE_RESTORE_FAILURE'): raise SystemExit(18)
elif action=='stop' and os.environ.get('FAKE_STOP_FAILURE'): raise SystemExit(19)
elif action=='permissions' and os.environ.get('FAKE_PERMISSIONS_FAILURE'): raise SystemExit(20)
elif action=='backup': sys.stdout.buffer.write(b'PGDMP synthetic local backup bytes\\n')
'''


def sidecar(archive):
    record = (hashlib.sha256(archive.read_bytes()).hexdigest() + '  -\n').encode() if archive.name == '-' else subprocess.check_output(['sha256sum', '--', archive.name], cwd=archive.parent)
    Path(str(archive) + '.sha256').write_bytes(record)
    return record


def execute(case, script, setup=None, expected=0, expected_actions=None,
            name='selected.dump', relative=None, confirmed=True, settings=None):
    with tempfile.TemporaryDirectory(prefix='golf-a5-probe-') as temporary:
        base = Path(temporary)
        work = base / 'work'
        work.mkdir()
        archive = work / name
        archive.parent.mkdir(parents=True, exist_ok=True)
        archive.write_bytes(b'PGDMP synthetic selected archive\n')
        sidecar(archive)
        environment_file = base / 'synthetic runtime.env'
        environment_file.write_text('# fake Docker only; no credentials\n')
        binary = base / 'bin'
        binary.mkdir()
        docker = binary / 'docker'
        docker.write_text(FAKE_DOCKER.format(python=sys.executable))
        docker.chmod(0o700)
        log = base / 'docker.jsonl'
        environment = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ['PATH'],
                           FAKE_DOCKER_LOG=str(log))
        environment.pop('CDPATH', None)
        environment.pop('CONFIRM_EMPTY_RESTORE', None)
        if confirmed:
            environment['CONFIRM_EMPTY_RESTORE'] = 'RESTORE_TO_EMPTY_DATABASE'
        environment.update(settings or {})
        if setup:
            setup(base, work, archive, environment)
        selected = str(archive) if relative is None else relative
        result = subprocess.run(['/bin/sh', str(script), str(environment_file), selected],
                                cwd=work, env=environment, capture_output=True, timeout=10)
        records = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
        actions = [record['action'] for record in records]
        required = expected_actions if expected_actions is not None else (
            ['empty-target', 'stop', 'restore', 'permissions'] if expected == 0 else [])
        failures = []
        if (result.returncode == 0) != (expected == 0):
            failures.append('exit contract')
        if actions != required:
            failures.append('Docker action/order contract')
        for record in records:
            args = record['args']
            if args[:5] != ['compose', '--env-file', str(environment_file), '-f', str(script.parent.parent / 'compose.production.yml')]:
                failures.append('Compose/environment contract')
            if record['action'] == 'empty-target':
                if not all(token in args[-1] for token in ['pg_class', 'pg_proc', 'pg_type', 'public']):
                    failures.append('pristine-schema scope')
            if record['action'] == 'restore':
                if not all(flag in args[-1] for flag in ['--exit-on-error', '--single-transaction', '--no-owner', '--no-privileges']):
                    failures.append('restore transaction/ownership flags')
                if record['sha256'] != hashlib.sha256((work / selected).read_bytes()).hexdigest():
                    failures.append('selected input bytes')
            if record['action'] == 'permissions' and args[5:] != ['--profile', 'tools', 'run', '--rm', 'permissions']:
                failures.append('permissions recovery')
        if (work / 'injected').exists():
            failures.append('filename interpreted as shell')
        if expected != 0 and b'Restore complete' in result.stdout:
            failures.append('false success message')
        return {'case': case, 'passed': not failures, 'exit': result.returncode,
                'docker_actions': actions, 'failures': failures}


def modify_manifest(content):
    def setup(_base, _work, archive, _environment):
        Path(str(archive) + '.sha256').write_bytes(content)
    return setup


def foreign(_base, work, archive, _environment):
    other = work / 'other.dump'
    other.write_bytes(b'Other intact file\n')
    Path(str(archive) + '.sha256').write_bytes(sidecar(other))


def newline_alias(_base, work, archive, _environment):
    alias = work / archive.name.rstrip('\n')
    alias.write_bytes(b'Unselected valid alias\n')
    Path(str(archive) + '.sha256').write_bytes(sidecar(alias))


def cdpath_decoy(base, work, archive, environment):
    archive.write_bytes(b'Tampered actual selected archive\n')
    decoy = base / 'decoy' / 'staging' / archive.name
    decoy.parent.mkdir(parents=True)
    decoy.write_bytes(b'Valid unselected CDPATH decoy\n')
    sidecar(decoy)
    environment['CDPATH'] = str(base / 'decoy')


def symlink_parent(base, work, archive, _environment):
    archive.write_bytes(b'Tampered logical directory decoy\n')
    # Logical cd link/.. chooses work; filesystem open link/../file chooses physical.
    physical = base / 'physical'
    (physical / 'child').mkdir(parents=True)
    (work / 'link').symlink_to(physical / 'child', target_is_directory=True)
    actual = physical / archive.name
    actual.write_bytes(b'Original actual selected archive\n')
    sidecar(actual)
    actual.write_bytes(b'Tampered actual selected archive\n')
    # The work directory's alias has a valid pair, unlike the selected file.
    sidecar(archive)


def appended(extra):
    def setup(_base, _work, archive, _environment):
        manifest = Path(str(archive) + '.sha256')
        manifest.write_bytes(manifest.read_bytes() + extra)
    return setup


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--baseline', action='store_true')
    parser.add_argument('--script', type=Path, default=ROOT / 'scripts/restore-production.sh')
    args = parser.parse_args()
    script = args.script.resolve()
    rows = []
    def check(label, **kwargs):
        rows.append(execute(label, script, **kwargs))
    def missing(_base, _work, archive, _environment):
        Path(str(archive) + '.sha256').unlink()
    check('missing_sidecar', setup=missing, expected=1)
    check('valid_digest_for_foreign_file', setup=foreign, expected=1)
    check('trailing_newline_alias_binding', name='selected.dump\n', setup=newline_alias, expected=1)
    check('cdpath_cannot_select_decoy', name='staging/selected.dump', relative='staging/selected.dump', setup=cdpath_decoy, expected=1)
    check('physical_parent_cannot_select_decoy', relative='link/../selected.dump', setup=symlink_parent, expected=1)
    if not args.baseline:
        check('canonical_valid')
        check('spaced_directory_and_basename', name='directory with spaces/final scores.dump', relative='directory with spaces/final scores.dump')
        check('leading_dash_basename', name='-scores.dump', relative='-scores.dump')
        check('leading_dash_directory', name='-directory/scores.dump', relative='-directory/scores.dump')
        check('escaped_backslash_basename', name='scores\\card.dump')
        check('escaped_middle_newline', name='scores\ncard.dump')
        check('escaped_trailing_newline', name='scores.dump\n')
        check('newline_directory', name='directory\n/scores.dump', relative='directory\n/scores.dump')
        check('shell_metacharacters_are_data', name='scores $(touch injected).dump')
        check('unicode_filename', name='rundens poeng æøå.dump')
        check('literal_stdin_sentinel_rejected', name='-', expected=1)
        check('empty_sidecar', setup=modify_manifest(b''), expected=1)
        check('malformed_record', setup=modify_manifest(b'not a checksum\n'), expected=1)
        check('wrong_digest', setup=modify_manifest(b'0' * 64 + b'  selected.dump\n'), expected=1)
        check('duplicate_records', setup=lambda b,w,a,e: appended(Path(str(a)+'.sha256').read_bytes())(b,w,a,e), expected=1)
        check('extra_blank_line', setup=appended(b'\n'), expected=1)
        check('appended_bytes', setup=appended(b'untrusted suffix'), expected=1)
        check('appended_nul', setup=appended(b'\x00'), expected=1)
        check('sidecar_directory', setup=lambda b,w,a,e: (Path(str(a)+'.sha256').unlink(),Path(str(a)+'.sha256').mkdir()), expected=1)
        check('unreadable_sidecar', setup=lambda b,w,a,e: Path(str(a)+'.sha256').chmod(0), expected=1)
        def no_final_newline(_b,_w,a,_e):
            manifest=Path(str(a)+'.sha256');manifest.write_bytes(manifest.read_bytes().rstrip(b'\n'))
        def binary_marker(_b,_w,a,_e):
            manifest=Path(str(a)+'.sha256');manifest.write_bytes(manifest.read_bytes().replace(b'  ',b' *',1))
        def uppercase_hash(_b,_w,a,_e):
            manifest=Path(str(a)+'.sha256');data=manifest.read_bytes();manifest.write_bytes(data[:64].upper()+data[64:])
        def utility_failure(command):
            def setup(base,_w,_a,_e):
                utility=base/'bin'/command;utility.write_text('#!/bin/sh\nexit 27\n');utility.chmod(0o700)
            return setup
        check('missing_final_newline',setup=no_final_newline,expected=1)
        check('binary_marker_is_not_backup_format',setup=binary_marker,expected=1)
        check('uppercase_digest_is_not_backup_format',setup=uppercase_hash,expected=1)
        check('hash_utility_failure',setup=utility_failure('sha256sum'),expected=1)
        check('comparison_utility_failure',setup=utility_failure('cmp'),expected=1)
        check('missing_archive',setup=lambda b,w,a,e:a.unlink(),expected=1)
        check('tampered_archive', setup=lambda b,w,a,e: a.write_bytes(b'Tampered selected archive\n'), expected=1)
        check('empty_archive', setup=lambda b,w,a,e: a.write_bytes(b''), expected=1)
        check('archive_directory', setup=lambda b,w,a,e: (a.unlink(),a.mkdir()), expected=1)
        check('unreadable_archive', setup=lambda b,w,a,e: a.chmod(0), expected=1)
        check('no_confirmation', confirmed=False, expected=1)
        check('nonempty_target', settings={'FAKE_OBJECT_COUNT':'9'}, expected=1, expected_actions=['empty-target'])
        check('target_query_failure', settings={'FAKE_PSQL_FAILURE':'1'}, expected=1, expected_actions=['empty-target'])
        check('restore_failure_blocks_permissions', settings={'FAKE_RESTORE_FAILURE':'1'}, expected=1, expected_actions=['empty-target','stop','restore'])
        check('permissions_failure_is_not_success', settings={'FAKE_PERMISSIONS_FAILURE':'1'}, expected=1, expected_actions=['empty-target','stop','restore','permissions'])
        check('existing_stop_failure_policy_retained', settings={'FAKE_STOP_FAILURE':'1'})
    summary={'baseline':args.baseline,'passed':sum(row['passed'] for row in rows),
             'failed':sum(not row['passed'] for row in rows),'cases':rows,
             'real_docker_or_database_used':False}
    print(json.dumps(summary,indent=2))
    raise SystemExit(bool(summary['failed']))


if __name__ == '__main__':
    main()

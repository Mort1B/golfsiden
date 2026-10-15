"""Independent retained-evidence, artifact, source and owned-cleanup audit."""
import ast
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
checks = {}


def read(name):
    return json.loads((HERE / (name + '.json')).read_text())


sequence, source, target, flows = [read(name) for name in ('sequence','source-cleanup','target-cleanup','source-flows')]
assert sequence['complete'] is True and sequence['production_scripts_unmodified'] is True
assert sequence['source_phases'] == [{'phase':name,'exit_code':0} for name in ('setup','fantasy','entry','numeric','final')]
assert flows['phases'] == sequence['source_phases'] and all(flows['checks'].values())
assert flows['checks'].items() <= source['checks'].items()
assert source['checks']['source_writes_quiesced_before_snapshot_and_actual_backup']
assert source['checks']['actual_backup_custom_archive_and_canonical_checksum']
for name in ('restore_target_started_without_migrations',
             'script_only_compose_config_bound_to_owned_project_and_exact_images',
             'independent_fresh_target_volume_credentials_and_zero_public_objects',
             'archive_pair_relocated_and_sha256_verified',
             'actual_restore_requires_explicit_confirmation',
             'actual_restore_rejects_tampered_pair_without_public_objects',
             'actual_current_archive_failure_and_verbose_late_work_witness_roll_back_to_zero_public_objects',
             'restored_actual_runtime_identity_schema39_and_ddl_history_denials',
             'all_public_table_counts_row_sha256_and_sequences_match_before_login',
             'actual_repeat_restore_rejects_populated_target_preserving_exact_state',
             'restored_login_reads_and_native_live_stream_survive_api_web_restart',
             'restored_database_outage_health200_ready503_then_reconnect200'):
    assert target['checks'][name]
checks['complete_real_script_recovery_and_owned_restart_sequence'] = True

parity = read('parity')
assert parity['all_public_tables_exact'] and parity['sequences_exact']
assert parity['table_count'] == len(parity['rows_by_table']) == 61
assert parity['row_count'] == sum(parity['rows_by_table'].values())
assert parity['rows_by_table']['_sqlx_migrations'] == 39
assert parity['rows_by_table']['scores'] == 144
for name in ('users','tournament_memberships','tournament_players','scorecard_confirmations',
             'fantasy_lineups','fantasy_selections','fantasy_configuration_audits'):
    assert parity['rows_by_table'][name] > 0
checks['all_61_public_tables_sequences_and_nonempty_sporting_fantasy_auth_facts_match'] = True
for name,count in (('browser',4),('browser-post-database',3)):
    receipt = read(name)
    assert len(receipt['checks']) == count and all(receipt['checks'].values())
    assert receipt['images'] == source['images'] == target['images']
assert read('browser-post-database')['native_stream'] == 'not exercised in this fresh-login mode'
checks['distinct_restored_ui_logins_73_88_94_replay_and_actual_live_reopen_receipts'] = True

baseline = json.loads((HERE.parent/'production-images-2026-10-15/baseline-builds.json').read_text())
web = json.loads((HERE.parent/'production-rehearsal-2026-10-15/web-build.json').read_text())
for record in baseline['builds']+[{'tag':web['tag'],'image':web['image']}]:
    image = json.loads(subprocess.check_output(['podman','image','inspect',record['tag']],text=True))[0]
    assert image['Id'] == record['image']['Id']
checks['four_original_candidate_image_ids_retained_exactly'] = True
for name in ('ownership','triage-ownership'):
    ownership = read(name)
    assert ownership['owned_aliases_removed'] and ownership['private_roots_removed']
    for root in ownership['private_roots']:
        assert not Path(root).exists()
    for alias in ownership['owned_aliases']:
        assert subprocess.run(['podman','image','exists',alias]).returncode == 1
for name in ('source-cleanup','target-cleanup','triage-source-cleanup','triage-target-cleanup'):
    receipt = read(name)
    assert receipt['owned_containers_networks_volumes_removed'] and receipt['private_configuration_diagnostics_ca_profile_removed']
    assert receipt['owned_resource_counts_before_removal'] == {'containers':3,'volumes':3,'networks':2}
assert not Path('/tmp/golf-deploy5-controls.json').exists()
checks['both_attempts_owned_resources_archives_roots_sockets_aliases_removed'] = True

subprocess.run(['git','diff','--exit-code','7155212bbc390421e49a0f4b6022a4dcd5032f1f','--',
                'backend','frontend','migrations','vendor','deploy','scripts','Cargo.toml','Cargo.lock',
                'compose.production.yml','.dockerignore','.env.production.example'],cwd=REPO,check=True)
subprocess.run(['git','diff','--exit-code','7155212bbc390421e49a0f4b6022a4dcd5032f1f','--',
                'docs/validation/critical-flows-2026-10-15',
                ':!docs/validation/critical-flows-2026-10-15/support.cjs',
                ':!docs/validation/critical-flows-2026-10-15/fantasy.cjs'],cwd=REPO,check=True)
checks['production_inputs_scripts_and_historical_critical_flow_evidence_unchanged'] = True
for file in HERE.glob('*.py'):
    ast.parse(file.read_text())
ast.parse((HERE.parent/'production-rehearsal-2026-10-15/rehearsal.py').read_text())
for file in [HERE/'browser.cjs',HERE.parent/'critical-flows-2026-10-15/support.cjs',HERE.parent/'critical-flows-2026-10-15/fantasy.cjs']:
    subprocess.run(['node','--check',str(file)],check=True)
subprocess.run(['sh','-n','scripts/backup-production.sh','scripts/restore-production.sh'],cwd=REPO,check=True)
subprocess.run(['git','diff','--check'],cwd=REPO,check=True)
checks['changed_harness_and_shell_syntax_and_whitespace'] = True
(HERE/'postconditions.json').write_text(json.dumps({'checks':checks,
    'sequence_sha256':hashlib.sha256((HERE/'sequence.json').read_bytes()).hexdigest()},indent=2)+'\n')
print(json.dumps(checks))

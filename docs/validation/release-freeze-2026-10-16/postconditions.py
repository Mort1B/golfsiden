"""Verify release receipts, immutable delivery, source scope and owned cleanup."""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]
checks={}


def read(name):return json.loads((HERE/(name+'.json')).read_text())


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def run(args):
    result=subprocess.run(args,cwd=REPO,capture_output=True,text=True,timeout=180)
    assert result.returncode==0,'Release postcondition command failed: '+args[0]
    return result.stdout


def passed(name,value):
    assert value,name
    checks[name]=True


build=read('candidate-builds')
passed('three_complete_builds_removed_context',len(build['builds'])==3 and build['temporary_context_removed'] and all(i['exit_code']==0 for i in build['builds']))
for name,expected in build['overlays'].items():passed('current_overlay_'+name,digest(REPO/name)==expected)
changed=run(['git','diff','--name-only',build['base_commit'],'--','backend','frontend','migrations','vendor','deploy','scripts','Cargo.toml','Cargo.lock','compose.production.yml','.dockerignore','.env.production.example']).splitlines()
passed('only_three_dockerfiles_change_production_inputs',set(changed)==set(build['overlays']))
equal=read('content-equality')
passed('unchanged_three_rust_executables_and_65_assets',equal['three_rust_executables_byte_identical'] and equal['web_assets_byte_identical'] and equal['web_asset_count']==65)
passed('patched_compiled_go_net_quic_modules',equal['caddy_go']=='1.27.2' and equal['caddy_x_net']=='0.60.0' and equal['caddy_quic_go']=='0.63.0' and equal['standard_http_modules_present'])
pre=read('preflight')
passed('fresh_23_guards_and_actual_pg1711',len(pre['checks'])==23 and all(i['passed'] for i in pre['checks']) and pre['postgres_version']=='postgres (PostgreSQL) 17.11')
images={i['service']:i['tag'] for i in build['builds']}
for name,count in [('source-cleanup',60),('target-cleanup',16)]:
    data=read(name)
    passed(name+'_checks_and_exact_candidate',len(data['checks'])==count and all(data['checks'].values()) and data['candidate_build']==build and data['images']==images)
    passed(name+'_owned_resources_removed',data['owned_containers_networks_volumes_removed'] and data['private_configuration_diagnostics_ca_profile_removed'])
passed('complete_ordered_five_phases',read('sequence')['complete'] and [i['phase'] for i in read('sequence')['source_phases']]==['setup','fantasy','entry','numeric','final'] and all(i['exit_code']==0 for i in read('sequence')['source_phases']))
for name,count in [('source-flows',25),('production-browser',7),('browser',4),('browser-post-database',3)]:
    data=read(name);passed(name+'_exact_candidate_checks',len(data['checks'])==count and all(data['checks'].values()) and data['images']==images)
parity=read('parity')
passed('61_tables_623_rows_and_sequences_exact',parity['all_public_tables_exact'] and parity['sequences_exact'] and parity['table_count']==61 and parity['row_count']==623)
own=read('ownership')
passed('owned_runtime_roots_and_aliases_removed',own['private_roots_removed'] and own['owned_aliases_removed'] and all(not Path(i).exists() for i in own['private_roots']))
for name in ('npm-audit','npm-production-audit'):passed(name+'_zero',read(name)['metadata']['vulnerabilities']['total']==0)
passed('optional_rust_normal_paths_absent',len(read('rust-reachability'))==6 and all(not i['normal_path_present'] for i in read('rust-reachability')))
for service in ('postgres','api','web'):
    for prefix in ('','candidate-'):passed(prefix+service+'_raw_scan_retained',bool(read(prefix+'image-'+service)['Results']))
passed('scanner_release_checksum_verified',read('scanner')['release_checksum_verified'])
web=read('candidate-image-web')
passed('web_no_high_or_critical_matches',all(v['Severity'] not in ('HIGH','CRITICAL') for result in web['Results'] for v in result.get('Vulnerabilities',[])))
bundle=read('bundle');root=Path(bundle['directory'])
passed('retained_bundle_checksum',digest(Path(bundle['bundle']))==bundle['bundle_sha256'])
passed('retained_manifest_matches_repository',digest(root/'manifest.json')==bundle['manifest_sha256'] and (root/'manifest.json').read_bytes()==(HERE/'manifest.json').read_bytes())
run(['python',str(root/'verify-release.py')])
passed('final_offline_archive_file_config_ids_verified',True)
for name in ('OPERATOR.md','verify-release.py'):passed('packaged_'+name+'_matches_source',(root/name).read_bytes()==(HERE/name).read_bytes())
transfer=read('transfer')
passed('ten_transfer_controls_match_final_package',len(transfer['checks'])==10 and all(transfer['checks'].values()) and transfer['bundle_sha256']==bundle['bundle_sha256'] and transfer['manifest_sha256']==bundle['manifest_sha256'])
passed('owned_transfer_roots_removed',not list(Path('/tmp').glob('golf-deploy6-transfer-*')))
passed('private_build_scan_cache_removed',not Path('/tmp/golf-deploy6-work-path').exists() and not Path('/tmp/golf-deploy6-dcimuf7z').exists())
historic=run(['git','diff','--name-only',build['base_commit'],'--','docs/validation']).splitlines()
allowed={'docs/validation/production-rehearsal-2026-10-15/rehearsal.py','docs/validation/production-rehearsal-2026-10-15/browser.cjs','docs/validation/recovery-2026-10-15/recovery.py','docs/validation/recovery-2026-10-15/browser.cjs'}
passed('historical_evidence_not_rewritten',all(i in allowed or i.startswith(str(HERE.relative_to(REPO))+'/') for i in historic))
old=json.loads((HERE.parent/'production-images-2026-10-15/baseline-builds.json').read_text())['builds']
oldweb=json.loads((HERE.parent/'production-rehearsal-2026-10-15/web-build.json').read_text())
for item in old+[oldweb]+build['builds']:
    actual=json.loads(run(['podman','image','inspect',item['tag']]))[0]
    passed('retained_image_'+item['tag'],actual['Id'].removeprefix('sha256:')==item['image']['Id'])
for path in list(HERE.glob('*.py'))+[REPO/name for name in allowed if name.endswith('.py')]:ast.parse(path.read_text())
for name in allowed:
    if name.endswith('.cjs'):run(['node','--check',name])
for name in ('scripts/backup-production.sh','scripts/restore-production.sh'):run(['sh','-n',str(root/name)])
passed('python_node_shell_syntax',True)
documents=[HERE/'README.md',HERE/'OPERATOR.md']+[REPO/'docs'/name for name in ('ARCHITECTURE.md','Documentation.md','LatestExplanation.md','PLANS.md','deployment_guide.md')]
for path in documents:
    for target in re.findall(r'\]\(([^)]+)\)',path.read_text()):
        if '://' in target or target.startswith('#'):continue
        destination=(path.parent/target.split('#')[0]).resolve()
        # This receipt is created only after the remaining checks pass below.
        if destination==HERE/'postconditions.json':continue
        passed('link_'+str(path.relative_to(REPO))+'_'+target,destination.exists())
run(['git','diff','--check'])
passed('diff_whitespace_clean',True)
passed('no_outstanding_material_review_findings',read('review')['remaining_material_findings']==[])
(HERE/'postconditions.json').write_text(json.dumps({'checks':checks,'count':len(checks),'native_docker_accepted':False,'hosted_deployment_accepted':False},indent=2)+'\n')
print(json.dumps({'passed':len(checks),'bundle_sha256':bundle['bundle_sha256']}))

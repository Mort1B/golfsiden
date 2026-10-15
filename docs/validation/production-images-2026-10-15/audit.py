"""Audit retained DEPLOY-2 evidence and exact local image identities."""
from pathlib import Path
import ast
import hashlib
import json
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
builds = json.loads((HERE / 'baseline-builds.json').read_text())
compose = json.loads((HERE / 'compose.json').read_text())
inspection = json.loads((HERE / 'inspection.json').read_text())
checks = {}
assert [item['service'] for item in builds['builds']] == ['postgres', 'api', 'web']
assert builds['context_from_git_archive_only'] and builds['overlays'] == {}
assert builds['temporary_context_and_logs_removed']
for item in builds['builds']:
    assert item['exit_code'] == 0 and item['iid_matches']
    assert hashlib.sha256((REPO / item['dockerfile']).read_bytes()).hexdigest() == item['dockerfile_sha256']
    current = json.loads(subprocess.check_output(['podman', 'image', 'inspect', item['tag']], text=True))[0]
    assert current['Id'] == item['image']['Id']
    labels = current['Config']['Labels']
    assert labels['io.golfsiden.source.commit'] == builds['base_commit']
    assert labels['io.golfsiden.context.sha256'] == builds['context_sha256']
    assert labels['io.golfsiden.validation.step'] == 'DEPLOY-2'
checks['three_exact_built_images_and_source_labels'] = True
assert len(compose['checks']) == 11 and all(item['passed'] for item in compose['checks'])
assert compose['private_env_removed'] and not compose['services_started']
assert hashlib.sha256((REPO / 'compose.production.yml').read_bytes()).hexdigest() == compose['compose_sha256']
checks['eleven_compose_contract_cases_and_private_cleanup'] = True
assert len(inspection['checks']) == 23 and all(item['passed'] for item in inspection['checks'])
assert inspection['context_sha256'] == builds['context_sha256']
assert inspection['owned_probe_containers_removed'] == 23 and inspection['private_env_files_removed']
assert not inspection['database_initialized'] and not inspection['proxy_server_started']
assets = json.loads((HERE / 'web-assets.json').read_text())
assert len(assets) == inspection['web_asset_count'] == 65 and 'index.html' in assets
assert inspection['rust_builder_version'].startswith('rustc 1.88.0 ')
assert inspection['node_builder_version'].startswith('v22.')
checks['twentythree_real_image_preflights_and_65_assets'] = True
leftover = subprocess.check_output(['podman', 'ps', '--all', '--filter',
                                   'label=io.golfsiden.validation.step=DEPLOY-2', '--format', '{{.ID}}'], text=True)
assert not leftover.strip()
checks['no_remaining_owned_probe_containers'] = True
subprocess.run(['git', 'diff', '--exit-code', builds['base_commit'], '--',
                'backend', 'frontend', 'migrations', 'vendor', 'deploy', 'scripts',
                'Cargo.toml', 'Cargo.lock', 'compose.production.yml', '.dockerignore',
                '.env.production.example'], cwd=REPO, check=True)
checks['all_current_build_runtime_inputs_unchanged_from_candidate'] = True
for name in ('build.py', 'compose.py', 'preflight.py', 'audit.py'):
    ast.parse((HERE / name).read_text())
checks['python_syntax'] = True
engine = subprocess.run(['docker', '--host', 'unix:///var/run/docker.sock', 'version',
                         '--format', '{{.Server.Version}}'], capture_output=True, text=True, timeout=30)
host = subprocess.check_output(['podman', 'info', '--format',
                                '{{.Host.Arch}} {{.Host.Security.Rootless}}'], text=True).split()
assert host == ['amd64', 'true']
socket_present = Path('/var/run/docker.sock').exists()
availability = {'docker_compose_cli_version': compose['compose_version'],
                'docker_local_engine_available': engine.returncode == 0,
                'docker_engine_version': engine.stdout.strip() if engine.returncode == 0 else None,
                'docker_daemon_socket_present': socket_present,
                'docker_engine_blocker': None if engine.returncode == 0 else (
                    'Permission denied accessing the local Docker API socket' if 'permission denied' in engine.stderr.lower()
                    else ('No local Docker daemon socket at /var/run/docker.sock' if not socket_present
                          else 'Local Docker daemon command failed')),
                'podman_version': builds['builder_version'],
                'rootless_podman_architecture': host[0],
                'rootless_podman': host[1] == 'true',
                'cached_base_images_with_pull_never': True,
                'public_host_visited': False}
(HERE / 'availability.json').write_text(json.dumps(availability, indent=2) + '\n')
(HERE / 'postconditions.json').write_text(json.dumps(checks, indent=2) + '\n')
print(json.dumps({'checks': checks, 'availability': availability}))

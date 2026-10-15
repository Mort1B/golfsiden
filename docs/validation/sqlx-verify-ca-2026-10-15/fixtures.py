"""CID-owned, loopback-only disposable PostgreSQL for the A1w probes/ladders."""
from pathlib import Path
import re
import subprocess
import time

IMAGE = 'docker.io/library/postgres:17-alpine'


def capture(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL,
                                   timeout=60).strip()


class Database:
    def __init__(self, root, name, password, certificate=None):
        self.root = Path(root)
        self.directory = self.root / name
        self.directory.mkdir()
        self.cidfile = self.directory / 'container.cid'
        self.password = password
        self.certificate = certificate
        self.evidence = {'fixture': name, 'image': IMAGE, 'removed': False}

    def __enter__(self):
        envfile = self.directory / 'pg.env'
        envfile.write_text('POSTGRES_USER=golf\nPOSTGRES_DB=golf\nPOSTGRES_PASSWORD=' + self.password + '\n')
        envfile.chmod(0o600)
        command = ['podman', 'create', '--cidfile', str(self.cidfile),
                   '--env-file', str(envfile), '--publish', '127.0.0.1::5432',
                   '--tmpfs', '/var/lib/postgresql/data:rw']
        if self.certificate is not None:
            command += ['--volume', str(self.root / 'pki') + ':/fixtures:ro,Z']
        command += [IMAGE]
        if self.certificate is not None:
            command += ['sh', '-c',
                        'cp "/fixtures/$1.crt" /tmp/server.crt && cp /fixtures/server.key /tmp/server.key && '
                        'chown postgres:postgres /tmp/server.crt /tmp/server.key && chmod 600 /tmp/server.key && '
                        'exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/server.crt -c ssl_key_file=/tmp/server.key',
                        'sh', self.certificate]
        try:
            capture(command)
            self.cid = self.cidfile.read_text().strip()
            capture(['podman', 'start', self.cid])
            deadline = time.monotonic() + 45
            while subprocess.run(['podman', 'exec', self.cid, 'pg_isready', '-h', '127.0.0.1', '-U', 'golf'],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                if time.monotonic() > deadline:
                    raise RuntimeError('Owned PostgreSQL startup timed out')
                time.sleep(.2)
            self.address = capture(['podman', 'port', self.cid, '5432/tcp'])
            self.port = self.address.rsplit(':', 1)[1]
            self.evidence['version'] = capture(['podman', 'exec', self.cid, 'psql', '-X', '-U',
                                                'golf', '-d', 'golf', '-Atc', 'SELECT version()'])
            self.evidence['image_id'] = capture(['podman', 'image', 'inspect', IMAGE, '--format', '{{.Id}}'])
            return self
        except BaseException:
            self.cleanup()
            raise

    def cleanup(self):
        if self.cidfile.exists():
            cid = self.cidfile.read_text().strip()
            if not re.fullmatch(r'[0-9a-f]{64}', cid):
                raise RuntimeError('Owned container ID is invalid; inspect fixture cleanup')
            capture(['podman', 'rm', '-f', cid])
            self.evidence['removed'] = not capture(['podman', 'ps', '-a', '--filter',
                                                   f'id={cid}', '--format', '{{.ID}}'])
            if not self.evidence['removed']:
                raise RuntimeError('Owned PostgreSQL cleanup not confirmed')

    def __exit__(self, *_args):
        self.cleanup()

"""Opt-in A4 fixtures: local-only temporary PKI, HTTPS and PostgreSQL TLS.
Requires cached postgres:17-alpine, Podman, OpenSSL, the built Rust example
and a plaintext PostgreSQL listener at127.0.0.1:55449 (preflighted).
Never installs a system root or retains private keys; owns only its TLS container.
"""
import http.server
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import ssl
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[3]


def run(args, **kwargs):
    result = subprocess.run(args, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True, **kwargs)
    if result.returncode:
        raise RuntimeError(args[0] + ' failed: ' + result.stdout[-2000:] + result.stderr[-2000:])
    return result


def certificates(directory):
    for name in ['ca', 'wrong-ca']:
        run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
             '-keyout', str(directory / (name + '.key')),
             '-out', str(directory / (name + '.crt')), '-days', '2',
             '-subj', '/CN=A4 synthetic ' + name,
             '-addext', 'basicConstraints=critical,CA:TRUE'])
    run(['openssl', 'req', '-newkey', 'rsa:2048', '-nodes',
         '-keyout', str(directory / 'server.key'),
         '-out', str(directory / 'server.csr'), '-subj', '/CN=localhost'])
    extensions = directory / 'extensions.cnf'
    extensions.write_text('subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n')
    run(['openssl', 'x509', '-req', '-in', str(directory / 'server.csr'),
         '-CA', str(directory / 'ca.crt'), '-CAkey', str(directory / 'ca.key'),
         '-CAcreateserial', '-out', str(directory / 'server.crt'), '-days', '2',
         '-extfile', str(extensions)])
    for path in directory.glob('*.key'):
        path.chmod(0o600)


class Handler(http.server.BaseHTTPRequestHandler):
    requests = 0
    authorization_seen = False

    def do_GET(self):
        Handler.requests += 1
        Handler.authorization_seen |= self.headers.get('authorization') is not None
        assert self.path == '/v1/courses/7k2m9qb4'
        payload = json.dumps({'course': {'id': '7k2m9qb4'}}).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_args):
        pass


def main():
    directory = Path(tempfile.mkdtemp(prefix='golf-a4-tls-'))
    server = None
    container = 'golf-a4-tls-' + directory.name.removeprefix('golf-a4-tls-')
    cidfile = directory / 'container.cid'
    try:
        with socket.create_connection(('127.0.0.1', 55449), timeout=2) as plaintext:
            plaintext.sendall(struct.pack('!II', 8, 80877103))
            if plaintext.recv(1) != b'N':
                raise RuntimeError('port55449 must be a plaintext-only PostgreSQL fixture')
        certificates(directory)
        run(['podman', 'create', '--cidfile', str(cidfile), '--name', container,
             '-e', 'POSTGRES_USER=golf', '-e', 'POSTGRES_PASSWORD=golf',
             '-e', 'POSTGRES_DB=golf', '-p', '127.0.0.1:55450:5432',
             '--tmpfs', '/var/lib/postgresql/data:rw',
             '-v', str(directory) + ':/fixtures:ro,Z',
             'docker.io/library/postgres:17-alpine', 'sh', '-c',
             'cp /fixtures/server.crt /tmp/server.crt && cp /fixtures/server.key /tmp/server.key && chown postgres:postgres /tmp/server.crt /tmp/server.key && chmod 600 /tmp/server.key && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/server.crt -c ssl_key_file=/tmp/server.key'])
        run(['podman', 'start', container])
        deadline = time.monotonic() + 30
        while True:
            ready = subprocess.run(['podman', 'exec', container,
                                    'pg_isready', '-h', '127.0.0.1', '-U', 'golf'],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if ready.returncode == 0:
                break
            if time.monotonic() >= deadline:
                logs = run(['podman', 'logs', container])
                raise RuntimeError('local TLS PostgreSQL did not become ready: ' + logs.stderr[-2000:])
            time.sleep(0.1)
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 55451), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(directory / 'server.crt', directory / 'server.key')
        server.socket = context.wrap_socket(server.socket, server_side=True)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        environment = {key: value for key, value in os.environ.items() if key.upper() not in {'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'NO_PROXY'}}
        environment.update(GOLF_TLS_FIXTURES=str(directory), NO_PROXY='localhost,127.0.0.1')
        for mode in ['database', 'provider']:
            probe = run([os.environ.get('GOLF_TLS_PROBE', str(ROOT / 'target/debug/examples/tls_compatibility')), mode],
                        env=environment, timeout=90)
            print(probe.stdout.rstrip(), flush=True)
        assert Handler.requests == 1, 'rejected provider TLS must send no HTTP request'
        assert not Handler.authorization_seen, 'synthetic provider bearer must not reach untrusted HTTPS'
        print('PASS local HTTPS received only the explicit-root transport request; no Authorization header', flush=True)
    finally:
        if server is not None:
            server.shutdown()
            server.server_close()
        try:
            if cidfile.is_file():
                container_id = cidfile.read_text().strip()
                if len(container_id) != 64 or any(c not in '0123456789abcdef' for c in container_id):
                    raise RuntimeError('invalid owned container id; cleanup needs inspection')
                run(['podman', 'rm', '-f', container_id])
        finally:
            shutil.rmtree(directory)
        print('CLEANUP local TLS container removed; temporary PKI removed', flush=True)


if __name__ == '__main__':
    main()

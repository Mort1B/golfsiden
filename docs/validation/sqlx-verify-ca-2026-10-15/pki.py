"""Temporary synthetic PKI for A1w; never installs roots or retains private keys."""
from datetime import datetime, timedelta, timezone
from pathlib import Path
import subprocess


def run(args, **kwargs):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, **kwargs)


def certificates(directory):
    directory = Path(directory)
    for name in ('ca', 'wrong-ca', 'intermediate'):
        run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
             '-keyout', str(directory / f'{name}.key'), '-out', str(directory / f'{name}.crt'),
             '-days', '3', '-subj', f'/CN=A1w synthetic {name}',
             '-addext', 'basicConstraints=critical,CA:TRUE',
             '-addext', 'keyUsage=critical,keyCertSign,cRLSign'])
    run(['openssl', 'req', '-newkey', 'rsa:2048', '-nodes',
         '-keyout', str(directory / 'server.key'), '-out', str(directory / 'server.csr'),
         '-subj', '/CN=localhost'])
    (directory / 'index').write_text('')
    (directory / 'serial').write_text('1000\n')
    (directory / 'newcerts').mkdir()
    config = directory / 'ca.cnf'
    config.write_text(f'''[ca]
default_ca=local
[local]
database={directory}/index
serial={directory}/serial
new_certs_dir={directory}/newcerts
certificate={directory}/ca.crt
private_key={directory}/ca.key
default_md=sha256
policy=policy
unique_subject=no
[policy]
commonName=supplied
[server]
subjectAltName=DNS:localhost
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
[client]
subjectAltName=DNS:localhost
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=clientAuth
''')
    now = datetime.now(timezone.utc)
    for name, start, end, extension in (
            ('valid', -1, 48, 'server'), ('expired', -48, -24, 'server'),
            ('future', 24, 48, 'server'), ('purpose', -1, 48, 'client')):
        date = lambda hours: (now + timedelta(hours=hours)).strftime('%Y%m%d%H%M%SZ')
        run(['openssl', 'ca', '-batch', '-notext', '-config', str(config),
             '-in', str(directory / 'server.csr'), '-out', str(directory / f'{name}.crt'),
             '-startdate', date(start), '-enddate', date(end), '-extensions', extension])
    der = bytearray(run(['openssl', 'x509', '-in', str(directory / 'valid.crt'),
                         '-outform', 'DER']).stdout)
    der[-1] ^= 1  # Corrupt only the certificate signature, preserving its key/ASN.1.
    run(['openssl', 'x509', '-inform', 'DER', '-out', str(directory / 'signature.crt')], input=der)
    # Replace the intermediate's self-signed certificate with one signed by the root.
    run(['openssl', 'req', '-new', '-key', str(directory / 'intermediate.key'),
         '-out', str(directory / 'intermediate.csr'), '-subj', '/CN=A1w intermediate'])
    intermediate_ext = directory / 'intermediate.cnf'
    intermediate_ext.write_text('basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n')
    run(['openssl', 'x509', '-req', '-in', str(directory / 'intermediate.csr'),
         '-CA', str(directory / 'ca.crt'), '-CAkey', str(directory / 'ca.key'),
         '-CAcreateserial', '-days', '2', '-extfile', str(intermediate_ext),
         '-out', str(directory / 'intermediate.crt')])
    leaf_ext = directory / 'leaf.cnf'
    leaf_ext.write_text('subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n')
    run(['openssl', 'x509', '-req', '-in', str(directory / 'server.csr'),
         '-CA', str(directory / 'intermediate.crt'), '-CAkey', str(directory / 'intermediate.key'),
         '-CAcreateserial', '-days', '2', '-extfile', str(leaf_ext),
         '-out', str(directory / 'missing.crt')])
    (directory / 'complete.crt').write_bytes((directory / 'missing.crt').read_bytes()
                                              + (directory / 'intermediate.crt').read_bytes())
    for path in directory.glob('*.key'):
        path.chmod(0o600)

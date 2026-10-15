"""Execute A1w baseline/candidate SQLx handshakes against exclusively owned PKI/PG."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from fixtures import Database
from pki import certificates

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['baseline', 'candidate'])
    args = parser.parse_args()
    result = {'phase': args.phase, 'cases': [], 'fixtures': []}
    with tempfile.TemporaryDirectory(prefix='golf-a1w-tls-') as temp:
        root = Path(temp)
        (root / 'pki').mkdir()
        password = secrets.token_urlsafe(24)
        try:
            certificates(root / 'pki')

            def probe(database, mode, host, expected, root_name='ca'):
                environment = dict(os.environ, GOLF_TLS_HOST=host, GOLF_TLS_PORT=database.port,
                                   GOLF_TLS_MODE=mode, GOLF_TLS_PASSWORD=password,
                                   GOLF_TLS_CA=str(root / 'pki' / f'{root_name}.crt'))
                completed = subprocess.run([str(ROOT / 'target/debug/examples/sqlx_verify_ca')],
                                           env=environment, text=True, capture_output=True, timeout=20)
                if completed.returncode:
                    raise RuntimeError('Synthetic TLS probe failed before classifying its handshake')
                observed = json.loads(completed.stdout)
                item = {'fixture': database.evidence['fixture'], 'mode': mode, 'host': host,
                        'root': root_name, 'expected': expected, **observed}
                item['accepted'] = observed['observed'] == expected
                result['cases'].append(item)
                print(json.dumps(item), flush=True)
                if not item['accepted']:
                    raise RuntimeError('Synthetic TLS case disagrees with its exact expected outcome')

            for certificate, reason in [('valid', None), ('expired', 'expired'),
                                        ('future', 'not_yet_valid'), ('signature', 'bad_signature'),
                                        ('purpose', 'invalid_purpose'), ('missing', 'unknown_issuer'),
                                        ('complete', None)]:
                database = Database(root, certificate, password, certificate)
                result['fixtures'].append(database.evidence)
                with database:
                    if reason is not None:
                        for mode in ('verify-ca', 'verify-full'):
                            probe(database, mode, '127.0.0.1', reason)
                    else:
                        for mode in ('verify-ca', 'verify-full'):
                            probe(database, mode, 'localhost', 'ok')
                        expected = 'hostname_context' if args.phase == 'baseline' else 'ok'
                        probe(database, 'verify-ca', '127.0.0.1', expected)
                        probe(database, 'verify-full', '127.0.0.1', 'hostname_context')
                        if certificate == 'valid':
                            for mode in ('verify-ca', 'verify-full'):
                                probe(database, mode, '127.0.0.1', 'unknown_issuer', 'wrong-ca')
                            for mode in ('require', 'prefer'):
                                probe(database, mode, 'localhost', 'ok', 'wrong-ca')
            database = Database(root, 'plaintext', password)
            result['fixtures'].append(database.evidence)
            with database:
                for mode in ('verify-ca', 'verify-full', 'require'):
                    probe(database, mode, '127.0.0.1', 'no_tls')
            assert len(result['cases']) == 25
            assert all(case['accepted'] for case in result['cases'])
            assert all(item['removed'] for item in result['fixtures'])
            result['accepted'] = True
        finally:
            (HERE / f'tls-{args.phase}.json').write_text(json.dumps(result, indent=2) + '\n')
    result['temporary_pki_removed'] = not root.exists()
    assert result['temporary_pki_removed']
    (HERE / f'tls-{args.phase}.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f'PASS {args.phase}: 25 handshake cases; eight owned fixtures and temporary PKI removed', flush=True)


if __name__ == '__main__':
    main()

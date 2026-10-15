"""After the browser closes native streams, inspect their real proxy errors.

Also cancel a marked health request to exercise sensitive header/URI canaries.
Run before on the original web and after on the rebuilt web, after browser.cjs.
"""
import json
import secrets
import sys
import time

from rehearsal import Rehearsal


def main(rehearsal, mode):
    assert mode in ('before', 'after')
    api = rehearsal.state['services']['api']['id']
    web = rehearsal.state['services']['web']['id']
    marker = 'deploy3-private-' + secrets.token_hex(12)
    rehearsal.run(['docker', 'pause', api])
    try:
        result = rehearsal.run(['curl', '--silent', '--insecure', '--max-time', '1',
                                '--header', 'X-CSRF-Token: ' + marker,
                                '--header', 'Cookie: probe=' + marker,
                                '--header', 'Authorization: Bearer ' + marker,
                                rehearsal.state['origin'] + '/api/health?probe=' + marker], ok=False)
        assert result.returncode == 28
    finally:
        rehearsal.run(['docker', 'unpause', api])
    rehearsal.wait_status('/api/ready', 200)
    time.sleep(1)
    result = rehearsal.run(['docker', 'logs', web])
    logs = result.stdout + result.stderr
    records = []
    for line in logs.splitlines():
        try:
            records.append(json.loads(line))
        except ValueError:
            pass
    upstream = [record for record in records if record.get('logger') == 'http.handlers.reverse_proxy' and record.get('level') in ('error', 'warn')]
    assert upstream, 'real upstream cancellation record required'
    assert any('error' in record and 'duration' in record for record in upstream)
    proxy_secret = rehearsal.state['values']['PROXY_SHARED_SECRET']
    if mode == 'before':
        assert proxy_secret in logs and marker in logs
        rehearsal.receipt('baseline_actual_upstream_cancellation_exposes_proxy_secret_reproduced')
    else:
        assert proxy_secret not in logs and marker not in logs
        access = [record for record in records if record.get('logger', '').startswith('http.log.access')]
        assert any(record.get('status') == 200 for record in access)
        for record in upstream + access:
            assert 'request' not in record and 'resp_headers' not in record
            assert 'logger' in record and 'level' in record and 'msg' in record
        rehearsal.receipt('rebuilt_web_actual_upstream_and_access_logs_no_secret_or_request_metadata')


if __name__ == '__main__':
    main(Rehearsal(sys.argv[2]), sys.argv[1])

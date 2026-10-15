"""Compare synthetic A8 probe output; never reads database configuration."""
import argparse
import json
import math
from pathlib import Path
import statistics


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--current', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before = json.loads(args.baseline.read_text())
    after = json.loads(args.current.read_text())
    assert before['postgres'] == after['postgres']
    assert before['settings'] == after['settings']
    assert len(before['regressions']) == 6 and not after['regressions']
    assert len(before['samples']) == len(after['samples']) == 12
    rows = []
    for a, b in zip(before['samples'], after['samples'], strict=True):
        assert (a['teams'], a['mode'], a['payload_sha256'], a['repeats']) == (b['teams'], b['mode'], b['payload_sha256'], b['repeats'])
        n = a['repeats']
        count = a['teams']
        assert a['team_calls'] == b['team_calls'] == n
        assert a['member_calls'] == count * n
        assert b['member_calls'] == (0 if count == 0 else n)
        assert a['authority_calls'] == b['authority_calls']
        assert a['transaction_calls'] == b['transaction_calls'] == 2*n
        assert all(len(v['warm_ms']) == n for v in [a, b])
        rows.append({
            'teams': count, 'mode': a['mode'], 'repeats': n,
            'data_selects_before': 1+count,
            'data_selects_after': 1 if count == 0 else 2,
            'authority_selects': b['authority_calls']//n,
            'recorded_transaction_statements': 2,
            'payload_sha256': a['payload_sha256'],
            'median_ms_before': statistics.median(a['warm_ms']),
            'median_ms_after': statistics.median(b['warm_ms']),
            'p95_ms_before': sorted(a['warm_ms'])[math.ceil(.95*n)-1],
            'p95_ms_after': sorted(b['warm_ms'])[math.ceil(.95*n)-1],
        })
    result = {'postgres': before['postgres'], 'settings': before['settings'],
              'complete_payload_hashes_identical': True, 'rows': rows}
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()

"""Aggregate the retained A1u before/after samples without service access."""
import json
import math
from pathlib import Path
import statistics

HERE = Path(__file__).resolve().parent


def timing(values):
    ordered = sorted(values)
    return {'n': len(values), 'median_ms': statistics.median(values),
            'p95_nearest_rank_ms': ordered[math.ceil(.95 * len(values)) - 1]}


def main():
    data = json.loads((HERE / 'measurements.json').read_text())
    summaries = []
    for sample in data['samples']:
        item = {'rounds': sample['rounds'], 'variants': {},
                'first_reads': [{key: value for key, value in row.items() if key != 'sql'} for row in sample['first_reads']],
                'browser': sample['browser']}
        for variant in ['baseline', 'candidate']:
            blocks = [b for b in sample['blocks'] if b['variant'] == variant]
            rows = [r for b in blocks if b['mode'] == 'top' for r in b['sql']]
            facts = [r for r in rows if r['fingerprint'].startswith('WITH target AS')]
            locks = [r for r in rows if r['fingerprint'] == 'SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE']
            item['variants'][variant] = {
                'http': {mode: timing([t for b in blocks if b['mode'] == mode for t in b['http_ms']]) for mode in ['none', 'top']},
                'statement_calls_per_read': sum(r['calls'] for r in rows) / 20,
                'sql_mean_ms_per_read': sum(r['total_exec_time'] for r in rows) / 20,
                'bulk_sql_mean_ms_per_read': sum(r['total_exec_time'] for r in facts) / 20,
                'bulk_owner_rows_per_read': sum(r['rows'] for r in facts) / 20,
                'round_lock_rows_per_read': sum(r['rows'] for r in locks) / 20,
                'shared_blocks_read': sum(r['shared_blks_read'] for r in rows),
                'all_body_parity_assertions_passed': all(b['body_equal'] for b in blocks),
            }
        summaries.append(item)
    (HERE / 'summary.json').write_text(json.dumps(summaries, indent=2) + '\n')
    print(json.dumps(summaries, indent=2))


if __name__ == '__main__':
    main()

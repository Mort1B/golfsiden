"""Derive report aggregates from sanitized A1t samples; no service access."""
from collections import Counter
import json
import math
from pathlib import Path
import statistics

HERE = Path(__file__).resolve().parent


def times(values):
    ordered = sorted(values)
    return {'n': len(values), 'median_ms': statistics.median(values),
            'p95_nearest_rank_ms': ordered[math.ceil(len(values) * .95) - 1],
            'min_ms': ordered[0], 'max_ms': ordered[-1]}


def sql_summary(rows, count):
    facts = [r for r in rows if r['fingerprint'].startswith('WITH target AS')]
    locks = [r for r in rows if r['fingerprint'] ==
             'SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE']
    total = sum(r['total_exec_time'] for r in rows)
    facts_total = sum(r['total_exec_time'] for r in facts)
    return {'calls_per_read': sum(r['calls'] for r in rows) / count,
            'sql_mean_ms_per_read': total / count,
            'bulk_fact_mean_ms_per_read': facts_total / count,
            'bulk_fact_share_of_sql_percent': 100 * facts_total / total,
            'bulk_fact_owner_rows_per_read': sum(r['rows'] for r in facts) / count,
            'round_lock_rows_per_read': sum(r['rows'] for r in locks) / count,
            'round_lock_mean_ms_per_read': sum(r['total_exec_time'] for r in locks) / count,
            'shared_blocks_read': sum(r['shared_blks_read'] for r in rows),
            'shared_blocks_hit': sum(r['shared_blks_hit'] for r in rows)}


def main():
    evidence = json.loads((HERE / 'measurements.json').read_text())
    result = []
    for sample in evidence['samples']:
        reads = {mode: times([t for block in sample['reads'] if block['mode'] == mode
                              for t in block['http_ms']]) for mode in ('none', 'top')}
        rows = [r for block in sample['reads'] if block['mode'] == 'top' for r in block['sql']]
        full = [s for s in sample['saves'] if s['mode'] == 'full']
        windows, routes = [], Counter()
        for s in full:
            reqs = s['requests']
            completed = [r for r in reqs if r['method'] == 'GET' and r['state'] == 'completed']
            windows.append({'revision': s['revision'], 'completed_gets': len(completed),
                            'completed_fantasy_gets': sum('/fantasy' in r['path'] for r in completed),
                            'cancelled_gets': sum(r['method'] == 'GET' and r['state'] == 'cancelled' for r in reqs),
                            'conflict_responses': sum(r.get('status') == 409 for r in reqs),
                            'pending_at_tail_cutoff': s['pending_at_tail_cutoff'],
                            'sql_exec_ms_entire_window': sum(r['total_exec_time'] for r in s['sql'])})
            routes.update((r['method'], r['path'], r['state'], r.get('status')) for r in reqs)
        result.append({'rounds': sample['rounds'], 'http_reads': reads,
                       'steady_sql': sql_summary(rows, 20),
                       'first_observed_settled_read': {k: v for k, v in sample['first_observed_settled_read'].items() if k != 'sql'},
                       'save_receipt_latency': {mode: times([s['receipt_ms'] for s in sample['saves'] if s['mode'] == mode])
                                                for mode in ('full', 'minimal')},
                       'observed_save_windows': windows,
                       'observed_save_routes': [{'method': m, 'path': p, 'state': state, 'status': status, 'count': count}
                                                for (m, p, state, status), count in sorted(routes.items(), key=str)],
                       'idle': sample['idle'], 'lock_controls': sample['lock_reads'],
                       'parity': sample['parity'], 'gates': sample['gates']})
    (HERE / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps([{k: v for k, v in row.items() if k != 'observed_save_routes'} for row in result], indent=2))


if __name__ == '__main__':
    main()

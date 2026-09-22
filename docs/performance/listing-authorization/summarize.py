"""Audit and summarize three exclusive release benchmark runs from OUTPUT/run-{1,2,3}."""
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys

source, output = map(Path, sys.argv[1:3])
output.mkdir(parents=True, exist_ok=True)
rows, raw = [], []
for run in range(1, 4):
    for size in [12, 24, 100]:
        data = json.loads((source / f'run-{run}' / f'{size}.json').read_text())
        assert data['membership_removal_denied']
        assert len(data['samples']) == (36 if size == 24 else 18)
        raw.append({'run': run, 'matches': size, **data})
        for sample in data['samples']:
            assert sample['semantic_parity'] and len(sample['warm_ms']) == 10
            k = 1 if sample['filtered'] else size
            role = sample['role']
            expected = 5+7*k+(role != 'viewer')
            selects = [q for q in sample['queries'] if q['query'].startswith('SELECT')]
            calls = sum(q['calls'] for q in selects)/10
            assert calls == expected, (size, role, calls, expected)
            owners = [q for q in selects if q['query'].startswith('SELECT rhs.player_id')]
            owner_calls = sum(q['calls'] for q in owners)/10
            owner_rows = sum(q['rows'] for q in owners)/10
            assert owner_calls == (0 if role == 'viewer' else 1)
            assert owner_rows == (0 if role == 'viewer' else (2*size if role in ['admin', 'scorer'] else 2))
            expected_writable = 0 if role in ['viewer', 'player_other'] or sample['state'] == 'locked' and role != 'admin' else k if role in ['admin', 'scorer'] else 1
            assert sample['writable'] == expected_writable
            categories = {
                'session': sum(q['calls'] for q in selects if q['query'].startswith('SELECT s.id AS session_id'))/10,
                'membership': sum(q['calls'] for q in selects if q['query'].startswith('SELECT role FROM tournament_memberships'))/10,
                'authorization_round': sum(q['calls'] for q in selects if q['query'].startswith('SELECT tournament_id FROM rounds'))/10,
            }
            assert categories == {'session':2, 'membership':1, 'authorization_round':0}
            rows.append({'run': run, **categories, **{f: sample[f] for f in ['matches','state','hidden','role','filtered','fresh_connection_first_ms','warm_ms']},
                         'selects':calls,'owner_calls':owner_calls,'owner_rows':owner_rows,
                         'server_select_ms':sum(q['execution_ms'] for q in selects)/10,
                         'owner_server_ms':sum(q['execution_ms'] for q in owners)/10})

def spread(values):
    return {'median': statistics.median(values), 'min':min(values), 'max':max(values)}
keys = ['matches','state','hidden','role','filtered']
groups = {}
for row in rows:
    groups.setdefault(tuple(row[k] for k in keys), []).append(row)
summary=[]
for key, group in groups.items():
    assert len(group)==3
    assert len({(r['selects'],r['owner_calls'],r['owner_rows']) for r in group})==1
    summary.append({**dict(zip(keys,key)), **{k:group[0][k] for k in ['selects','owner_calls','owner_rows','session','membership','authorization_round']},
                    'warm_ms':spread([v for r in group for v in r['warm_ms']]),
                    'fresh_connection_first_ms':spread([r['fresh_connection_first_ms'] for r in group]),
                    'server_select_ms':spread([r['server_select_ms'] for r in group]),
                    'owner_server_ms':spread([r['owner_server_ms'] for r in group])})
paths=sorted(str(p) for p in Path('backend/src').rglob('*.rs'))
paths += ['backend/tests/singles_match.rs', 'backend/tests/singles_match/listing_authority.rs', 'backend/tests/match_authorization_measurement.rs'] + [str(p) for p in Path('backend/tests/match_authorization_measurement').glob('*.rs')]
metadata={'baseCommit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
          'rust':subprocess.check_output(['rustc','--version'],text=True).strip(),
          'profile':'release; default Cargo release options',
          'comparison':'retained database-authorization baseline; same configured conditions, separately measured',
          'sourceSha256':{p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in paths},
          'conditions':'exclusive PostgreSQL 17 container; tmpfs data; loopback TCP; one pooled measurement connection; 3 warmups then 10 samples; new-connection first call excludes connect time; DB/OS caches not flushed'}
(output/'evidence.json').write_text(json.dumps({'metadata':metadata,'runs':raw},indent=2)+'\n')
(output/'summary.json').write_text(json.dumps({'samples':len(rows)*10,'fresh_connection_samples':len(rows),'cases':len(summary),'counts_reproduced':True,'groups':summary},indent=2)+'\n')
print(f'{len(summary)} cases; {len(rows)*10} warmed samples; exact query counts/owner rows reproduced in all 3 runs')

baseline_path=Path('docs/performance/database-authorization/summary.json')
baseline=json.loads(baseline_path.read_text())['groups']
comparison=[]
for current in summary:
    previous=next(row for row in baseline if all(row[k]==current[k] for k in keys))
    comparison.append({**{k:current[k] for k in keys},
        'before':{k:previous[k] for k in ['selects','owner_calls','owner_rows','warm_ms']},
        'after':{k:current[k] for k in ['selects','owner_calls','owner_rows','warm_ms']}})
(output/'comparison.json').write_text(json.dumps({'baseline_sha256':hashlib.sha256(baseline_path.read_bytes()).hexdigest(),
    'timing_caveat':'separately measured sessions under the same configured conditions; not an interleaved trial',
    'cases':comparison},indent=2)+'\n')

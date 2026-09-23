import subprocess,json,hashlib,sys
from pathlib import Path
root=Path('/tmp/golf-rehearsal-738ec1d')
project=sys.argv[1]
assert project in ['source','restore']
container=f'golf-rehearsal-{project}-738ec1d-postgres-1'
def query(sql):
 p=subprocess.run(['podman','exec','-i',container,'psql','-U','golfsiden_owner','-d','golfsiden','-X','-q','-t','-A','-v','ON_ERROR_STOP=1'],input=sql,text=True,capture_output=True,check=True)
 return p.stdout.strip()
names=query("SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename;").splitlines()
result={}
for name in names:
 identifier='"'+name.replace('"','""')+'"'
 rows=query(f'SELECT row_to_json(t)::text FROM public.{identifier} t ORDER BY row_to_json(t)::text;')
 result[name]={'rows':len(rows.splitlines()),'sha256':hashlib.sha256(rows.encode()).hexdigest()}
(root/f'{project}-rows.json').write_text(json.dumps(result,indent=2)+'\n')
print(f'{project}: {len(result)} public tables fingerprinted; {sum(x["rows"] for x in result.values())} rows')
print(json.dumps({k:v['rows'] for k,v in result.items() if v['rows']},sort_keys=True))
if project=='restore':
 assert result==json.loads((root/'source-rows.json').read_text()),'Restored table data differs'
 print('Every table count and canonical row-data SHA-256 matches source exactly.')

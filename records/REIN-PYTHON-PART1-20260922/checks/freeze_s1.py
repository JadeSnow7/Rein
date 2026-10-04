"""Freeze local Python S1 content and observed offline cases; never call live mode."""
import hashlib
import json
import platform
from pathlib import Path
import subprocess
import sys
root=Path(__file__).resolve().parents[3]
part=root/'python/part1'
files=[p for p in part.rglob('*') if p.is_file() and not any(x in p.parts for x in ['.venv','__pycache__']) and p.name!='S1-manifest.json']
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
file_hashes={str(p.relative_to(root)):sha(p) for p in sorted(files)}
if '--check' in sys.argv:
    saved=json.loads((part/'S1-manifest.json').read_text())
    assert saved['source_files']==file_hashes,'S1 no longer matches current source or fixtures'
    print(json.dumps({'passed':True,'manifest_files':len(file_hashes)}))
    raise SystemExit(0)
scenarios=[('F01-opinion',['hello'],0,'opinion'),('F02-direct',['read','--read-mode','direct'],0,'read'),('F02-tool',['read'],0,'read'),('C01',['suggest'],0,'unique'),('C01-replay',['suggest','--response','python/part1/fixtures/responses/c01.json'],0,'unique'),('C02-correct',['suggest','--workspace','python/part1/fixtures/correct'],0,'no_change'),('C02-duplicate',['suggest','--workspace','python/part1/fixtures/duplicate'],1,'ambiguous_match'),('C03-empty',['hello','--response','python/part1/fixtures/responses/empty.json'],1,'empty_final'),('C03-timeout',['hello','--response','python/part1/fixtures/responses/timeout.json','--timeout','0.02'],1,'timeout'),('C04-path',['read','--path','../README.md'],1,'path_invalid')]
observed=[]
for name,args,code,status in scenarios:
    command=['python3','python/part1/rein.py',*args]
    result=subprocess.run(command,cwd=root,text=True,capture_output=True,timeout=5)
    value=json.loads(result.stdout)
    assert result.returncode==code and value['result']['status']==status,(name,result.stdout,result.stderr)
    observed.append({'case':name,'argv':command,'exit':code,'record':value['record'],'result':value['result']})
assert all(sha(root/p)==digest for p,digest in file_hashes.items()),'S1 input changed'
manifest={'version':'S1-python/1','framework':'six-parts-framework/1','scope':'00-03 Python suggestion-only teaching artifact; not a Git tag or Rust migration','environment':{'python':platform.python_version(),'sdk':'openai==2.26.0; validated separately with HTTPX MockTransport'},'limits':{'default_deadline_seconds':30,'max_file_bytes':4096},'source_files':file_hashes,'cases':observed,'further_boundary_cases':['python/part1/tests/test_part1_contract.py','python/part1/tests/test_sdk_mock.py'],'not_verified':['live model/provider availability and semantic quality','novice-reader learning acceptance','Rust equivalence in new chapter04'],'targets_unchanged':True}
(part/'S1-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'passed':True,'files':len(file_hashes),'cases':len(observed),'manifest':'python/part1/S1-manifest.json'}))

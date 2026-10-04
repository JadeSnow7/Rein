"""Primary-thread black-box checks, independent of implementation helpers."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

root=Path(__file__).resolve().parents[3]
cli=root/'python/part1/rein.py'
observations=[]
def check(args, expected, status, count=None):
    started=time.monotonic()
    p=subprocess.run([sys.executable,str(cli),*args],cwd=root,capture_output=True,text=True,timeout=4)
    elapsed=time.monotonic()-started
    assert p.returncode==expected,(args,p.returncode,p.stdout,p.stderr)
    data=json.loads(p.stdout)
    assert data['result']['status']==status,(args,data)
    if count is not None: assert data['record']['request_count']==count,(args,data)
    observations.append({'args':args,'exit':p.returncode,'status':status,'seconds':elapsed,'request_count':data['record'].get('request_count')})
    return data

with tempfile.TemporaryDirectory(prefix='rein-cli-review-') as directory:
    folder=Path(directory)
    target=folder/'README.md';target.write_bytes(b'header\n\nnpm run start\n')
    (folder/'notes.md').write_text('not allowed')
    before=target.read_bytes()
    base=['--workspace',directory]
    opinion=check(['hello',*base],0,'opinion',1)
    assert 'raw_digest' in json.dumps(opinion),opinion
    check(['read','--read-mode','direct',*base],0,'read',1)
    check(['read',*base],0,'read',2)
    unique=check(['suggest',*base],0,'unique',2)
    assert unique['result']['start_line']==3
    assert unique['result']['source_digest']==hashlib.sha256(before).hexdigest()
    check(['read','--path','notes.md',*base],1,'path_invalid')
    check(['read','--path','../README.md',*base],1,'path_invalid')
    for timeout in ['0','-1','nan','inf']:
        check(['hello','--timeout',timeout,*base],1,'invalid_task',0)
    check(['hello','--mode','live','--response','unused.json'],1,'invalid_task',0)
    check(['hello','--mode','live','--timeout','0'],1,'invalid_task',0)
    call={'tool_call':{'id':'review-read','name':'read_file','arguments':{'path':'README.md'}}}
    valid={'path':'README.md','original':'npm run start','suggested':'npm run dev','reason':'maintainer instruction'}
    def replay(events):
        path=folder/'replay.json';path.write_text(json.dumps(events));return ['--response',str(path)]
    for event, expected_status in [({'text':''},'empty_final'),({'choices':[]},'response_invalid'),({'delay':0.2,'text':'too late'},'timeout')]:
        check(['hello',*base,*replay([event]),'--timeout','0.02'],1,expected_status,1)
    check(['suggest',*base,*replay([call,{'delay':0.2,'suggestion':valid}]),'--timeout','0.02'],1,'timeout',2)
    for patch,status in [({'start_line':88},'response_invalid'),({'path':'elsewhere.md'},'path_invalid'),({'reason':''},'response_invalid'),({'original':''},'response_invalid'),({'original':'run start'},'original_missing')]:
        check(['suggest',*base,*replay([call,{'suggestion':{**valid,**patch}}])],1,status,2)
    deleted=check(['suggest',*base,*replay([call,{'suggestion':{**valid,'suggested':''}}])],0,'unique',2)
    assert deleted['result']['suggested']==''
    assert target.read_bytes()==before
    target.write_bytes(b'npm run start\n\nnpm run start\n')
    dup=check(['suggest',*base],1,'ambiguous_match',2)
    assert dup['result']['candidates']==[1,3]
    target.write_bytes(b'npm run dev\n')
    check(['suggest',*base],0,'no_change',2)
print(json.dumps({'passed':True,'cases':len(observations),'network':False,'observations':observations},ensure_ascii=False,indent=2))

"""Run public Harness APIs through the actual SDK and an in-memory HTTP transport."""
import asyncio
import json
from pathlib import Path
import sys
import tempfile
import httpx
from openai import AsyncOpenAI
root=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(root/'python/part1'))
from rein_core import OpenAIAdapter, Task, run

def completion(message):
    return {'id':'local-mock','object':'chat.completion','created':0,'model':'fixture-model','choices':[{'index':0,'finish_reason':'tool_calls' if message.get('tool_calls') else 'stop','message':{'role':'assistant',**message}}]}

async def main():
    reports=[]
    with tempfile.TemporaryDirectory(prefix='rein-sdk-review-') as directory:
        folder=Path(directory);target=folder/'README.md';target.write_text('npm run start\n')
        for scenario in ['hello','read','suggest','read-second-tool']:
            stage='read' if scenario=='read-second-tool' else scenario
            requests=[]
            async def handler(request):
                body=json.loads(request.content);requests.append(body)
                text=json.dumps(body['messages'],ensure_ascii=False)
                assert all(set(m) <= {'role','content','tool_calls','tool_call_id','name'} for m in body['messages']),body['messages']
                assert 'npm run dev' in text
                if stage=='hello':
                    assert 'tools' not in body
                    assert 'npm run start' not in text
                    return httpx.Response(200,json=completion({'content':'先检查真实文件中的启动说明'}))
                if len(requests)==1:
                    assert body['tools'][0]['function']['name']=='read_file'
                    assert body['tools'][0]['function']['parameters']['additionalProperties'] is False
                    return httpx.Response(200,json=completion({'content':'先读取 README。','tool_calls':[{'id':'sdk-read-1','type':'function','function':{'name':'read_file','arguments':'{"path":"README.md"}'}}]}))
                assert len(requests)==2
                assert not body.get('tools') or body.get('tool_choice')=='none'
                tool=next(m for m in body['messages'] if m['role']=='tool')
                assert tool['tool_call_id']=='sdk-read-1'
                assert 'npm run start' in tool['content']
                assert any(m.get('tool_calls') for m in body['messages'] if m['role']=='assistant')
                if scenario=='read-second-tool':
                    return httpx.Response(200,json=completion({'content':'继续读取', 'tool_calls':[{'id':'again','type':'function','function':{'name':'read_file','arguments':'{"path":"README.md"}'}}]}))
                final='本轮 README 仍含旧命令' if stage=='read' else json.dumps({'path':'README.md','original':'npm run start','suggested':'npm run dev','reason':'维护者给出的当前命令'},ensure_ascii=False)
                return httpx.Response(200,json=completion({'content':final}))
            async with AsyncOpenAI(api_key='mock-key',base_url='https://fixture.invalid/v1',max_retries=0,http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler))) as client:
                adapter=OpenAIAdapter(client,'fixture-model',retries=0)
                task=Task('维护者给出的当前开发命令是 npm run dev；请检查 README',folder,('README.md',),'opinion' if stage=='hello' else 'suggest')
                record=await run(task,stage,adapter,timeout=2)
                expected='error' if scenario=='read-second-tool' else {'hello':'opinion','read':'read','suggest':'unique'}[stage]
                assert record.status==expected,(stage,record)
                assert record.request_count==(1 if stage=='hello' else 2),(stage,record)
                assert record.error==('response_invalid' if scenario=='read-second-tool' else None),(scenario,record)
            assert client.is_closed()
            assert target.read_text()=='npm run start\n'
            reports.append({'stage':scenario,'requests':len(requests),'status':record.status})
    print(json.dumps({'passed':True,'network':False,'stages':reports},ensure_ascii=False))
asyncio.run(main())

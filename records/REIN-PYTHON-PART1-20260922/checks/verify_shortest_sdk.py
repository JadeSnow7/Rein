"""Execute the final chapter's shortest SDK code against HTTPX MockTransport."""
import json
import os
from pathlib import Path
import re
from unittest.mock import patch
import httpx
import openai

root = Path(__file__).resolve().parents[3]
text = (root / 'docs/chapters/python-model-call.md').read_text()
blocks = re.findall(r'```python\n(.*?)\n```', text, re.S)
code = next(b for b in blocks if 'client.chat.completions.create(' in b)
real_client = openai.AsyncOpenAI
requests = []
def handler(request):
    payload = json.loads(request.content)
    requests.append(payload)
    assert payload['messages'][0]['role'] == 'user'
    assert payload['model'] == 'mock-model'
    assert payload['stream'] is False
    assert 'tools' not in payload
    return httpx.Response(200, json={'id': 'mock', 'object': 'chat.completion', 'created': 0, 'model': 'mock-model', 'choices': [{'index': 0, 'finish_reason': 'stop', 'message': {'role': 'assistant', 'content': 'shortest SDK mock passed'}}]})
def client_factory(**kwargs):
    assert kwargs['max_retries'] == 0
    return real_client(**kwargs, http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler)))
with patch.dict(os.environ, {'REIN_BASE_URL': 'https://fixture.invalid/v1', 'REIN_API_KEY': 'test-only-key', 'REIN_MODEL': 'mock-model'}), patch.object(openai, 'AsyncOpenAI', client_factory):
    exec(compile(code, 'chapter01-shortest-sdk', 'exec'), {'__name__': '__main__'})
assert len(requests) == 1
print(json.dumps({'passed': True, 'sdk': openai.__version__, 'requests': len(requests), 'network': False}))

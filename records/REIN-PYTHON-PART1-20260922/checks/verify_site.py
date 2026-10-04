import hashlib
import json
from pathlib import Path
import re

root=Path(__file__).resolve().parents[3]
record=root/'records/REIN-PYTHON-PART1-20260922'
baseline=json.loads((record/'baseline/manifest.json').read_text())
preserved=['book/chapters.json','docs/chapters/task-map.md','docs/chapters/model-hello.md','docs/chapters/task-spec.md','docs/chapters/tool-roundtrip.md','docs/chapters/model-hello-rust.md']
for path in preserved:
    assert hashlib.sha256((root/path).read_bytes()).hexdigest()==baseline['files'][path], path
slugs=['minimal-agent','python-model-call','python-file-read','python-suggestions']
for i,slug in enumerate(slugs):
    html=(root/'docs/.vitepress/dist/chapters'/f'{slug}.html').read_text()
    assert '第一部分' in html and 'Python' in html
    assert not re.search(r'class="[^"]*source-version-switch',html), slug
    match=re.search(r'<nav[^>]*class="[^"]*prev-next.*?</nav>',html)
    # Check the generated pager rather than unrelated sidebar links.
    if not match: match=re.search(r'<div[^>]*class="[^"]*prev-next.*?</div>\s*</footer>',html)
    assert match, f'pager absent {slug}'
    pager=match[0]
    expected=f'{slugs[i+1]}.html' if i<3 else 'toc.html'
    assert expected in pager, (slug,expected,pager)
    assert 'provider-adapter.html' not in pager
    if i>0: assert f'{slugs[i-1]}.html' in pager
print(json.dumps({'passed':True,'new_chapters':4,'historical_files_unchanged':preserved},ensure_ascii=False))

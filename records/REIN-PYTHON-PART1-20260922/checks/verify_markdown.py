"""Run final Markdown's offline shell blocks in order, preserving shell variables."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
chapters = ['minimal-agent', 'python-model-call', 'python-file-read', 'python-suggestions']
selected = []
skipped = []
for chapter in chapters:
    file = ROOT / 'docs' / 'chapters' / (chapter + '.md')
    for number, block in enumerate(re.findall(r'```bash\n(.*?)\n```', file.read_text(), re.S), 1):
        if any(marker in block for marker in ['pip install', 'export REIN_', '--mode live']):
            skipped.append({'chapter': chapter, 'block': number, 'reason': 'credential/service setup; separately mock SDK; no live request'})
            continue
        expected = 1 if any(marker in block for marker in ['--response "$part1_response_dir/empty.json"', '--response "$part1_response_dir/broken.json"', '--path ../README.md', 'write_bytes(b"a" * 4097)', 'fixtures/duplicate', '--response "$part1_candidate_dir/replay.json"']) else 0
        selected.append((chapter, number, block, expected))
fixtures = ROOT / 'python/part1/fixtures'
before = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in fixtures.rglob('*') if p.is_file()}
with tempfile.TemporaryDirectory(prefix='rein-markdown-') as tmp:
    base = Path(tmp)
    commands = ['#!/bin/bash', 'set +e']
    for i, (chapter, number, block, expected) in enumerate(selected):
        path = base / f'block-{i}.sh'; path.write_text(block + '\n')
        output = base / f'output-{i}.txt'
        commands += [f'source {path} > {output} 2>&1', f'printf "%s" "$?" > {base / f"exit-{i}.txt"}']
    script = base / 'all.sh'; script.write_text('\n'.join(commands) + '\n')
    proc = subprocess.run(['bash', str(script)], cwd=ROOT, capture_output=True, text=True, timeout=150)
    assert proc.returncode == 0, proc.stderr
    results = []
    for i, (chapter, number, block, expected) in enumerate(selected):
        code = int((base / f'exit-{i}.txt').read_text())
        output = (base / f'output-{i}.txt').read_text()
        assert code == expected, (chapter, number, code, expected, output)
        if 'empty.json' in block: assert 'empty_final' in output
        if 'broken.json' in block or 'candidate.pop' in block: assert 'response_invalid' in output
        if '--path ../README.md' in block: assert 'path_invalid' in output
        if 'fixtures/duplicate' in block: assert 'ambiguous_match' in output
        if 'fixtures/correct' in block and 'cp ' not in block: assert 'no_change' in output
        results.append({'chapter': chapter, 'block': number, 'command': block, 'exit': code, 'output': output})
after = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in fixtures.rglob('*') if p.is_file()}
assert before == after, 'tutorial altered checked-in fixtures'
print(json.dumps({'passed': True, 'blocks': len(results), 'skipped': skipped, 'fixtures_unchanged': True, 'results': results}, ensure_ascii=False, indent=2))

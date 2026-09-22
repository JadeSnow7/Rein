# Python Part 1 API contract

This contract freezes the small, offline-first surface used by chapters 00–03.
The implementation is intentionally standard-library only in offline mode. A
live adapter may be installed separately with `openai==2.26.0`, but live calls
are never made by the tests or by the default command.

## Package surface

`python/part1/rein.py` is the executable entry point. The implementation may
split code into modules under the same directory, but these names and meanings
remain stable:

```python
Task(goal: str, workspace: Path, allowed_paths: tuple[str, ...], acceptance: str, requested_path: str = "README.md")
ModelResponse(text: str, raw_digest: str, provider: str)
RunRecord(task_id: str, request_count: int, status: str, error: str | None, source_digest: str | None = None, result: dict | None = None, messages: list[dict] | None = None)
ToolCall(id: str, name: str, arguments: dict[str, object])
ToolResult(tool_call_id: str, ok: bool, output: str | None, error: str | None)
safe_read(workspace: Path, path: str, max_bytes: int = 4096) -> ReadResult
ModelAdapter.request(task: Task, messages: list[dict[str, object]], deadline: float) -> ModelResponse
async run(task: Task, stage: str, adapter: ModelAdapter, read_mode: str = "tool", timeout: float = 30.0) -> RunRecord
verify_source(workspace: Path, source: ReadResult) -> ReadResult
load_config(environ: Mapping[str, str]) -> LiveConfig
OpenAIAdapter(client: object, model: str, retries: int = 0)  # async request; tools are selected per run stage
OfflineAdapter(events: list[dict[str, object]] | None = None)  # derives suggestions from supplied tool/direct content
```

`ReadResult` exposes `path`, decoded `text`, raw-byte `digest`, and `size` on
success; failures expose a structured `error` category. The exact concrete
class is implementation detail, but callers must not receive fabricated text
for a failed read.

The suggestion result is JSON-shaped and contains only program-derived
position data:

```json
{
  "status": "unique",
  "path": "README.md",
  "start_line": 8,
  "end_line": 8,
  "original": "npm run start",
  "suggested": "npm run dev",
  "reason": "the preview command is npm run dev",
  "source_digest": "..."
}
```

Allowed statuses are `unique`, `ambiguous_match`, `original_missing`,
`no_change`, `response_invalid`, `path_invalid`, `source_changed`, and the
bounded runtime errors `model_error`, `empty_final`, and `timeout`.

## CLI

Run from the repository root:

```text
python3 python/part1/rein.py hello
python3 python/part1/rein.py read --workspace python/part1/fixtures/outdated --path README.md
python3 python/part1/rein.py suggest --workspace python/part1/fixtures/outdated --path README.md
```

Defaults are `--mode offline`, `--read-mode tool`, and `--timeout 30`. The
optional `--response FILE` supplies a JSON replay. `--mode live` is the only
mode allowed to inspect `REIN_BASE_URL`, `REIN_API_KEY`, and `REIN_MODEL`.
Every command emits JSON to stdout and writes no target file. Exit status 0
means a successful opinion/read/suggestion, including `no_change`; status 1
means a structured failure.

`read` results include the final non-empty model `text`, `provider`,
`raw_digest`, source path, and source digest. `RunRecord.messages` retains the
system/user/assistant/tool messages. Tool results include `ok`, `output`, and
`error`; failed reads keep `ok: false`, `output: null`, and the original call
ID. A second tool call is invalid because the second request is final-only.

## Frozen behavior

`safe_read` accepts only a workspace-relative path, rejects absolute paths,
`..`, symlink components, directories, invalid UTF-8, and files larger than
4096 bytes. It reads at most 4097 bytes and checks the final size. Direct and
tool reads use this same function. The implementation documents that this is
for a self-controlled teaching directory and is not an OS security boundary.

The tool flow has one `read_file` call and at most two model requests. The
second offline response must consume the current tool result; changing the
fixture changes the request evidence. A model-supplied line number is invalid:
the program maps the returned `original` text to the current source and owns
`start_line`/`end_line`. Duplicate matches are ambiguous and never guessed.
Before emitting a candidate, the source digest is checked again; a changed
source yields `source_changed`. No stage applies a suggestion.

The offline `ReplayAdapter` consumes a JSON sequence of typed events rather
than a fixed final sentence. A tool event is
`{"tool_call":{"id":"call_c01","name":"read_file","arguments":{"path":"README.md"}}}`;
the next response receives the actual `ToolResult` identified by that ID.
The adapter must expose the messages it received so tests can compare direct
and tool reads and prove that changing the file changes the second request.
Missing tool arguments and unknown tools are structured failures. A replay
entry can also model non-JSON, empty choices, empty text, transport failure,
or a late response; all are bounded and never become a suggestion. An empty
`choices` array is `response_invalid`; `empty_final` is reserved for a valid
response whose final text is empty.
`load_config` reports `config_missing` when any of the three live environment
variables is absent; it never includes the API key in a `RunRecord` or replay.

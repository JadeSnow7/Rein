"""Small offline-first file suggestion harness used by chapters 00-03."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import math
import time
from urllib.parse import urlparse
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any, Mapping


class ReinError(Exception):
    def __init__(self, code: str, detail: str = ""):
        self.code = code
        self.detail = detail
        super().__init__(f"{code}: {detail}" if detail else code)


@dataclass(frozen=True)
class Task:
    goal: str
    workspace: Path
    allowed_paths: tuple[str, ...]
    acceptance: str
    requested_path: str = "README.md"


@dataclass(frozen=True)
class ReadResult:
    path: str
    text: str
    digest: str
    size: int


@dataclass(frozen=True)
class ModelResponse:
    text: str
    raw_digest: str
    provider: str
    event: dict[str, Any] | None = None


@dataclass
class RunRecord:
    task_id: str
    request_count: int
    status: str
    error: str | None = None
    source_digest: str | None = None
    result: dict[str, Any] | None = None
    messages: list[dict[str, Any]] | None = None


@dataclass(frozen=True)
class ToolCall:
    id: str
    name: str
    arguments: dict[str, Any]


@dataclass(frozen=True)
class ToolResult:
    tool_call_id: str
    ok: bool
    output: str | None = None
    error: str | None = None


@dataclass(frozen=True)
class LiveConfig:
    base_url: str
    api_key: str
    model: str


def _relative_path(path: str) -> Path:
    candidate = Path(path)
    if not path or candidate.is_absolute() or any(part == ".." for part in candidate.parts):
        raise ReinError("path_invalid", "path must be relative and stay inside workspace")
    return candidate


def safe_read(workspace: Path, path: str, max_bytes: int = 4096) -> ReadResult:
    if not isinstance(path, str):
        raise ReinError("path_invalid", "path must be a string")
    rel = _relative_path(path)
    root = Path(workspace)
    if root.is_symlink():
        raise ReinError("path_invalid", "workspace symlink is not allowed")
    current = root
    for part in rel.parts:
        current = current / part
        if current.is_symlink():
            raise ReinError("path_invalid", "symlink components are not allowed")
    target = root / rel
    try:
        root_real = root.resolve()
        target_real = target.resolve()
        if target_real != root_real and root_real not in target_real.parents:
            raise ReinError("path_invalid", "target escapes workspace")
        if not target.is_file():
            raise ReinError("path_invalid", "target is not a regular file")
        with target.open("rb") as handle:
            raw = handle.read(max_bytes + 1)
        if len(raw) > max_bytes or target.stat().st_size > max_bytes:
            raise ReinError("file_too_large", f"limit is {max_bytes} bytes")
        text = raw.decode("utf-8")
    except ReinError:
        raise
    except UnicodeDecodeError as exc:
        raise ReinError("invalid_utf8", str(exc)) from exc
    except OSError as exc:
        raise ReinError("path_invalid", str(exc)) from exc
    return ReadResult(str(rel), text, hashlib.sha256(raw).hexdigest(), len(raw))


def verify_source(workspace: Path, source: ReadResult) -> ReadResult:
    current = safe_read(workspace, source.path)
    if current.digest != source.digest:
        raise ReinError("source_changed", "source changed after it was read")
    return current


def locate_suggestion(source: ReadResult, payload: Mapping[str, Any], expected_digest: str | None = None) -> dict[str, Any]:
    if not isinstance(payload, Mapping):
        return {"status": "response_invalid", "path": source.path, "source_digest": source.digest}
    required = {"path", "original", "suggested", "reason"}
    allowed = required | {"status"}
    if not required <= set(payload) or not set(payload) <= allowed or set(payload) & {"start_line", "end_line"}:
        return {"status": "response_invalid", "path": source.path, "source_digest": source.digest}
    if payload["path"] != source.path:
        return {"status": "path_invalid", "path": source.path, "source_digest": source.digest}
    if not all(isinstance(payload[key], str) for key in required) or not payload["original"] or not payload["reason"]:
        return {"status": "response_invalid", "path": source.path, "source_digest": source.digest}
    if "status" in payload and payload["status"] not in {"unique", "no_change"}:
        return {"status": "response_invalid", "path": source.path, "source_digest": source.digest}
    if expected_digest is not None and expected_digest != source.digest:
        return {"status": "source_changed", "path": source.path, "source_digest": source.digest}
    original = str(payload["original"]).replace("\r\n", "\n").replace("\r", "\n")
    lines = source.text.replace("\r\n", "\n").replace("\r", "\n").splitlines()
    wanted = original.split("\n")
    matches = [index for index in range(len(lines) - len(wanted) + 1) if lines[index:index + len(wanted)] == wanted]
    base = {"path": source.path, "original": payload["original"], "suggested": payload["suggested"], "reason": payload["reason"], "source_digest": source.digest}
    if not matches:
        return {**base, "status": "original_missing"}
    if len(matches) > 1:
        return {**base, "status": "ambiguous_match", "candidates": [index + 1 for index in matches]}
    start = matches[0] + 1
    result = {**base, "status": "no_change" if payload["original"] == payload["suggested"] else "unique", "start_line": start, "end_line": start + len(wanted) - 1}
    return result


class ModelAdapter:
    async def request(self, task: Task, messages: list[dict[str, Any]], deadline: float) -> ModelResponse:
        raise NotImplementedError


class ReplayAdapter(ModelAdapter):
    def __init__(self, events: list[dict[str, Any]]):
        self.events = list(events)
        self.messages: list[dict[str, Any]] = []

    async def request(self, task: Task, messages: list[dict[str, Any]], deadline: float) -> ModelResponse:
        if time.monotonic() >= deadline:
            raise ReinError("timeout", "deadline elapsed")
        self.messages = list(messages)
        if not self.events:
            raise ReinError("response_invalid", "replay exhausted")
        event = self.events.pop(0)
        if not isinstance(event, dict):
            raise ReinError("response_invalid", "replay event must be an object")
        delay = event.get("delay")
        if delay:
            await asyncio.sleep(float(delay))
        if time.monotonic() >= deadline:
            raise ReinError("timeout", "deadline elapsed")
        raw = json.dumps(event, ensure_ascii=False, sort_keys=True)
        if event.get("error") == "non_json":
            raise ReinError("response_invalid", "replay is not JSON")
        if event.get("error") == "model_error":
            raise ReinError("model_error", "replayed model error")
        text = event.get("text") if isinstance(event.get("text"), str) else raw
        return ModelResponse(text, hashlib.sha256(raw.encode()).hexdigest(), event.get("provider", "replay"), event)


class OfflineAdapter(ModelAdapter):
    """Deterministic adapter that derives its final answer from tool content."""
    def __init__(self, events: list[dict[str, Any]] | None = None):
        self.messages: list[dict[str, Any]] = []
        self._replay = ReplayAdapter(events) if events else None

    async def request(self, task: Task, messages: list[dict[str, Any]], deadline: float) -> ModelResponse:
        if self._replay:
            return await self._replay.request(task, messages, deadline)
        self.messages = list(messages)
        if task.acceptance == "opinion":
            event = {"text": "请检查 README 的 start 命令", "provider": "offline"}
            raw = json.dumps(event, ensure_ascii=False)
            return ModelResponse(event["text"], hashlib.sha256(raw.encode()).hexdigest(), "offline", event)
        material = next((message["content"] for message in messages if message.get("role") == "tool"), None)
        if material is None:
            material = next((message["content"] for message in messages if message.get("role") == "user" and message.get("content") != task.goal), None)
        if material is not None:
            content = material
            try:
                data = json.loads(content)
                text = data["text"]
            except (ValueError, KeyError, TypeError) as exc:
                raise ReinError("response_invalid", str(exc)) from exc
            lines = text.replace("\r\n", "\n").replace("\r", "\n").splitlines()
            for line in lines:
                if "npm run start" in line:
                    replacement = line.replace("npm run start", "npm run dev")
                    event = {"suggestion": {"path": task.requested_path, "original": line, "suggested": replacement, "reason": "preview uses the development command"}}
                    raw = json.dumps(event, ensure_ascii=False)
                    return ModelResponse(raw, hashlib.sha256(raw.encode()).hexdigest(), "offline", event)
                if "npm run dev" in line:
                    event = {"suggestion": {"path": task.requested_path, "original": line, "suggested": line, "reason": "preview already uses the development command"}}
                    raw = json.dumps(event, ensure_ascii=False)
                    return ModelResponse(raw, hashlib.sha256(raw.encode()).hexdigest(), "offline", event)
            raise ReinError("empty_final", "no command found")
        event = {"tool_call": {"id": "offline_read_1", "name": "read_file", "arguments": {"path": task.requested_path}}}
        raw = json.dumps(event)
        return ModelResponse(raw, hashlib.sha256(raw.encode()).hexdigest(), "offline", event)


class OpenAIAdapter(ModelAdapter):
    def __init__(self, client: Any, model: str, retries: int = 0):
        self.client, self.model, self.retries = client, model, retries
        if hasattr(client, "max_retries"):
            client.max_retries = retries

    async def request(self, task: Task, messages: list[dict[str, Any]], deadline: float) -> ModelResponse:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ReinError("timeout", "deadline elapsed")
        try:
            wire_messages = []
            for message in messages:
                wire = {key: message[key] for key in ("role", "content", "tool_calls", "tool_call_id", "name") if key in message}
                wire_messages.append(wire)
            kwargs = {"model": self.model, "messages": wire_messages}
            if getattr(self, "request_tools", False):
                kwargs["tools"] = [{"type": "function", "function": {"name": "read_file", "description": "Read one authorized UTF-8 file", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"], "additionalProperties": False}}}]
                kwargs["tool_choice"] = {"type": "function", "function": {"name": "read_file"}}
            response = await asyncio.wait_for(self.client.chat.completions.create(**kwargs), remaining)
        except asyncio.TimeoutError as exc:
            raise ReinError("timeout", "deadline elapsed") from exc
        except Exception as exc:
            if "timeout" in type(exc).__name__.lower():
                raise ReinError("timeout", "provider timeout") from exc
            raise ReinError("model_error", type(exc).__name__) from exc
        if isinstance(response, Mapping):
            choices = response.get("choices")
            if not isinstance(choices, list) or not choices:
                raise ReinError("response_invalid", "choices missing or empty")
            message = choices[0].get("message", {})
            content = message.get("content")
            tool_calls = message.get("tool_calls")
        else:
            choices = getattr(response, "choices", None)
            if not isinstance(choices, (list, tuple)) or not choices:
                raise ReinError("response_invalid", "choices missing or empty")
            message = getattr(choices[0], "message", None)
            content = getattr(message, "content", None)
            tool_calls = getattr(message, "tool_calls", None)
        if tool_calls and not getattr(self, "request_tools", False):
            raise ReinError("response_invalid", "tool call is not allowed in final response")
        if not isinstance(content, (str, type(None))):
            raise ReinError("response_invalid", "content must be text")
        if (getattr(self, "request_tools", False) and tool_calls) or not content or not content.strip():
            if tool_calls:
                if len(tool_calls) != 1:
                    raise ReinError("response_invalid", "exactly one tool call is required")
                call = tool_calls[0]
                if isinstance(call, Mapping):
                    function = call.get("function", {})
                    call_id, name, arguments = call.get("id"), function.get("name"), function.get("arguments")
                else:
                    function = getattr(call, "function", None)
                    call_id, name, arguments = getattr(call, "id", None), getattr(function, "name", None), getattr(function, "arguments", None)
                if not isinstance(call_id, str) or name != "read_file" or not isinstance(arguments, str):
                    raise ReinError("response_invalid", "invalid tool call")
                try:
                    parsed = json.loads(arguments)
                except json.JSONDecodeError as exc:
                    raise ReinError("response_invalid", "invalid tool arguments") from exc
                if not isinstance(parsed, dict) or not isinstance(parsed.get("path"), str) or set(parsed) != {"path"}:
                    raise ReinError("response_invalid", "invalid tool arguments")
                event = {"tool_call": {"id": call_id, "name": name, "arguments": parsed}}
                raw = json.dumps(event, ensure_ascii=False)
                return ModelResponse(raw, hashlib.sha256(raw.encode()).hexdigest(), "openai", event)
            raise ReinError("empty_final", "empty model content")
        raw = str(content)
        return ModelResponse(raw, hashlib.sha256(raw.encode()).hexdigest(), "openai")


def load_config(environ: Mapping[str, str]) -> LiveConfig:
    values = [environ.get(name) for name in ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL")]
    if any(not isinstance(value, str) or not value.strip() for value in values):
        raise ReinError("config_missing", "REIN_BASE_URL, REIN_API_KEY and REIN_MODEL are required")
    parsed = urlparse(values[0])
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ReinError("config_invalid", "REIN_BASE_URL must use HTTP(S)")
    return LiveConfig(*values)  # type: ignore[arg-type]


async def _request(adapter: ModelAdapter, task: Task, messages: list[dict[str, Any]], deadline: float) -> ModelResponse:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise ReinError("timeout", "deadline elapsed")
    try:
        response = await asyncio.wait_for(adapter.request(task, messages, deadline), remaining)
        if time.monotonic() > deadline:
            raise ReinError("timeout", "late response discarded")
        return response
    except asyncio.TimeoutError as exc:
        raise ReinError("timeout", "deadline elapsed") from exc


def _event(response: ModelResponse) -> dict[str, Any]:
    if response.event is not None:
        return response.event
    try:
        value = json.loads(response.text)
    except json.JSONDecodeError as exc:
        raise ReinError("response_invalid", "response is not JSON") from exc
    if not isinstance(value, dict):
        raise ReinError("response_invalid", "response must be an object")
    return value


def _final_text(response: ModelResponse) -> str:
    if response.event is not None:
        if "tool_call" in response.event or "tool_calls" in response.event:
            raise ReinError("response_invalid", "tool call is not allowed in final response")
        if "choices" in response.event:
            raise ReinError("response_invalid", "choices wrapper is not a final text response")
        text = response.event.get("text")
        if text is None and isinstance(response.event.get("suggestion"), dict):
            text = response.event["suggestion"].get("reason")
    else:
        text = response.text
    if not isinstance(text, str) or not text.strip():
        raise ReinError("empty_final", "empty final text")
    return text


async def run(task: Task, stage: str, adapter: ModelAdapter, read_mode: str = "tool", timeout: float = 30.0) -> RunRecord:
    record = RunRecord(task.goal, 0, "model_error")
    if not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or timeout <= 0 or not task.goal or not task.allowed_paths:
        record.error = "invalid_task"
        record.status = "error"
        return record
    deadline = time.monotonic() + timeout
    try:
        source: ReadResult | None = None
        prompt = "你是只读文件建议器。回答维护者如何检查任务，不读取文件，不输出建议 JSON。" if stage == "hello" else ("你是只读文件建议器。请依据资料回答读取结果，不要修改文件。" if stage == "read" else "你是只读文件建议器。最终只输出 JSON 对象 path/original/suggested/reason，不包含行号。")
        messages: list[dict[str, Any]] = [{"role": "system", "content": prompt}, {"role": "user", "content": task.goal}]
        if stage == "hello":
            record.request_count += 1
            response = await _request(adapter, task, messages, deadline)
            if time.monotonic() > deadline:
                raise ReinError("timeout", "late response discarded")
            text = _final_text(response)
            record.status, record.result = "opinion", {"status": "opinion", "text": text, "provider": response.provider, "raw_digest": response.raw_digest}
            return record
        path = task.requested_path
        if path not in task.allowed_paths:
            raise ReinError("path_invalid", "path is not allowed")
        if read_mode == "direct":
            source = safe_read(task.workspace, path)
            messages.append({"role": "user", "content": json.dumps({"path": source.path, "text": source.text, "digest": source.digest}, ensure_ascii=False)})
            record.messages = list(messages)
            record.request_count += 1
            response = await _request(adapter, task, messages, deadline)
            if stage == "read":
                text = _final_text(response)
                record.status, record.source_digest = "read", source.digest
                record.result = {"status": "read", "path": source.path, "text": text, "source_digest": source.digest, "provider": response.provider, "raw_digest": response.raw_digest}
                return record
        else:
            setattr(adapter, "request_tools", True)
            record.request_count += 1
            response = await _request(adapter, task, messages, deadline)
            event = _event(response)
            if event.get("text") == "":
                raise ReinError("empty_final", "empty final text")
            call = event.get("tool_call")
            if not isinstance(call, dict) or not isinstance(call.get("id"), str) or not isinstance(call.get("name"), str):
                record.messages = messages + [{"role": "tool", "tool_call_id": "unknown", "content": None, "ok": False, "output": None, "error": "response_invalid"}]
                raise ReinError("response_invalid", "tool call missing")
            if call["name"] != "read_file":
                record.messages = messages + [{"role": "tool", "tool_call_id": call["id"], "content": None, "ok": False, "output": None, "error": "unknown_tool"}]
                raise ReinError("unknown_tool", call["name"])
            args = call.get("arguments")
            if not isinstance(args, dict) or set(args) != {"path"} or not isinstance(args.get("path"), str) or not args["path"] or not call["id"]:
                record.messages = messages + [{"role": "tool", "tool_call_id": call["id"], "content": None, "ok": False, "output": None, "error": "invalid_tool_call"}]
                raise ReinError("invalid_tool_call", "path is required")
            if args["path"] not in task.allowed_paths:
                record.messages = messages + [{"role": "tool", "tool_call_id": call["id"], "content": None, "ok": False, "output": None, "error": "path_invalid"}]
                raise ReinError("path_invalid", "path is not allowed")
            assistant_message = {"role": "assistant", "tool_calls": [{"id": call["id"], "type": "function", "function": {"name": call["name"], "arguments": json.dumps(args, ensure_ascii=False)}}]}
            try:
                source = safe_read(task.workspace, args["path"])
            except ReinError as exc:
                failed = {"role": "tool", "tool_call_id": call["id"], "content": None, "ok": False, "output": None, "error": exc.code}
                record.messages = messages + [assistant_message, failed]
                raise
            tool_payload = json.dumps({"path": source.path, "text": source.text, "digest": source.digest}, ensure_ascii=False)
            messages.extend([assistant_message, {"role": "tool", "tool_call_id": call["id"], "content": tool_payload, "ok": True, "output": tool_payload, "error": None}])
            record.messages = list(messages)
            setattr(adapter, "request_tools", False)
            record.request_count += 1
            response = await _request(adapter, task, messages, deadline)
            if stage == "read":
                text = _final_text(response)
                record.status, record.source_digest = "read", source.digest
                record.result = {"status": "read", "path": source.path, "text": text, "source_digest": source.digest, "provider": response.provider, "raw_digest": response.raw_digest}
                return record
        assert source is not None
        event = _event(response)
        if event.get("text") == "":
            raise ReinError("empty_final", "empty final text")
        suggestion = event.get("suggestion") if isinstance(event, dict) else None
        if suggestion is None and isinstance(event, dict) and {"path", "original", "suggested", "reason"} <= set(event):
            suggestion = event
        if not isinstance(suggestion, dict):
            if event.get("choices") == []:
                raise ReinError("response_invalid", "empty choices")
            raise ReinError("response_invalid", "suggestion missing")
        result = locate_suggestion(source, suggestion)
        verify_source(task.workspace, source)
        record.status, record.result, record.source_digest = result["status"], result, source.digest
        if result["status"] not in {"unique", "no_change"}:
            record.error = result["status"]
        return record
    except ReinError as exc:
        record.error = exc.code
        record.status = "error"
        return record

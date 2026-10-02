"""Model adapters for the three chapter exercises."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any

from .core import HelloError, classify_sdk_error


class HTTPError(Exception):
    def __init__(self, status_code: int, detail: str = ""):
        self.status_code = status_code
        super().__init__(detail or f"HTTP {status_code}")


@dataclass(frozen=True)
class Reply:
    text: str


MAX_CANDIDATE_BYTES = 16384


def validate_candidate(value: Any) -> dict[str, str]:
    """Accept only {"code": str, "reason": str} with non-empty, bounded values."""
    if not isinstance(value, dict) or set(value) != {"code", "reason"}:
        raise HelloError("response_invalid", "candidate must contain only code and reason")
    code, reason = value["code"], value["reason"]
    if not isinstance(code, str) or not code.strip():
        raise HelloError("response_invalid", "candidate code must be a non-empty string")
    if not isinstance(reason, str) or not reason.strip():
        raise HelloError("response_invalid", "candidate reason must be a non-empty string")
    if len(code.encode("utf-8")) > MAX_CANDIDATE_BYTES:
        raise HelloError("response_invalid", "candidate code is too large")
    return {"code": code, "reason": reason}


def strip_code_fence(text: str) -> str:
    """Remove one surrounding Markdown fence such as ```json ... ```, if present."""
    stripped = text.strip()
    match = re.fullmatch(r"```[A-Za-z0-9_-]*[ \t]*\n(.*?)\n?```", stripped, re.DOTALL)
    return match.group(1).strip() if match else stripped


class ScriptedModel:
    """Deterministic model whose events represent complete assistant turns."""

    def __init__(self, events: list[dict[str, Any]]):
        self.events = list(events)
        self.messages: list[dict[str, Any]] = []
        self.provider = "scripted"

    def request(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.messages = list(messages)
        if not self.events:
            raise HelloError("response_invalid", "scripted model exhausted")
        event = self.events.pop(0)
        if not isinstance(event, dict):
            raise HelloError("response_invalid", "assistant event must be an object")
        return event


def _function_tool(name: str, description: str, properties: dict[str, Any], required: list[str]) -> dict[str, Any]:
    parameters: dict[str, Any] = {"type": "object", "properties": properties, "additionalProperties": False}
    if required:
        parameters["required"] = required
    return {"type": "function", "function": {"name": name, "description": description, "parameters": parameters}}


TOOLS = [
    _function_tool(
        "bash",
        "Run one fixed, read-only inspection command in the exercise workspace",
        {"command": {"type": "string", "enum": ["pwd", "ls -1"]}},
        ["command"],
    ),
    _function_tool(
        "read_file",
        "Read hello.cpp or compiler.log",
        {"path": {"type": "string", "enum": ["hello.cpp", "compiler.log"]}},
        ["path"],
    ),
    _function_tool("read_environment", "Read bounded compiler environment", {}, []),
]


class LiveModel:
    """Translate one Chat Completions response into a tool request or a candidate."""

    def __init__(self, *, client: Any, model: str, timeout: float = 30.0, tools_enabled: bool = True):
        self.client = client
        self.model = model
        self.timeout = timeout
        self.messages: list[dict[str, Any]] = []
        self.tools_enabled = tools_enabled
        self.provider = "live"
        self.tools = TOOLS

    def _send(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        request: dict[str, Any] = {"model": self.model, "messages": messages, "timeout": self.timeout, "stream": False}
        if self.tools_enabled:
            request["tools"] = self.tools
        try:
            response = self.client.create(**request)
            if hasattr(response, "model_dump"):
                response = response.model_dump()
            return response["choices"][0]["message"]
        except (KeyError, IndexError, TypeError, AttributeError) as exc:
            raise HelloError("response_invalid", "live response shape is invalid") from exc
        except Exception as exc:
            raise classify_sdk_error(exc) from exc

    def request(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.messages = list(messages)
        message = self._send(messages)
        if message.get("tool_calls"):
            return {"tool_calls": [_parse_tool_call(call) for call in message["tool_calls"]]}
        content = message.get("content") or ""
        if not content.strip():
            raise HelloError("response_invalid", "live response is empty")
        try:
            return {"candidate": json.loads(strip_code_fence(content))}
        except json.JSONDecodeError as exc:
            raise HelloError("response_invalid", "candidate is not JSON") from exc


def _parse_tool_call(call: dict[str, Any]) -> dict[str, Any]:
    """Turn the provider shape into {"id", "name", "arguments"}; arguments are JSON text."""
    function = call.get("function") or {}
    if not call.get("id") or not function.get("name"):
        raise HelloError("response_invalid", "tool call id/name is missing")
    try:
        arguments = json.loads(function.get("arguments") or "{}")
    except json.JSONDecodeError as exc:
        raise HelloError("response_invalid", "tool arguments are not JSON") from exc
    return {"id": call["id"], "name": function["name"], "arguments": arguments}


HELLO_WITHOUT_SEMICOLON = re.compile(r'(std::cout\s*<<\s*"Hello, world!\\n")(\s*(?:return\s+0\s*;\s*)?})', re.MULTILINE)


def _offline_repair(source: str, log: str, environment: Any) -> str:
    """Repair only the known missing-semicolon sample; refuse everything else."""
    if not log or not isinstance(environment, dict) or "compiler_version" not in environment:
        raise HelloError("context_incomplete", "offline repair needs source, compiler log and environment")
    if not HELLO_WITHOUT_SEMICOLON.search(source):
        raise HelloError("unsupported", "offline mode only repairs the missing-semicolon sample")
    return HELLO_WITHOUT_SEMICOLON.sub(r"\1;\2", source)


class OfflineModel(ScriptedModel):
    """A labelled stand-in that follows one fixed tool sequence for the sample."""

    SEQUENCE = [
        {"id": "inspect_workspace_offline", "name": "bash", "arguments": {"command": "ls -1"}},
        {"id": "read_source_offline", "name": "read_file", "arguments": {"path": "hello.cpp"}},
        {"id": "read_log_offline", "name": "read_file", "arguments": {"path": "compiler.log"}},
        {"id": "read_env_offline", "name": "read_environment", "arguments": {}},
    ]

    def __init__(self):
        super().__init__([])
        self.provider = "offline"

    def request(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.messages = list(messages)
        user = next((m.get("content", "") for m in reversed(messages) if m.get("role") == "user"), "")
        tool_results = [m for m in messages if m.get("role") == "tool"]
        if user.lstrip().startswith("{"):
            return self._direct_candidate(user)
        if "Inspect hello.cpp" not in user and not tool_results:
            return {"text": "你好，我是一个可以帮助你阅读、修正和验证代码的模型。"}
        if len(tool_results) < len(self.SEQUENCE):
            return {"tool_call": dict(self.SEQUENCE[len(tool_results)])}
        return self._tool_candidate(tool_results)

    def _direct_candidate(self, user: str) -> dict[str, Any]:
        try:
            payload = json.loads(user)
            code = _offline_repair(payload["source"], payload.get("compiler_log"), payload.get("environment"))
        except (ValueError, KeyError, TypeError) as exc:
            raise HelloError("context_incomplete", "offline repair context is invalid") from exc
        return {"candidate": {"code": code, "reason": "补上 Hello World 输出语句缺失的分号"}}

    def _tool_candidate(self, tool_results: list[dict[str, Any]]) -> dict[str, Any]:
        try:
            payloads = [json.loads(message["content"]) for message in tool_results]
            files = {p["path"]: p["text"] for p in payloads if isinstance(p, dict) and "text" in p}
            environment = next(p for p in payloads if isinstance(p, dict) and "compiler_version" in p)
            code = _offline_repair(files["hello.cpp"], files["compiler.log"], environment)
        except (ValueError, KeyError, TypeError, StopIteration) as exc:
            raise HelloError("context_incomplete", "offline repair context is invalid") from exc
        return {"candidate": {"code": code, "reason": "补上缺失的分号"}}

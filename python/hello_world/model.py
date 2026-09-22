"""Model adapters for the three chapter exercises."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any

from .core import HelloError


class HTTPError(Exception):
    def __init__(self, status_code: int, detail: str = ""):
        self.status_code = status_code
        super().__init__(detail or f"HTTP {status_code}")


@dataclass(frozen=True)
class Reply:
    text: str


def validate_candidate(value: Any) -> dict[str, str]:
    if not isinstance(value, dict) or set(value) != {"code", "reason"}:
        raise HelloError("response_invalid", "candidate must contain only code and reason")
    if not isinstance(value["code"], str) or not value["code"].strip() or not isinstance(value["reason"], str) or not value["reason"].strip():
        raise HelloError("response_invalid", "candidate code and reason must be non-empty strings")
    if len(value["code"].encode("utf-8")) > 16384:
        raise HelloError("response_invalid", "candidate code is too large")
    return {"code": value["code"], "reason": value["reason"]}


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


class LiveModel:
    def __init__(self, *, client: Any, model: str, timeout: float = 30.0, tools_enabled: bool = True):
        self.client = client
        self.model = model
        self.timeout = timeout
        self.messages: list[dict[str, Any]] = []
        self.tools_enabled = tools_enabled
        self.provider = "live"
        self.tools = [
            {"type": "function", "function": {"name": "read_file", "description": "Read hello.cpp or compiler.log", "parameters": {"type": "object", "properties": {"path": {"type": "string", "enum": ["hello.cpp", "compiler.log"]}}, "required": ["path"], "additionalProperties": False}}},
            {"type": "function", "function": {"name": "read_environment", "description": "Read bounded compiler environment", "parameters": {"type": "object", "properties": {}, "additionalProperties": False}}},
        ]

    def request(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.messages = list(messages)
        try:
            request = {"model": self.model, "messages": messages, "timeout": self.timeout, "stream": False}
            if self.tools_enabled:
                request["tools"] = self.tools
            response = self.client.create(**request)
            if hasattr(response, "model_dump"):
                response = response.model_dump()
            message = response["choices"][0]["message"]
        except (KeyError, IndexError, TypeError, AttributeError) as exc:
            raise HelloError("response_invalid", "live response shape is invalid") from exc
        except TimeoutError as exc:
            raise HelloError("timeout", "model request timed out") from exc
        except Exception as exc:
            if type(exc).__name__ in {"APITimeoutError", "ReadTimeout", "Timeout"}:
                raise HelloError("timeout", "model request timed out") from exc
            if hasattr(exc, "status_code"):
                status = int(exc.status_code)
                body = getattr(exc, "body", None)
                error = body.get("error", body) if isinstance(body, dict) else {}
                error_code = error.get("code") if isinstance(error, dict) else None
                error_type = error.get("type") if isinstance(error, dict) else None
                code = "auth_error" if status in {401, 403} else "quota_exhausted" if error_code == "insufficient_quota" else "rate_limit" if error_code == "rate_limit_exceeded" or error_type == "rate_limit_exceeded" else "server_error" if status >= 500 else "http_error"
                raise HelloError(code, f"HTTP {status}") from exc
            raise HelloError("model_error", type(exc).__name__) from exc
        if message.get("tool_calls"):
            calls = []
            for call in message["tool_calls"]:
                function = call.get("function", {})
                if not call.get("id") or not function.get("name"):
                    raise HelloError("response_invalid", "tool call id/name is missing")
                try:
                    arguments = json.loads(function.get("arguments", "{}"))
                except json.JSONDecodeError as exc:
                    raise HelloError("response_invalid", "tool arguments are not JSON") from exc
                calls.append({"id": call["id"], "name": function["name"], "arguments": arguments})
            return {"tool_calls": calls}
        content = message.get("content", "")
        if not content:
            raise HelloError("response_invalid", "live response is empty")
        try:
            return {"candidate": json.loads(content)}
        except json.JSONDecodeError as exc:
            raise HelloError("response_invalid", "candidate is not JSON") from exc


class OfflineModel(ScriptedModel):
    def __init__(self):
        super().__init__([])
        self.provider = "offline"

    def request(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.messages = list(messages)
        tool_messages = [m for m in messages if m.get("role") == "tool"]
        if tool_messages:
            if len(tool_messages) == 1:
                return {"tool_call": {"id": "read_log_offline", "name": "read_file", "arguments": {"path": "compiler.log"}}}
            if len(tool_messages) == 2:
                return {"tool_call": {"id": "read_env_offline", "name": "read_environment", "arguments": {}}}
            try:
                source = json.loads(tool_messages[0]["content"])["text"]
                log = json.loads(tool_messages[1]["content"])["text"]
                environment = json.loads(tool_messages[2]["content"])
                if not log or "compiler_version" not in environment:
                    raise HelloError("context_incomplete", "offline repair needs source, compiler log and environment")
                if not re.search(r'std::cout\s*<<\s*"Hello, world!\\n"(?=\s*(?:return\s+0\s*;\s*)?})', source, re.MULTILINE):
                    raise HelloError("unsupported", "offline mode only repairs the missing-semicolon sample")
                code = re.sub(r'(std::cout\s*<<\s*"Hello, world!\\n")(\s*(?:return\s+0\s*;\s*)?})', r'\1;\2', source, flags=re.MULTILINE)
            except (ValueError, KeyError, TypeError) as exc:
                raise HelloError("context_incomplete", "offline repair context is invalid") from exc
            return {"candidate": {"code": code, "reason": "补上缺失的分号"}}
        user = next((m.get("content", "") for m in reversed(messages) if m.get("role") == "user"), "")
        if "Inspect hello.cpp" in user:
            return {"tool_call": {"id": "read_source_offline", "name": "read_file", "arguments": {"path": "hello.cpp"}}}
        if user.lstrip().startswith("{"):
            try:
                payload = json.loads(user)
                source = payload["source"]
                if not payload.get("compiler_log") or "compiler_version" not in payload.get("environment", {}):
                    raise HelloError("context_incomplete", "offline repair needs source, compiler log and environment")
                if not re.search(r'std::cout\s*<<\s*"Hello, world!\\n"(?=\s*(?:return\s+0\s*;\s*)?})', source, re.MULTILINE):
                    raise HelloError("unsupported", "offline mode only repairs the missing-semicolon sample")
                code = re.sub(r'(std::cout\s*<<\s*"Hello, world!\\n")(\s*(?:return\s+0\s*;\s*)?})', r'\1;\2', source, flags=re.MULTILINE)
            except (ValueError, KeyError, TypeError) as exc:
                raise HelloError("context_incomplete", "offline repair context is invalid") from exc
            return {"candidate": {"code": code, "reason": "补上 Hello World 输出语句缺失的分号"}}
        return {"text": "你好，我是一个可以帮助你阅读、修正和验证代码的模型。"}

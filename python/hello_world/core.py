"""Bounded file, model, review, and C++ checks for chapters 01-03."""

from __future__ import annotations

import difflib
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MAX_READ_BYTES = 4096
ALLOWED_FILES = {"hello.cpp", "compiler.log"}
MAX_BASH_OUTPUT = 4096
BASH_TIMEOUT = 2
BASH_COMMANDS = {
    "pwd": ("/bin/pwd",),
    "ls -1": ("/bin/ls", "-1"),
}
TIMEOUT_ERROR_NAMES = {"APITimeoutError", "Timeout", "ReadTimeout"}


class HelloError(Exception):
    def __init__(self, code: str, detail: str = ""):
        self.code = code
        self.detail = detail
        super().__init__(f"{code}: {detail}" if detail else code)


@dataclass(frozen=True)
class ReadResult:
    path: str
    text: str
    digest: str
    size: int


@dataclass(frozen=True)
class HelloResult:
    text: str


@dataclass(frozen=True)
class Diagnosis:
    candidate_code: str
    reason: str
    source_digest: str
    messages: list[dict[str, Any]]
    provider: str = "offline"
    request_count: int = 0
    tools_used: tuple[str, ...] = ()


@dataclass(frozen=True)
class ApplyResult:
    status: str
    backup_path: str | None = None


@dataclass(frozen=True)
class CheckResult:
    command: list[str]
    compile_returncode: int
    run_returncode: int | None
    stdout: str
    stderr: str
    passed: bool
    run_timed_out: bool = False


def _allowed(path: str) -> None:
    candidate = Path(path)
    if path not in ALLOWED_FILES or candidate.is_absolute() or any(part in {"..", "."} for part in candidate.parts):
        raise HelloError("path_invalid", "only hello.cpp and compiler.log are allowed")


def safe_read(workspace: Path, path: str, max_bytes: int = MAX_READ_BYTES) -> ReadResult:
    if not isinstance(path, str):
        raise HelloError("path_invalid", "path must be a string")
    _allowed(path)
    root = Path(workspace)
    target = root / path
    if root.is_symlink() or target.is_symlink() or not target.is_file():
        raise HelloError("path_invalid", "target must be a regular file in workspace")
    try:
        with target.open("rb") as handle:
            raw = handle.read(max_bytes + 1)
        if len(raw) > max_bytes:
            raise HelloError("file_too_large", f"limit is {max_bytes} bytes")
        text = raw.decode("utf-8")
    except HelloError:
        raise
    except UnicodeDecodeError as exc:
        raise HelloError("invalid_utf8", str(exc)) from exc
    except OSError as exc:
        raise HelloError("path_invalid", str(exc)) from exc
    return ReadResult(path, text, hashlib.sha256(raw).hexdigest(), len(raw))


def environment_snapshot() -> dict[str, str]:
    compiler = shutil.which("c++") or "unavailable"
    if compiler == "unavailable":
        version = "unavailable"
    else:
        try:
            result = subprocess.run([compiler, "--version"], capture_output=True, text=True, timeout=5)
            version = (result.stdout or result.stderr).splitlines()[0] if result.returncode == 0 else "unavailable"
        except (OSError, subprocess.TimeoutExpired):
            version = "unavailable"
    return {
        "os": platform.platform(),
        "python": platform.python_version(),
        "compiler": compiler,
        "compiler_version": version,
    }


def run_bash(
    workspace: Path,
    command: str,
    *,
    max_bytes: int = MAX_BASH_OUTPUT,
    timeout: float = BASH_TIMEOUT,
) -> dict[str, Any]:
    """Run one fixed, read-only inspection command in the exercise workspace.

    The command is selected from ``BASH_COMMANDS`` and is never interpreted by
    a shell.  This deliberately excludes pipes, redirects, substitutions, and
    every command that could modify the workspace.
    """
    if not isinstance(command, str) or command not in BASH_COMMANDS:
        raise HelloError("bash_command_invalid", "allowed commands are pwd and ls -1")
    root = Path(workspace)
    if root.is_symlink() or not root.is_dir():
        raise HelloError("path_invalid", "workspace must be a regular directory")
    try:
        completed = subprocess.run(
            list(BASH_COMMANDS[command]),
            cwd=root,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise HelloError("bash_timeout", f"command timed out after {timeout:g}s") from exc
    except OSError as exc:
        raise HelloError("bash_failed", str(exc)) from exc
    raw = completed.stdout
    if len(raw) > max_bytes:
        raise HelloError("bash_output_too_large", f"limit is {max_bytes} bytes")
    try:
        text = raw.decode("utf-8")
        stderr = completed.stderr.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise HelloError("invalid_utf8", str(exc)) from exc
    if completed.returncode != 0:
        raise HelloError("bash_failed", stderr.strip() or f"exit code {completed.returncode}")
    return {"command": command, "stdout": text, "stderr": stderr, "returncode": completed.returncode}


def _config(environ: dict[str, str]) -> dict[str, str]:
    names = ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL")
    missing = [name for name in names if not environ.get(name)]
    if missing:
        raise HelloError("config_missing", "missing " + ", ".join(missing))
    if not environ["REIN_BASE_URL"].startswith(("http://", "https://")):
        raise HelloError("config_invalid", "REIN_BASE_URL must use HTTP(S)")
    return {name: environ[name] for name in names}


def make_sdk_client(config: dict[str, str]) -> Any:
    """Create the Chat Completions client: 30 s timeout, no automatic retry."""
    from openai import OpenAI

    sdk = OpenAI(api_key=config["REIN_API_KEY"], base_url=config["REIN_BASE_URL"], timeout=30, max_retries=0)
    return sdk.chat.completions


def _extract_response(response: Any) -> str:
    if hasattr(response, "model_dump"):
        response = response.model_dump()
    elif hasattr(response, "choices"):
        try:
            response = {"choices": [{"message": {"content": response.choices[0].message.content}}]}
        except (AttributeError, IndexError, TypeError) as exc:
            raise HelloError("response_invalid", "response has no choices/message/content") from exc
    try:
        content = response["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError) as exc:
        raise HelloError("response_invalid", "response has no choices/message/content") from exc
    if not isinstance(content, str) or not content.strip():
        raise HelloError("response_invalid", "response content is empty")
    return content


def classify_sdk_error(exc: Exception) -> HelloError:
    """Map SDK and transport failures to short categories without response bodies or keys."""
    if isinstance(exc, TimeoutError) or type(exc).__name__ in TIMEOUT_ERROR_NAMES:
        return HelloError("timeout", "model request timed out")
    status = getattr(exc, "status_code", None)
    if status is None:
        return HelloError("model_error", type(exc).__name__)
    status = int(status)
    body = getattr(exc, "body", None)
    error = body.get("error", body) if isinstance(body, dict) else {}
    if not isinstance(error, dict):
        error = {}
    if status in {401, 403}:
        code = "auth_error"
    elif error.get("code") == "insufficient_quota":
        code = "quota_exhausted"
    elif "rate_limit_exceeded" in {error.get("code"), error.get("type")}:
        code = "rate_limit"
    elif status >= 500:
        code = "server_error"
    else:
        code = "http_error"
    return HelloError(code, f"HTTP {status}")


# region live_request
def hello(
    prompt: str,
    *,
    mode: str = "offline",
    environ: dict[str, str] | None = None,
    client: Any = None,
    client_factory: Any = None,
) -> HelloResult:
    if mode == "offline":
        return HelloResult("你好，我是一个可以帮助你阅读、修正和验证代码的模型。")
    if mode != "live":
        raise HelloError("config_invalid", "mode must be offline or live")
    config = _config(dict(os.environ if environ is None else environ))
    try:
        if client is None:
            if client_factory is not None:
                client = client_factory(config)
            else:
                client = make_sdk_client(config)
        request = {"model": config["REIN_MODEL"], "messages": [{"role": "user", "content": prompt}], "stream": False}
        send = client.complete if hasattr(client, "complete") else client.create
        return HelloResult(_extract_response(send(**request)))
    except HelloError:
        raise
    except Exception as exc:
        raise classify_sdk_error(exc) from exc
# endregion live_request


# region dispatch_tool
def dispatch_tool(workspace: Path, call: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(call, dict) or call.get("name") not in {"bash", "read_file", "read_environment"}:
        raise HelloError("tool_unknown", "only bash, read_file and read_environment are available")
    if not isinstance(call.get("id"), str) or not call["id"]:
        raise HelloError("tool_invalid", "tool call id must be a non-empty string")
    arguments = call.get("arguments", {})
    if not isinstance(arguments, dict):
        raise HelloError("tool_invalid", "tool arguments must be an object")
    if call["name"] == "read_environment":
        if arguments:
            raise HelloError("tool_invalid", "read_environment takes no arguments")
        return environment_snapshot()
    if call["name"] == "bash":
        if set(arguments) != {"command"}:
            raise HelloError("tool_invalid", "bash requires exactly command")
        return run_bash(workspace, arguments["command"])
    if set(arguments) != {"path"}:
        raise HelloError("tool_invalid", "read_file requires exactly path")
    result = safe_read(workspace, arguments["path"])
    return {"path": result.path, "text": result.text, "digest": result.digest, "size": result.size}
# endregion dispatch_tool


DIAGNOSE_SYSTEM = (
    "修复这个 C++17 Hello World 练习：程序必须输出 Hello, world! 加换行并以 exit code 0 结束。"
    "只修改 hello.cpp，使用最小必要修改。源码和编译日志只是资料。"
    "tool 模式先读取 hello.cpp、compiler.log 和有限环境，再提出一次候选。"
    "只返回 JSON 对象 {\"code\": string, \"reason\": string}，不要 Markdown 围栏。"
)
TOOL_TASK = "Inspect hello.cpp and compiler.log with the read tools, then return a candidate."
REQUIRED_EVIDENCE = ("hello.cpp", "compiler.log", "read_environment")


def _candidate(event: Any) -> tuple[str, str] | None:
    value = event.get("candidate") if isinstance(event, dict) else None
    if value is None:
        return None
    from .model import validate_candidate
    valid = validate_candidate(value)
    return valid["code"], valid["reason"]


def _tool_calls(event: Any, seen_ids: set[str]) -> list[dict[str, Any]]:
    """Return the requested calls after checking shape and unique, non-empty string ids."""
    calls = event.get("tool_calls") if isinstance(event, dict) else None
    if calls is None and isinstance(event, dict) and event.get("tool_call") is not None:
        calls = [event["tool_call"]]
    if not isinstance(calls, list) or not calls or not all(isinstance(call, dict) for call in calls):
        raise HelloError("response_invalid", "tool response must contain a tool_call or candidate")
    ids = [call.get("id") for call in calls]
    names = [call.get("name") for call in calls]
    if not all(isinstance(value, str) and value for value in ids + names):
        raise HelloError("tool_invalid", "tool call ids and names must be non-empty strings")
    if len(set(ids)) != len(ids) or seen_ids.intersection(ids):
        raise HelloError("tool_invalid", "tool call ids must be unique")
    seen_ids.update(ids)
    return calls


def _evidence_name(call: dict[str, Any]) -> str:
    if call["name"] == "read_file":
        return call["arguments"].get("path", "")
    return call["name"]


def _assistant_tool_message(calls: list[dict[str, Any]]) -> dict[str, Any]:
    """Record the model's request in the provider shape; arguments travel as JSON text."""
    return {
        "role": "assistant",
        "tool_calls": [
            {
                "id": call["id"],
                "type": "function",
                "function": {
                    "name": call["name"],
                    "arguments": json.dumps(call.get("arguments", {}), ensure_ascii=False),
                },
            }
            for call in calls
        ],
    }


def _diagnose_direct(source: ReadResult, log: ReadResult, model: Any, messages: list[dict[str, Any]]) -> Diagnosis:
    payload = {"source": source.text, "compiler_log": log.text, "environment": environment_snapshot()}
    messages.append({"role": "user", "content": json.dumps(payload, ensure_ascii=False)})
    result = _candidate(model.request(messages))
    if result is None:
        raise HelloError("response_invalid", "direct response did not contain candidate")
    messages = list(getattr(model, "messages", messages))
    return Diagnosis(result[0], result[1], source.digest, messages, getattr(model, "provider", "offline"), 1, ())


# region diagnose
def diagnose(workspace: Path, *, read_mode: str, model: Any, max_requests: int = 6) -> Diagnosis:
    if read_mode not in {"direct", "tool"}:
        raise HelloError("read_mode_invalid", "read_mode must be direct or tool")
    source = safe_read(workspace, "hello.cpp")  # digest for the later stale-source check
    try:
        log = safe_read(workspace, "compiler.log")
    except HelloError as exc:
        if exc.code == "path_invalid":
            raise HelloError("missing_log", "compiler.log is required") from exc
        raise
    messages: list[dict[str, Any]] = [{"role": "system", "content": DIAGNOSE_SYSTEM}]
    if read_mode == "direct":
        return _diagnose_direct(source, log, model, messages)
    messages.append({"role": "user", "content": TOOL_TASK})
    seen_ids: set[str] = set()
    seen_evidence: set[str] = set()
    tools_used: list[str] = []
    for request_count in range(1, max_requests + 1):
        event = model.request(messages)
        result = _candidate(event)
        if result is not None:
            missing = [name for name in REQUIRED_EVIDENCE if name not in seen_evidence]
            if not missing:
                provider = getattr(model, "provider", "offline")
                code, reason = result
                return Diagnosis(code, reason, source.digest, messages, provider, request_count, tuple(tools_used))
            # Too early: keep the answer in history and ask for the missing evidence.
            messages.append({"role": "assistant", "content": json.dumps(event["candidate"], ensure_ascii=False)})
            messages.append({"role": "user", "content": "候选暂不接受。请先用工具读取：" + "、".join(missing)})
            continue
        calls = _tool_calls(event, seen_ids)
        messages.append(_assistant_tool_message(calls))
        for call in calls:
            payload = dispatch_tool(workspace, call)
            tools_used.append(call["name"])
            seen_evidence.add(_evidence_name(call))
            content = json.dumps(payload, ensure_ascii=False)
            messages.append({"role": "tool", "tool_call_id": call["id"], "content": content})
        model.messages = list(messages)
    raise HelloError("request_budget", f"maximum {max_requests} model requests reached")
# endregion diagnose


# region render_review
RED, GREEN, RESET = "\x1b[31m", "\x1b[32m", "\x1b[0m"


def _numbered(text: str) -> list[str]:
    return [f"{index:>4} | {line}" for index, line in enumerate(text.splitlines(), 1)]


def render_review(original: str, candidate: str, *, color: str = "auto") -> str:
    if color not in {"auto", "always", "never"}:
        raise HelloError("color_invalid", "color must be auto, always or never")
    lines = ["=== original file ===", *_numbered(original)]
    lines += ["=== candidate file ===", *_numbered(candidate), "=== diff ==="]
    diff = difflib.unified_diff(
        original.splitlines(True),
        candidate.splitlines(True),
        fromfile="hello.cpp",
        tofile="hello.cpp (candidate)",
    )
    is_terminal = getattr(sys.stdout, "isatty", lambda: False)()
    use_color = color == "always" or (color == "auto" and is_terminal)
    shown = []
    for line in diff:
        shade = {"-": RED, "+": GREEN}.get(line[:1], "") if use_color else ""
        shown.append(f"{shade}{line}{RESET}" if shade else line)  # "-" and "+" stay visible
    return "\n".join(lines) + "\n" + "".join(shown)
# endregion render_review


def accept_choice(value: str) -> bool:
    return value.strip().lower() in {"y", "yes", "接受", "a", "accept"}


# region apply_candidate
def apply_candidate(workspace: Path, candidate: str, *, expected_digest: str, accept: bool) -> ApplyResult:
    if not accept:
        return ApplyResult("rejected")  # rejecting never touches the workspace
    source = safe_read(workspace, "hello.cpp")
    if source.digest != expected_digest:
        raise HelloError("source_changed", "hello.cpp changed while waiting for approval")
    root = Path(workspace)
    backup = root / f"hello.cpp.bak-{time.time_ns()}"
    try:
        backup.write_bytes(source.text.encode("utf-8"))  # the bytes that were just checked
        (root / "hello.cpp").write_text(candidate, encoding="utf-8")
    except OSError as exc:
        raise HelloError("write_failed", str(exc)) from exc
    return ApplyResult("accepted", str(backup))
# endregion apply_candidate


# region check_cpp
COMPILE_TIMEOUT = 10
EXPECTED_OUTPUT = "Hello, world!\n"


def _log_section(title: str, returncode: Any, stdout: str, stderr: str) -> str:
    return f"{title} exit={returncode}\nstdout:\n{stdout}stderr:\n{stderr}"


def check_cpp(workspace: Path, *, run_timeout: float = 2.0) -> CheckResult:
    root = Path(workspace)
    safe_read(root, "hello.cpp")
    log_path = root / "compiler.log"
    if log_path.is_symlink() or (log_path.exists() and not log_path.is_file()):
        raise HelloError("path_invalid", "compiler.log must be a regular file")
    with tempfile.TemporaryDirectory(prefix="hello-world-") as temp:
        binary = str(Path(temp) / "hello")  # a fresh binary; old outputs are never run
        command = ["c++", "-std=c++17", "hello.cpp", "-o", binary]
        try:
            compiled = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=COMPILE_TIMEOUT)
        except subprocess.TimeoutExpired as exc:
            raise HelloError("compile_timeout", "C++ compilation timed out") from exc
        except FileNotFoundError as exc:
            raise HelloError("compiler_missing", "c++ was not found") from exc
        compile_log = _log_section("compile", compiled.returncode, compiled.stdout or "", compiled.stderr or "")
        log_path.write_text(compile_log, encoding="utf-8")
        if compiled.returncode != 0:
            return CheckResult(command, compiled.returncode, None, "", compile_log, False)
        try:
            run = subprocess.run([binary], cwd=root, capture_output=True, text=True, timeout=run_timeout)
            stdout, stderr, returncode, timed_out = run.stdout, run.stderr, run.returncode, False
        except subprocess.TimeoutExpired as exc:
            stdout = _text(exc.stdout)
            stderr = _text(exc.stderr) + "execution timed out"
            returncode, timed_out = None, True
        # The run result becomes evidence for the next diagnosis as well.
        run_log = _log_section("run", "timeout" if timed_out else returncode, stdout, stderr)
        log_path.write_text(compile_log + run_log, encoding="utf-8")
        passed = not timed_out and returncode == 0 and stdout == EXPECTED_OUTPUT
    return CheckResult(command, compiled.returncode, returncode, stdout, stderr, passed, timed_out)


def _text(value: Any) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", "replace")
    return value or ""
# endregion check_cpp


def generate_code(
    prompt: str,
    *,
    mode: str = "offline",
    environ: dict[str, str] | None = None,
    client: Any = None,
) -> str:
    if mode == "offline":
        return '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n";\n}\n'
    result = hello(prompt + " Return only complete C++ source code.", mode="live", environ=environ, client=client)
    return result.text

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
    "cat compiler.log": ("/bin/cat", "compiler.log"),
}


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
    return {"os": platform.platform(), "python": platform.python_version(), "compiler": compiler, "compiler_version": version}


def run_bash(workspace: Path, command: str, *, max_bytes: int = MAX_BASH_OUTPUT, timeout: float = BASH_TIMEOUT) -> dict[str, Any]:
    """Run one fixed, read-only inspection command in the exercise workspace.

    The command is selected from ``BASH_COMMANDS`` and is never interpreted by
    a shell.  This deliberately excludes pipes, redirects, substitutions, and
    every command that could modify the workspace.
    """
    if not isinstance(command, str) or command not in BASH_COMMANDS:
        raise HelloError("bash_command_invalid", "allowed commands are pwd, ls -1 and cat compiler.log")
    root = Path(workspace)
    if root.is_symlink() or not root.is_dir():
        raise HelloError("path_invalid", "workspace must be a regular directory")
    if command == "cat compiler.log":
        log = root / "compiler.log"
        if log.is_symlink() or not log.is_file():
            raise HelloError("path_invalid", "compiler.log must be a regular file in workspace")
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


# region live_request
def hello(prompt: str, *, mode: str = "offline", environ: dict[str, str] | None = None, client: Any = None, client_factory: Any = None) -> HelloResult:
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
                from openai import OpenAI
                sdk = OpenAI(api_key=config["REIN_API_KEY"], base_url=config["REIN_BASE_URL"], timeout=30, max_retries=0)
                client = sdk.chat.completions
        if hasattr(client, "complete"):
            response = client.complete(model=config["REIN_MODEL"], messages=[{"role": "user", "content": prompt}], stream=False)
        else:
            response = client.create(model=config["REIN_MODEL"], messages=[{"role": "user", "content": prompt}], stream=False)
        return HelloResult(_extract_response(response))
    except HelloError:
        raise
    except TimeoutError as exc:
        raise HelloError("timeout", "model request timed out") from exc
    except Exception as exc:
        if hasattr(exc, "status_code"):
            status = int(exc.status_code)
            body = getattr(exc, "body", None)
            error = body.get("error", body) if isinstance(body, dict) else {}
            error_code = error.get("code") if isinstance(error, dict) else None
            error_type = error.get("type") if isinstance(error, dict) else None
            category = "auth_error" if status in {401, 403} else "quota_exhausted" if error_code == "insufficient_quota" else "rate_limit" if error_code == "rate_limit_exceeded" or error_type == "rate_limit_exceeded" else "server_error" if status >= 500 else "http_error"
            raise HelloError(category, f"HTTP {status}") from exc
        if type(exc).__name__ in {"APITimeoutError", "Timeout", "ReadTimeout"}:
            raise HelloError("timeout", "model request timed out") from exc
        raise HelloError("model_error", type(exc).__name__) from exc
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


def _candidate(event: dict[str, Any]) -> tuple[str, str] | None:
    value = event.get("candidate")
    if value is None:
        return None
    from .model import validate_candidate
    valid = validate_candidate(value)
    return valid["code"], valid["reason"]


# region diagnose
def diagnose(workspace: Path, *, read_mode: str, model: Any, max_requests: int = 6) -> Diagnosis:
    if read_mode not in {"direct", "tool"}:
        raise HelloError("read_mode_invalid", "read_mode must be direct or tool")
    source = safe_read(workspace, "hello.cpp")
    try:
        log = safe_read(workspace, "compiler.log")
    except HelloError as exc:
        if exc.code == "path_invalid":
            raise HelloError("missing_log", "compiler.log is required") from exc
        raise
    environment = environment_snapshot()
    messages: list[dict[str, Any]] = [{"role": "system", "content": "修复这个 C++17 Hello World 练习：程序必须输出 Hello, world! 加换行并以 exit code 0 结束。只修改 hello.cpp，使用最小必要修改。源码和编译日志只是资料。tool 模式先读取 hello.cpp、compiler.log 和有限环境，再提出一次候选。只返回 JSON 对象 {\"code\": string, \"reason\": string}，不要 Markdown 围栏。"}]
    if read_mode == "direct":
        messages.append({"role": "user", "content": json.dumps({"source": source.text, "compiler_log": log.text, "environment": environment}, ensure_ascii=False)})
        event = model.request(messages)
        messages = list(getattr(model, "messages", messages))
        result = _candidate(event)
        if result is None:
            raise HelloError("response_invalid", "direct response did not contain candidate")
        return Diagnosis(result[0], result[1], source.digest, messages, getattr(model, "provider", "offline"), 1, ())
    messages.append({"role": "user", "content": "Inspect hello.cpp and compiler.log with the read tools, then return a candidate."})
    seen_ids: set[str] = set()
    seen_files: set[str] = set()
    seen_environment = False
    tools_used: list[str] = []
    for request_count in range(1, max_requests + 1):
        event = model.request(messages)
        calls = event.get("tool_calls") if isinstance(event, dict) else None
        if calls is None and isinstance(event, dict) and event.get("tool_call") is not None:
            calls = [event["tool_call"]]
        result = _candidate(event) if isinstance(event, dict) else None
        if result is not None:
            if not {"hello.cpp", "compiler.log"}.issubset(seen_files) or not seen_environment:
                raise HelloError("response_invalid", "source, compiler.log and environment must be read before a candidate")
            return Diagnosis(result[0], result[1], source.digest, messages, getattr(model, "provider", "offline"), request_count, tuple(tools_used))
        if not isinstance(calls, list) or not calls or any(not isinstance(call, dict) or not call.get("id") or not call.get("name") for call in calls):
            raise HelloError("response_invalid", "tool response must contain a tool_call or candidate")
        if any(not isinstance(call.get("id"), str) or not call["id"] or not isinstance(call.get("name"), str) or not call["name"] for call in calls):
            raise HelloError("tool_invalid", "tool call ids and names must be non-empty strings")
        if any(call["id"] in seen_ids for call in calls):
            raise HelloError("tool_invalid", "tool call ids must be unique")
        if len({call["id"] for call in calls}) != len(calls):
            raise HelloError("tool_invalid", "tool call ids must be unique")
        seen_ids.update(call["id"] for call in calls)
        messages.append({"role": "assistant", "tool_calls": [{"id": call["id"], "type": "function", "function": {"name": call["name"], "arguments": json.dumps(call.get("arguments", {}), ensure_ascii=False)}} for call in calls]})
        for call in calls:
            result_payload = dispatch_tool(workspace, call)
            tools_used.append(call["name"])
            if call["name"] == "read_file":
                seen_files.add(call["arguments"].get("path", ""))
            elif call["name"] == "read_environment":
                seen_environment = True
            messages.append({"role": "tool", "tool_call_id": call["id"], "content": json.dumps(result_payload, ensure_ascii=False)})
        model.messages = list(messages)
    raise HelloError("request_budget", f"maximum {max_requests} model requests reached")
# endregion diagnose


# region render_review
def render_review(original: str, candidate: str, *, color: str = "auto") -> str:
    if color not in {"auto", "always", "never"}:
        raise HelloError("color_invalid", "color must be auto, always or never")
    lines = ["=== original file ==="]
    lines += [f"{index:>4} | {line}" for index, line in enumerate(original.splitlines(), 1)]
    lines.append("=== candidate file ===")
    lines += [f"{index:>4} | {line}" for index, line in enumerate(candidate.splitlines(), 1)]
    lines.append("=== diff ===")
    diff = list(difflib.unified_diff(original.splitlines(True), candidate.splitlines(True), fromfile="hello.cpp", tofile="hello.cpp (candidate)"))
    rendered = "\n".join(lines)
    use_color = color == "always" or (color == "auto" and getattr(getattr(sys, "stdout", None), "isatty", lambda: False)())
    if use_color:
        colored = []
        for line in diff:
            prefix = line[:1]
            shade = "\x1b[31m" if prefix == "-" else "\x1b[32m" if prefix == "+" else ""
            colored.append(f"{shade}{line}\x1b[0m" if shade else line)
        diff_text = "".join(colored)
    else:
        diff_text = "".join(diff)
    rendered += "\n" + diff_text
    return rendered
# endregion render_review


def accept_choice(value: str) -> bool:
    return value.strip().lower() in {"y", "yes", "接受", "a", "accept"}


# region apply_candidate
def apply_candidate(workspace: Path, candidate: str, *, expected_digest: str, accept: bool) -> ApplyResult:
    source = safe_read(workspace, "hello.cpp")
    if source.digest != expected_digest:
        raise HelloError("source_changed", "hello.cpp changed while waiting for approval")
    if not accept:
        return ApplyResult("rejected")
    backup = Path(workspace) / f"hello.cpp.bak-{time.time_ns()}"
    try:
        backup.write_bytes((Path(workspace) / "hello.cpp").read_bytes())
        (Path(workspace) / "hello.cpp").write_text(candidate, encoding="utf-8")
    except OSError as exc:
        raise HelloError("write_failed", str(exc)) from exc
    return ApplyResult("accepted", str(backup))
# endregion apply_candidate


# region check_cpp
def check_cpp(workspace: Path, *, run_timeout: float = 2.0) -> CheckResult:
    root = Path(workspace)
    safe_read(root, "hello.cpp")
    log_path = root / "compiler.log"
    if log_path.is_symlink() or (log_path.exists() and not log_path.is_file()):
        raise HelloError("path_invalid", "compiler.log must be a regular file")
    with tempfile.TemporaryDirectory(prefix="hello-world-") as temp:
        binary = str(Path(temp) / "hello")
        command = ["c++", "-std=c++17", "hello.cpp", "-o", binary]
        try:
            compiled = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=10)
        except subprocess.TimeoutExpired as exc:
            raise HelloError("compile_timeout", "C++ compilation timed out") from exc
        except FileNotFoundError as exc:
            raise HelloError("compiler_missing", "c++ was not found") from exc
        compiler_log = f"compile exit={compiled.returncode}\nstdout:\n{compiled.stdout or ''}stderr:\n{compiled.stderr or ''}"
        log_path.write_text(compiler_log, encoding="utf-8")
        if compiled.returncode != 0:
            return CheckResult(command, compiled.returncode, None, "", compiler_log, False)
        timed_out = False
        try:
            run = subprocess.run([binary], cwd=root, capture_output=True, text=True, timeout=run_timeout)
            stdout, stderr, returncode = run.stdout, run.stderr, run.returncode
            (root / "compiler.log").write_text(compiler_log + f"run exit={returncode}\nstdout:\n{stdout}stderr:\n{stderr}", encoding="utf-8")
        except subprocess.TimeoutExpired as exc:
            timed_out = True
            stdout = exc.stdout or b""
            stderr = (exc.stderr or b"")
            if isinstance(stdout, bytes):
                stdout = stdout.decode("utf-8", "replace")
            if isinstance(stderr, bytes):
                stderr = stderr.decode("utf-8", "replace")
            stderr += "execution timed out"
            returncode = None
            (root / "compiler.log").write_text(compiler_log + "run exit=timeout\nstdout:\n" + stdout + "stderr:\n" + stderr, encoding="utf-8")
        passed = not timed_out and returncode == 0 and stdout == "Hello, world!\n"
    return CheckResult(command, compiled.returncode, returncode, stdout, stderr, passed, timed_out)
# endregion check_cpp


def generate_code(prompt: str, *, mode: str = "offline", environ: dict[str, str] | None = None, client: Any = None) -> str:
    if mode == "offline":
        return '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n";\n}\n'
    result = hello(prompt + " Return only complete C++ source code.", mode="live", environ=environ, client=client)
    return result.text

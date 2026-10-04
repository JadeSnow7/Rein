#!/usr/bin/env python3
"""Chapter 04 parity check: run Python and Rust on the same frozen samples and compare behavior.

Compared: status category, exit code, file digests, backups, check results and review text.
Not compared: wording of error details, and environment fields that name the language.

Usage (from the repository root, after `cargo build --manifest-path rust-hello-world/Cargo.toml`):
    python3 rust-hello-world/compare_parity.py
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from hello_world import core  # noqa: E402

PY_CLI = [sys.executable, str(ROOT / "python" / "hello_world" / "cli.py")]
RUST_BIN = Path(os.environ.get("REIN_HELLO_BIN", ROOT / "rust-hello-world" / "target" / "debug" / "rein-hello"))
FIXTURES = ROOT / "python" / "hello_world" / "fixtures"
BROKEN = (FIXTURES / "broken.cpp").read_text(encoding="utf-8")
FIXED = (FIXTURES / "hello.cpp").read_text(encoding="utf-8")
LOG = "hello.cpp:4:35: error: expected ';' before '}' token\n"


def digest(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def workspace(source: str = BROKEN, log: str | None = LOG) -> Path:
    root = Path(tempfile.mkdtemp(prefix="parity-"))
    (root / "hello.cpp").write_text(source, encoding="utf-8")
    if log is not None:
        (root / "compiler.log").write_text(log, encoding="utf-8")
    return root


def run(command: list[str], stdin: str | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(command, input=stdin or "", capture_output=True, text=True, timeout=60)  # "" means EOF


def error_code(stderr: str) -> str | None:
    for line in reversed(stderr.strip().splitlines()):
        try:
            return json.loads(line).get("status")
        except (ValueError, AttributeError):
            continue
    return None


# ---- tools ------------------------------------------------------------------------------

def python_tool(root: Path, call: dict) -> dict:
    try:
        return {"ok": core.dispatch_tool(root, call)}
    except core.HelloError as exc:
        return {"error": exc.code}


def rust_tool(root: Path, call: dict) -> dict:
    done = run([str(RUST_BIN), "tool", "--workspace", str(root), "--call", json.dumps(call)])
    return {"ok": json.loads(done.stdout)} if done.returncode == 0 else {"error": error_code(done.stderr)}


def comparable_tool(result: dict) -> dict:
    ok = result.get("ok")
    if isinstance(ok, dict) and "compiler_version" in ok:  # language-specific keys are not compared
        return {"ok": {"compiler": ok["compiler"], "compiler_version": ok["compiler_version"]}}
    return result


def tool_case(call: dict, prepare: Callable[[Path], None] = lambda root: None) -> Callable[[], tuple[Any, Any]]:
    def check() -> tuple[Any, Any]:
        results = []
        for runner in (python_tool, rust_tool):
            root = workspace()
            prepare(root)
            before = digest(root / "hello.cpp")
            result = comparable_tool(runner(root, {"id": "t1", **call}))
            results.append({**result, "source_unchanged": digest(root / "hello.cpp") == before})
        return results[0], results[1]
    return check


def symlink_source(root: Path) -> None:
    outside = Path(tempfile.mkdtemp()) / "secret.txt"
    outside.write_text("secret\n", encoding="utf-8")
    (root / "hello.cpp").unlink()
    (root / "hello.cpp").symlink_to(outside)


# ---- edit and check ---------------------------------------------------------------------

def proposal_for(root: Path) -> Path:
    done = run(PY_CLI + ["diagnose", "--workspace", str(root), "--read-mode", "tool"])
    if done.returncode != 0:
        raise SystemExit(f"cannot build proposal: {done.stderr}")
    path = Path(tempfile.mkdtemp()) / "proposal.json"
    path.write_text(done.stdout, encoding="utf-8")
    return path


def observe_edit(root: Path, done: subprocess.CompletedProcess, original: str) -> dict:
    lines = done.stdout.splitlines()
    status = next((line for line in lines if line in {"accepted", "rejected"}), None)
    backups = sorted(root.glob("hello.cpp.bak-*"))
    review = done.stdout.split("接受修改？[y/N] ")[0]
    check = None
    start = done.stdout.rfind("\n{\n")
    if start >= 0:  # the pretty-printed check report comes last
        report = json.loads(done.stdout[start + 1:])
        check = {key: report[key] for key in ("passed", "compile_returncode", "run_returncode", "stdout")}
    return {
        "exit": done.returncode,
        "status": status or error_code(done.stderr),
        "review": review,
        "source": (root / "hello.cpp").read_text(encoding="utf-8"),
        "backups": [b.read_text(encoding="utf-8") for b in backups],
        "check": check,
        "original_kept": (root / "hello.cpp").read_text(encoding="utf-8") == original or bool(backups),
    }


def edit_case(answer: str | None, color: str = "never", change_first: bool = False) -> Callable[[], tuple[Any, Any]]:
    def check() -> tuple[Any, Any]:
        results = []
        for side in ("python", "rust"):
            root = workspace()
            proposal = proposal_for(root)
            if change_first:  # the user edits hello.cpp while the candidate waits
                (root / "hello.cpp").write_text("// edited while waiting\n" + BROKEN, encoding="utf-8")
            original = (root / "hello.cpp").read_text(encoding="utf-8")
            if side == "python" and change_first:
                done = run_python_edit_with_proposal(root, proposal, answer, color)
            elif side == "python":
                # Python diagnoses again inside `edit`; offline it returns the same candidate and digest.
                done = run(PY_CLI + ["edit", "--workspace", str(root), "--color", color], answer)
            else:
                done = run([str(RUST_BIN), "edit", "--workspace", str(root), "--proposal", str(proposal), "--color", color], answer)
            results.append(observe_edit(root, done, original))
        return results[0], results[1]
    return check


def run_python_edit_with_proposal(root: Path, proposal: Path, answer: str | None, color: str) -> subprocess.CompletedProcess:
    """Python `edit` re-diagnoses the current file, so a stale proposal is replayed through core directly."""
    script = f"""
import json, sys
sys.path.insert(0, {str(ROOT / 'python')!r})
from pathlib import Path
from hello_world import core
from hello_world import cli
proposal = json.loads(Path({str(proposal)!r}).read_text(encoding='utf-8'))
diagnosis = core.Diagnosis(proposal['code'], proposal['reason'], proposal['source_digest'], [])
try:
    raise SystemExit(cli.run_edit(Path({str(root)!r}), diagnosis, {color!r}))
except core.HelloError as exc:
    print(json.dumps({{'status': exc.code, 'detail': exc.detail}}), file=sys.stderr)
    raise SystemExit(1)
"""
    return run([sys.executable, "-c", script], answer)


def check_case(source: str, prepare: Callable[[Path], None] = lambda root: None) -> Callable[[], tuple[Any, Any]]:
    def check() -> tuple[Any, Any]:
        results = []
        for command in (PY_CLI + ["check"], [str(RUST_BIN), "check"]):
            root = workspace(source)
            prepare(root)
            done = run(command + ["--workspace", str(root)])
            observed: dict[str, Any] = {"exit": done.returncode}
            if done.returncode in (0, 1) and done.stdout.strip():
                report = json.loads(done.stdout)
                observed.update(
                    passed=report["passed"],
                    compiled=report["compile_returncode"] == 0,
                    run_returncode=report["run_returncode"],
                    stdout=report["stdout"],
                    log_has_run=("run exit=" in (root / "compiler.log").read_text(encoding="utf-8")),
                )
            else:
                observed["error"] = error_code(done.stderr)
            results.append(observed)
        return results[0], results[1]
    return check


def symlink_log(root: Path) -> None:
    (root / "compiler.log").unlink()
    (root / "compiler.log").symlink_to(Path(tempfile.mkdtemp()) / "elsewhere.log")


CASES: list[tuple[str, str, Callable[[], tuple[Any, Any]]]] = [
    ("B03", "read_file hello.cpp", tool_case({"name": "read_file", "arguments": {"path": "hello.cpp"}})),
    ("B03", "read_file ../hello.cpp", tool_case({"name": "read_file", "arguments": {"path": "../hello.cpp"}})),
    ("B03", "read_file /etc/passwd", tool_case({"name": "read_file", "arguments": {"path": "/etc/passwd"}})),
    ("B03", "read_file through symlink", tool_case({"name": "read_file", "arguments": {"path": "hello.cpp"}}, symlink_source)),
    ("B03", "oversized log", tool_case({"name": "read_file", "arguments": {"path": "compiler.log"}},
                                        lambda r: (r / "compiler.log").write_text("x" * 5000))),
    ("B03", "non-UTF-8 log", tool_case({"name": "read_file", "arguments": {"path": "compiler.log"}},
                                        lambda r: (r / "compiler.log").write_bytes(b"\xff\xfe"))),
    ("B03", "unknown tool", tool_case({"name": "write_file", "arguments": {"path": "hello.cpp"}})),
    ("B03", "extra argument", tool_case({"name": "read_file", "arguments": {"path": "hello.cpp", "x": 1}})),
    ("B03", "missing id", lambda: tool_case({"name": "read_file", "arguments": {"path": "hello.cpp"}, "id": ""})()),
    ("B03", "bash ls -1", tool_case({"name": "bash", "arguments": {"command": "ls -1"}})),
    ("B03", "bash cat is refused", tool_case({"name": "bash", "arguments": {"command": "cat compiler.log"}})),
    ("B03", "bash shell text", tool_case({"name": "bash", "arguments": {"command": "ls -1; rm hello.cpp"}})),
    ("B03", "read_environment", tool_case({"name": "read_environment", "arguments": {}})),
    ("B03", "read_environment with args", tool_case({"name": "read_environment", "arguments": {"env": True}})),
    ("B04", "reject with n", edit_case("n\n")),
    ("B04", "reject on EOF", edit_case(None)),
    ("B04", "accept with y", edit_case("y\n")),
    ("B04", "accept, colored review", edit_case("y\n", color="always")),
    ("B04", "stale source, accept", edit_case("y\n", change_first=True)),
    ("B04", "stale source, reject", edit_case("n\n", change_first=True)),
    ("B05", "correct program", check_case(FIXED)),
    ("B05", "compile error", check_case(BROKEN)),
    ("B05", "wrong output", check_case(FIXED.replace("Hello, world!", "Hi"))),
    ("B05", "run timeout", check_case("int main() { for (;;) {} }\n")),
    ("B05", "compiler.log symlink", check_case(FIXED, symlink_log)),
]


def main() -> int:
    if not RUST_BIN.exists():
        print(f"Rust binary not found: {RUST_BIN}. Run cargo build first.", file=sys.stderr)
        return 2
    failures = 0
    for sample, name, case in CASES:
        python_result, rust_result = case()
        same = python_result == rust_result
        failures += not same
        print(f"{'PASS' if same else 'DIFF'}  {sample}  {name}")
        if not same:
            print(f"      python: {json.dumps(python_result, ensure_ascii=False)[:600]}")
            print(f"      rust:   {json.dumps(rust_result, ensure_ascii=False)[:600]}")
    print(f"\n{len(CASES) - failures}/{len(CASES)} samples behave the same.")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

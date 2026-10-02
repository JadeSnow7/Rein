#!/usr/bin/env python3
"""Offline-first command line entry point for the three chapters."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

if __package__ in {None, ""}:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from hello_world import core, model
else:
    from . import core, model

HelloError = core.HelloError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    hello = sub.add_parser("hello")
    hello.add_argument("--prompt", default="你好，请做自我介绍。")
    hello.add_argument("--mode", choices=("offline", "live"), default="offline")
    generate = sub.add_parser("generate")
    generate.add_argument("--prompt", default="请生成 C++ Hello World。")
    generate.add_argument("--mode", choices=("offline", "live"), default="offline")
    diagnose = sub.add_parser("diagnose")
    diagnose.add_argument("--workspace", required=True)
    diagnose.add_argument("--read-mode", choices=("direct", "tool"), default="direct")
    diagnose.add_argument("--mode", choices=("offline", "live"), default="offline")
    edit = sub.add_parser("edit")
    edit.add_argument("--workspace", required=True)
    edit.add_argument("--read-mode", choices=("direct", "tool"), default="tool")
    edit.add_argument("--mode", choices=("offline", "live"), default="offline")
    edit.add_argument("--color", choices=("auto", "always", "never"), default="auto")
    check = sub.add_parser("check")
    check.add_argument("--workspace", required=True)
    return parser


def print_json(value: dict) -> None:
    print(json.dumps(value, ensure_ascii=False, indent=2))


def check_report(result: core.CheckResult) -> dict:
    return {
        "passed": result.passed,
        "compile_returncode": result.compile_returncode,
        "run_returncode": result.run_returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def make_model(args: argparse.Namespace):
    """Offline is the labelled default; live mode needs the three REIN_* settings."""
    if args.mode == "offline":
        print("# offline example", file=sys.stderr)
        return model.OfflineModel()
    config = core._config(dict(os.environ))
    try:
        client = core.make_sdk_client(config)
    except ImportError as exc:
        raise HelloError("sdk_missing", "run with python/hello_world/.venv/bin/python") from exc
    return model.LiveModel(client=client, model=config["REIN_MODEL"], tools_enabled=args.read_mode == "tool")


def run_edit(root: Path, diagnosis: core.Diagnosis, color: str) -> int:
    original = (root / "hello.cpp").read_text(encoding="utf-8")
    print(core.render_review(original, diagnosis.candidate_code, color=color))
    try:
        choice = input("接受修改？[y/N] ")
    except EOFError:
        choice = ""
    if not sys.stdin.isatty():
        print()  # keep the status on its own line when the answer was piped in
    accept = core.accept_choice(choice)
    digest = diagnosis.source_digest
    applied = core.apply_candidate(root, diagnosis.candidate_code, expected_digest=digest, accept=accept)
    print(applied.status)
    if applied.status != "accepted":
        return 0
    print(f"backup={applied.backup_path}")
    checked = core.check_cpp(root)
    print_json(check_report(checked))
    return 0 if checked.passed else 1


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command in {"hello", "generate"}:
            if args.mode == "offline":
                print("# offline example", file=sys.stderr)
            if args.command == "generate":
                print(core.generate_code(args.prompt, mode=args.mode, environ=os.environ))
            else:
                print(core.hello(args.prompt, mode=args.mode, environ=os.environ).text)
            return 0
        root = Path(args.workspace)
        if args.command == "check":
            result = core.check_cpp(root)
            print_json(check_report(result))
            return 0 if result.passed else 1
        diagnosis = core.diagnose(root, read_mode=args.read_mode, model=make_model(args))
        if args.command == "diagnose":
            print_json({
                "provider": diagnosis.provider,
                "request_count": diagnosis.request_count,
                "tools": list(diagnosis.tools_used),
                "message_roles": [message.get("role") for message in diagnosis.messages],
                "code": diagnosis.candidate_code,
                "reason": diagnosis.reason,
                "source_digest": diagnosis.source_digest,
            })
            return 0
        return run_edit(root, diagnosis, args.color)
    except HelloError as exc:
        print(json.dumps({"status": exc.code, "detail": exc.detail}, ensure_ascii=False), file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("\n已退出。", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())

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

if __package__ in {None, ""}:
    from hello_world import model
    from hello_world.core import HelloError, accept_choice, apply_candidate, check_cpp, diagnose, generate_code, hello, render_review
else:
    from . import model
    from .core import HelloError, accept_choice, apply_candidate, check_cpp, diagnose, generate_code, hello, render_review


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    hello_parser = sub.add_parser("hello")
    hello_parser.add_argument("--prompt", default="你好，请做自我介绍。")
    hello_parser.add_argument("--mode", choices=("offline", "live"), default="offline")
    gen = sub.add_parser("generate")
    gen.add_argument("--prompt", default="请生成 C++ Hello World。")
    gen.add_argument("--mode", choices=("offline", "live"), default="offline")
    diag = sub.add_parser("diagnose")
    diag.add_argument("--workspace", required=True)
    diag.add_argument("--read-mode", choices=("direct", "tool"), default="direct")
    diag.add_argument("--mode", choices=("offline", "live"), default="offline")
    edit = sub.add_parser("edit")
    edit.add_argument("--workspace", required=True)
    edit.add_argument("--read-mode", choices=("direct", "tool"), default="tool")
    edit.add_argument("--mode", choices=("offline", "live"), default="offline")
    edit.add_argument("--color", choices=("auto", "always", "never"), default="auto")
    check = sub.add_parser("check")
    check.add_argument("--workspace", required=True)
    args = parser.parse_args(argv)
    try:
        if args.command in {"hello", "generate"}:
            if args.command == "generate":
                if args.mode == "offline":
                    print("# offline example", file=sys.stderr)
                print(generate_code(args.prompt, mode=args.mode, environ=os.environ))
                return 0
            if args.mode == "offline":
                print("# offline example", file=sys.stderr)
            result = hello(args.prompt, mode=args.mode, environ=os.environ)
            print(result.text)
            return 0
        root = Path(args.workspace)
        if args.command == "check":
            result = check_cpp(root)
            print(json.dumps({"passed": result.passed, "compile_returncode": result.compile_returncode, "run_returncode": result.run_returncode, "stdout": result.stdout, "stderr": result.stderr}, ensure_ascii=False, indent=2))
            return 0 if result.passed else 1
        if getattr(args, "mode", "offline") == "live":
            from openai import OpenAI
            config = __import__("hello_world.core", fromlist=["_config"])._config(dict(os.environ))
            sdk = OpenAI(api_key=config["REIN_API_KEY"], base_url=config["REIN_BASE_URL"], timeout=30, max_retries=0)
            adapter = model.LiveModel(client=sdk.chat.completions, model=config["REIN_MODEL"], tools_enabled=args.read_mode == "tool")
        else:
            print("# offline example", file=sys.stderr)
            adapter = model.OfflineModel()
        result = diagnose(root, read_mode=args.read_mode, model=adapter)
        if args.command == "diagnose":
            print(json.dumps({"provider": result.provider, "request_count": result.request_count, "tools": list(result.tools_used), "message_roles": [m.get("role") for m in result.messages], "code": result.candidate_code, "reason": result.reason, "source_digest": result.source_digest}, ensure_ascii=False, indent=2))
            return 0
        source = root / "hello.cpp"
        original = source.read_text(encoding="utf-8")
        print(render_review(original, result.candidate_code, color=args.color))
        try:
            choice = input("接受修改？[y/N] ")
        except EOFError:
            choice = ""
        applied = apply_candidate(root, result.candidate_code, expected_digest=result.source_digest, accept=accept_choice(choice))
        print(applied.status)
        if applied.backup_path:
            print(f"backup={applied.backup_path}")
        if applied.status == "accepted":
            checked = check_cpp(root)
            print(json.dumps({"passed": checked.passed, "compile_returncode": checked.compile_returncode, "run_returncode": checked.run_returncode, "stdout": checked.stdout, "stderr": checked.stderr}, ensure_ascii=False, indent=2))
            return 0 if checked.passed else 1
        return 0
    except HelloError as exc:
        print(json.dumps({"status": exc.code, "detail": exc.detail}, ensure_ascii=False), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

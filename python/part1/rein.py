#!/usr/bin/env python3
import argparse
import asyncio
import json
import os
import sys
import math
from pathlib import Path

from rein_core import OfflineAdapter, OpenAIAdapter, ReplayAdapter, ReinError, Task, load_config, run


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("stage", choices=("hello", "read", "suggest"))
    parser.add_argument("--workspace", default=str(Path(__file__).parent / "fixtures" / "outdated"))
    parser.add_argument("--path", default="README.md")
    parser.add_argument("--read-mode", choices=("direct", "tool"), default="tool")
    parser.add_argument("--mode", choices=("offline", "live"), default="offline")
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--response")
    args = parser.parse_args()
    task = Task("维护者给出的当前开发命令是 npm run dev；请检查 README", Path(args.workspace), ("README.md",), "opinion" if args.stage == "hello" else "suggest", args.path)
    client = None
    try:
        if not math.isfinite(args.timeout) or args.timeout <= 0:
            raise ReinError("invalid_task", "timeout must be finite and positive")
        if args.mode == "live" and args.response:
            raise ReinError("invalid_task", "live mode cannot use replay response")
        if args.mode == "live":
            config = load_config(os.environ)
            try:
                from openai import AsyncOpenAI
            except ImportError as exc:
                raise ReinError("model_error", "install openai==2.26.0 for live mode") from exc
            client = AsyncOpenAI(api_key=config.api_key, base_url=config.base_url, timeout=args.timeout, max_retries=0)
            adapter = OpenAIAdapter(client, config.model, retries=0)
        else:
            adapter = OfflineAdapter()
        if args.response:
            with open(args.response, encoding="utf-8") as handle:
                loaded = json.load(handle)
            adapter = ReplayAdapter(loaded if isinstance(loaded, list) else [loaded])
        async def execute():
            try:
                return await run(task, args.stage, adapter, args.read_mode, args.timeout)
            finally:
                if client is not None:
                    await client.close()
        record = asyncio.run(execute())
        output = {"record": {"task_id": record.task_id, "request_count": record.request_count, "status": record.status, "error": record.error, "messages": record.messages}, "result": record.result or {"status": record.error or record.status}}
        print(json.dumps(output, ensure_ascii=False, indent=2))
        return 0 if record.error is None else 1
    except ReinError as exc:
        safe_error = "model_error" if args.mode == "live" and exc.code in {"config_missing", "config_invalid"} else exc.code
        print(json.dumps({"record": {"task_id": task.goal, "request_count": 0, "status": "error", "error": safe_error, "messages": getattr(locals().get("record", None), "messages", None)}, "result": {"status": safe_error, "detail": exc.code}}, ensure_ascii=False))
        return 1
    except Exception as exc:
        print(json.dumps({"record": {"task_id": task.goal, "request_count": 0, "status": "error", "error": "response_invalid"}, "result": {"status": "response_invalid", "detail": type(exc).__name__}}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

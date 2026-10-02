#!/usr/bin/env python3
"""A complete live request program whose task comes from the user."""

from __future__ import annotations

import os
import sys
from pathlib import Path

if __package__ in {None, ""}:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from hello_world.core import HelloError, hello


def main() -> int:
    task = " ".join(sys.argv[1:]).strip() or input("请输入任务：").strip()
    if not task:
        print("任务不能为空", file=sys.stderr)
        return 2
    try:
        result = hello(task, mode="live", environ=os.environ)
    except HelloError as exc:
        print(f"请求失败（{exc.code}）：{exc.detail}", file=sys.stderr)
        return 1
    print(result.text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

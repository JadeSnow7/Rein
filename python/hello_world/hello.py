#!/usr/bin/env python3
"""The shortest complete live SDK example used in chapter 01."""

from __future__ import annotations

import os
import sys

def main() -> int:
    """The complete request is visible here for the first lesson."""
    base_url = os.environ.get("REIN_BASE_URL")
    api_key = os.environ.get("REIN_API_KEY")
    model = os.environ.get("REIN_MODEL")
    if not base_url or not api_key or not model:
        print("请求失败（config_missing）：请设置 REIN_BASE_URL、REIN_API_KEY、REIN_MODEL", file=sys.stderr)
        return 1
    try:
        from openai import OpenAI
        client = OpenAI(api_key=api_key, base_url=base_url, timeout=30, max_retries=0)
        response = client.chat.completions.create(
            model=model,
            messages=[{"role": "user", "content": "你好，请简单介绍一下你能帮助我完成哪些编程任务。"}],
            stream=False,
        )
        print(response.choices[0].message.content)
        return 0
    except Exception as exc:
        print(f"请求失败（{type(exc).__name__}）：请检查配置、网络和模型名称", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""A complete, single-request SDK example used in chapter 01."""

from __future__ import annotations

import os
import sys


CONFIG_NAMES = ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL")


def read_config() -> dict[str, str]:
    """Read the three lesson settings, removing accidental surrounding spaces."""
    return {name: os.environ.get(name, "").strip() for name in CONFIG_NAMES}


def missing_config(config: dict[str, str]) -> list[str]:
    return [name for name in CONFIG_NAMES if not config.get(name)]


def classify_error(error: BaseException) -> str:
    """Return a short diagnosis without exposing provider response bodies or keys."""
    status = getattr(error, "status_code", None)
    name = type(error).__name__.lower()
    if status in (401, 403) or "authentication" in name or "permission" in name:
        return "认证失败：请检查 API Key。"
    if status == 429 or "ratelimit" in name or "rate" in name and "limit" in name:
        return "请求受限：额度不足或请求过于频繁，请检查服务商控制台（HTTP 429）。"
    if isinstance(error, TimeoutError) or "timeout" in name:
        return "请求超时，未取得完整回答。"
    if "connect" in name or "network" in name:
        return "无法连接模型服务。"
    if isinstance(status, int) and status >= 500:
        return f"服务请求失败：HTTP {status}。"
    if isinstance(error, ValueError) or "response" in name or "invalid" in name:
        return "未收到有效的回答文本。"
    return "请求失败：请检查配置、网络和模型名称。"


def main() -> int:
    """Start locally, then make one visible non-streaming model request."""
    print("你好！程序已启动。")
    config = read_config()
    missing = missing_config(config)
    if missing:
        print("API 尚未正确配置，本次没有发送模型请求。")
        print(f"缺少配置：{'、'.join(missing)}")
        return 1
    try:
        from openai import OpenAI
        client = OpenAI(api_key=config["REIN_API_KEY"], base_url=config["REIN_BASE_URL"], timeout=30, max_retries=0)
        response = client.chat.completions.create(
            model=config["REIN_MODEL"],
            messages=[{"role": "user", "content": "你好，请简单介绍一下你能帮助我完成哪些编程任务。"}],
            stream=False,
        )
        choices = getattr(response, "choices", None) or []
        if not choices:
            raise ValueError("empty choices")
        message = getattr(choices[0], "message", None)
        content = getattr(message, "content", "")
        if not isinstance(content, str) or not content.strip():
            raise ValueError("empty response")
        print(f"模型：{content.strip()}")
        return 0
    except Exception as exc:
        print(f"{classify_error(exc)}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

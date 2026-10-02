#!/usr/bin/env python3
"""A small, readable terminal chat client for chapter 01."""

from __future__ import annotations

import os
import sys
from typing import Any


CONFIG_NAMES = ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL")


def read_config() -> dict[str, str]:
    return {name: os.environ.get(name, "").strip() for name in CONFIG_NAMES}


def missing_config(config: dict[str, str]) -> list[str]:
    return [name for name in CONFIG_NAMES if not config.get(name)]


def classify_error(error: BaseException) -> str:
    """Keep diagnostics useful while hiding provider bodies, URLs, and API keys."""
    status = getattr(error, "status_code", None)
    name = type(error).__name__.lower()
    if status in (401, 403) or "authentication" in name or "permission" in name:
        return "认证失败：请检查 API Key。"
    if status == 429 or "ratelimit" in name or ("rate" in name and "limit" in name):
        return "请求受限：额度不足或请求过于频繁，请检查服务商控制台（HTTP 429）。"
    if isinstance(error, (TimeoutError,)) or "timeout" in name:
        return "请求超时，未取得完整回答。"
    if "connect" in name or "network" in name:
        return "无法连接模型服务。"
    if isinstance(status, int) and status >= 500:
        return f"服务请求失败：HTTP {status}。"
    if isinstance(error, ValueError) or "response" in name or "invalid" in name:
        return "未收到有效的回答文本。"
    return "请求失败：请检查配置、网络和模型名称。"


def ask(client: Any, messages: list[dict[str, str]], model: str) -> str:
    response = client.chat.completions.create(
        model=model,
        messages=messages,
        stream=False,
        timeout=30,
    )
    choices = getattr(response, "choices", None) or []
    if not choices:
        raise ValueError("empty choices")
    content = getattr(getattr(choices[0], "message", None), "content", "")
    if not isinstance(content, str) or not content.strip():
        raise ValueError("empty content")
    return content.strip()


def main() -> int:
    config = read_config()
    print("终端对话已启动，输入 /exit 退出。")
    missing = missing_config(config)
    if missing:
        print("API 尚未正确配置，本次没有发送模型请求。")
        print(f"缺少配置：{'、'.join(missing)}")
        return 1
    try:
        from openai import OpenAI

        client = OpenAI(
            api_key=config["REIN_API_KEY"],
            base_url=config["REIN_BASE_URL"],
            timeout=30,
            max_retries=0,
        )
    except KeyboardInterrupt:
        print("\n已退出。")
        return 0
    except Exception as error:
        print(f"{classify_error(error)}", file=sys.stderr)
        return 1

    messages: list[dict[str, str]] = [
        {"role": "system", "content": "你是一个简洁、可靠的编程助手。"},
    ]
    while True:
        try:
            question = input("你：")
        except (EOFError, KeyboardInterrupt):
            print("\n已退出。")
            return 0
        if question.strip() == "/exit":
            print("已退出。")
            return 0
        if not question.strip():
            continue

        candidate = messages + [{"role": "user", "content": question}]
        try:
            answer = ask(client, candidate, config["REIN_MODEL"])
        except KeyboardInterrupt:
            print("\n已退出。")
            return 0
        except Exception as error:
            print(f"{classify_error(error)}", file=sys.stderr)
            continue

        messages.extend(
            [
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        )
        print(f"模型：{answer}")


if __name__ == "__main__":
    raise SystemExit(main())

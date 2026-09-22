---
title: 用 Python 完成第一次模型调用
---

# 01 用 Python 完成第一次模型调用

上一章把任务缩小到一份 README：维护者说启动命令已经改变，我们要提出有依据的建议。不过在读取文件之前，先要解决最基本的一步——Python 程序怎样把一句话发给模型，再把回答取回来？

本章只完成一次请求。程序还不会打开 README，也不会输出已核对的修改位置。我们先运行不需要账户的离线版本，看清输入、响应和失败状态，再解释真实 SDK 请求怎样接入同一条路径。

## 1、准备 Python 环境

以下命令都从 **Rein 仓库根目录**执行。请使用包含 `python/part1/` 的本轮代码；旧版 `ch01` 标签和旧 TS/Rust 教程没有这些文件。

```bash
python3 --version
```

本轮检查使用 Python **3.14.6**，示例使用的 `asyncio.timeout` 要求 Python 3.11 或更新版本。Windows 可在 WSL 中按本文命令操作；原生 Windows 环境若没有 `python3` 命令，可使用对应的 Python 启动器，并调整后面的虚拟环境路径。

先创建独立环境。虚拟环境把本教程的依赖放在一个单独目录，不改变其他 Python 项目的安装：

```bash
python3 -m venv python/part1/.venv
```

离线示例只用标准库，可以直接运行。真实调用另外安装固定 SDK 版本：

```bash
python/part1/.venv/bin/python -m pip install "openai==2.26.0"
```

这里固定的是本轮 SDK 基线，不表示它是最新版本。后续运行真实请求时使用这个环境里的解释器；离线命令则继续写 `python3`，便于直接复制。SDK 的请求用法可对照[官方 Python 库](https://github.com/openai/openai-python/tree/v2.26.0)，本书在当前阶段选择 Chat Completions，以便下一章直接观察 `assistant` 与 `tool` 消息。

## 2、先取得一条离线回答

不需要填写密钥，直接运行：

```bash
python3 python/part1/rein.py hello
```

终端输出 JSON。JSON 用键和值表达结构，适合让后续程序继续读取；字符串有双引号，字段之间用逗号分隔。先关注输出中的三件事：回答来自 `offline`，`request_count` 是 `1`，文本是“请检查 README 的 start 命令”。完整输出还会带上状态与响应摘要。

这里没有访问远端模型。离线适配器返回预先准备的回答，让我们能反复观察程序如何处理它。它也没有读取目标 README，因此这句话只能算一条未核对的意见。

这次命令的完整输出如下：

```json
{
  "record": {
    "task_id": "维护者给出的当前开发命令是 npm run dev；请检查 README",
    "request_count": 1,
    "status": "opinion",
    "error": null,
    "messages": null
  },
  "result": {
    "status": "opinion",
    "text": "请检查 README 的 start 命令",
    "provider": "offline",
    "raw_digest": "1b810f3c75ec68e88cfa26152b44b54660b6f2b74c5bb6605d08d2f0f8d818fa"
  }
}
```

`opinion` 只表示适配器取得了一条可用意见；`provider` 是 `offline`，所以这次运行没有读取 README，也没有访问远端模型。

我们可以自己准备另一条响应，看看同一段程序会怎样处理。先在临时目录创建回放文件，再交给入口；这些命令只创建练习响应，不修改目标文档：

```bash
part1_response_dir=$(mktemp -d)
cat > "$part1_response_dir/hello.json" <<'JSON'
{"text":"先确认 README 中是否仍有旧命令。","provider":"offline"}
JSON
python3 python/part1/rein.py hello --response "$part1_response_dir/hello.json"
```

`--response` 指定本地回放，不会开启真实服务。改变响应文本后，输出里的 `raw_digest` 也应该改变；它标记本轮响应数据，不是目标文件的摘要。程序依旧只处理一次请求。

## 3、获取 API key 并配置真实请求

如果使用 OpenAI，先登录 [API keys 页面](https://platform.openai.com/api-keys)，选择要计费和调用的项目，创建一枚 API key，并在创建后立即保存。官方的首次调用说明见 [OpenAI quickstart](https://developers.openai.com/api/docs/quickstart)。账户、项目和模型是否可用取决于你的平台设置；本教程不保证免费额度或账户一定能调用某个模型。

如果使用兼容服务，请从该服务自己的控制台取得三件套：API key、兼容接口的 base URL、账户可用的模型标识。三者必须属于同一服务，不能把一个服务的 key 与另一个服务的 URL 或模型混用。OpenAI 的 base URL 示例是 `https://api.openai.com/v1`。

真实服务需要三个配置：`REIN_BASE_URL` 指定兼容接口地址，`REIN_API_KEY` 用于认证，`REIN_MODEL` 指定账户可以调用的模型。它们由当前终端的环境变量传给程序，不是发送给模型的任务内容。把下面的占位内容换成自己的配置；密钥不要写入正文、回放文件或 Git。

在 bash 或 zsh 中运行：

```bash
# OpenAI 示例；兼容服务请替换成该服务提供的三件套。
export REIN_BASE_URL='https://api.openai.com/v1'
export REIN_MODEL='你的账户可用的 Chat Completions 模型标识'
printf 'API key（输入不回显）：'
read -r -s REIN_API_KEY
printf '\n'
export REIN_API_KEY
```

执行 `read` 后输入密钥并按回车，终端不显示输入内容。代码不会自动读取 `.env`；新开终端后，需要重新提供配置。第一部分的真实模式要求有效的 HTTP(S) 基础地址，账户或端点是否可用仍由实际服务决定。

当前终端的环境变量只对这个终端及其子进程有效。下面的检查只显示是否已配置和 URL、模型文本，不会打印密钥值：

```bash
python3 - <<'PY'
import os

key = os.getenv("REIN_API_KEY", "")
base_url = os.getenv("REIN_BASE_URL", "")
model = os.getenv("REIN_MODEL", "")
print({
    "api_key_configured": bool(key),
    "base_url": base_url or None,
    "model": model or None,
})
PY
```

若三个字段都已填写，`api_key_configured` 应为 `True`。缺少任意一项时，真实入口会报告 `model_error`，其中的 `detail` 为 `config_missing`。本教程使用 `REIN_API_KEY`，它不会自动映射 SDK 默认读取的 `OPENAI_API_KEY`；这里显式把它传给 `AsyncOpenAI`，因此无需额外设置后者。

下面是一次 SDK 调用的最短结构。`async def` 定义可以等待 I/O 的函数；`await` 等待请求完成；`asyncio.run` 启动这段异步程序。你可以把它另存为练习文件，用虚拟环境的 Python 执行；执行会向你配置的服务发送一次真实请求。

```python
import asyncio
import os
from openai import AsyncOpenAI

async def main():
    async with AsyncOpenAI(
        base_url=os.environ["REIN_BASE_URL"],
        api_key=os.environ["REIN_API_KEY"],
        max_retries=0,
        timeout=30.0,
    ) as client:
        async with asyncio.timeout(30.0):
            response = await client.chat.completions.create(
                model=os.environ["REIN_MODEL"],
                messages=[{
                    "role": "user",
                    "content": "维护者说当前开发命令为 npm run dev。应如何检查 README？",
                }],
                stream=False,
            )
        print(response.choices[0].message.content)

asyncio.run(main())
```

`messages` 是发给模型的消息列表。这里只有一条 `user` 消息，既没有工具，也没有文件内容。`stream=False` 表示等待完整响应后再处理；`choices[0].message.content` 是这段最短例子取出回答的位置。

这个短例子用来辨认请求路径，还没有把空列表、空文本和格式错误变成友好的结果。实际入口在外面加上配置检查、结构检查和统一错误记录。要用配套实现发送同一阶段的真实请求，运行：

```bash
python/part1/.venv/bin/python python/part1/rein.py hello --mode live
```

本轮自动测试使用离线响应和 SDK 的本地模拟；下面另列一次手工真实模式运行结果。自动测试的通过不等于真实服务连通，真实运行也可能失败。

在未配置三个变量的环境中，实际输出是：

```json
{
  "record": {
    "task_id": "维护者给出的当前开发命令是 npm run dev；请检查 README",
    "request_count": 0,
    "status": "error",
    "error": "model_error",
    "messages": null
  },
  "result": {
    "status": "model_error",
    "detail": "config_missing"
  }
}
```

这是配置检查结果，没有发出网络请求。真实请求成功时，输出中的 `provider` 是当前适配器使用的标签 `openai`，不等于对兼容服务身份的判断；服务端或网络异常统一记录为 `model_error`，命令行不会把某个 HTTP 状态码当成单独的教程状态。遇到这类错误时，先按服务文档检查 base URL 的基础路径（不要填完整的 `/chat/completions`）、key 是否属于该服务项目、模型标识是否是账户可用的 Chat Completions 模型，再检查网络连通性。

2026-09-22 的一次真实模式实测结果如下。该次请求尝试了 1 次，进程退出码为 `1`，没有得到模型回答；CLI 只输出统一的 `model_error`，没有显示 HTTP 状态码或具体异常，因此不能仅凭这条记录判断是密钥、余额、模型还是网络原因：

```json
{
  "record": {
    "task_id": "维护者给出的当前开发命令是 npm run dev；请检查 README",
    "request_count": 1,
    "status": "error",
    "error": "model_error",
    "messages": null
  },
  "result": {
    "status": "model_error"
  }
}
```

## 4、把请求收进适配器

离线版本与真实版本的区别集中在取得响应的地方。入口仍然负责创建任务、开始计时、调用一次适配器并记录结果。后续加入工具时，我们就不需要把配置和网络细节散落到每个步骤里。

打开仓库中的 `python/part1/rein.py`，先看命令行参数怎样选择 `hello` 与 `offline/live`。随后在 `python/part1/rein_core.py` 中找到 `OpenAIAdapter`：它把刚才的 SDK 请求放进 `request` 方法，再把响应整理为后续流程需要的对象。阅读本章时先跟随 `hello` 分支，工具和定位分支留到后两章。

配套实现的核心接口如下；这是接口示意，完整定义在 `python/part1/`，不必把这段单独复制运行：

```text
await ModelAdapter.request(task, messages, deadline) -> ModelResponse
ModelResponse(text, raw_digest, provider)
RunRecord(task_id, request_count, status, error)
```

`task` 保留任务目标和读取范围，`messages` 是本次实际发送的内容，`deadline` 是这次运行最晚允许继续的时刻。`ModelResponse` 只整理本章需要的回答、摘要和来源；`RunRecord` 记录发生了几次请求，以及为什么停止。第 01 章成功时只意味着取得可用意见，还没有完成文件维护任务。

SDK 自动重试在这里设为 `0`，避免一次教学请求在背后变成多次网络请求。30 秒预算覆盖本轮等待；超时后程序不采纳迟到响应。关闭客户端与停止等待不能证明远端服务没有收到请求，这一点在后面的取消章节还会继续讨论。

## 5、让失败有具体含义

我们分三层检查响应。首先是能不能取得响应；然后是响应有没有预期结构；最后才是内容是否为非空文字。这样，读者看到错误后就知道下一步该查哪里。

| 状态 | 在哪里失败 | 先检查什么 |
| --- | --- | --- |
| `model_error` | 配置或服务调用无法完成 | 环境变量、端点、账户与网络 |
| `response_invalid` | 响应结构不符合要求 | JSON、`choices`、消息类型 |
| `empty_final` | 回答是空文本 | 模型是否提供可用最终文本 |
| `timeout` | 运行超过截止时间 | 服务等待与任务预算 |

配置缺失时，查看 `result.detail` 是否为 `config_missing`；这是本地检查缺少 `REIN_BASE_URL`、`REIN_API_KEY` 或 `REIN_MODEL` 的明确提示。服务调用阶段的异常保持为 `model_error`，不要根据这条状态推断具体的 HTTP 401 或 429；程序的统一输出不会单独显示这些状态码。

沿用刚才创建的临时目录，准备空文本和损坏的 JSON：

```bash
cat > "$part1_response_dir/empty.json" <<'JSON'
{"text":"","provider":"offline"}
JSON
python3 python/part1/rein.py hello --response "$part1_response_dir/empty.json"
```

该命令应输出 `empty_final` 并以退出码 `1` 结束。失败是本次练习的预期结果，不要为了得到成功状态而填入伪造回答。接着运行：

```bash
printf '{broken' > "$part1_response_dir/broken.json"
python3 python/part1/rein.py hello --response "$part1_response_dir/broken.json"
```

这次应得到 `response_invalid`。测试还会注入空 `choices`、服务异常和超过短预算的延迟，检查迟到结果没有进入后续步骤。它们属于程序边界检查，并不能测出某个真实服务通常需要多久。

## 6、检查本章产物

运行配套测试时，仍从仓库根执行：

```bash
python3 -m unittest discover -s python/part1/tests -v
```

没有安装 SDK 时，与 SDK 有关的单独检查可能跳过；离线请求、文件与建议检查不需要 SDK。完整本地验证请换用前面虚拟环境的解释器。测试同时覆盖后两章的增量，你可以先关注一次请求、无效响应与超时相关结果，再随着阅读理解其他用例。

本章练习是比较默认回答、自定义回答和空回答：记录各自的来源、请求次数、状态和摘要，并说明为什么非空回答仍不代表已经读过 README。你无需提交自己的密钥或真实原始响应。

到这里，我们有了一个能清楚报告成功与失败的一次请求器。下一章[让模型读取真实文件](./python-file-read.md)，会把真实磁盘内容接入同一条消息路径。

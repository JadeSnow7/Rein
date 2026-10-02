---
title: 让 AI 读取代码与日志
---

# 02 让 AI 读取代码与日志

上一章，我们与模型对话，让它生成了 `hello.cpp`，再亲自保存、编译和运行。现在想修改这段代码：例如删掉一个分号，看看模型能否根据编译报错提出修复建议。重新打开对话时，模型不会自动看见电脑上的文件或终端输出。每次都复制源码和日志，既费事，也容易贴进过期内容。

这一章给模型增加取得现场信息的工具：`read_file` 读取指定文件，`read_environment` 查询编译环境，受限的 `bash` 查看练习目录。我们会跟完一次“模型请求工具 → Python 检查并执行 → 结果返回模型”的过程。章末产物是**候选源码和修复理由**；诊断程序不会根据候选写入 `hello.cpp`。如何展示差异、让用户决定并验证写入结果，是下一章的任务。

::: info 本章设计
配套程序按下面的约定写成，你只需知道“它是这么设计的”，不必逐条记住：

- 只开放三个工具：`read_file`、`read_environment` 和受限 `bash`；没有任何写文件的工具。
- `read_file` 只接受 `hello.cpp` 和 `compiler.log`；`bash` 只接受 `pwd` 和 `ls -1`，而且不启动真正的 shell。
- 模型读完源码、日志和环境之前提出的候选不被采纳，程序会提醒它补读。
- 一次诊断最多向模型请求 6 次，超出就停止并报告。
:::

## 1、留下这次编译失败的现场

以下命令从 Rein 仓库根目录执行。沿用第一章保存的 `hello_workspace`。若要独立复现，先创建新练习目录，把已知正确的样本放进去：

```bash
hello_workspace=$(mktemp -d)
cp python/hello_world/fixtures/hello.cpp "$hello_workspace/hello.cpp"
```

打开 `hello.cpp`，删除输出语句末尾的分号。仓库还提供一份固定坏样本；选择手工删除或复制样本中的一种即可：

```bash
cp python/hello_world/fixtures/broken.cpp "$hello_workspace/hello.cpp"
```

复制固定样本后，源码的结尾应是：

```cpp
std::cout << "Hello, world!\n"
}
```

编译一次，并保存**本轮**编译器的输出和退出码：

```bash
c++ -std=c++17 "$hello_workspace/hello.cpp" -o "$hello_workspace/hello" > "$hello_workspace/compiler.log" 2>&1
hello_compile_status=$?
printf '编译退出码：%s\n' "$hello_compile_status"
cat "$hello_workspace/compiler.log"
```

预期退出码非零。不同编译器的报错措辞和行号可能不同，只要日志来自这份缺分号源码即可。`>` 保存标准输出，`2>&1` 把错误输出放进同一份日志。必须紧接编译命令保存 `$?`；如果先运行 `cat` 再查看 `$?`，得到的是 `cat` 的退出码。编译失败后不要运行旧的 `hello` 可执行文件，它可能来自上次成功编译。

此时有三项现场资料：当前 `hello.cpp`、本轮 `compiler.log`，以及刚用过的编译命令。改动源码后要重新编译，旧日志不能证明新源码出了什么问题。

## 2、先体验一次手工提供资料

第一章的终端对话程序 `python/hello_world/chat.py` 可以继续接受问题。你可以把下面的任务贴入同一次输入，但要把括号中的说明替换成刚取得的实际内容：

```text
请根据当前源码和本轮编译结果修复 C++17 Hello World。
目标：编译成功；运行只输出 Hello, world! 和一个换行；正常退出。
范围：只建议 hello.cpp 的必要改动，不要声称已经修改或运行它。

当前 hello.cpp 全文：
（粘贴刚才的源码）

编译命令：c++ -std=c++17 hello.cpp -o hello
本轮编译退出码和 compiler.log 全文：
（粘贴刚才的退出码和日志）

请说明依据、报错原因，并给出完整的候选源码。
```

如果已配置真实模型服务，可以从仓库根目录运行下面的程序，再输入整理好的任务；这一步会把你粘贴的内容发送给服务商：

```bash
python/hello_world/.venv/bin/python python/hello_world/chat.py
```

第一章说过，模型只能看到程序发给它的 `messages`。放进 `messages` 的这些资料，就是模型这一次判断所依据的**上下文**。只说“编译失败了”，上下文里没有文件、命令和报错，模型只能猜；粘贴现场让回答有了依据，但文件一变又要重新复制。先保留坏样本和日志，不要应用这次建议；后面的工具实验仍使用同一份输入，才便于比较。

## 3、把读取文件交给本地程序

我们先增加一个用途明确的工具：`read_file`。它只接收 `hello.cpp` 或 `compiler.log` 两个名字。模型可以提出读取请求，但真正打开文件的是运行在你电脑上的 Python 程序。发给模型的工具声明是这样的：

```json
{
  "type": "function",
  "function": {
    "name": "read_file",
    "description": "Read hello.cpp or compiler.log",
    "parameters": {
      "type": "object",
      "properties": {
        "path": {"type": "string", "enum": ["hello.cpp", "compiler.log"]}
      },
      "required": ["path"],
      "additionalProperties": false
    }
  }
}
```

例如，模型返回 `read_file` 和参数 `{"path":"hello.cpp"}`，程序就检查名称与参数，从练习目录读取当前文件，把内容作为工具结果交回模型。工具声明中的 `enum` 只是给模型的说明；本地执行器仍须检查收到的路径，不能因为模型填了参数就允许读取任意文件。

`core.py` 中的工具分发函数展示这一步怎样落到本地操作。留意它先检查工具名、参数形状和允许的文件名，再返回可序列化的结果：

<<< ../../python/hello_world/core.py#dispatch_tool

## 4、环境信息与受限 Bash

第一章请模型给出运行步骤时，它不知道你用的是什么系统、有没有编译器，只能按你的描述回答。`read_environment` 补上这一点：它不接收参数，只返回操作系统、Python 版本、编译器路径和编译器版本四项信息，不会把环境变量、密钥或整个系统状态交给模型。

`read_file` 适合已知文件名的时候。诊断时，模型有时还想先确认练习目录里有什么，再决定读哪份资料。为此再给它一个名为 `bash` 的检查工具，本章只允许两个固定命令：

| 命令 | 本章用途 |
| --- | --- |
| `pwd` | 查看当前练习目录 |
| `ls -1` | 列出该目录中的文件名 |

例如，模型可以提出 `{"command":"ls -1"}`。执行器先检查它与允许的命令完全一致，再在练习目录中运行对应的程序。虽然工具名叫 `bash`，这一版**没有启动 Bash 解释器**，也不会解释模型提供的管道、重定向或命令替换。

为什么只开白名单？真正的 Bash 可以读任何文件、改写和删除文件，也能运行别的程序。**不提供 `write` 工具，并不能保证文件不被修改**：只要开放通用命令，模型就可能通过它做到。本章的执行器因此在运行前核对整条命令，并限定工作目录、输出长度和等待时间；读取文件内容统一交给 `read_file`。通用命令的权限问题，会在后面的可信 Harness 部分展开。

## 5、跟完一次工具往返

保持第一节的坏源码和本轮日志不变，运行工具诊断入口。它默认使用离线替身，便于在没有 API 配置时观察固定的工具请求；文件读取与本地工具执行仍是真实发生的。

```bash
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode tool
```

观察输出中的 `provider`、`tools`、`request_count`、`message_roles`、`code` 和 `reason`。这份固定样本的离线运行会显示 `tools` 依次为 `bash`、`read_file`、`read_file`、`read_environment`，`request_count` 为 `5`：前四次模型请求各换来一次工具执行，第五次才给出候选。离线替身只认识这份缺分号练习，不能据此判断真实模型的诊断质量。

程序开始时，会先自己读一遍源码和日志：一来记录源码摘要，下一章用它判断文件在等待期间有没有变化；二来缺少日志时可以立即停止，不向模型发送请求。这一步不等于模型已经看见文件内容。只有工具结果进入 `messages`，模型才得到它们。

### 工具请求在网络上长什么样

用真实服务时，SDK 发出的第一个请求除了 `model` 和 `messages`，还带一个 `tools` 列表，也就是上面三个工具的声明。模型想读文件时，响应里的消息不含正文，而是一个 `tool_calls` 列表：

```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_1",
      "type": "function",
      "function": {"name": "read_file", "arguments": "{\"path\":\"hello.cpp\"}"}
    }
  ]
}
```

程序执行后，下一次请求要在 `messages` 末尾依次带上这条 assistant 请求和对应的工具结果：

```json
[
  {"role": "assistant", "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "read_file", "arguments": "{\"path\": \"hello.cpp\"}"}}]},
  {"role": "tool", "tool_call_id": "call_1", "content": "{\"path\": \"hello.cpp\", \"text\": \"#include <iostream>\\n...\", \"digest\": \"530d46ff…\", \"size\": 71}"}
]
```

以上是配套程序通过 SDK 实际构造的请求，服务响应是测试中的模拟响应；`call_1` 这类 ID 由服务生成，每次不同。你只需要看懂三件事：`arguments` 是一段 **JSON 字符串**，程序要先解析、检查，不能当作代码或 shell 命令运行；`tool_call_id` 必须指回对应的请求；模型每次看到的，都是程序重新组装的完整 `messages`。

### 有界的多轮

诊断流程就在下面。读的时候只要抓住主干：请求模型 → 若是候选，检查资料是否读齐 → 若是工具请求，逐个执行并把结果放回 `messages` → 再请求一次。

<<< ../../python/hello_world/core.py#diagnose

这已经是一个小循环：工具结果推动了下一次模型请求。但它是**有界、目的固定的**：只为收集同一份现场资料并产出一份候选，最多请求 6 次；模型过早给出候选时，程序不采纳，而是提醒它先读取缺少的资料。它不会自动修改、编译、再修复。让模型自己决定下一步、由程序决定何时停止的开放循环，是第二部分的主题。

如果已配置真实模型服务，可以显式切换到 `live`。这会向服务发送本次工具返回的源码、日志及有限环境信息，调用可能产生费用：

```bash
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode tool --mode live
```

真实模型可能改变工具请求顺序，也可能提出执行器不允许的请求；不能把离线顺序当作真实服务承诺。遇到非法请求、资料不足或达到请求上限，程序会报错停止。

## 6、检查候选，也检查失败

正常结果应包含候选源码和修复理由。检查它是否仅补上必要的分号、是否保留原来的输出，并确认诊断程序没有写回 `hello.cpp`。模型说“已经修好”不是文件变更或编译成功的证据。你可以把候选另存为临时副本并人工编译，但不要把人工验证说成模型已执行验证。

再试一次资料缺失。在新的临时目录只放坏源码，不放日志：

```bash
missing_log_workspace=$(mktemp -d)
cp python/hello_world/fixtures/broken.cpp "$missing_log_workspace/hello.cpp"
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$missing_log_workspace" --read-mode tool
echo $?
```

预期程序报告 `missing_log`，退出码为 `1`，不会编造一份“看过报错”的诊断。若源码或日志不是普通 UTF-8 文件，或路径越界、内容超限，也应明确失败。配套测试检查工具拒绝越界读取、不允许的 Bash 命令、错误参数、过早的候选和请求次数上限：

```bash
python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v
```

最后，在坏源码第一行上方加一行注释，例如 `// second try`，**重新编译生成对应日志**，再运行诊断。比较两次工具返回的源码，而不是只看候选是否相同；相同的修复建议不能证明程序读取了最新文件。

## 本章边界卡

| 角色 | 本章结束时 |
| --- | --- |
| 模型 | 可以请求读取两个文件、查询环境、查看目录，并给出候选源码和理由 |
| 程序 | 检查每个工具请求并真实执行；保证候选前资料已读齐；限制请求次数 |
| 用户 | 提供现场；阅读候选；决定是否手工采用 |
| 还不能 | 展示差异、确认后写入、自动编译验证 |

现在我们已经不必反复把源码和日志贴进对话，但得到的仍是待审查的候选。下一章[做一个终端代码修改助手](./python-suggestions.md)会展示原文件与候选的差异，让你选择接受或放弃；接受后才写入，并用本地编译、运行结果判断是否完成。

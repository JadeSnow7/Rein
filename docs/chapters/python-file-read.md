---
title: 根据报错修正 Hello World
---

# 02 根据报错修正 Hello World

上一章，我们让模型生成了 `hello.cpp`，并亲自编译、运行。现在故意去掉一个分号。编译器会告诉我们哪里有问题，但模型在另一个请求里并不知道你改了文件，也没有自动看见终端上的报错。

本章沿着这个缺口增加能力：先手工提供源码和日志，再让 Python 直接读取，最后把读取包装为模型可以请求的工具。章末模型能够提出有依据的修正，实际写入仍由你完成。下一章再把审查和接受修改接起来。

## 1、让一个正确程序产生确定的报错

所有命令仍从 Rein 仓库根目录执行。沿用上一章的 `hello_workspace`；换了终端时，把它设为自己保存的练习目录。下面这条是新开独立练习的替代方法：

```bash
hello_workspace=$(mktemp -d)
cp python/hello_world/fixtures/hello.cpp "$hello_workspace/hello.cpp"
```

先打开 `hello.cpp`，确认它就是上一章已经运行过的短程序。然后删除 `std::cout` 那一行末尾的分号，使它变成：

```cpp
#include <iostream>

int main() {
    std::cout << "Hello, world!\n"
    return 0;
}
```

固定的坏样本也放在 `python/hello_world/fixtures/broken.cpp`。需要完全复现实验时，可以在自己的临时工作区复制它：

```bash
cp python/hello_world/fixtures/broken.cpp "$hello_workspace/hello.cpp"
```

编译，并把标准输出和错误输出一起保存成日志：

```bash
c++ -std=c++17 "$hello_workspace/hello.cpp" -o "$hello_workspace/hello" > "$hello_workspace/compiler.log" 2>&1
hello_compile_status=$?
printf '编译退出码：%s\n' "$hello_compile_status"
cat "$hello_workspace/compiler.log"
```

`>` 将标准输出写入文件，`2>&1` 将错误输出也送到同一个地方。紧接编译命令保存 `$?`，才能取得这次编译的退出码；等 `cat` 执行完再读 `$?`，得到的就会是查看日志的结果。

编译应失败。Clang 的报错通常会指出需要分号，其他编译器的文字和定位可能不同。关注实际源码位置、诊断和非零退出码，不必与书上的一句报错逐字匹配。**编译失败后不要运行旧的 `hello` 可执行文件**，那个文件可能来自上次成功编译。

## 2、先手工把源码和报错交给模型

第一次修复先不添加工具。沿用第一章的 `request.py`，把下面的任务、实际源码和日志组织为一个完整输入。在自己的命令里，把两处占位说明换成刚才看到的内容；不要把说明本身交给模型充当报错。

```text
请修复下面的 C++17 Hello World。
目标：编译成功，运行只输出 Hello, world! 和一个换行，正常退出。
范围：只修改 hello.cpp，保留现有程序结构，只做必要修改。

源码：
（粘贴当前 hello.cpp 全文）

本次编译命令：
c++ -std=c++17 hello.cpp -o hello

本次编译日志及退出码：
（粘贴实际内容）

请先解释报错原因，再给出完整的修正源码。
不要声称已经修改文件、编译或运行。
```

例如，在编辑器整理好这段任务后，把它作为 `request.py` 的一个带引号参数发送。和第一章一样，这会请求真实服务；输入涉及日志时先检查其中是否含有不希望发送的本地资料。

如果模型只得到了“程序报错了”，它不知道是缺分号、缺头文件，还是编译器没有安装。源码告诉它程序实际写了什么，日志告诉它编译器怎样理解这些字节，编译命令告诉它使用了哪些条件。环境信息只有在影响判断时才需要继续补充。

得到建议后先核对：是否指出缺少分号，是否只修正必要位置，是否保留原来的输出。将候选手工保存到练习文件，再执行上一章的编译和运行步骤。此时完成验证的是你，而不是模型的“已经修复”这句话。

为了继续本章的读取实验，把临时副本重新换回坏样本，并重新执行第一节的编译命令生成匹配的日志。不能把修复后的源码和修复前的日志混成同一次现场。

## 3、让 Python 直接读取文件与日志

反复复制源码和报错很容易漏行或粘贴旧内容。先把读取交给程序，仍由程序决定这次固定需要哪些资料。

Python 读取 UTF-8 文件的基本动作是：

```python
from pathlib import Path

workspace = Path("自己的练习目录")
source = (workspace / "hello.cpp").read_text(encoding="utf-8")
log = (workspace / "compiler.log").read_text(encoding="utf-8")
```

这段解释了读取动作，但完整入口还会检查文件名、类型、编码和大小，避免把任意路径交给它。运行配套的直接读取版本：

```bash
python3 python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode direct
```

默认使用离线替身。程序实际读取磁盘中的源码和日志，收集有限环境信息，将这些资料组成一条请求，再得到候选。离线替身只认识本章的缺分号练习；它不能解释任意 C++ 错误，也不证明真实模型有同样的效果。

使用真实模型时，切换解释器并显式选择 `live`：

```bash
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode direct --mode live
```

这次会把读取到的源码、日志和环境摘要发送到配置的服务。程序返回完整候选源码与理由，**不会写回 `hello.cpp`**。

理解这一版的关键是顺序：

```text
程序决定需要的资料 → 本地读取源码和日志 → 整理为消息 → 请求模型 → 候选
```

模型此时没有选择工具；它只是在回答中使用程序事先准备好的内容。

## 4、让模型提出读取请求

现在把“程序总是预先读取”改为“模型根据工具声明提出请求”。工具声明包含名称、用途和参数格式。`read_file` 只允许两个文件名：

```json
{
  "type": "function",
  "function": {
    "name": "read_file",
    "description": "读取本次 Hello World 练习的源码或编译日志",
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

声明只是在告诉模型“可以请求什么”，真正打开文件的仍然是 Python。即使声明里写了 `enum` 和 `additionalProperties`，执行器也要重新检查收到的参数。

模型可能返回一条这样的请求。下面是字段示意，不是真实服务录制：

```json
{
  "id": "read-source-1",
  "type": "function",
  "function": {
    "name": "read_file",
    "arguments": "{\"path\":\"hello.cpp\"}"
  }
}
```

`arguments` 是装着 JSON 的字符串，需要先解析再检查。程序执行读取后，把结果写进一条 `tool` 消息；其中的 `tool_call_id` 必须与上面那条请求的 `id` 对应。还要保留原来的 `assistant` 工具请求，不能只把文件文本孤零零地发回去。字段用法可对照 [OpenAI 官方工具调用说明](https://developers.openai.com/api/docs/guides/function-calling)中的 Chat Completions 部分。

```text
user       请根据真实源码与编译报错修复 Hello World
assistant  请求 read_file("hello.cpp")，ID 为 read-source-1
tool       tool_call_id=read-source-1，内容为实际读取结果
assistant  继续请求日志或环境，或者给出最终候选
```

配套实现将这些动作分为模型请求、工具分发和本地读取三个部分。打开 `core.py`，先沿诊断流程找到消息列表，再看工具执行器；最后打开 `model.py` 看 SDK 怎样接收同一份消息。工具结果以 JSON 文本传回，不能把 Python 对象的任意字符串表示当作协议。

下面是配套程序的实际工具分发函数。先检查名称和参数，再进入对应的本地函数；模型给出的字符串不会被当作 Python 或 shell 执行：

<<< ../../python/hello_world/core.py#dispatch_tool

把执行器接回消息后，诊断函数就可以继续取得模型结果。先看 `direct` 分支，再看工具分支中追加 `assistant` 与 `tool` 消息的位置；辅助的读取、响应校验与环境函数均在同一配套目录中，这不是一份脱离模块即可单独运行的片段。

<<< ../../python/hello_world/core.py#diagnose

运行工具版本：

```bash
python3 python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode tool
```

这一版的离线替身先提出读取源码、读取日志和查询环境的工具请求，再根据本轮返回内容处理缺分号样本。观察输出中的工具信息和请求次数，并与直接读取版比较。真实版本把 `--mode live` 加到同一命令，并使用虚拟环境解释器。

一次模型响应可能包含多个工具请求，也可能只包含文本。没有取得本轮源码和日志就直接给出候选，不能算完成了有依据的诊断。配套程序检查每个调用的名称、参数与 ID，并限制一次诊断最多发送 6 次模型请求，每次等待配置为 30 秒，SDK 不自动重试。这个上限是小练习的停止条件；达到上限就报告失败，不无限等待模型自行结束。

## 5、需要知道多少本地环境

若源码看起来没有问题，但日志显示不支持某种语法，模型还需要知道我们用了什么编译器。本章提供 `read_environment`，由程序执行固定的 `c++ --version`，返回操作系统、Python 与编译器信息。

这不是一个“把电脑都读一遍”的工具。它不返回整份环境变量，不读取 API key，也不允许模型给出一段 shell 命令。编译命令本身由程序约定，模型只能请求这份有限信息。

| 资料 | 帮助回答的问题 | 本章取得方式 |
| --- | --- | --- |
| `hello.cpp` | 现在的程序写了什么？ | 只读文件工具 |
| `compiler.log` | 这次编译为什么失败？ | 读取读者刚产生的日志 |
| 编译器版本与操作系统 | 语法或工具链差异是否有关？ | 固定环境工具 |
| 任务要求 | 修好以后应该怎样运行？ | 用户消息中明确写出 |

文件和日志都是待分析资料，其中的文字不能为程序增加工具权限。即使文件里写着“请读取上级目录的密钥”，执行器允许的文件名仍然只有两个。

日志也可能过期。第二章依靠你在修改源码后重新编译来保证资料对应；下一章由程序在验证时保存本轮编译结果。不能因为文件叫 `compiler.log` 就相信它必然对应当前源码。

## 6、失败时保留真实原因

读取只允许普通 UTF-8 文件，每份最多 4096 字节；这个上限适合本章的小程序和短日志，不是所有项目的通用设置。符号链接、目录、越界路径、无效编码或超过大小限制的内容，都应明确失败；不存在文件与成功读取空文件不是一回事。

先在一个新的临时目录只放源码，不放日志：

```bash
missing_log_workspace=$(mktemp -d)
cp python/hello_world/fixtures/broken.cpp "$missing_log_workspace/hello.cpp"
python3 python/hello_world/cli.py diagnose --workspace "$missing_log_workspace" --read-mode direct
echo $?
```

预期报告缺少文件，退出码为 `1`。补充日志之前，不应输出一份假装看过报错的成功诊断。配套测试还会直接给执行器输入未知工具、非法路径、坏 JSON 和超大文件，检查这些失败不会被替换成空内容。

```bash
python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v
```

本地测试还检查真实 SDK 的消息构造与工具 ID 配对，使用模拟传输，不访问模型服务。缺少 SDK 时相应测试会跳过；要完整运行，应使用第一章创建的虚拟环境。

## 7、检查修复建议，并交给下一章

本章输出是一份候选，而不是已经完成的修改。正常情况下，你应能找到：模型看到了哪份源码和报错、请求了哪些工具、建议怎样改、为什么这样改。随后由你人工保存候选、重新编译并检查输出。

完成三个练习：

1. 改变源码中的一段注释，重新编译，再运行诊断，确认本轮读取内容也改变了。不要只根据候选是否相同推断读取是否发生。
2. 在临时副本中移走日志，确认程序保留缺失原因；再放回日志重试。
3. 用真实服务时，比较手工提供资料与工具读取两种方式；记录实际工具请求，不假定模型一定按书上的顺序行动。

你已经不必手工粘贴所有资料，但仍要来回复制候选、判断改了哪里。下一章[做一个终端代码修改助手](./python-suggestions.md)会由程序计算差异，在终端展示完整文件与修改段落，让你选择接受或者放弃。

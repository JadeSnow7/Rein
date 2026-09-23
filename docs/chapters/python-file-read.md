---
title: 让 AI 读取代码与日志
---

# 02 让 AI 读取代码与日志

上一章，我们与模型对话，让它生成了 `hello.cpp`，再亲自保存、编译和运行。现在想修改这段代码：例如删掉一个分号，看看模型能否根据编译报错提出修复建议。重新打开对话时，模型不会自动看见电脑上的文件或终端输出。每次都复制源码和日志，既费事，也容易贴进过期内容。

这一章给模型增加两种取得现场信息的方法：`read_file` 读取指定文件，受限的 `bash` 检查练习目录和已有日志。我们会跟完一次“模型请求工具 → Python 执行 → 结果返回模型”的过程。章末产物是**候选源码和修复理由**；诊断程序不会根据候选写入 `hello.cpp`。如何展示差异、让用户决定并验证写入结果，是下一章的任务。

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

这里的重点是观察信息从哪里来。只说“编译失败了”，模型无法知道是哪份文件、哪个编译命令、什么报错。粘贴现场可以让回答更有依据，但下一次文件变化时又要重新复制。先保留坏样本和日志，不要应用这次建议；后面的工具实验仍使用同一份输入，才便于比较。

## 3、把读取文件交给本地程序

我们先增加一个用途明确的工具：`read_file`。它只接收 `hello.cpp` 或 `compiler.log` 两个名字。模型可以提出读取请求，但真正打开文件的是运行在你电脑上的 Python 程序。

```json
{
  "type": "function",
  "function": {
    "name": "read_file",
    "description": "读取本次练习的源码或编译日志",
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

诊断入口在请求开始时也会检查源码、日志是否存在，并记录源码摘要；这不等于模型已经看见文件内容。只有把内容放进消息，或者响应模型的读取请求后，模型才会得到它们。文件内容和编译报错都是待分析资料；其中即使写着“请读取其他目录”，也不能改变执行器允许的范围。

## 4、用受限 Bash 看清练习目录

`read_file` 适合已知文件名的时候。诊断时，我们有时还想先确认练习目录里有什么，再决定读哪份资料。为此给模型增加一个名为 `bash` 的检查工具。本章只允许下面三个固定命令：

| 命令 | 本章用途 |
| --- | --- |
| `pwd` | 查看当前练习目录 |
| `ls -1` | 列出该目录中的文件名 |
| `cat compiler.log` | 查看已有的编译日志 |

例如，模型可以提出 `{"command":"ls -1"}`。执行器先检查它与允许的命令完全一致，再在练习目录中运行对应的程序。虽然工具名叫 `bash`，这一版**没有启动 Bash 解释器**，也不会解释模型提供的管道、重定向或命令替换。它只是让读者认识命令工具的调用过程；通用 Bash 权限留待后续章节讨论。

Bash 本身可以写文件、删除文件，甚至运行别的程序；**只是不提供 `write` 工具，并不能保证文件不被修改**。本章的执行器因此在运行前核对整条命令，限定工作目录、输出长度和等待时间。源码仍由 `read_file` 精确读取，Bash 只帮助确认目录和已有日志的情况。它不是通用的项目终端，也不能让模型自行执行编译或改写命令。

本章工具的权限由 Python 程序决定，而不是由模型回答或日志文字决定。后续章节会系统讨论更一般的工作区授权、安全边界与写入；这里先做一个可跟随、可拒绝越界请求的小练习。

## 5、跟完一次工具往返

保持第一节的坏源码和本轮日志不变，运行工具诊断入口。它默认使用离线替身，便于在没有 API 配置时观察固定的工具请求；文件读取与本地工具执行仍是真实发生的。

```bash
python3 python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode tool
```

观察输出中的 `provider`、`tools`、`request_count`、`message_roles`、`code` 和 `reason`。这份固定样本的离线运行会显示 `tools` 依次为 `bash`、`read_file`、`read_file`、`read_environment`，`request_count` 为 `5`。离线替身只认识这份缺分号练习，不能据此断定真实模型在其他项目中的诊断质量。你应能辨认下面几个动作：

```text
user       请求检查当前练习，依据源码和报错提出候选
assistant  请求 bash 查看练习目录
tool       返回检查结果，保留对应的 tool_call_id
assistant  请求 read_file("hello.cpp")
tool       返回当前源码
assistant  请求 read_file("compiler.log")
tool       返回本轮日志
assistant  继续请求必要资料，或者给出候选
```

每次工具请求都有 ID。返回消息中的 `tool_call_id` 必须指向这次请求；下一次模型请求还需带上原来的 `assistant` 工具请求和相应 `tool` 结果。服务响应中的 `arguments` 是 JSON 字符串，程序要解析和检查，不能直接把它当作 Python 代码或任意 shell 命令运行。配套程序的诊断流程就在下面：

<<< ../../python/hello_world/core.py#diagnose

如果已配置真实模型服务，可以显式切换到 `live`。这会向服务发送本次工具返回的源码、日志及有限环境信息，调用可能产生费用：

```bash
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$hello_workspace" --read-mode tool --mode live
```

真实模型可能改变工具请求顺序，也可能提出执行器不允许的请求；不能把离线顺序当作真实服务承诺。当前程序对模型请求次数设上限，遇到非法请求或资料不足会报错停止。所谓“迭代”，在本章只是**多轮取得资料、重新判断并形成一份候选**，尚不是自动修改、编译、再修复的循环。

## 6、检查候选，也检查失败

正常结果应包含候选源码和修复理由。检查它是否仅补上必要的分号、是否保留原来的输出，并确认诊断程序没有写回 `hello.cpp`。模型说“已经修好”不是文件变更或编译成功的证据。你可以把候选另存为临时副本并人工编译，但不要把人工验证说成模型已执行验证。

再试一次资料缺失。在新的临时目录只放坏源码，不放日志：

```bash
missing_log_workspace=$(mktemp -d)
cp python/hello_world/fixtures/broken.cpp "$missing_log_workspace/hello.cpp"
python3 python/hello_world/cli.py diagnose --workspace "$missing_log_workspace" --read-mode tool
echo $?
```

预期程序报告缺少 `compiler.log`，退出码为 `1`，不会编造一份“看过报错”的成功诊断。若源码或日志不是普通 UTF-8 文件，或路径越界、内容超限，也应明确失败。配套测试检查工具拒绝越界读取、不允许的 Bash 命令、错误参数和请求次数上限：

```bash
python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v
```

最后，修改坏源码中的一段注释，**重新编译生成对应日志**，再运行诊断。比较两次工具返回的源码，而不是只看候选是否相同；相同的修复建议不能证明程序读取了最新文件。

现在我们已经不必反复把源码和日志贴进对话，但得到的仍是待审查的候选。下一章[做一个终端代码修改助手](./python-suggestions.md)会展示原文件与候选的差异，让你选择接受或放弃；接受后才写入，并用本地编译、运行结果判断是否完成。

---
title: 让模型读取真实文件
---

# 02 让模型读取真实文件

上一章取得了“请检查 README”的回答，但模型还没有见过磁盘上的文件。如果它此时说“第 8 行需要修改”，我们没有理由相信这个位置。接下来要把真实文件接入请求，并且能够解释究竟是哪段程序读到了它。

我们分两步完成：先由程序直接读取，再把同一个读取函数包装成模型可请求的工具。两条路径使用相同的边界检查，区别只在于谁提出读取动作。本章只做一次工具往返，连续自主循环留给 Rust 部分。

## 1、先检查练习目录

以下命令都在仓库根执行，继续使用第 01 章准备的 Python。默认样本是 `python/part1/fixtures/outdated/README.md`，它与本书根目录的 README 是两个不同文件。

```bash
cat python/part1/fixtures/outdated/README.md
```

样本第 8 行是完整的一行 `npm run start`。维护者在任务里提供的新命令是 `npm run dev`；此时我们只检查文档与任务说明是否一致，不运行这两个 npm 命令，也不声称练习目录里有可以启动的网站。

先让程序主动读取：

```bash
python3 python/part1/rein.py read --read-mode direct
```

这条路径先调用 `safe_read`，取得文件文本、字节数和摘要，然后把资料放入模型消息。默认仍是离线模式，模型请求次数应为 `1`。输出中应能找到本轮读取的路径与摘要；读取失败时，不会发送一份伪造的空文档继续请求。

直接读取很适合路径已经确定的任务。它还帮助我们把问题拆开：如果这一条路径都不能正确限制文件范围，把读取动作交给模型选择也不会使它更可靠。

## 2、让读取函数先检查边界

Python 的 `pathlib.Path` 把文件路径表示为对象，`root / relative_path` 可以拼接目录与相对路径。但拼接本身不会授予权限，也不会阻止 `..` 返回上级目录，因此不能把模型给出的字符串直接交给 `read_text()`。

打开 `python/part1/rein_core.py` 的 `safe_read`，沿着“检查路径—打开文件—有界读取—解码”的顺序阅读。任务允许列表在调用它之前核对；直接模式和工具模式都经过这一层。配套实现先完成下面这些检查，再产生可用内容：

| 检查 | 为什么现在就需要 |
| --- | --- |
| 参数是非空相对路径，不含 `..` | 不能从任务目录跳到其他位置 |
| 路径属于任务的 `allowed_paths` | 工作区内的文件也不一定都允许读取 |
| 逐级检查路径组件，拒绝符号链接 | 不能用一个目录内链接指向目录外 |
| 规范路径仍在工作区内，目标为普通文件 | 不把目录或其他对象当成文本 |
| 最多读取 4097 字节，再核对 4096 字节上限 | 文件不能在检查大小之后悄悄长大而绕过限制 |
| 用 UTF-8 严格解码 | 无效编码要报错，不能静默丢弃字节 |

摘要使用读取到的原始字节计算，随后才把字节解码为文本。这样，Windows 常见的 CRLF 换行与 LF 换行不会在摘要里被混为一谈。一个成功的读取对象保留 `path`、`text`、`digest` 和 `size`，失败则返回明确错误，不提供可冒充成功的文本。

这些检查针对自己控制的教学目录。逐级检查与实际打开文件之间仍可能发生竞争；它不是用来抵抗另一个进程恶意替换路径的操作系统隔离机制。先理解这里的限制，后面才能判断什么时候需要更强的文件访问方案。

## 3、把同一次读取包装成工具

现在改用工具路径：

```bash
python3 python/part1/rein.py read --read-mode tool
```

这次应出现两次模型请求。第一轮模型提出 `read_file` 调用，Harness 验证名称和参数，调用同一个 `safe_read`，再把真实结果交给第二轮。第二轮只作出回答；即使它继续请求工具，本章也不会开始无限循环。

工具声明告诉模型有哪些动作可选。这里的 JSON 是协议形状示意，实际声明在配套源码中组装：

```json
{
  "type": "function",
  "function": {
    "name": "read_file",
    "description": "读取任务允许的 UTF-8 文件，最多 4096 字节",
    "parameters": {
      "type": "object",
      "properties": {"path": {"type": "string"}},
      "required": ["path"],
      "additionalProperties": false
    }
  }
}
```

Schema 描述参数形式，程序仍要重新检查名称、字段和允许范围。模型返回了一个结构正确的 `path`，不代表那个路径已经被授权。第一部分不会提供执行 shell 或写文件的工具，因此资料中的文字无法凭空调用这些能力。

工具往返里还有一个容易漏掉的连接：调用 ID。假设模型提出的调用 ID 是 `read-1`，Harness 返回的 `tool` 消息就必须携带同一个 `tool_call_id`。消息序列大致如下：

```text
user      任务：检查 README 的开发命令
assistant 请求 read_file，id=read-1，arguments={"path":"README.md"}
tool      tool_call_id=read-1，ok=true，output=本轮读取内容
assistant 根据上面的工具内容给出意见
```

回传时要保留模型提出调用的那条 `assistant` 消息，再追加与其配对的 `tool` 消息。否则即使文本内容正确，服务端也可能不知道它对应哪一次请求。

工具结果在程序里整理成 `ToolResult{tool_call_id, ok, output, error}`。成功时有实际读取内容；失败时保留错误原因。工具报错会进入本轮消息记录，但流程停止，不再要求模型用缺失资料编出答案。测试会检查这个 ID 与第二轮实际收到的内容，而不只检查最终句子里有没有“README”。

## 4、用文件变化检验消息路径

如果每次离线运行都返回同一句话，无法证明真实内容进入了模型。这里的离线适配器会读取本轮工具消息，按一个非常小的规则处理它：见到旧命令就建议新命令，见到新命令就指出已经一致，找不到依据就停止。它是用于观察数据流的教学替身，不是通用语言模型。

先复制一个临时工作区，然后运行工具读取：

```bash
part1_workspace=$(mktemp -d)
cp python/part1/fixtures/outdated/README.md "$part1_workspace/README.md"
python3 python/part1/rein.py read --workspace "$part1_workspace"
```

接下来，我们人工把这份临时副本改成“已经正确”的版本，再用同一条命令读取：

```bash
cp python/part1/fixtures/correct/README.md "$part1_workspace/README.md"
python3 python/part1/rein.py read --workspace "$part1_workspace"
```

第二次输出应反映新内容，读取摘要也会不同。上面的 `cp` 是读者为了实验主动改动副本；建议器本身没有写入。测试另外检查第二轮请求确实含有本轮工具文本，防止只更换终端上的结果文字来制造假象。

同一份未变化的文件经 `direct` 与 `tool` 路径读取，摘要应一致。请求次数分别为一和二，取得的文件证据却应该相同。

## 5、尝试一个不允许的路径

第一部分 CLI 默认只允许读取工作区中的 `README.md`。`--path` 是本次请求的目标，不会顺便扩大允许列表。运行：

```bash
python3 python/part1/rein.py read --path ../README.md
```

程序应输出 `path_invalid`，退出码为 `1`。它不会尝试在上级目录搜索同名文件。绝对路径、未授权子目录文件与符号链接也要被拒绝。

文件合法并不等于内容可读。沿用上一节的临时工作区，创建一个超过上限的 README：

```bash
python3 - "$part1_workspace" <<'PY'
import sys
from pathlib import Path
(Path(sys.argv[1]) / "README.md").write_bytes(b"a" * 4097)
PY
python3 python/part1/rein.py read --workspace "$part1_workspace"
```

这次应该看到文件过大的失败。空文件和读取失败要区分：空文件是成功读取了零字节；不存在、权限不足、超大或无法解码则没有取得可用资料。配套测试还会模拟未知工具、缺失参数与读取异常，确认程序不会把它们包装成成功文本。

## 6、完成一次可以解释的读取

本章练习是在临时工作区中分别准备正常 README、指向外部文件的符号链接和超大 README，记录每次状态与请求次数。然后试着把文件移到子目录并请求它：默认允许列表仍只有 `README.md`，所以这次应被拒绝。若要扩展练习，必须同时在程序构造的 `Task.allowed_paths` 中明确加入该相对路径，不能只改变模型参数。

完成时，你应能从代码与输出解释：谁提出读取，谁检查路径，哪个函数打开文件，第二轮消息如何拿到结果，以及为什么两次模型请求还不构成一个通用 Agent Loop。

到这里，我们已经有了真实原文，但最终回答中的位置仍需核对。下一章[在终端展示修改位置与建议](./python-suggestions.md)，由程序给原文编号，并处理找不到、重复和文件已经变化的情况。

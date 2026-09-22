---
title: 在终端展示修改位置与建议
next: false
---

# 03 在终端展示修改位置与建议

现在程序已经读到了 README，模型也能根据资料提出“把 start 改成 dev”。但审阅一份修改时，我们还需要知道它具体指向哪里，原文是否吻合，以及这份文件在等待模型期间有没有变化。

本章把这些检查交给程序。模型提供待改原文、替换内容和理由；程序负责匹配完整行片段、计算位置、核对文件摘要，再展示建议。第一部分到这里结束，磁盘中的目标文件依然保持不变。

## 1、先运行完整建议流程

继续在仓库根执行：

```bash
python3 python/part1/rein.py suggest
```

默认样本的旧命令在第 8 行。输出的 `result` 应包含下面这些内容；这里省略运行记录，并用说明代替每次实际计算的摘要：

```json
{
  "path": "README.md",
  "start_line": 8,
  "end_line": 8,
  "original": "npm run start",
  "suggested": "npm run dev",
  "reason": "preview uses the development command",
  "source_digest": "这里是本轮读取字节的 SHA-256"
}
```

这份输出对应 `README.md:8`，不是“已经修好 README”。终端使用缩进 JSON，便于逐项查看，也便于下一阶段的 Rust 程序比较同样的字段。实际输出中还要看状态和来源，不能只截取一句建议当作完成证明。

当前依据来自维护者提供的新命令说明。模型没有执行 `npm run dev`，程序也没有检查完整应用是否可用。第一部分验证的是建议的来源和定位，而不是替换之后的项目行为。

## 2、让模型只描述要改什么

模型提供的意见使用一个很小的对象：

```json
{
  "path": "README.md",
  "original": "npm run start",
  "suggested": "npm run dev",
  "reason": "维护者说明当前开发命令为 npm run dev"
}
```

这里故意没有 `start_line` 和 `end_line`。如果模型同时写了行号，程序将其视为 `response_invalid`，不会因为它看起来准确就直接接受。模型还可能返回不存在的路径、遗漏 `reason`，或用一段 Markdown 包裹本应为 JSON 的对象；这些情况也要在定位之前被拒绝。

`original` 必须是一个或多个完整行组成的片段。如果文件里写的是 `运行 npm run start 以启动预览`，模型只返回 `npm run start` 就不足以定位这个版本的建议。应让它返回完整原文，再由程序核对。这样的规则比较严格，但它让第一部分的行号语义保持简单：我们展示的是完整行的替换建议。

`suggested` 可以与 `original` 相同，这表示检查后无须修改；也可以为空字符串，表示建议删除这一段。删除仍然只是意见，程序不会提前执行。

## 3、从真实原文计算行号

打开 `python/part1/rein_core.py` 的 `locate_suggestion`。它接收上章实际读取的内容与模型意见，先检查字段，再寻找原文位置。为了比较行片段，先把 CRLF 规范成 LF，再按行分开。规范化只用于匹配和展示，原始字节摘要保持不变，也不会重写文件的换行符。

假设源文件各行是 `source_lines`，模型返回的原文各行是 `original_lines`。定位的核心动作可以写成下面的 Python 片段；这是匹配算法示意，完整实现还检查字段类型、空原文和路径：

```python
positions = []
width = len(original_lines)
for index in range(len(source_lines) - width + 1):
    if source_lines[index:index + width] == original_lines:
        positions.append(index + 1)
```

Python 列表从 `0` 开始计数，人读文件通常从第 `1` 行开始，因此匹配成功后加一。如果一个两行片段从第 8 行开始，结束行就是 `8 + 2 - 1 = 9`。程序产生这两个位置，模型无法用一个随意填写的数字覆盖它们。

得到候选位置之后，程序还有三种不同处理：没有匹配，返回 `original_missing`；有一个匹配，才能生成定位建议；有多个匹配，返回 `ambiguous_match` 并列出候选起始行。空文件也不能凭空匹配一个空原文。

## 4、让重复原文暴露出来

仓库准备了两处相同命令的样本，分别位于第 3 行与第 9 行：

```bash
python3 python/part1/rein.py suggest --workspace python/part1/fixtures/duplicate
```

程序应返回 `ambiguous_match`，候选为 `[3, 9]`，退出码为 `1`。它没有足够信息决定该改哪一处，因此不生成一份看似已经确定位置的建议。

解决歧义的方法是取得更多上下文，或者把 `original` 扩成足以唯一定位的多行片段。例如把“开发环境”标题与其下一行命令一起返回，程序就能核对整个片段。本章不会自动让模型反复重试；先保留明确失败，后面的核心循环才能决定是否继续请求。

再试一份已经正确的文档：

```bash
python3 python/part1/rein.py suggest --workspace python/part1/fixtures/correct
```

这次应为 `no_change`，退出码为 `0`。输出仍须包含文件摘要、找到的原文、位置和说明。只说“没问题”却无法给出核对依据，不足以成为这个程序的成功结果。

如果同一个已经正确的片段也出现多处，程序仍会报告歧义。当前算法不凭“替换前后一样”绕过定位要求。

## 5、在展示前再次核对文件

真实请求可能需要等待。在这段时间里，你可能在编辑器中改过 README，另一个程序也可能改过它。即使模型的建议符合第一次读取的内容，它也可能已经过期。

配套流程在展示候选之前再次使用安全读取函数取得文件字节，并与第一次读取的摘要比较。摘要改变时返回 `source_changed`；文件无法重新读取时也不会继续输出可用候选。要继续处理，应重新读取、重新生成建议，而不是强行沿用旧位置。

这个检查能发现两次读取之间的变化，不能保证展示以后文件永远不变。我们目前没有写入动作，所以它只决定建议是否还对应当前读到的内容；后面真正应用修改时，还需要在应用边界重新检查。

验证“不写文件”时也使用字节摘要。下面的练习从仓库根执行，启动一次建议器后比较目标文件：

```bash
python3 - <<'PY'
import hashlib
import subprocess
import sys
from pathlib import Path

target = Path("python/part1/fixtures/outdated/README.md")
before = hashlib.sha256(target.read_bytes()).hexdigest()
subprocess.run([sys.executable, "python/part1/rein.py", "suggest"], check=True)
after = hashlib.sha256(target.read_bytes()).hexdigest()
assert before == after, "建议流程改变了目标文件"
print("目标文件字节未改变")
PY
```

该检查只证明这一次运行前后的目标字节相同。配套测试还会覆盖错误分支，并在模型响应期间主动改变临时文件，确认主流程确实拒绝了过期结果。

## 6、检查一份无效建议

第一部分的失败都应有明确含义，不能在解析失败后随手拼接一个“看起来合理”的结果。

| 情况 | 结果 |
| --- | --- |
| JSON 无法解析、字段缺失或模型夹带行号 | `response_invalid` |
| 建议指向本轮未读取的文件 | `path_invalid` |
| 原文找不到，或只有子串相同 | `original_missing` |
| 原文匹配多个位置 | `ambiguous_match`，保留候选行 |
| 生成期间源文件变化 | `source_changed` |
| 模型超时、文件过大或无效编码 | 保留对应运行或读取错误，停止展示 |

运行完整本地测试：

```bash
python3 -m unittest discover -s python/part1/tests -v
```

测试中包含 LF、CRLF、多行片段、空文件、重复原文、越界路径、超时与文件变化。SDK 的模拟请求需要第 01 章的虚拟环境；不安装 SDK 时，那部分会明确跳过，不影响离线示例运行。

仓库还保存了一份完整的两轮回放 `python/part1/fixtures/responses/c01.json`。下面复制它并删掉最终建议的 `reason`，让字段检查确实失败：

```bash
part1_candidate_dir=$(mktemp -d)
cp python/part1/fixtures/responses/c01.json "$part1_candidate_dir/replay.json"
python3 - "$part1_candidate_dir/replay.json" <<'PY'
import json
import sys
from pathlib import Path
path = Path(sys.argv[1])
events = json.loads(path.read_text())
candidate = events[-1].get("suggestion", events[-1])
candidate.pop("reason")
path.write_text(json.dumps(events, ensure_ascii=False))
PY
python3 python/part1/rein.py suggest --response "$part1_candidate_dir/replay.json"
```

预期为 `response_invalid`，退出码 `1`。接着自己在临时目录复制默认 README，新增一处相同命令，观察歧义；随后只改动其中一处，重新运行并核对行号和摘要。修改回放和临时文件是你主动执行的实验，不是建议器获得了写入能力。

## 7、把第一部分交给 Rust 迁移

第一部分已经把一条短流程接起来：任务说明进入请求，真实读取结果回到消息，模型提出意见，程序关联原文与位置并检查是否过期。我们没有建立自主循环，也没有批准或应用修改。

配套目录中的 `S1-manifest.json` 用来登记这一阶段的代码、输入样本、回放与预期结果。它是本轮教学产物的内容清单，不是名为 `S1` 的 Git 标签。未来迁移时，应使用相同文件字节、相同受控响应和相同错误样本，比较定位、错误类别、请求次数与目标文件不变；不要拿真实模型两次回答是否逐字相同来判定语言迁移。

六部分框架的新第 04 章将从这份清单开始用 Rust 复现，再逐渐增加协议、循环、预算和取消。该章本轮尚未重写；已有的 Rust hello 与旧迁移材料保留在[历史入口](../history.md)。你可以回到[全书目录](../toc.md)查看后续路线。

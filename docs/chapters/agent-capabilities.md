---
title: Agent 的核心能力
---

# 04 Agent 的核心能力

**状态：初稿。** 本章的 Rust 迁移只覆盖不依赖模型协议的部分，并已通过对照脚本；模型调用留到第 05 章。本章是[第二部分：核心循环](../roadmap/part-02.md)的起点。

第一部分做出了一个受限的 Python 终端代码修改助手：它能读取 `hello.cpp` 和真实编译日志，取得修复候选，展示程序计算的 diff，在用户接受后写入，并编译运行。本章回答两个问题：这些动作里，哪些构成 Agent 的核心能力，哪些决定应当由 Harness 保管？以及，要继续加入循环、预算和取消，现在的写法缺了什么？

本章也是全书工作方式的转折点。前三章你只需知道配套程序“是按什么设计写的”；从这一章起，**由你确定设计和验收条件，让 Claude Code、Codex 这类代码助手编写代码**，再由程序判定结果。本章不增加读者可见的新功能：迁移到 Rust 以后，助手的行为应当和 Python 版完全一致。

## 1、第一部分留下的问题

先看第一部分配套代码里的三处真实情况：

| 位置 | 现在的写法 | 隐患 |
| --- | --- | --- |
| 写入结果 | `ApplyResult.status` 是字符串 `"accepted"`、`"rejected"` | 拼错一个字母也能运行；调用方无法确认处理了所有情况 |
| 模型事件 | 诊断循环同时兼容 `tool_call` 和 `tool_calls` 两种形状 | 同一件事有两种表示，每个读取它的地方都要记得两种都处理 |
| 错误 | 所有失败都是 `HelloError`，只靠 `code` 字符串区分 | 新增一种失败时，没有地方提醒你哪些调用方要跟着改 |

还有两个缺陷，在第一部分定稿前的审查中才被发现：

- **放弃和复查的顺序。** 早先的 `apply_candidate` 先复查源文件，再处理“放弃”。结果是：等待期间你改了文件、又选择放弃，得到的是 `source_changed` 错误，而不是正常的放弃。测试没有发现它，因为验收样本里没有“文件已变 + 放弃”这个组合。
- **过严的资料检查。** 早先模型只要在读齐源码、日志和环境之前给出候选，诊断就直接失败；真实模型经常这样做。设计里只写了“候选前必须读过资料”，却没写“没读齐时怎么办”。

两个缺陷都已修复，并补上了对应的测试样本。它们说明了本章的两个出发点：

1. **状态要显式。** 只有一个文件、一轮决定时，靠字符串和函数返回值还能维持。第二部分要加入多轮行动、预算和取消，每一步都得清楚“现在处于什么状态、能转到哪里”。Rust 的 `enum` 和 `Result` 能让“忘了处理某种情况”在编译时就暴露出来。
2. **验收样本由人来定。** 代码写得再整齐，也只会满足你写下的样本；样本里没有的组合，就没有人替你检查。

## 2、从三张边界卡到能力地图

把 01–03 章末的边界卡放在一起，就能看出一个编码 Agent 需要哪些能力，以及第一部分各做到了哪一步：

| 核心能力 | 第一部分已有的具体行为 | 本章工作 | 后续深化 |
| --- | --- | --- | --- |
| 明确任务与结果 | 限定修复 `hello.cpp`，按输出和退出码验收 | 保留任务输入与成功条件 | 15 任务验证，16 计划模式 |
| 获取环境信息 | 受限读取源码、日志和环境摘要 | 迁移工具的检查与执行 | 10 资料获取，19 MCP |
| 控制行动边界 | 固定文件和命令；用户确认后写入 | 用类型区分放弃、写入与失败 | 12 通用权限，14 持久批准 |
| 维护过程状态 | 候选、用户决定、写入与验证结果 | 把状态写成枚举，把失败写成错误类别 | 06 循环，07 预算，08 取消，09 运行状态 |
| 审查与验证 | 程序计算 diff、复查源字节、备份、编译运行 | 同一组样本对照迁移前后 | 13 候选补丁，15 有限修复 |
| 扩展与交付 | 没有通用扩展或 IDE | 不涉及 | 17–20 扩展，21–24 IDE |

还要承认一件事：第二章的诊断已经是一个**有界、目的固定的小循环**，工具结果推动了下一次模型请求，最多 6 次。第 06 章要做的，是把它推广成开放的循环：由模型决定下一步做什么，由 Harness 决定是否继续、何时停止。

## 3、代表性编码 Agent 怎样分工

公开产品里同样能看到这些能力，也能看到它们各自怎样在人与 AI 之间分配决定。下表只列可以从官方文档核对的机制，不做产品排名，也不推断内部实现；资料核对于 2026-09-23，功能会随版本变化。

| 产品 | 可观察机制 | 人与 AI 的分工 | 对应第一部分 |
| --- | --- | --- | --- |
| Claude Code | 权限模式决定哪些动作不必询问；`default` 模式下改文件、跑命令前询问用户，`plan` 模式只读、先给方案 | 用户选择放权程度，可以要求先看计划再允许修改 | `[y/N]` 确认；诊断只读、不写文件 |
| Codex | 沙箱限定命令能访问的范围，批准策略决定何时停下来问人，两者分开配置 | 用户分别决定“能做什么”和“做之前要不要问” | 固定命令白名单与写入前确认是两道独立的关 |
| Gemini CLI | 策略引擎对每个工具调用给出允许、拒绝或询问用户三种决定，并有 `default`、`autoEdit`、`plan`、`yolo` 等批准模式 | 规则由人写好，程序逐次执行判定 | `dispatch_tool` 先检查再执行 |
| GitHub Copilot 云端 Agent | 在分支上修改代码，由人审查后再创建和合并拉取请求 | Agent 负责改，人负责审查和合并 | 程序计算 diff，用户审查后才写入 |

来源：[Claude Code：选择权限模式](https://code.claude.com/docs/en/permission-modes)、[Codex：Agent 批准与安全](https://developers.openai.com/codex/agent-approvals-security)、[Gemini CLI：策略引擎](https://geminicli.com/docs/reference/policy-engine/)、[GitHub Docs：关于 Copilot 云端 Agent](https://docs.github.com/copilot/concepts/agents/coding-agent/about-coding-agent)。这些资料支持能力拆解，不证明 Rein 已实现同等功能。

留意最后一列：这些产品的做法，第一部分都有一个最小版本。本章接下来要用的代码助手本身就是 Harness，我们也会按同样的原则使用它。

## 4、换一种工作方式：人写设计，代码助手实现

从这一章起，每章按这样的顺序推进：

```text
问题 → 设计卡（人）→ 验收样本（人）→ 代码助手实现 → 审查要点（人）→ 运行验收 → 边界卡
```

使用 Claude Code、Codex 时，先记住五条最基本的做法：

1. 交给它设计卡和验收样本，而不是一句“帮我用 Rust 重写”。
2. 告诉它允许修改的目录和文件，参考代码只读。
3. 让它先说明计划，你看过再让它动手（例如 Claude Code 的 `plan` 模式）。
4. 小步推进，每一步都运行验收；不接受“已完成”的自述。
5. 审查 diff 时，按本章列出的审查要点逐条看。

这里只给最低限度的用法。怎样写好交给代码助手的提示词、怎样防御 AI 常见的失误，会在后续相应章节展开。

## 5、设计卡：Rust Harness core 的第一块

设计卡由人来写，写的是“要什么”和“不要什么”，不写函数体。下面是本章的设计卡，依据第一部分 Python 实现整理。

**目标**：把第一部分助手中不依赖模型协议的行为等价迁移到 Rust，放在独立的 crate `rust-hello-world/` 中。候选暂时从 Python `diagnose` 输出的 `proposal.json` 读取；模型调用留给第 05 章。

**模块边界**：

| 模块 | 负责 | 不负责 |
| --- | --- | --- |
| `workspace` | 只读 `hello.cpp`、`compiler.log`，限制大小与编码，算摘要 | 任何写入 |
| `tools` | 三个工具的参数检查与执行 | 决定何时调用工具 |
| `review` | 行号全文与 unified diff | 判断改动好不好 |
| `apply` | 用户决定；唯一写 `hello.cpp` 的地方 | 编译与运行 |
| `check` | 固定命令编译、只运行本轮产物、判定输出 | 修改源码 |

**状态与错误**：用类型表达，而不是字符串。

```rust
pub enum Decision { Accept, Reject }                 // 只有明确的 y 才是 Accept

pub enum ApplyOutcome { Rejected, Applied { backup: PathBuf } }

pub enum EditOutcome {                                // 一次 edit 停在哪里
    Rejected,
    Verified { backup: PathBuf, check: CheckResult }, // check.passed 可能为 false
}

pub enum ErrorKind {                                  // code() 与 Python 的错误字符串一致
    PathInvalid, FileTooLarge, InvalidUtf8, ToolUnknown, ToolInvalid,
    BashCommandInvalid, BashTimeout, BashFailed, BashOutputTooLarge,
    ResponseInvalid, SourceChanged, WriteFailed, CompileTimeout, CompilerMissing,
}

pub fn apply_candidate(workspace: &Path, candidate: &str, expected_digest: &str,
                       decision: Decision) -> Result<ApplyOutcome, HelloError>;
pub fn check_cpp(workspace: &Path, run_timeout: Duration) -> Result<CheckResult, HelloError>;
pub fn dispatch_tool(workspace: &Path, call: &ToolCall) -> Result<Value, HelloError>;
```

**不变的规则**：放弃不读、不写、不备份；写入前复查源文件摘要，备份刚核对过的字节；程序不经过 shell 启动；外部输入出错时返回错误类别，不让进程崩溃。

**明确不做**：模型调用、新工具、新的写入路径、多文件、循环、预算、取消。

## 6、验收样本：什么算“等价”

验收样本同样由人来定。我们把第一部分 B03–B05 中不涉及模型的部分冻结成 25 个样本，在 Python 和 Rust 上分别运行，比较可观察的结果：

| 样本组 | 内容 | 比较什么 |
| --- | --- | --- |
| B03 工具（14 个） | 正常读取；`..`、绝对路径、符号链接、超大文件、非 UTF-8；未知工具、多余参数、空 ID；`ls -1`；被拒绝的 `cat` 和 shell 文本；环境查询 | 结果内容或错误类别；源文件是否未变 |
| B04 编辑（6 个） | 输入 `n`、直接 EOF、输入 `y`、彩色显示；等待期间文件被改后接受或放弃 | 退出码、状态、审查文本、写入后的源码、备份内容、验证结果 |
| B05 检查（5 个） | 正确程序、编译错误、输出错误、运行超时、`compiler.log` 是符号链接 | 退出码、是否通过、是否编译成功、运行退出码、输出、日志是否记录运行 |

比较规则：比较**语义和副作用**，不比较错误说明的措辞；环境查询只比较编译器路径和版本，操作系统字段和语言名称本来就不同。

## 7、用代码助手实现，再审查它的产出

把第 5、6 节交给代码助手，就是本章末尾的提示词。它的产出应当是一个 crate 和一个对照脚本：

| 文件 | 内容 |
| --- | --- |
| `rust-hello-world/src/*.rs` | 七个模块，约 900 行 |
| `rust-hello-world/tests/behavior.rs` | 7 组 Rust 测试，覆盖读取边界、工具检查、放弃、复查、备份、diff 和检查 |
| `rust-hello-world/compare_parity.py` | 在 25 个样本上运行两边并比较 |

审查时不必逐行读完。先看最要紧的写入模块：

<<< ../../rust-hello-world/src/apply.rs

对照设计卡，重点检查五处：

1. **放弃在最前面。** `Decision::Reject` 直接返回，之前没有任何读写。这正是第 1 节那个缺陷的反面。
2. **复查在备份之前，备份写的是刚核对过的字节。** `source.text` 来自同一次 `safe_read`，没有再读一次文件。
3. **外部输入不用 `unwrap`。** 读文件、解析 JSON、启动进程的每一处失败都经 `?` 变成 `HelloError`。`unwrap()` 遇到错误会让整个进程崩溃，这恰好违反“失败要收敛为错误类别”。可以用 `grep -n unwrap rust-hello-world/src` 快速检查：`src` 中只在已知安全的位置出现 `unwrap_or…` 一类带默认值的写法。
4. **没有 shell。** `tools.rs` 和 `check.rs` 都用 `Command::new(程序).args(参数)` 启动固定程序，参数不经过字符串拼接。
5. **错误类别对得上。** `ErrorKind::code()` 的每个字符串都能在 Python 版里找到。

读这些代码需要的 Rust 知识，到“能看懂并审查”为止就够了：`enum` 加 `match` 让每种情况都必须处理；`Result<T, E>` 和 `?` 把错误逐层交回调用方；`&Path`、`&str` 这类引用表示“借来读一下”，函数不会拿走也不会修改调用方的数据。

## 8、运行验收

在仓库根目录执行：

```bash
cargo build --manifest-path rust-hello-world/Cargo.toml
cargo test --manifest-path rust-hello-world/Cargo.toml
python3 rust-hello-world/compare_parity.py
```

对照脚本逐个打印样本结果，最后一行应当是：

```text
25/25 samples behave the same.
```

对照脚本本身也要检验：它真的能发现不一致吗？做一次反向实验：把 `apply.rs` 中“放弃直接返回”的判断挪到复查之后，也就是第 1 节那个旧缺陷的写法，重新构建并运行对照脚本。它应当报告：

```text
DIFF  B04  stale source, reject
24/25 samples behave the same.
```

改回来以后恢复 25/25。只跑正确版本，无法证明验收脚本有能力发现问题。

本章配套实现的一次完整运行记录（环境、命令、输出，以及上面的反向实验）见 `records/REIN-CH04-RUST-20260923/`。那次运行在 Linux 上完成，macOS 上的结果需要你自己运行后记录。

## 本章边界卡

| 角色 | 本章结束时 |
| --- | --- |
| 模型 | 暂不参与 Rust 版；候选仍来自 Python 诊断 |
| 程序（Rust） | 检查并执行工具；计算 diff；按用户决定写入；固定命令验证；状态和错误都有明确类型 |
| 用户 | 写设计卡和验收样本；让代码助手实现；审查并运行验收 |
| 还不能 | Rust 版调用模型、多轮循环、预算、取消 |

第 05 章接收本章的类型和工具结果，统一模型消息与调用 ID，让 Rust 版自己向模型请求候选；第 06 章让工具结果推动开放的循环；第 07 章加入预算和重复动作的判断；第 08 章加入取消和事件观察。每一章都继续回归本章这 25 个样本。

## 提示词

**来源：据设计反推，未经原样运行。** 这份提示词由本章的设计卡和验收样本整理而来。配套的 Rust 代码是由 Claude（Cowork 会话）按同一份设计实现并通过验收的，但并没有把下面这段文字原样交给 Claude Code 或 Codex 运行，所以不标为“已试用”。如果你用它完成了本章，欢迎记录起始快照、工具与模型、实际 diff 和验收结果。规则见[提示词示例说明](../prompt-examples.md)。

```text
请在仓库根目录新建独立的 Rust crate rust-hello-world/（包名 rein-hello-core，
自带空的 [workspace]，不加入根 workspace），把 python/hello_world/ 中不依赖
模型协议的行为等价迁移过来。

参考代码：python/hello_world/core.py、cli.py，只读，不要修改。
允许新增或修改的文件：只限 rust-hello-world/ 目录。

设计卡：<粘贴本章第 5 节>
验收样本：<粘贴本章第 6 节>

约束：
- 只实现三个子命令：check、tool、edit。edit 从 --proposal 指定的
  proposal.json 读取 code、reason、source_digest，不调用任何模型。
- 错误类别字符串、check 与 edit 的输出字段与 Python 版一致。
- 外部输入不用 unwrap/expect；不经 shell 启动任何程序。
- 依赖只允许 serde、serde_json、sha2。
- 另写 compare_parity.py：在同一组样本上分别运行 Python 与 Rust，
  比较状态类别、退出码、文件摘要、备份、检查结果和审查文本，不比较错误措辞。

先给出计划和文件列表，等我确认后再动手。每完成一个模块就运行 cargo test。
最后报告：实际运行的命令、输出与退出码；未通过或未执行的检查如实列出。
不以“已完成”代替验收结果。
```

这段提示词里，每一句都能对应到设计卡、验收样本或本章第 4 节的某条做法。“先给计划、等确认”和“报告实际命令”两句，是在用第一部分学到的 Harness 原则约束代码助手本身。

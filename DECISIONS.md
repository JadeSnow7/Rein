# 工程决策记录

本文件记录影响仓库长期结构的决定。正文写作过程中如需推翻，请在此追加修订而非直接改写历史条目。

当前跨项目分工见末尾的[职责边界同步](#职责边界同步2026-10-04)；历史教学安排保留原义。

## D1　代码快照策略：单一演进树 + git tag，独立章节另设 examples

`ts/src/` 始终只有一份最新代码，随正文逐章演进。每章结束打 tag：`ch01`…`ch16`，阶段发布另打 `v0.1`/`v0.2`/`v0.3`/`v1.0`。

读者获取某一章代码：

```bash
git checkout ch03
```

标注"可独立阅读"的第 08、09、11、12、14 章，额外在 `ts/examples/<topic>/` 放一份自包含最小示例，不依赖主线状态。

理由：主线的重构过程本身是教学内容——第 04 章"从已有调用代码中提炼合同"只有在代码真的先长歪过才成立。逐章复制目录会把这段叙事抹平，且同一个缺陷要改十六处。

## D2　工程形态：npm workspaces + tsx + vitest

- 根 `package.json` 声明 `workspaces: ["ts"]`，一次 `npm ci` 装齐文档与代码依赖。
- 运行方式用 `tsx`，不用 Node 原生类型剥离。读者环境的 Node 小版本不统一，第 01 章不该卡在这里。
- 测试用 `vitest`，与 vitepress 同属 Vite 生态，配置最少。
- 暂不引入 lint/format 工具。需要时再加，不预先背依赖。

Rust track 保持空目录，推迟到第 05 章（附录 A.2）之前再建 Cargo manifest。

## D3　模型提供方：OpenAI 兼容接口为主线，Anthropic Messages 为第 04 章对照

主线代码通过 `REIN_BASE_URL` 指向任意 OpenAI 兼容端点，不绑定单一厂商。第 04 章需要对照第二种响应形状时，取 Anthropic Messages API——两者在工具调用与多轮消息上的差异足够典型。

密钥经 `ts/.env.example` 约定，不入库。

**录制约定**：任何进入正文的真实请求/响应，都要落盘到 `fixtures/responses/<provider>/`。API 形状会变，当时不录，后续章节的对照就写不出来。由此第 01 章即引入可替换的 transport 层，使测试全程走录制样本、不触网——这是 CI 能在无密钥环境下运行的前提。

## D4　fixtures：现在只冻结目录约定，合同本身留到第 04 章

目录结构见 `fixtures/README.md`。现在定死的是三层布局与命名，不是消息形状。

`contracts/` 在第 04 章由正文推导出首个版本。提前定义会毁掉那一章的教学价值：读者需要先经历"两家响应对不上"的具体麻烦，抽象才有意义。

## D5　CI：新增代码测试工作流，无代码时跳过

`.github/workflows/test.yml` 按 `ts/`、`contracts/`、`fixtures/` 路径触发，改文档不跑。在第一份真实实现落地前，工作流检测到 `ts/src/` 无 TypeScript 源码即跳过，不制造"通过"假象。

第一个真实测试来自第 03 章的只读工具验收。

### D5 修订（2026-09-11）：首批测试提前到第 01 章

原条目写"第一个真实测试来自第 03 章的只读工具验收"，与 D3 不一致——D3 要求 transport 层在第 01 章就位、测试全程走录制样本不触网，那一层落地时就已经可测，没有理由空到第 03 章。

第 01 章交付 `ts/tests/` 下的三个文件：配置读取的缺失与格式错误、一次模型调用的成功路径与四类失败（网络、非 2xx、超时、响应形状）、transport 层两种实现各自的行为。输入取自本地桩与回环地址上的临时服务，不访问外网。

工作流本身不改：`ts/src` 有 TypeScript 源码即运行，条件从第 01 章起自然成立。第 03 章仍是 `fixtures/cases/` 驱动的验收用例的起点，改变的只是"第一个测试"的位置。

### D5 补充（2026-09-12）：录制与回放测试补齐

当前 `ts/tests/` 共五个测试文件。在上述三个文件之外，`record.test.ts` 检查录制命令的参数与 provider 名称推导，`recorded.test.ts` 检查两份历史录制的回放与样本脱敏。测试输入增加了 `fixtures/` 中的录制，仍不访问外网。

## 推进顺序

### 阅读 0 的双语言预备练习（2026-09-12）

阅读 0 分为公共导读、TypeScript 版与 Rust 版；两版使用同一个本地响应解析任务和相同的基础验收条件。TS 练习位于 `ts/examples/reading-00/`，Rust 练习位于 `rust/examples/reading-00/`，都与正式 Harness 实现分开。

这是 D2 中“推迟建立 Rust Cargo manifest”的预备教学例外：仅在独立练习目录建立 Cargo 项目，`rust/` 根目录的正式工程仍待后续章节建设。阅读 0 的教学数据不作为 D4 中待提炼的运行时合同。TS 与 Rust 两种实现路线分别标注材料和代码状态，后续章节范围随交付更新。

### 主线顺序

绪论最后写——它需要成品演示才写得出。

```
ts/ 工程初始化 → 01 章 → 阅读材料 0 → 02 章 → 03 章（首批 fixture 与测试）
→ 小结 / v0.1 → 回头补 00 章
```

## D6　章节组织修订（2026-09-13），共同章节与双语言实现

读者按同一个 Rein 项目的能力递进阅读，保留 00–16 章、阅读材料、阶段汇总和附录。每个实现型主题使用公共导读和已交付的语言版，统一知识点锚点、练习编号与验收问题。概念为主的章节不机械拆成多篇。

本条修订旧有「TS 主线 + Rust 对照附录」的阅读定位。TS 与 Rust 都是实现路线，完成进度分别记录；目前阅读 0 两版有独立练习，第 01 章仅有 TS 正式调用。此次内容调整没有新增 Rust 正式工程，也不冻结 D4 留到第 04 章提炼的运行时合同。

`docs/chapters/01.md` 保留为公共导读，原 TS 操作与代码讲解移入 `01-ts.md`，旧小节链接在公共页提供对应入口。阅读 0 保留既有 URL，调整内容顺序并统一对照小节。侧栏每主题一个入口，语言选择只进入已提供版本；缺失版本明确提示，不创建伪实现页面。

各章必需的语言知识进入对应正文，A.1–A.5 改为实现对照与深入讨论。附录当前仍为规划，不将重命名记为正文完成。历史工程决策和已存在的 `ch01` tag 均保留，此次只追加修订。

## D7　阅读材料是按需补充，主线章节承载主线任务（2026-09-13）

阅读材料用于补充正文中可能遗漏、或读者可能尚未掌握的知识。它们不是独立任务、前置关卡或主线验收；正文各章保持连续推进，遇到知识缺口时再链接到相应阅读材料。读者默认从第 01 章开始，读完阅读 0 或完成其中的示例都不能成为进入正文的条件。

据此修订将阅读材料列为主线必经步骤的推进顺序：主线按第 01 章 → 第 02 章 → 第 03 章 → 阶段汇总连续推进，知识补充按需穿插。阅读 0 的公共、TS 与 Rust 页面改为按需知识补充，现有小例子和练习保留为可选自测。阅读材料 1–3 只补充对应主题的边界知识；核心工程机制仍由主线章节推导、实现和验收。阅读材料的可选练习不计入项目交付或主线验收。已交付状态保持不变，Rust 正式实现仍未提供。

## D8　第一章从 SDK 的 hello 开始（2026-09-13）

第 01 章改为「HelloWorld——从模型调用开始」。主线按环境与源码准备、API key、最简 SDK 调用、网络视角的逐步错误处理、结果记录、课后练习推进。TypeScript 与 Rust 共用任务、锚点和练习编号；主线只发一条非流式 `user` 消息，默认输入 `hello`，关闭自动重试并设置等待上限。

本条修订 D2 的 Rust 正式工程延后安排，在 `rust/` 建立 Cargo 工程；修订 D3 的第一章教学顺序，手写 transport 与录制回放保留为延伸材料，不再成为第一次调用的前置步骤。TS 采用官方 `openai` SDK，Rust 采用社区 `async-openai` SDK，依赖版本由各自锁文件固定。原 TS 手写实现与既有录制保留，不以新 SDK 入口假冒已经迁移所有调用方。

本章最低程序验收是取得非空回答并正常退出；读者另行确认是否回应了问候。SDK 失败、本地配置、网络与超时、服务拒绝、响应格式分别解释；HTTP 429 需进一步区分速率与额度，超时不证明服务端未执行。真实调用与本地测试分别记证据。

旧 `ch01` 标签保持不变。新版快照名为 `ch01-helloworld`，只在审查与本地验证后建立。未推送期间须在正文说明新克隆远程仓库不能获取本地标签，不能把本地完成写成 Pages 已发布。

## 职责边界同步（2026-10-04）

状态：按用户本轮明确的三方分工同步文档与接口候选；没有迁移源码或实现新运行能力。采用日期标识，避免与在途 PR #3 中已有 D9–D18 编号冲突。

### 审查依据与现有接口

| 仓库 / 固定版本 | 已存在的能力与限制 |
| --- | --- |
| Rein main `93fd7203428962ae741cc86b8d7e87657341df64` | `python/hello_world/core.py` 的 `diagnose / dispatch_tool / apply_candidate / check_cpp`；Python CLI 的五个子命令；`python/part1/API-CONTRACT.md`；TS 的 `chat / requestHello` 与 transport；Rust 的 `chat / chat_with_timeout`。这些是教学接口，没有统一产品 runtime API |
| Rein [PR #3](https://github.com/JadeSnow7/Rein/pull/3) `44e3454337c06f53f13880a2fe9e35336a387812` | `core/`、`runtime/`、四份生成 schema、R1a CLI；`HarnessStep::advance`、`Runtime::start / inspect / cancel / recover_explicitly` 和 `ArtifactRef` 可作为适配基础。源码已有固定 verifier，未实现通用控制协议、Coordinator 或 DAG |
| Veriflow main `599c380a8dfeaadeff3a4d542b34071800996a29` | EDD skill、schema 1.1 的 `validate_task.py`、模板及只读门槛校验；没有可运行调度器 |
| Veriflow [PR #3](https://github.com/JadeSnow7/Veriflow/pull/3) `422f012822e91c0b7b34f87f7ed462698965e04a` | 新 skill 入口、schema 1.3 的 Spec 绑定、执行记录器及整合辅助工具；属于未合并能力，不能记作 main 交付或 RuntimePort 的实现 |

两仓 main 与审查的在途树均未提供生效的根 AGENTS.md。Rein 已有 DECISIONS.md；Veriflow main 没有根 DECISIONS / ADR。本轮为后者增加对应决定。Web Studio 只定义协作边界，没有审查或修改其实现，不能据此宣称其 provider 已可用。

### 当前职责决定

- **Rein 保留 runtime primitive**：单 Agent 模型适配与上下文、工具声明/执行/反馈、单次执行的状态转换、权限执行与批准绑定、局部预算/截止/取消、会话检查点与恢复、事件顺序及原始 artifact / effect 回执。它不决定任务依赖、选择其他 Agent、统筹全局预算或安排业务重试。
- **Veriflow 拥有 workflow 与整体验收**：AcceptanceContract、TaskGraph、AttemptAssignment、VerificationPlan / Receipt、FailureDiagnosis / RepairRequest、EvidenceBundle / ReviewDecision。失败是否修复、换 Agent 或终止由此层决定；Rein 只执行已经提交的有界尝试。
- **Web Studio 拥有环境与观测**：工作空间、浏览器/终端、CDP、页面操作、截图/日志/状态采样、预览、diff 和人工审阅交互。通用工具调用留在 Rein，Web 能力以 provider 提供；Web Studio 不是 Harness 或工作流状态权威。
- **验证分工**：Rein 可执行固定检查、保存事实回执并报告局部验证状态；Veriflow 选择检查、绑定 Spec/候选/环境、判断证据是否仍有效并聚合整体验收。Runtime 的 `Accepted` 不能直接映射成 workflow `accepted`。目标项目仍拥有业务契约。

连接采用 [RuntimePort 0.1 候选](contracts/runtime-port-v0.1.md)。Veriflow 保存 workflow/task/attempt 的依赖与分派，Rein 保存 run/session/effect 的执行事实；二者以 ID 和不可变摘要关联，不共享可写数据库。上下文由 Veriflow 交接任务目标与已验证依赖，Rein 构造模型上下文；Web Studio 提供环境句柄与原始观测。

### 对在途设计的修订与合并规则

PR #3 的 D12–D14、README、`REIN-DESIGN.md`、`REIN-MODULES.md`、`REIN-FILE-TREE.md`、`REIN-INTEGRATION-SPEC.md` 与 `REIN-IMPLEMENTATION-PLAN.md` 仍按“Rein 拥有 Coordinator / DAG / Acceptance、Veriflow 仅提供方法与投影”规划。本条替代其中的职责归属，不改写这些历史来源、不声称已经移动模块。

本轮从 main 建文档分支，不合并或改写 PR #3。合并两条分支时保留本条和 README 的当前边界，把上述设计里的 Coordinator、任务依赖、全局预算、修复策略、EvidenceBundle 和整体验收改由 Veriflow 拥有；R1a 固定 verifier、底层 effect 恢复与局部结果仍在 Rein。旧 `task.submit/run` 控制草案里的任务图归 Veriflow；单次运行经 RuntimePort 适配，不能维护两套互相独立的 Task/Acceptance 状态机。

### 后续实现切片（均待实现、待验收）

| 切片 | 最小改动 | 完成条件 |
| --- | --- | --- |
| R0：整合在途文档 | 在 PR #3 合并时同步旧设计的归属；保留现有源码目录与历史教学接口 | README、决定和设计没有相反分工；公开状态仍区分已实现 / 在途 / 计划 |
| R1：只读 RuntimePort adapter | 基于 R1a 的单会话 primitive，补版本/能力查询、start/get、ID 绑定与幂等请求 | 同请求重发只有一个 run；同键不同输入拒绝；不读写 Veriflow 数据库 |
| R2：事件与恢复 | 只读事件游标、artifact 校验、cancel / resume 与未知结果查询 | 重复事件不重复推进；取消后无新派发；未知 effect 不盲重放；摘要不符拒绝 |
| R3：权限与真实模型 | 模型/provider 适配、执行策略与批准回执逐项接入；不支持的能力明确拒绝 | 越权/过期批准拒绝；预算耗尽可区分；一个真实 Agent 纵切独立验证 |

Veriflow 先用串行的两个依赖任务消费 R1/R2，再增加跨 Agent 与修复调度；Web Studio 的 provider 与 UI 接入另行验收。无需先建设 daemon、通用项目管理平台或迁移所有教学代码。

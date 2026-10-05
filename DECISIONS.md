# 工程决策记录

本文件记录影响仓库长期结构的决定。正文写作过程中如需推翻，请在此追加修订而非直接改写历史条目。

当前跨项目分工见 [D19](#d19)；D9–D18 保留历史原文，具体取代范围及未决问题由 D19 说明。

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

### D6 修订（2026-09-14）：第 01 章 TypeScript 规范入口

根据用户明确指定，手工修订的 `docs/chapters/01.md` 作为第 01 章 TypeScript 正文 canonical 入口；`docs/chapters/01-ts.md` 收敛为保留旧锚点的兼容页，Rust 正文入口保持不变。语言切换仅使用两版实际共有的六个锚点：`from-reading-0`、`first-run`、`live-call`、`request-response`、`implementation`、`failures`；旧版 TS 独有锚点不再跨语言跳转。

## D7　阅读材料是按需补充，主线章节承载主线任务（2026-09-13）

阅读材料用于补充正文中可能遗漏、或读者可能尚未掌握的知识。它们不是独立任务、前置关卡或主线验收；正文各章保持连续推进，遇到知识缺口时再链接到相应阅读材料。读者默认从第 01 章开始，读完阅读 0 或完成其中的示例都不能成为进入正文的条件。

据此修订将阅读材料列为主线必经步骤的推进顺序：主线按第 01 章 → 第 02 章 → 第 03 章 → 阶段汇总连续推进，知识补充按需穿插。阅读 0 的公共、TS 与 Rust 页面改为按需知识补充，现有小例子和练习保留为可选自测。阅读材料 1–3 只补充对应主题的边界知识；核心工程机制仍由主线章节推导、实现和验收。阅读材料的可选练习不计入项目交付或主线验收。已交付状态保持不变，Rust 正式实现仍未提供。

## D8　第一章从 SDK 的 hello 开始（2026-09-13）

第 01 章改为「HelloWorld——从模型调用开始」。主线按环境与源码准备、API key、最简 SDK 调用、网络视角的逐步错误处理、结果记录、课后练习推进。TypeScript 与 Rust 共用任务、锚点和练习编号；主线只发一条非流式 `user` 消息，默认输入 `hello`，关闭自动重试并设置等待上限。

本条修订 D2 的 Rust 正式工程延后安排，在 `rust/` 建立 Cargo 工程；修订 D3 的第一章教学顺序，手写 transport 与录制回放保留为延伸材料，不再成为第一次调用的前置步骤。TS 采用官方 `openai` SDK，Rust 采用社区 `async-openai` SDK，依赖版本由各自锁文件固定。原 TS 手写实现与既有录制保留，不以新 SDK 入口假冒已经迁移所有调用方。

本章最低程序验收是取得非空回答并正常退出；读者另行确认是否回应了问候。SDK 失败、本地配置、网络与超时、服务拒绝、响应格式分别解释；HTTP 429 需进一步区分速率与额度，超时不证明服务端未执行。真实调用与本地测试分别记证据。

旧 `ch01` 标签保持不变。新版快照名为 `ch01-helloworld`，只在审查与本地验证后建立。未推送期间须在正文说明新克隆远程仓库不能获取本地标签，不能把本地完成写成 Pages 已发布。

## D9　Rust core 与 TypeScript 扩展（2026-09-14）

后续主线以 Rust core 作为唯一权威运行时；当前已确定由它持有 session、task、request/call 身份、loop、budget、dispatch policy 与 evidence，approval、journal、recovery 是后续实现时仍归 core 的状态边界。TypeScript 承担宿主、SDK 与领域集成，连接、缓存和临时 handles 不取得 core 权威。跨进程扩展只采用小型、语言中立的协议。第 01–03 章保持 TypeScript 快速入门，Rust hello 作为可选准备；第 04 章只推导 provider adapter contract，并与后续 plugin stdio 明确区分。第 05–07 章转入 Rust core 的增量能力与 TypeScript plugin，第 08–14 章沿用同一混合路线，第 15–16 章再深入 plugin、MCP 与 hooks。

这条决定取代 D6 中“每个实现型主题都提供 TS 与 Rust 完整版本”的后续承诺，也修正 D1/D2 对后续完整双路线的假设；原因是维护两个权威核心会产生同步负担，而 TS 生态工具仍应保留。历史正文、快照、标签和旧入口继续保留，作为可恢复的历史对照。插件连接与缓存可以接入，但不能取代 core；受信任的首方 Node 进程仍不等于沙箱。已派发调用结果未知时不自动重放，不把成功输出当作任务证明；批准、journal、恢复和委派在尚未实现前只记录为未来能力。


## D10　章节顺序、主题身份与快照分离（2026-09-17）

用户已确认全书 v2：五篇、00–24 共25章。04 移至第二篇，汇总1位于03后，适配器检查进入09后的汇总2；第五篇为“可选能力分支与结项”。名称继续使用 Rein。

`book/chapters.json` 是新版顺序、主题、导航、开放属性和交付状态的唯一索引。`order` 是排版顺序，`slug` 是稳定身份，标题单独维护。新正文使用 `docs/chapters/<slug>.md`，新快照使用 `chapter-snapshots/<slug>/v1/`，后续修订使用新版本，不依赖 latest。未冻结的输入输出快照保持 null，不因目录就绪声明章节验收。

本决定修订 D1 的未来快照命名和 D6 的新版正文入口。未来经另行授权创建的标签采用 `chapter/<slug>/v1`；不再创建数字章号系列标签。本次实施不创建任何 Git 标签。D8 历史快照继续保留；D9 Rust core/TypeScript 宿主分工继续有效。旧决策中的章号按旧主题解释。

旧数字 URL、锚点、标签和归档保持原义，页面提供主题迁移说明，不把旧同号入口映射到新同号的不同主题。概念章可共享代码快照。顺序主干输入须来自前章输出；24 的基础输入来自17，18–23为可选分支。

开放安排：00–03、05、14及汇总1永久免费；阅读0–3永久免费；其余新增单元及四策略独立实验限时免费。免费承诺按内容追溯，14必须容纳原11的独立解释与案例，不能仅指向受限章节。

## D11　Rein 英文全名（2026-09-20）

用户明确采用 **REIN — Runtime for Emergent Intelligence Networks** 作为英文全名。产品简称继续使用 Rein，正式全名展示使用 REIN。

命名用于产品设计与后续运行时入口；书名、历史章节、包名、协议标识、URL 和快照保持原义，不因本次命名直接迁移。规格与接口设计见 [REIN-DESIGN.md](REIN-DESIGN.md)。该文档中的实现方案与待决事项不因名称确认而全部转为已批准决定。

## D12　Rein core 保留完整 Harness，并可担任调度 Agent（2026-09-20）

用户明确要求保留 Rein core 作为完整 Harness 的能力，并使它能够作为调度 Agent 参与调度 Codex、Claude Code 等外部 Agent。core 的模型调用、会话、上下文、工具循环、预算、批准与证据能力继续作为产品主线；外部 Agent 接入是在此基础上的扩展。

本决定延续 D9 的 Rust 权威运行时与 TypeScript 宿主分工，取代 REIN-DESIGN 0.3、REIN-MODULES 0.1 及当日调研建议中“core 仅做任务控制、内置 Harness 后续另立包”的组织方向。历史研究来源和旧合同保留原义，当前结构以 REIN-DESIGN 0.4 和 REIN-MODULES 0.2 为准。

设计据此区分 core 内部的 Harness、Coordinator 与确定性控制规则：Coordinator 是使用同一 Harness 的 Agent 角色，可以自行调用工具、提出子任务、选择允许的执行资源、观察结果并继续规划；任务状态、授权、预算预留与验收仍由确定性规则提交。AgentMux 翻译外部 Harness 协议，runtime 提供 I/O 和装配。具体模块路径、委派工具、阶段划分和验证场景是配套设计，不因本条而被描述成已实现能力。

现有 loop、ToolExecutor、0.1/0.2 工具协议与受控维护合同采用兼容演进。完整 Harness 的产品化和 Coordinator 的持久委派闭环仍需实施与验证；本次只确认架构方向，没有执行源码迁移或真实 Agent 调度。

## D13　原生 Agent loop 采用可恢复步进状态机（2026-09-20）

来源：本次设计评审提出，用户要求将评审结论写入文档。**本决定不改变 D12 的完整 Harness、Coordinator 与三包方向；以下机制尚未实施。** D12 所述 MODULES 0.2 是当轮版本，本次机制以 [REIN-MODULES.md](REIN-MODULES.md) 0.3-draft 和 [REIN-FILE-TREE.md](REIN-FILE-TREE.md) 0.2-draft 为准。

`core/harness` 的 Agent loop 定义为纯步进状态机：输入会话状态和一个模型 turn、工具结果、子任务结果、取消或超时等观察，输出下一意图及新的 session revision。core/harness 不 await 任何 I/O，不持有需要跨重启保存的 Future。`runtime/driver/native.rs` 执行已经持久提交的意图，将结果作为观察送回；每步都通过 TaskApplication 校验并原子提交会话进度、观察去重记录和下一效果意图，提交失败不派发。

`agents.await` 使父会话进入持久等待，提交后释放活跃模型槽；重启从检查点、委派句柄及结果游标继续同一 Session/Attempt，不序列化 Future、不重复委派，也不自动释放仍被实际效果占用的锁或 lease。消息体与工具结果按摘要保存在 runtime/artifact，会话检查点保存引用、游标、revision 和必要控制元数据。

现有 `rust/src/rein/loop.rs` 的 async 循环采用兼容改造，目标为 core 纯步进机 + runtime driver；旧入口暂由兼容桥保持命令和行为，退出要求见 [MIGRATIONS.md](MIGRATIONS.md#rein-core-bridge)。步进机制本身不保证外部 exactly-once，未知派发仍须核对。合同及正式 DTO 生成方向最迟 R0 冻结，源码改造在 R1a 按原基准实施与验证；本轮仅修订设计文档。

## D14　产品线分工与首个整合闭环（2026-09-21）

来源：用户提出“Rein 作为基础设施，Web Studio 作为第一验证场景，Veriflow 作为方法论”，并要求按“需求分析—规格拆解—基准先行—约束实现—可信验证—规范分发”修订现有计划。该分工与路径作为产品线依据；接口、样例和阶段机制由配套设计展开，不表示运行能力已经交付。

沿用 D12 完整 Harness/Coordinator 和 D13 纯步进机 + driver。Rein 拥有运行状态、执行约束与 Acceptance；Veriflow 拥有方法和规则评估；Web Studio 提供任务交互与证据消费。目标项目的 Spec 和公开接口继续是业务权威。用三者开发 Web Studio 的工程验证与从 Web Studio 使用 Rein 的产品验证分别验收。

实施计划将 Web Studio 最小客户端提前为 R1c，在多资源 R2 和完整 DAG R3 之前形成产品纵切；客户端可在 R1a 控制/证据合同稳定后与 R1b 首次真实委派并行实施。完整 P0 仍按 S01–S12、S15/S16 验收，R1c 不代表双账号并发或 DAG 已完成。R1c 先验任务变更与证据交付，R4 再验产品安装与正式发行。

跨项目要求、Veriflow adapter 语义及首个网页样例的基准集中在 [REIN-INTEGRATION-SPEC.md](REIN-INTEGRATION-SPEC.md)。设计更新为 DESIGN 0.5 / MODULES 0.4 / FILE-TREE 0.3；这批修改仅更新计划和规格，不修改三个项目的运行时代码、不执行跨仓回写或发行，也不把历史测试刷新为当前通过。

## D15　统一 Apache-2.0 并开始首批实现（2026-09-21）

来源：用户明确要求“使用子代理开始实现，统一采用 Apache-2 协议开源”。当前产品线自有成果采用 SPDX 标识 `Apache-2.0`。Rein 补齐根 LICENSE、包元数据与当前教材开放声明；Web Studio 和 Veriflow 的根许可已是 Apache-2.0，本轮只读核对，不改写其第三方条款。

Rein 当前自有代码、文档和教材永久开放，取代 D10 对当前内容的限时免费安排。历史决策、冻结原稿、快照及回执保留原义；冻结稿中的旧标签以当前开放说明和页面附注解释，不以本次许可变更伪造旧正文或旧验收状态。第三方依赖和引入材料仍遵循各自许可。

开始 R0/R1a 第一个原生只读增量，实施合同见 [R1A-SLICE-1](records/REIN-RUNTIME-R1A-20260921/implementation-spec.md)。实现由 coder 负责，主线程维护合同并审查基准、差异和原始结果。该增量不取代完整 R1a 的兼容桥、真实模型、写入授权、工作区和控制服务要求；R1b/R1c 及产品发行继续按实施计划推进。本条记录实施授权与范围，不构成实现完成或正式发布的证明。

## D16　设计决策复审与连续演化评测（2026-09-22）

来源：用户讨论 AI 架构设计和长期维护能力不足后，确认开始补充设计决策及其失效机制、连续演化评测。首批范围见 [EVOLUTION-SLICE-1](records/REIN-EVOLUTION-20260922/implementation-spec.md)，执行状态见 [任务摘要](records/REIN-EVOLUTION-20260922/task-summary.md)。

采用独立的本地开发工具验证机制，保持 D12–D14 的产品边界。`architecture/decisions.json` 记录当前被纳入检查的决策引用、成立前提和复审条件；本文件继续保留决定的历史来源。文件前提变化只产生复审请求，缺失依据保持无法确认；检查器不能自动更新基线或批准新的决定。摘要匹配不证明架构正确，也不能替代业务取舍的审查。

连续演化评测要求同一轨迹逐轮继承实际产物，保留仍适用的历史验证项，并以明确理由记录需求替代。独立验证结果、原始进程记录、文件摘要和回归事件共同支持评分；适配器退出成功不能直接成为任务通过。先用正确与故意回归的离线适配器校准评分器，再开展模型、工具、预算一致的真实比较。离线校准不构成 AI 效果证据。

工具分别位于 [scripts/architecture](scripts/architecture/README.md) 和 [scripts/evolution](scripts/evolution/README.md)，第 18、24 章提供对应教学增量。此阶段不接管 runtime 数据库、验收或调度状态，不扩张 R1a 的完成含义，也不改变章节索引与历史快照的验收状态。后续若接入 ContextBundle 或 Veriflow adapter，须先固定字段合同和运行边界，再按同一基准检验。

## D17　六部分现行书稿与第四章定位（2026-09-23）

用户要求以 [六部分书稿框架](book/framework-six-parts/README.md)组织 Rein，并将第 04 章定为“Agent 的核心能力”。现行正式清单为 `book/chapters.json` 的六部分、00–29 共 30 章；D10 的五篇 25 章顺序及 D3、D4、D9 中的旧 04 主题按历史版本解释，清单原貌存于 `book/history/chapters-v2.json`。旧数字 URL、旧主题 slug、作者稿、快照和执行证据继续保持原义，由历史入口访问，不占现行目录。

04 先分析代表性编码 Agent 的公开工具、权限、状态与交付机制，再以第一部分 `python/hello_world/` 的既有行为规划 Rust Harness core 的等价迁移。本章增加架构表达和 Rust 实现路线，不增加读者可见任务功能。第一部分已经允许用户接受后单文件写入与编译验证；通用权限、持久批准、多文件事务和恢复仍属后续章节。05 统一模型消息与工具协议，06–08 再分别加入循环、停止、取消和观察。

从 04 起，每章正式正文末尾附推荐提示词模板，明确仅供参考、不代表实际使用的提示词；实际开发提示词和效果只能由独立运行记录说明。04 目前是章节规划，Rust 等价迁移尚未实施或验收；05–29 的现行入口为规划页，不能继承旧五篇的 `draft`、测试或快照状态。详见[本轮同步规格](reports/2026-09-23-six-part-ch04-sync.md)。

## D18　书名、人机分工与提示词来源（2026-09-23）

用户确认书名改为《Rein：从零构建 Agent》，不再使用“从零手写一个 Agent Harness”。Harness 仍是全书核心概念，保留在副标题、首页说明和正文定义中。

第一至五部分采用“人类确定架构和设计，AI 编写代码”的分工：人负责任务与验收条件、边界、接口与数据形状、状态设计、验收样本、审查结论和是否合入；AI 编程助手按设计实现代码并补写测试代码；程序执行验收。什么算通过由人决定。第六部分才研究把设计也交给 Agent 的完全 vibe coding，考察其前提与效果。

读者参与分三段递进：01–03 只需知道配套程序按怎样的设计写成，每章以“本章设计”小框展示，不要求细读契约，也不要求使用 AI 编程助手；04–24 读者参与设计卡和验收样本，并用 Claude Code、Codex 等代码助手实现、审查和验收；25–29 研究交出设计需要的条件。04 对代码助手用法只作简短引导，提示词工程、防御性编程及 AI 痛点在后续相应章节展开。

本条修订 D17 的提示词说明：仍从 04 起附提示词，但不再笼统声明“不代表实际使用”，而是逐条标注来源——“作者实际使用”须链接开发记录，“据设计反推”须说明依据并注明未经运行，“已试用”须记录起始快照、工具与模型、实际 diff 和验收结果。04 只迁移不依赖模型协议的部分（受限读取、差异、确认写入与编译检查），模型适配放到 05 及以后。

## D19　任务权威外移：Veriflow 持有 Workflow/Task，Rein 持有 Session/Run/Effect（2026-10-02 拟议，2026-10-04 确认）

<a id="d19"></a>

来源：2026-10-02 边界审计及拟议 D19、用户在 2026-10-04 确认的三方分工，以及原 [PR #4](https://github.com/JadeSnow7/Rein/pull/4) 的职责决定。本条合并两份表述，状态为**已确认的职责决定**；未决接口与实施边界单列，不由本文补定。审计来源见[边界审计复核](reports/2026-10-02-boundary-audit-review.md)。文档确认不表示相关运行能力已实现。

### 当前职责与状态权威

| 对象 | 权威 | 边界 |
| --- | --- | --- |
| Workflow、Task、任务图、AttemptAssignment、跨运行修复策略与总预算、整体验收与最终接受 | Veriflow | 选择执行资源、提交有界尝试、判断是否修复、换 Agent 或终止；持有 AcceptanceDecision |
| Session、Run、Effect、尝试与运行的绑定、运行内预算、动作授权、执行检查点与恢复 | Rein | 单 Agent 模型上下文、工具声明/执行/反馈、局部截止与取消、事件顺序及原始回执；不决定任务依赖、选择其他 Agent、统筹全局预算或安排业务重试 |
| 工作空间、浏览器/终端、CDP、页面操作、截图/日志/状态采样、预览、diff 与人工审阅交互 | Web Studio | 环境与观测的提供方；通用工具调用留在 Rein，Web 能力以 provider 提供；不是 Harness 或 workflow 状态权威 |
| 产品需求、公开接口、业务测试 | 目标项目 | Spec 与公开接口仍为业务权威 |

Rein 可执行固定检查、保存事实回执并报告局部验证状态；Veriflow 选择检查、绑定 Spec/候选/环境、判断证据有效性并聚合整体验收。运行结束、验证通过、最终接受是三个独立状态；Rein 运行终态使用 `finished`，不能据此生成 workflow accepted。现有代码的 `Accepted` 尚待 [RESULT-SPLIT-1](reports/2026-10-02-run-verification-acceptance-split.md) 拆分，不能将目标写成已实现。

动作授权由 Rein 检查并绑定具体 effect、范围与候选版本；产物接受由 Veriflow 决定并绑定 Spec、产物和证据版本，二者不能互相替代。Rein 管本次运行预算，Veriflow 管跨运行累计、依赖和修复次数。Rein 在一次运行内调用检查并根据结果继续行动仍属单次运行能力；跨任务修复与调度归 Veriflow。工具层委派的未决边界见问题 1。

连接采用 [RuntimePort 0.1 候选](contracts/runtime-port-v0.1.md)。Veriflow 保存 workflow/task/attempt 的依赖与分派，Rein 保存 run/session/effect 的执行事实；二者以 ID 与不可变摘要关联，不共享可写数据库。Veriflow 交接任务目标与已验证依赖，Rein 构造模型上下文，Web Studio 提供受信环境句柄与原始观测。Rein core 不依赖 Veriflow 或 Web Studio；独立 CLI 的具体终点与输出合同见问题 2。

### 取代范围与历史保留

以下保留 2026-10-02 拟议稿的取代范围原文，明确哪些历史条款需要追溯、哪些机制不受影响：

取代范围限定如下，其余原文保留。D12 中“core 保留完整 Harness、可作为调度 Agent 委派外部 Agent”继续有效；被取代的是第三段“任务状态、授权、预算预留与验收仍由确定性规则提交”里**任务状态与验收**归 core 的部分，动作授权与运行内预算仍归 Rein。D14 中“Rein 拥有运行状态、执行约束与 Acceptance；Veriflow 拥有方法和规则评估”改为 Rein 拥有运行状态与执行约束，Veriflow 拥有方法、规则评估、任务状态与 Acceptance；Web Studio 作为第一验证场景、目标项目 Spec 是业务权威、两条验证线分别验收、R1c 提前等内容不变。D9 的 Rust 权威运行时、D13 的纯步进机与 driver、D15–D18 不受影响；D13 文中的 TaskApplication 作为 Session/Run 级的统一提交入口保留，是否改名另定。

其中“可作为调度 Agent 委派外部 Agent”的工具层含义，PR #4 未给出完整答案，不能据此重新授权 Rein 持有全局 Coordinator、DAG 或选择其他 Agent；已确认的当前职责以上表及 PR #4 为准，尚需澄清的委派句柄、父子等待和旧句子的适用范围列入问题 1。本次不修改 D9–D18 原文，不在未决处新增实施决定。

### 原五个问题：PR #4 的回答与未决项

以下引文均来自原 PR #4 的固定提交 [`cf62ae8`](https://github.com/JadeSnow7/Rein/blob/cf62ae8/DECISIONS.md)，引文是设计依据，不是实现证据。

#### 问题 1：工具层委派是否留在 Rein，全局任务图是否归 Veriflow？

原句：“它不决定任务依赖、选择其他 Agent、统筹全局预算或安排业务重试。”另有：“把上述设计里的 Coordinator、任务依赖、全局预算、修复策略、EvidenceBundle 和整体验收改由 Veriflow 拥有”。

已回答：全局任务图、Coordinator 与跨 Agent 选择归 Veriflow。**未决**：拟议稿中“工具层委派留在 Rein”的具体边界、委派句柄与父子等待如何归属，以及取代范围中保留的 D12“调度 Agent”旧句应如何限定。PR #4 没有逐项定义，不能把旧句保留当作这些细节已获批准。

#### 问题 2：没有 Veriflow 时，Rein CLI 是否只报告运行结果和可选验证结果，不提供接受语义？

原句：“Rein 可执行固定检查、保存事实回执并报告局部验证状态；Veriflow 选择检查、绑定 Spec/候选/环境、判断证据是否仍有效并聚合整体验收。”另有：“Runtime 的 `Accepted` 不能直接映射成 workflow `accepted`。”

已回答：局部检查结果与整体验收分属两层，Rein 不产生 workflow 接受结论。**未决**：PR #4 没有直接规定独立 CLI 的终点、命令输出及本地最小 Adapter 合同。RESULT-SPLIT-1 是另一个待实施规格，不能冒充 PR #4 已回答这些 CLI 细节。

#### 问题 3：Veriflow 的任务状态怎样持久化？

原句：“Veriflow 保存 workflow/task/attempt 的依赖与分派，Rein 保存 run/session/effect 的执行事实；二者以 ID 和不可变摘要关联，不共享可写数据库。”

已回答：保存责任与跨层关联方式。**未决**：任务状态的具体持久化形式、存储布局与可运行状态机；PR #4 的固定版本表只列规则、记录器与整合工具，没有给出这些实现。不得据此选定数据库或新增调度器。

#### 问题 4：Rein 的 RunResult 是否成为 Veriflow 任务状态的只读输入？

原句：“不能维护两套互相独立的 Task/Acceptance 状态机。”结合问题 3 的“不共享可写数据库”，以及“Rein 只执行已经提交的有界尝试”。

已回答：Veriflow 持有任务与接受权威，消费 Rein 的运行事实；不能通过投影双向独立修改 Rein 会话状态或另立一套任务接受权威。RunResult 的具体可用类型与事件/回执映射仍以 RESULT-SPLIT-1 和 RuntimePort 后续实现为准，本文不声明该只读接入已经存在。

#### 问题 5：原 R1b、R3 的 Coordinator 持久委派、DAG、有界修复在哪里实施？

原句：“把上述设计里的 Coordinator、任务依赖、全局预算、修复策略、EvidenceBundle 和整体验收改由 Veriflow 拥有；R1a 固定 verifier、底层 effect 恢复与局部结果仍在 Rein。”

已回答：全局 Coordinator、DAG 与跨任务有界修复按 Veriflow 路线实施，不先在 Rein 建成后再迁出；Rein 保留单次运行与底层 effect 恢复。涉及单次运行的工具层委派细节仍受问题 1 的未决项约束，不据旧里程碑扩张职责。

### Web Studio 尚未定义

- 任务展示订阅 Rein 还是 Veriflow。
- provider 的形状与版本。

这里只登记问题，不选择订阅来源、不补造 provider 接口；Web Studio 现有交互不能作为接口已经可用的证明。

### 固定审查依据与合并后状态

| 仓库 / 固定版本 | 已存在的能力与限制 |
| --- | --- |
| Rein main `93fd7203428962ae741cc86b8d7e87657341df64` | `python/hello_world/core.py` 的 `diagnose / dispatch_tool / apply_candidate / check_cpp`；Python CLI 的五个子命令；`python/part1/API-CONTRACT.md`；TS 的 `chat / requestHello` 与 transport；Rust 的 `chat / chat_with_timeout`。这些是教学接口，没有统一产品 runtime API |
| Rein [PR #3](https://github.com/JadeSnow7/Rein/pull/3) `44e3454337c06f53f13880a2fe9e35336a387812` | `core/`、`runtime/`、四份生成 schema、R1a CLI；`HarnessStep::advance`、`Runtime::start / inspect / cancel / recover_explicitly` 和 `ArtifactRef` 可作为适配基础。源码已有固定 verifier，未实现通用控制协议、Coordinator 或 DAG |
| Veriflow main `599c380a8dfeaadeff3a4d542b34071800996a29` | EDD skill、schema 1.1 的 `validate_task.py`、模板及只读门槛校验；没有可运行调度器 |
| Veriflow [PR #3](https://github.com/JadeSnow7/Veriflow/pull/3) `422f012822e91c0b7b34f87f7ed462698965e04a` | 新 skill 入口、schema 1.3 的 Spec 绑定、执行记录器及整合辅助工具；属于未合并能力，不能记作 main 交付或 RuntimePort 的实现 |

两仓 main 与审查的在途树均未提供生效的根 AGENTS.md。Rein 已有 DECISIONS.md；Veriflow main 没有根 DECISIONS / ADR。本轮为后者增加对应决定。Web Studio 只定义协作边界，没有审查或修改其实现，不能据此宣称其 provider 已可用。

以上表格及其说明保留的是 **2026-10-04 审查时点**，其中“main”“未合并”不是当前状态。2026-10-05，Rein PR #3 已以 `e649708f1f8d68c2c259f150835f54f85126a44f` 合并，R1a 离线运行时与六部分书稿已进入 main；全部现有实现保留。原 PR #4 也已合并，职责内容在本条统一为 D19。

五份 REIN 设计文档入口已标明旧职责被当前决定替代，本次将其引用统一指向 D19；正文仍为历史方案，不是现行跨项目实施合同。RuntimePort 仍是候选，没有新增 adapter 或调度器。旧测试只支持其原固定版本，不刷新为本次能力验收。

### 文档维护范围与后续切片

本次仅统一决定、候选用词、登记及设计入口；下列正文仍待后续修订：`REIN-INTEGRATION-SPEC.md` 第 1、4 节及 INT-S01/INT-S05；`REIN-MODULES.md` 的 F01–F03、F08 与 §4.3；`REIN-DESIGN.md` §8.1、§9.6；`REIN-IMPLEMENTATION-PLAN.md` 的 R1b、R3；`REIN-FILE-TREE.md` 的旧任务图与 Coordinator 布局。章节归属由 `book/chapters.json` 另行决定，本次不改清单。

`architecture/decisions.json` 追加 `DEC-CORE-BOUNDARY-V2`，保留旧登记及摘要原值。顶层 `version: 1` 是现有检查器的文件格式版本，不是 D19 修订号；新增登记不证明候选已经实现。

沿用 PR #4 的 RuntimePort 路线编号：

| 切片 | 范围与状态 | 验收边界 |
| --- | --- | --- |
| R0：职责文档同步 | 当前决定及五份入口已同步；旧设计正文仍待修订 | 不宣称历史正文已全部改写，不移动源码 |
| R1：只读 RuntimePort adapter | 待实现版本/能力查询、start/get、ID 绑定与幂等请求 | 同请求重发只有一个 run，同键不同输入拒绝，不读写 Veriflow 数据库 |
| R2：事件与恢复 | 待实现事件游标、artifact 校验、cancel/resume 与未知结果查询 | 重复事件不重复推进；取消后无新派发；未知 effect 不盲重放；摘要不符拒绝 |
| R3：权限与真实模型 | 待接入模型/provider、执行策略与批准回执 | 越权/过期批准拒绝；预算耗尽可区分；真实 Agent 纵切另行验证 |

Veriflow 可按其后续切片先验证串行依赖任务，再增加跨 Agent 与修复调度；Web Studio provider 与 UI 另行验收。状态拆分可以先行，不以本条未决接口为前提；本次没有实现状态拆分、RuntimePort、跨项目调度或新的教学能力。

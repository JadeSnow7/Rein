# Rein 实施计划

> **现行职责覆盖说明（2026-10-05）**：本文保留旧版设计及历史验收编号。涉及 Coordinator、TaskGraph/DAG、跨 Agent 资源选择、全局预算、修复策略、EvidenceBundle 与整体验收的归属，均由 [D19（2026-10-04 确认）](DECISIONS.md#d19) 替代，现归 Veriflow；本文中 Rein 拥有这些职责、Veriflow 仅作方法/投影的表述不再生效。Rein 仅保留单 Agent 运行、局部执行约束、固定检查与原始回执、effect 恢复；Web Studio 负责环境与观测。跨层连接以 [RuntimePort 候选](contracts/runtime-port-v0.1.md) 为准。旧目录和里程碑是历史方案，后续按新决定的 R1–R3 与 Veriflow V1–V4 推进；本次不搬迁源码，也不宣称候选能力已实现。

**REIN — Runtime for Emergent Intelligence Networks**

| 项目 | 内容 |
| --- | --- |
| 日期 | 2026-09-21 |
| 状态 | 产品线整合计划已修订，R0/R1a 首批增量开始实施；对应 DESIGN `0.5-draft`，各项完成状态以当前证据记录为准 |
| 目标来源 | [REIN-DESIGN.md](REIN-DESIGN.md) `0.5-draft`；跨项目合同与阶段门槛以 [REIN-INTEGRATION-SPEC.md](REIN-INTEGRATION-SPEC.md) 为准 |
| 已确认决定 | [D12、D13、D14、D15](DECISIONS.md)：完整 Harness/Coordinator、纯步进机 + driver、三产品整合边界、Apache-2.0 与首批实施 |
| 核查基线 | HEAD `1dfa82b9d90196c2294f61914d2017449e5548e3` 加已保存快照的当前工作区；历史测试不自动成为当前通过证据 |

## 1. 产品线结论

Rein 是基础设施，负责受约束执行、任务状态、委派、工作区、产物和证据；Web Studio 是首个验证场景，负责用户提交、观察、授权、审阅和交付；Veriflow 是方法论，负责需求、规格、基准、证据和分发门槛。三者通过版本化合同整合，职责和阶段入口/出口不在本文件复制，统一引用 [REIN-INTEGRATION-SPEC.md](REIN-INTEGRATION-SPEC.md) 的 `INT-S01..S08` 与 `INT-B01..B12`。

必须保留两条验证线：

1. 用 Rein + Veriflow 开发和验证 Web Studio，检验基础设施与方法是否能约束产品开发过程。
2. 从 Web Studio 调用 Rein 执行专用网页任务，检验客户端是否能完成一次可观察、可取消、可审阅、有证据的交付。

两条线共享任务、授权、证据和版本合同，但一条线的记录不能自动证明另一条线的产品验收。

## 2. 六阶段合同

流程固定为“需求分析 → 规格拆解（含设计）→ 基准先行 → 约束实现 → 可信验证 → 规范分发”。每阶段的必备产物、进入条件、失败和 `unknown` 处理、证据绑定与阶段门槛以整合 Spec 的 `INT-S01..S08` 为唯一权威；本计划只说明实施顺序。

需求分析冻结目标、场景、范围、权限和未决项；规格拆解同时冻结模块职责、接口、状态和兼容边界；基准先行保存实现前输入、预期和原始结果；约束实现执行固定工作区、能力、预算和授权；可信验证绑定当前 Spec/代码/输入/验证器并保留原始回执；规范分发按里程碑分别核对任务交付包和产品发行包。未知状态先核对，不能按成功或失败推断。

## 3. 架构与现状约束

沿用 D12 的完整 Harness/Coordinator、D13 的纯步进 + driver、三包和旧教学兼容方向：

| 包 | 主责 | 边界 |
| --- | --- | --- |
| `core/` → `rein-core` | Harness 步进机、Coordinator、任务/运行/尝试、准入、预算、状态与 acceptance | 不 await I/O，不依赖具体 CLI、数据库或 OS 实现；只定义端口并产生效果意图 |
| `agentmux/` → `rein-agentmux` | 外部 Harness 协议、能力探测、事件和审批回传 | 不直接写业务状态，不隐藏重试，不自行选择账号 |
| `runtime/` → `rein-runtime` | driver、SQLite、进程、凭证、工作区、产物、验证、控制接口和打包 | 执行 core 已提交意图，回送观察并事务保存状态 |

Rust 类型 → schemars → `contracts/runtime/schemas/*.json` → TypeScript 是权威 schema 生成链路。新根 Cargo workspace 按三个产品包逐步落地并排除旧 `rust/`；章节命令、旧 Cargo.lock 和教学协议继续独立保留，临时兼容桥与退出条件见 [迁移条目](MIGRATIONS.md#rein-core-bridge)。产品 crate、生成工具和持久 runtime 的首批实施范围见本文第 10 节。现有 executor 的源码路径定位、内存事件数组和同步批准回调只能作为迁移输入，不能当作产品分发、持久恢复或授权证据。

### 3.1 当前源码复用与迁移输入

| 当前入口 | 已存在的能力 | 实施时需要的变化 |
| --- | --- | --- |
| [rust/Cargo.toml](rust/Cargo.toml)、[main.rs](rust/src/main.rs) | 包名仍是 `rein-ch01-helloworld`，默认程序执行一次模型问答 | 新建产品 CLI/daemon 入口，保留旧命令与章节使用方式 |
| [loop.rs](rust/src/rein/loop.rs) | async 循环在内存中 await 模型/工具；含局部预算、取消信号和 `LoopResult.events` | 兼容改造为 core/harness/agent_loop.rs 纯步进机 + runtime/driver/native.rs I/O driver；原生与 Coordinator 共用；每步经 TaskApplication 提交引用式会话和 revision |
| [extension02_executor.rs](rust/src/rein/extension02_executor.rs) | 单调用 Node 子进程、协议校验、内存调用记录与批准表 | 维持 Tool Extension 定位；新增产品级进程监督、持久批准与执行核对，不把它改造成通用 Agent 会话 |
| [extension02-host.ts](ts/src/rein/extension02-host.ts) | 每个宿主只执行一次调用后结束；有只读与维护处理器 | 可复用工具语义；需要产品打包、环境 allowlist 和可配置的宿主路径 |
| [maintenance.rs](rust/src/rein/maintenance.rs) | 单个已有普通文件的完整替换、基线哈希、候选批准 | 保留教学合同；新增产品级 ChangeSet、调用者授权和持久批准记录 |
| [maintenance_session.rs](rust/src/rein/maintenance_session.rs) | 同步 `generate`/`approve` 回调、事件数组、有限修复 | 产品层使用可等待的生成/批准流程，等待期间可取消、可持久化、可恢复；不能把长时审批塞进同步回调 |
| [context_methods.rs](rust/src/rein/context_methods.rs) | 有界上下文策略实验与来源记录 | 局部策略归 core/harness/context；跨执行 ContextBundle、来源策略与版本检查归 core/handoff |
| [test.yml](.github/workflows/test.yml) | `rust/`、`ts/` 等路径的 Linux 检查 | 新增产品目录触发规则、合同/迁移/恢复检查；涉及本机凭证和进程行为时增加适用平台验证 |

特别需要处理的工程耦合：当前 executor 从 `CARGO_MANIFEST_DIR` 推导仓库路径，并启动仓库内的 TypeScript 宿主；这适用于教学工程，不能直接作为可分发产品的安装定位。产品启动配置应显式给出已校验的 Agent 可执行文件、宿主资产、状态目录与工作区根。

当前对直接子进程的 `start_kill` / `wait` 处理也不能直接证明完整进程树已回收。产品必须针对实际平台和外部 Agent 的进程结构重新实现并验证退出与恢复语义。

### 3.2 实施前缺口与冻结时点

以下是对草案的具体补充建议，应在相关阶段开始前进入正式合同。它们解决实现分歧，不代替功能交付。

| 项目 | 草案目前的缺口 | 建议补充 | 最迟阶段 |
| --- | --- | --- | --- |
| 阶段验收范围 | P0 包含原生 Harness、Coordinator、双外部 Agent、DAG 和恢复，R1 只覆盖子集 | 给每阶段列出 S/V 编号与未覆盖项；R1 完成不得称完整 P0 完成 | R0 |
| 执行与计费维度 | 已分 executionMode/fundingSource；原生模型与委派合法组合待冻结 | 模块设计明确模型、Harness、资金来源和执行环境分离；实施时补齐组合校验 | R0 |
| 正式 DTO | 多个端口仍只写了返回对象名称，生成链路尚未实施 | 补齐 Run、Attempt、HarnessSession、DelegationLink、CapabilityReport、Profile、Reservation、Approval、Artifact、Evidence、Acceptance 的字段与枚举；Rust 类型为权威，用 schemars 生成 `contracts/runtime/schemas/*.json`，再由 schema 生成 TypeScript 类型；`scripts/runtime/check-contracts.mjs` 只核对一致性，不手写第二套；事件 payload 由 core Rust 类型定义，控制 DTO 引用 | R0 |
| Harness 步进与检查点 | 原 async 循环尚未改造为可恢复步进机 | 按模块设计 5.7 冻结状态+观察→意图+revision、逐步提交/去重/等待恢复；消息体和工具结果进 artifact，会话仅存引用、游标与控制元数据 | R0 |
| Coordinator 回路 | 模型提案、准入、父子等待及结果消费需形成合同 | 冻结委派工具、执行上下文提供的 caller 身份、深度/子预算、结果去重、父取消与等待资源移交 | R0/R1 |
| 状态转换 | 有状态表，但尚无完整命令/事件转换矩阵 | 对每个转换定义前置状态、版本条件、事务写集合、重复请求和迟到事件处理；未知结果只通过核对事件解除 | R1 |
| 凭证所有权 | Profile lease 与凭证 generation 是概念合同 | 明确每种适配器由谁读写/刷新凭证、何时可复制、如何识别实际主体；不支持的模式显式拒绝 | R0/R2 |
| 事务与副作用 | 核心事务包含预留，但文件锁/启动属于外部操作 | 分开数据库逻辑 lease 与 OS 锁/目录准备；明确准备失败的补偿、派发 outbox 和启动不确定窗口 | R1 |
| 取消与恢复 | 支持层级尚未量化 | 能力分别声明请求取消、确认退出、回收后代、原生会话续接、重启后核对；不能用一个 `supportsResume` 概括 | R0/R1 |
| 预算与截止时间 | 已有默认值，缺少账本转换合同 | 明确预留/已用/未知金额、尝试扣减点、跨节点汇总、单调时钟与持久截止时间；未知费用不能按零结算 | R1/R2 |
| 额度观测 | 多窗口与共享池已有模型，来源关系待细化 | 定义 observation 替代规则、重复/过期观测、共享池归属、外部用量与本地预留如何避免重复扣减 | R2 |
| 输入与 ChangeSet | 当前实现只支持单文件替换 | 定义多文件清单、修改/新增/删除的支持集合、未跟踪输入、文件模式、路径策略、部分失败和整合后重验 | R1/R3 |
| 控制协议 | JSON-RPC 仅有方法草案 | 正式 DTO/请求与事件 schema 随权威生成合同在 R0 冻结；framing、握手、版本兼容、最大帧、错误码、事件游标和断线行为在 R1 补齐 | R0/R1 |
| 摘要与证据 | 规定内容摘要但未冻结规范化算法 | 随引用式会话和正式 DTO 固定摘要覆盖字段、对象键顺序、数组顺序、文本编码和排除字段；配跨语言一致性样本 | R0 |
| 权限变化 | 有批准绑定，撤销和运行中策略变化未展开 | 定义何时阻止新派发、是否取消在途动作、旧批准如何失效；输出目录与外发策略分别检查 | R1/R2 |

R0 的适配能力报告应记录具体 Agent/CLI 版本、平台、接入方式和原始探测证据。当前文档没有证明任一目标产品提供了所有所需能力，因此不能把未探测能力预设为支持。状态机和离线合同工作可以先推进；受真实接口限制的保证在探测后冻结。

跨项目新增合同、INT 要求及基准以整合规格为准；上表保留 Rein 原有工程缺口，随 R0/R1a/R1b/R1c 及后续范围分别闭合。

## 4. 改动包与共享状态

改动按纵向闭环穿过多个包，保留既有改动包及完成条件，新增 K/L/M。具体布局以 [模块设计](REIN-MODULES.md) 和 [文件树](REIN-FILE-TREE.md) 为准。

| 改动包 | 新增或调整内容 | 首个可验收结果 |
| --- | --- | --- |
| A：合同与领域 | `contracts/runtime/`、domain 类型、摘要规范、状态转换规则 | 同一输入在 Rust/TS 边界得到相同引用与错误；非法转换被拒绝 |
| B：存储与任务服务 | 数据库迁移、Task/Run/Attempt、事件、幂等键、逻辑 lease、outbox | 重复提交得到同一 Run；重启保留已提交状态；未知启动不会重发 |
| C：原生执行与外部适配 | core Harness 兼容抽取、Model/Tool 端口、NativeAgentAdapter、FakeAgent、首个真实外部适配器 | 无外部 CLI 的原生闭环与外部执行生命周期分别通过 |
| J：Coordinator | 同一 Harness 的角色配置、委派工具、DelegationLink、等待与结果回填 | 原生调度 Agent 发出一次委派、消费真实子结果并继续；重启/重复不重派 |
| D：工作区与产物 | 输入快照、执行副本/工作树、写锁、产物封存 | 用户原工作区保持基线，输出以可校验 ChangeSet 交付 |
| E：验证与批准 | Verifier、Evidence、Acceptance、持久批准请求 | Agent 退出 0 但检查失败时不会验收；审批等待可恢复 |
| F：本地控制接口 | `rein`、`reind`、socket、事件订阅、状态查询 | 客户端断开后任务继续；重连可读取遗漏事件 |
| G：账号与资源 | Profile、凭证后端、资源观测、共享池、预算预留 | 不同 Profile 绑定不串用；未知额度明确显示，未授权 API 不启动 |
| H：调度与任务图 | 就绪队列、规则路由、第二个适配器、DAG、ContextBundle、修复 | 两资源可并发处理独立任务，依赖只消费已验收的指定产物 |
| I：打包与回归 | 宿主资产定位、配置约定、CI 触发、迁移与平台检查 | 脱离源码绝对路径仍可启动；旧教程命令继续通过原基准 |
| K：Veriflow adapter | 固定规则包、schema 1.3 投影、单执行 recorder、gate 原始结果 | 匹配当前版本的证据可被采纳；不可用/过期/失败不能转为通过 |
| L：Web Studio 消费者 | Swift 控制连接、任务交互、预览、diff/证据展示 | INT-S06 原生走查与网页行为分别留证；断线续读不重派 |
| M：任务交付与产品发行 | 变更/证据/manifest、应用回读；后续三产品安装与发行 | R1c 通过 INT-S07；R4 独立通过 INT-S08 |

Rein 数据库是任务状态的唯一写入权威。Veriflow 适配器只生成不可变的 Veriflow 1.3 投影，调用固定版本校验器，保存原始结果，再由 `core/acceptance` 依据当前合同决定状态；`task-state.json` 不能与数据库并行修改同一任务。若 `record_execution.py` 作为 recorder 使用，它是检查命令的单一执行所有者，Rein 不重复执行同一命令后再拼接结果。

授权必须绑定原始用户/调用者上下文、操作范围、基线、候选和消费记录；不能从记录结构、文件存在或“验证通过”反推授权。模型、工具、委派和应用动作分别记录主体与版本。任务交付与产品发行中的实际外部动作核对各自目标与已有授权，缺失时补充相应授权；结果为 `unknown` 时先做外部核对。

## 5. 实施顺序

### R0：冻结最小跨项目合同

锁定最小任务、控制、授权、证据、Veriflow 版本、文件 hash、旧基准和能力探测结果；冻结 Rust 权威 schema、引用式会话/工具检查点、状态转换、摘要算法和兼容范围。保存旧 loop、宿主、维护示例的原始输入与反例基准，证明拆分前行为。

能力探测至少覆盖 Agent/CLI 版本、平台、启动方式、结构化输出、身份核对、配置隔离、取消、额度观测、重启核对和真实进程边界。首个外部 adapter 只要具备推进 R1b 的能力即可进入候选；第二个 adapter 的探测与实现不得阻塞原生 R1a 或 R1c 的最小集成。FakeAgent 用于未启动、正常输出、写入后断连、忽略取消、延迟退出和重复事件，不能替代真实 adapter 验收。

### R1：原生闭环、首次真实委派与最小客户端

**R1a 原生闭环。** 将现有 async loop 兼容抽取为 `core/harness` 纯步进机和 `runtime/driver/native.rs`，接入任务/运行/尝试、独立工作区、产物、固定验证、控制和可恢复持久状态。每步提交会话 revision；消息体和工具结果进入 artifact，会话只保存引用、游标和控制元数据。旧入口按相同基准复跑，但历史结果不自动成为本轮通过证据。

**R1b 首次真实委派。** Coordinator 使用同一 Harness 发出有界委派，接收首个通过 R0 能力探测的真实外部 Agent 结果后继续父任务。必须证明子结果进入父模型下一次请求，并覆盖身份、幂等、等待恢复、预算、取消和重复事件；不能用手动 CLI 或预置文字代替闭环。

**R1c Web Studio 最小客户端。** 在 R2/R3 之前接入 Web Studio。使用专用本地网页 fixture，任务是修复“空白或仅空格输入仍可提交”；Rein 创建隔离 workdir 并管理 loopback dev server，固定浏览器 verifier 执行外部行为检查。Web Studio 至少能提交、观察、取消、回复授权、查看 diff/证据并导出任务交付包。网页在外部浏览器中的行为和 Web Studio 原生 UI 的提交/取消/预览/证据打开分别验收。R1c 依赖 R1a 的控制与证据合同，客户端工作可与 R1b 委派实现并行；集成里程碑结束时必须分别核对两者。

R1a 仍须在无外部 Agent CLI 的环境取得原生模型—工具—反馈闭环和旧基准证据，覆盖 V19；R1b 覆盖 V20–V22 的单子任务范围，以及 V01、V09–V12、V15、V17 中相关部分。INT-B01–INT-B12 只补充跨项目整合验证，不替代这些 Rein 基准。R1 不代表完整 P0；完整范围仍为 DESIGN S01–S12、S15/S16 及适用 V01–V22，包括 Codex/Claude Code 两类外部目标的实际适配验收。

### R2：多 Profile 与资源队列

接入第二个真实 adapter、Profile 与凭证写入者、共享额度池、额度观测版本、父子汇总预算、原子并发预留和已授权 fallback。用户指定、规则选择与 Coordinator 提案走同一准入。R1 可先使用一个已核验 Profile，完整轮换和资源隔离在此扩展；实际身份、发送次数和文件归属需有证据。

### R3：DAG、交接与有界修复

以产物引用表达依赖，输入只能来自匹配 Spec 的有效 Acceptance；加入图校验、就绪计算、依赖失败传播、ContextBundle、整合节点和新 Attempt。保留旧维护示例的有限修复思想，产品层使用剩余预算驱动异步生成/批准的新尝试。完整 P0 的图、并发、恢复和修复范围在此补齐；R1c 最小整合不等待 R2/R3。

### R4：丰富垂直体验与正式发行

扩展 Web Studio 的真实垂直场景、预览、事件时间线、证据浏览和安装后核对；完成 Rein 可脱离源码路径的宿主资产与状态目录定位，Web Studio 签名、公证和安装启动检查，Veriflow 固定规则包与兼容样例。任务交付验收和产品发行验收分开记录。

### R5：优化与维护

在已通过的纵向合同上优化性能、资源路由、并发、可观测性和恢复体验。任何优化先复跑受影响的 `INT-B` 基准；需求、合同或验证器变化时提升版本并标记旧证据失效。

设计与维护的早期工具增量可以先于 R5 独立验证。按 [D16](DECISIONS.md) 和 [EVOLUTION-SLICE-1](records/REIN-EVOLUTION-20260922/implementation-spec.md)，补充文件前提驱动的设计决策复审检查，以及逐轮继承产物的连续演化评分器。工具入口见 [决策检查](scripts/architecture/README.md) 与 [演化评测](scripts/evolution/README.md)；教学入口为第 18、24 章。当前阶段只在独立目录运行，不改 core/runtime 状态合同。

该增量的退出条件是：决策变更/缺失反例可检测，12 轮离线正确/回归轨迹能够校准评分，命令与报告可复跑。真实效果另按第 24 章方案比较普通助手、设计记忆和完整闭环，绑定相同任务序列、模型/工具预算和多次重复，观察功能完成、累计回归、后续变更成本与人工介入。完成离线工具不表示真实比较已执行，不替代 R1–R4 的产品与整合验收。

## 6. 分发与证据

一次任务交付至少包含：变更/ChangeSet、Veriflow evidence、manifest、基线与 hash、验证原始回执、Acceptance 状态及应用后核对。任务交付证明该变更在指定 Spec 和环境下可验收，不证明产品已发行。

产品发行另行形成 Rein 资产与安装路径/升级核对、Web Studio 签名公证安装验证、Veriflow 固定版本规则包与兼容样例。按每项动作的目标、范围和影响核对已有授权，在授权范围内继续执行，缺失时仅补充所需授权；任何外部动作的回执为 `unknown` 时先查证实际状态再重试。

## 7. 验证与迁移要求

基准必须先于实现保存，能区分正确和错误实现；复验绑定当前 Spec、revision、输入、验证器和环境。记录 `passed`、`failed`、`undetermined` 或未运行，代码、Spec、摘要或验证器变化后将旧证据标为 `stale` 并重验。检查应覆盖真实文件字节、进程存活与回收、身份/发送次数、重复派发、重启事件序列、授权边界、浏览器行为和原生 UI 行为；文档、测试或产物存在不能推导完整产品验收。

原工作区已有修改和未跟踪输入以快照及哈希纳入基线，不从 HEAD 重建后假装保留了它们。产品目录实施时同步 CI 路径触发；旧 Rust/TS、混合宿主与维护示例按原基准复跑，离线 CI 与真实 Agent/平台验证分别留证。

## 8. 计划修订记录（实施之前）

本轮把目标更新为 DESIGN `0.5-draft`，引用 D14 与整合 Spec，确立 Rein/Web Studio/Veriflow 的职责和两条验证线，补齐六阶段门槛、Veriflow adapter、唯一任务状态、单执行 recorder、原始授权上下文、任务交付与产品发行边界，并将 R1c 提前到 R2/R3 之前。跨仓代码、控制 API、adapter、Web Studio 客户端、验证器和发行链路尚未修改；本文件的计划文字不构成已实现或已通过声明。

## 9. 可立刻开始的 R0 最小单元

冻结一个跨项目最小 Spec 版本及其内容 hash；登记 Veriflow 1.3 投影与固定校验器版本；保存旧基准和能力探测原始记录；定义 Rein DB 的任务状态唯一写入边界；为网页 fixture 预先写好 `INT-B01..B12` 所需输入、预期和失败反例；列出 R1a 控制/证据合同、R1b 首个 adapter 和 R1c Web Studio 消费者的依赖。

R0 可编写所需基准、fixture、FakeAgent 与能力探测工具；当前 Spec 对应的基准就绪并审查后，才开始生产运行时代码改造。当前跨仓代码尚未改动，未知项不得判为通过；主线程需审查本文件与新整合 Spec 的一致性，再委派 coder 实现。

## 10. 已开始的首批实施

用户已授权子代理开始实现，并统一 Apache-2.0。首批范围由 [R1A-SLICE-1](records/REIN-RUNTIME-R1A-20260921/implementation-spec.md) 冻结，当前结果、原始证据和剩余项见 [任务摘要](records/REIN-RUNTIME-R1A-20260921/task-summary.md)。先保存旧基准及新行为基准，再实现纯 core、SQLite 状态/outbox、artifact、原生只读模型—工具—反馈—验证闭环与 CLI。许可改动与运行时代码分别委派和审查。

旧教学 API 本增量保留原样并复跑，兼容 wrapper 尚属后续任务；离线模型用于验证反馈和恢复机制，不能替代真实模型的 R1a 验收。首批完成也不意味着独立工作树、写入批准、daemon、R1b 真实委派或 R1c Web Studio 已完成。Web Studio 与 Veriflow 的 Apache-2.0 根许可已核对，本轮无跨仓源码改动。


首批新增根 Cargo workspace 的 `rein-core` / `rein-runtime`，保留旧 `rust/` 独立工程。离线纵切已接通纯状态转换、SQLite 事务/outbox、内容寻址产物、冻结验证计划、独立验证回执和 `demo/show/resume/cancel`。Rust DTO 生成 JSON schema 与 TypeScript，CI 同步覆盖新路径。使用与边界见 [运行时说明](runtime/README.md)，最终有效性以同 Spec 的绑定执行回执和任务摘要为准。

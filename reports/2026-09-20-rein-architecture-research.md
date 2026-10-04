# Rein 架构调研：CLI、聚合配置与多 Agent 执行

**REIN — Runtime for Emergent Intelligence Networks**

| 项目 | 内容 |
| --- | --- |
| 调研日期 | 2026-09-20，Asia/Shanghai |
| 范围 | 当前 Rein 预期 S01–S14；Codex、CC Switch、pi、Gemini CLI 的选定源码；Claude Code 的公开仓库与官方集成文档 |
| 方法 | 锁定远端提交，读取目录树，下载 42 个选定文件并检查关键路径；没有安装、构建或运行这些第三方项目 |
| 本地基线 | HEAD `1dfa82b9d90196c2294f61914d2017449e5548e3` 加未提交工作区 |
| 结论性质 | 源码研究与 Rein 设计建议；不是适配器兼容性、账号隔离、费用控制或恢复验收 |
| 配套 | [模块设计](../REIN-MODULES.md)、[产品规格](../REIN-DESIGN.md)、[实施计划](../REIN-IMPLEMENTATION-PLAN.md)、[来源清单](2026-09-20-rein-research-sources.json) |

> 后续架构修订：用户已通过 [D12](../DECISIONS.md) 确定 Rein core 保留完整 Harness，并可担任调度 Agent。本报告第三方源码观察与固定提交保持有效；下面“core 仅做任务控制、内置 Harness 延后独立”的组织和实施建议已被 [当前模块设计](../REIN-MODULES.md) 与 [实施计划](../REIN-IMPLEMENTATION-PLAN.md) 替代。第 10 节的检查记录绑定修订前文档，只作为历史证据。

## 1. 结论先行

Rein 的功能可以归为五个逻辑层：客户端、任务控制、外部 Agent 接入、执行基础设施、内置 Harness。首轮建议落到 **`core/`、`agentmux/`、`runtime/` 三个 Rust 产品包**，在包内细分职责；内置 Harness 延后独立成包。目录和包是后续实施目标，本轮只形成文档。

最需要避免的混淆是把“能调用多个模型”“能切换多个配置”“能启动多个子 Agent”当成同一种能力。参考项目各自覆盖不同部分：

| 参考对象 | 本次实际看到的机制 | Rein 借鉴点 | 不能直接推导的保证 |
| --- | --- | --- | --- |
| Codex | App Server 协议、内部 Agent 树控制、共享预算、认证后端、独立状态 crate | 外部会话协议边界；根任务与子执行关系；认证存储可替换 | 内部 Rust 控制器是外部稳定 SDK；所有内部能力均可从公开协议获得 |
| CC Switch | 原生配置读写、供应商目录、用量缓存、HTTP 代理适配、故障转移与熔断 | Profile 格式转换、来源区分、健康观测、配置写入失败补偿 | 全局活动配置切换等同于并发任务的独立身份绑定 |
| pi | AI 类型、Agent loop、会话、RPC、subagent 示例；另有新的 Harness/session/recovery 实现 | 模型/循环/会话拆分；子进程委派；恢复时不盲目重复模型调用 | 示例并行已提供工作区隔离、持久 DAG、跨订阅调度和产品验收 |
| Gemini CLI | 模型策略链、工具 Scheduler、Policy Engine、确认消息总线 | 显式路由决定、可组合规则、执行前策略检查与异步确认 | 名叫 Scheduler 的模块都应成为 Rein 的任务调度器 |
| Claude Code | CLI 非交互入口与 Agent SDK 官方接口；公开仓库不提供完整主循环实现供本次核查 | 通过正式集成入口接入，能力按认证方式与版本验证 | 与 Codex 相同的源码可见性；CLI、SDK、订阅认证具有相同能力 |

各项目事实对应下文逐项来源；Rein 的层次与包划分是本次综合设计判断。

## 2. 研究快照与复核方法

| 项目 | 实际仓库 | 锁定提交 | 本次读取范围 |
| --- | --- | --- | --- |
| Codex | `openai/codex` | `5c5308fc9a9ee789049d646ef11e5400384b9c6f` | 协议、内部 Agent 控制、状态、登录存储 |
| CC Switch | `farion1231/cc-switch` | `1408f382798f0a7fccfaf36643303f3ad73bd44b` | Provider、原生配置、代理路由与缓存 |
| pi | `earendil-works/pi` | `19451accdeec671c1f4da9eafac8fc270f510ef4` | AI/Agent/会话、RPC、委派示例、新 Harness |
| Gemini CLI | `google-gemini/gemini-cli` | `cfbcaa8df13ea4610bb379b377b56d62980c0032` | 模型路由、工具调度、策略、确认、Agent 执行相关文件 |
| Claude Code | `anthropics/claude-code` | `7974a70773fa229e4cc65aa1b356cc21f5c216c4` | README、LICENSE、目录树；集成行为另查官网 |

`badlogic/pi-mono` 在本次 GitHub API 查询中重定向到 `earendil-works/pi`。本文把用户所称 pi 解释为这一项目及其仓库中的 subagent 扩展示例；没有把名称相近的第三方路由项目混入结论。

来源清单保存完整提交、抓取时间、源码路径与 SHA-256。以下源码链接固定到提交，官网文档记录查阅日期而不冒充版本快照。远端默认分支可能先于正式发行版；后续接入测试仍须另行锁定本机二进制版本。本轮不进行第三方源码复制或依赖引入。

## 3. Codex：参考协议与会话控制，避免耦合内部实现

### 3.1 公开协议是首选适配边界

协议代码注册了 `initialize`、`thread/start`、`turn/start`、`turn/interrupt`、`account/rateLimits/read` 等消息。官方 App Server 文档描述连接初始化、线程/轮次和事件交互。因此建议 CodexAdapter 首选 App Server 的公开稳定部分，并按实际版本生成或固定协议测试样本。[协议注册][C1]、[官方 App Server 文档][C2]

Rein 保存自己的 Run/Attempt ID，并建立外部 thread/turn ID 映射。请求已接受、轮次已结束、进程已退出、产物已验收是不同事件，不能压成一个 `success`。

源码中的 `ChatgptAuthTokens` 带有明确的内部使用与不稳定标记。不能因为类型可见就将该模式写成 Rein 的默认认证合同；首批走经实际验证的正常登录/Profile 路径。[认证模式声明][C3]

### 3.2 内部多 Agent 与共享预算

`LocalAgentControl` 以根会话为范围共享注册表、执行限制器和 rollout budget，并分别组织 spawn、interrupt、completion、execution 等功能。这可启发 Rein 保存父子执行关系、避免每个子任务各算一份独立总预算；但它是 Codex 内部类型，不能直接当作异构任务调度库。[控制器][C4]、[预算处理][C5]

对 Rein 的额外要求：外部 Agent 自己派生的子 Agent 是否可观察、可禁用、可计量，要单独进入 CapabilityReport。Rein 只限制启动了两个外部主进程，并不能证明只发生了两路模型执行。

### 3.3 存储与认证

Codex 将状态存储和登录存储放在独立代码区域，认证存储有文件与 keyring 等具体后端。`state` 的入口还描述了从 rollout 中提取元数据的角色；不能据此宣称“Codex 所有权威状态都只有一个 SQLite 表源”。Rein 应自行定义事件、业务状态与产物索引的一致性模型。[状态入口][C6]、[认证存储][C7]

## 4. CC Switch：拆开配置聚合、用量观测和代理路由

### 4.1 配置格式的归属

`services/provider/live.rs` 处理各 CLI 的原生配置读写，并区分当前配置同步和部分应用的多配置同步。pi 专用服务还展示了原生配置写入与数据库保存失败时的补偿处理。这是外部配置适配职责，适合进入 Rein 的 AgentMux/ProfileCodec，而任务状态与资源账本不应进入这些格式模块。[原生配置同步][S1]、[pi 配置服务][S2]

Rein 应将该思路改造成每次执行的配置实例化：输出绑定指定 Profile/version 的执行配置，记录写入与核对结果。是否可以创建独立配置目录、认证写入者是谁、轮换后能否安全回收，必须针对每个 CLI 实测。不能预设复制一个认证文件就解决并发身份隔离。

### 4.2 ProviderRouter 路由的对象是 HTTP 上游

`ProviderAdapter` 负责 base URL、认证头和请求/响应转换；`ProviderRouter` 返回当前供应商或按故障队列排序的候选，并使用熔断状态。这是请求层的供应商选择，不是带工作区、任务依赖、批准和产物的 Rein Attempt 派发。[代理接口][S3]、[路由器][S4]

这个版本还显式排除 Codex 官方账号跨卡 failover，避免复用入站认证跨越账号边界。Rein 据此应强化原有绑定规则：跨 Profile 的 fallback 由 core 创建新 Attempt，经重新授权的上下文交接后启动；AgentMux 不得在同一 Attempt 内悄悄换账号。[账号边界处理][S5]

### 4.3 用量缓存不是预算账本

`UsageCache` 区分应用订阅、托管 Codex 账号及脚本查询，并注明自身为进程内展示缓存。Rein 可借鉴来源键，不能直接把它作为并发预留或重启恢复的账本。[用量缓存][S6]

Rein 需要另建 QuotaObservation、UsageObservation、Reservation 三类记录：远端可见余额、某执行的消耗观测、本地已承诺容量。百分比、token 和货币各有单位；观测到的外部消耗与本地已结算量要避免重复扣减。

## 5. pi：同时区分 CLI 示例与新 Harness 代码

### 5.1 三个基础层

`pi-ai` 定义模型/提供方/消息/流参数；`pi-agent-core` 定义 AgentMessage、上下文变换、模型边界转换、工具调用与事件；coding-agent 再叠加会话和应用资源。当前 SessionManager 保存带 parentId 的会话条目，不能把这个会话树直接当作 Rein 的任务 DAG。[AI 类型][P1]、[Agent 类型][P2]、[循环][P3]、[会话条目][P4]

借鉴方式：Rein 内置 Harness 的模型协议不进入任务领域；跨 Agent 的 ContextBundle 也不等同于某一模型的 messages 数组。外部 Agent 的内部压缩、分支、工具 history 保留原生语义，Rein 只保存必要句柄和交接产物。

### 5.2 subagent 示例的实际边界

示例支持单个 Agent、并行和 chain；源码将最大任务数设为 8、并发设为 4，启动独立 `pi` 进程，以 JSON 输出收集结果。子进程默认使用 `--no-session`，cwd 可以继承父级；chain 把前一步最终文本替换进 `{previous}`。[示例说明][P5]、[启动和输出处理][P6]、[chain/parallel][P7]

这证明了一种清晰、轻量的委派实现，但不提供默认的独立工作树、Rein 式持久任务恢复或已验收产物交接。Rein 借鉴角色模板和有界 fan-out；交接使用版本化 Artifact/ContextBundle，而不是仅拼接上一段文本。

### 5.3 当前源码已经有进一步的运行时分层

不能简单写成“pi 只有轻量循环”。当前仓库还包含 Harness lanes、持久 session 的 JSONL transaction 实现、pico3 task scheduler，以及孤立模型请求的恢复路径。后者从已提交片段恢复消息，不再调用提供方，并保留外部结果未知的说明。[Harness][P8]、[JSONL 存储][P9]、[任务 scheduler][P10]、[恢复][P11]

这里仅确认这些实现存在。coding-agent 的传统 Agent/SessionManager 与新 Harness 都在仓库中；本次没有通过运行证据证明所有 CLI 模式都使用新路径，不能把整个目录的能力直接合并为一个已交付功能表。恢复代码生成的零 usage 也不能转译成 Rein 的“外部实际消耗为零”。

### 5.4 RPC 接入

pi RPC 文档明确 prompt 的成功响应表示接受/排队/即时处理，后续错误通过事件报告；协议按 LF 分帧。若以后接入 Rust Rein，建议使用独立 PiAdapter 经 RPC 通信，保持 Node 依赖在外部边界。[RPC 文档][P12]

## 6. Gemini CLI：模型路由与工具执行的职责可分开参考

`RoutingDecision` 包含选中的 model、来源、原因和延迟。ModelRouterService 构造有顺序的策略链，包含 fallback、override、approval-mode、分类策略及最终默认策略。Rein 可借鉴“结果 + 决策轨迹”的结构；R1/R2 先采用确定性规则，模型分类选择延后，并纳入成本与数据策略。[路由合同][G1]、[策略装配][G2]

另一个 `Scheduler` 调度 ToolCall，调用 Policy 与 confirmation 逻辑；`scheduleAgentTools` 为 Agent 组装工具注册表和确认消息总线。因此 Rein 的 TaskScheduler、内置 Harness 的 ToolScheduler、异步批准通道必须各有所有者。[工具调度][G3]、[Agent 工具调度入口][G4]

Rein 不直接照搬其整个 Config 对象到 core；提供更窄的策略快照和能力描述，防止核心通过配置取得模型客户端、文件系统或 UI。

## 7. Claude Code：按公开集成合同研究

本次公开仓库读取范围不能提供完整核心执行循环；其根 LICENSE 写明权利保留及适用商业条款。因此这里只分析官方集成界面，不将 Claude Code 与其他项目笼统归为相同源码开放程度。[仓库许可证][A1]

官方文档支持 `claude -p` 和结构化输出；SDK 提供更多程序化交互。特别是 `--bare` 的认证前提与普通订阅登录不同，不能为了减少自动配置加载便把它设为所有 Profile 的默认启动方式。默认发现的 hooks/MCP、取消信号与会话结束语义都需按选定模式核对。[CLI 集成][A2]、[Agent SDK][A3]

Rein 的设计结论：先验证普通 CLI 路径能否满足所选订阅 Profile；需要双向工具批准时再选择具有该能力的 SDK bridge 或正式接口。无法响应审批的接入路径只能运行其能力允许的任务，不能自动放行审批。首轮不假定 SDK 与订阅登录可以任意组合。

## 8. 转化为 Rein 的功能分界

| 易混概念 | 正确所有者 | 输入 → 输出 | 本轮优先级 |
| --- | --- | --- | --- |
| Task routing | `core/scheduling` | 就绪节点 + 策略/资源快照 → BindingProposal/等待原因 | R1/R2 |
| Model routing | 内置 Harness；外部 Agent 内部选择只能观测/配置 | 模型上下文 + 已授权候选 → ModelDecision | 内置产品路径之后 |
| Endpoint failover | API transport/未来 gateway | 同一受权请求的端点集合 → 请求结果/不确定状态 | 延后，不阻塞 P0 |
| Tool scheduling | 内置 Harness 或外部 Harness 自己 | 工具调用 + 工具策略 → 结果/批准请求 | 维持现有；产品化后提取 |
| Profile catalog | `core/identity` | 主体/组织/配置版本 → ProfileRef | R1 最小，R2 扩展 |
| Native profile format | `agentmux/profiles` | Profile + 格式版本 → 实例化配方 | 每个适配器必需 |
| Secret storage | `runtime/credentials` | 私有引用 + lease → 受控句柄/轮换回执 | R1 最小，R2 扩展 |
| Acceptance | `core/acceptance` | Spec + 封存产物 + Evidence → 接受/拒绝/未确定 | R1 必需 |

对应 14 项规格的完整主责、协作者、接口和检查清单见 [REIN-MODULES.md](../REIN-MODULES.md)。当前教学源码如何保留与抽取见该文档第 9 节。

## 9. 推荐接入次序与剩余不确定性

1. 先完成 FakeAgent 和持久任务链路；Codex 作为首个真实适配候选，原因是可核查的正式 App Server 入口与源码较完整。只有实际探测通过才进入该路径的支持列表。
2. Claude Code 作为第二个 P0 目标；将 CLI/SDK、订阅/API 认证组合拆开验证，不能用某一 API-key 路径的成功代替订阅目标。
3. pi 用于验证第三种事件/会话模型和内置 Harness 设计；Gemini CLI 用作路由/审批设计参考与后续适配候选。两者并非本轮新增 P0 必交付项。
4. CC Switch 首轮借鉴格式管理和观测，不嵌入其完整桌面应用、代理服务或反向实现的认证路径。若需要配置导入，单独做用户选择范围的只读 importer，再转为 Rein Profile 草案。

尚未获得的证据包括：真实 CLI 版本兼容性、两个 Profile 并发时的认证隔离、凭证轮换写入者、进程树取消、断线后恢复、内部子 Agent 预算，以及跨 Agent 同一任务的最终验收。源码阅读只决定接下来测试什么，不能填充这些能力为 supported。


## 10. 本轮文档验证

Markdown 解析、引用解析、本地文件链接、JSON 示例与来源清单检查通过；S01–S14 在模块主责表中各出现一次，29 个源码引用均固定到已下载的提交/路径/行范围。42 个源码文件哈希与来源记录一致。本轮开始时保存的 195 个源码、合同、夹具与教学文件哈希未变化。

上述是文档和文件一致性检查；没有重跑 Rust/TS 运行测试，也没有执行真实 Agent。结构化结果见 [检查记录](2026-09-20-rein-research-checks.json)。

[C1]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/app-server-protocol/src/protocol/common.rs#L507-L565
[C3]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/app-server-protocol/src/protocol/common.rs#L23-L37
[C4]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/core/src/agent/control.rs#L75-L154
[C5]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/core/src/agent/control/budget.rs#L1-L36
[C6]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/state/src/lib.rs#L1-L39
[C7]: https://github.com/openai/codex/blob/5c5308fc9a9ee789049d646ef11e5400384b9c6f/codex-rs/login/src/auth/storage.rs#L167-L223
[S1]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/services/provider/live.rs#L1664-L1708
[S2]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/services/provider/pi.rs#L26-L85
[S3]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/proxy/providers/adapter.rs#L1-L60
[S4]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/proxy/provider_router.rs#L38-L131
[S5]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/proxy/provider_router.rs#L15-L20
[S6]: https://github.com/farion1231/cc-switch/blob/1408f382798f0a7fccfaf36643303f3ad73bd44b/src-tauri/src/services/usage_cache.rs#L1-L59
[P1]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/ai/src/types.ts#L17-L81
[P2]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/types.ts#L156-L209
[P3]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/agent-loop.ts#L1-L134
[P4]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/coding-agent/src/core/session-manager.ts#L40-L87
[P5]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/coding-agent/examples/extensions/subagent/README.md#L1-L102
[P6]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/coding-agent/examples/extensions/subagent/index.ts#L300-L410
[P7]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/coding-agent/examples/extensions/subagent/index.ts#L550-L660
[P8]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/harness/runtime/harness.ts#L28-L79
[P9]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/harness/session/jsonl/storage.ts#L123-L152
[P10]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/harness/pico3/scheduler.ts#L36-L84
[P11]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/agent/src/harness/runtime/drive/recovery.ts#L24-L90
[P12]: https://github.com/earendil-works/pi/blob/19451accdeec671c1f4da9eafac8fc270f510ef4/packages/coding-agent/docs/rpc.md#L1-L85
[G1]: https://github.com/google-gemini/gemini-cli/blob/cfbcaa8df13ea4610bb379b377b56d62980c0032/packages/core/src/routing/routingStrategy.ts#L13-L78
[G2]: https://github.com/google-gemini/gemini-cli/blob/cfbcaa8df13ea4610bb379b377b56d62980c0032/packages/core/src/routing/modelRouterService.ts#L27-L69
[G3]: https://github.com/google-gemini/gemini-cli/blob/cfbcaa8df13ea4610bb379b377b56d62980c0032/packages/core/src/scheduler/scheduler.ts#L9-L49
[G4]: https://github.com/google-gemini/gemini-cli/blob/cfbcaa8df13ea4610bb379b377b56d62980c0032/packages/core/src/agents/agent-scheduler.ts#L44-L93
[A1]: https://github.com/anthropics/claude-code/blob/7974a70773fa229e4cc65aa1b356cc21f5c216c4/LICENSE.md#L1-L1
[C2]: https://learn.chatgpt.com/docs/app-server
[A2]: https://code.claude.com/docs/en/headless
[A3]: https://code.claude.com/docs/en/agent-sdk/overview

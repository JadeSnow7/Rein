# Rein RuntimePort 0.1 候选

| 项目 | 内容 |
| --- | --- |
| 契约标识 | `rein.runtime-port/0.1-draft` |
| 状态 | 待实现的文档契约；main 没有对应 API；本次没有新增 SDK、服务或 CLI 命令 |
| 所有者 | Rein：单次运行的执行语义、状态、权限与原始回执 |
| 消费者 | Veriflow：有界尝试分派、验证与整体验收；Web Studio 任务展示的订阅来源尚未定义（见 D19） |
| 实现依据 | [PR #3](https://github.com/JadeSnow7/Rein/pull/3) 已合并的 R1a，合并提交 `e649708f1f8d68c2c259f150835f54f85126a44f` |
| 关联决定 | [D19](../DECISIONS.md#d19) |

本文冻结候选的职责、数据归属及兼容原则，供首个 adapter 实现和验证。以下操作名是语言中立的 port 方法，不是现有命令或 HTTP 路由。传输可先用进程内调用或本地 CLI adapter，不要求 daemon、网络服务或新协议框架。缺少适配的操作返回 `unsupported_capability`，不能悄悄运行另一种行为。

## 1. 权威与绑定

Veriflow 拥有 `workflow_id / task_id / attempt_id`、任务依赖和验收契约；Rein 为一次尝试分配 `run_id / session_id` 并拥有 session revision、effect、执行事实。开始后固定绑定，不能把旧尝试的回复送入新尝试。

`ArtifactRef = { hash: string, len: uint64 }` 沿用 R1a 的引用形状；`hash` 是原始字节的 SHA-256 小写十六进制，`len` 是字节数。引用不等于访问授权；adapter 必须通过受信 artifact 读取器取回并检查字节。跨 JSON/语言边界的 uint64 按权威 schema 映射；消费者不能把超出安全整数范围的值静默截断。

Veriflow 提供或引用不可变的 `spec_ref / input_ref / verification_plan_ref`；Rein 保持请求中的引用并将输出与该组绑定一起返回。新的 Spec、输入或计划产生新的尝试。传入引用必须存在于可访问的受信 artifact store 中，adapter 先核验字节，不接收模型任意指定的远程 URL 或本地绝对路径作为授权。首次接入允许用有界导入适配器复制并校验字节，不要求两层共用数据库。

`workspace_ref` 是受信环境 provider 的不透明工作空间句柄，不能由模型文本直接扩张目录。Web Studio 拥有 Web/终端资源生命周期；Rein 通过已授权 provider 能力调用它们。全局任务状态和 Web UI 状态不放入 Rein 的会话表。

## 2. 最小逻辑操作

所有变更请求携带 `contract_version / request_id`；对已有 run 的变更还携带 `run_id / attempt_id / expected_revision`。重发同一 request_id 与相同规范化内容返回原回执；同键不同内容返回 `request_conflict`，不得重复派发。读取不新建任务、不继续执行，也不暗中复验产物。

| 操作 | 输入 | 输出与约束 |
| --- | --- | --- |
| `capabilities` | 消费者支持的契约版本 | `contract_version`、`runtime_version`、支持的操作、模型/工具/预算与批准能力；选择共同的精确版本。无共同版本则拒绝 |
| `start` | StartRequest（下表） | RunSnapshot；提交绑定、会话和首个意图后才能执行；一个 start 只启动一个 Agent 的一次尝试 |
| `get` | `run_id / attempt_id` | 当前 RunSnapshot；不调用模型或工具 |
| `events` | `run_id / attempt_id / after_cursor / limit` | 有界事件列表与 `next_cursor`；游标严格递增，允许重复投递。历史已不可续读则返回 `cursor_expired`，消费方先 get 核对，不能重新 start |
| `cancel` | 变更请求绑定与理由 | 接受后的 RunSnapshot；阻止后续派发，已经在途的副作用仍须记录；不承诺撤销已经执行的效果 |
| `resume` | 变更请求绑定 | 仅在可恢复的同一次尝试上继续；不创建新 attempt、不复活 terminal run。未知派发先 reconcile，不盲重放 |
| `reconcile` | `run_id / attempt_id / effect_id` | 查询受信 provider 的实际结果与回执；无法确认保持 `outcome_unknown`。它不授权新动作，确认后才允许按原策略恢复 |
| `artifact.get` | run 绑定与 ArtifactRef | 受限原始字节；取回后核验 hash/len；无权访问或损坏不能返回成功 |
| `approval.respond`（可选能力） | 变更请求绑定、批准请求 ID、候选摘要、批准/拒绝及受信授权来源引用 | 只作用于匹配的当前 effect、范围、候选与版本；模型文本和 UI 的显示状态都不能作为批准 |

StartRequest 的必需字段：

| 字段 | 含义 |
| --- | --- |
| `contract_version / request_id` | 协议版本与幂等键；在受信调用者身份 + workspace 范围内唯一 |
| `workflow_id / task_id / attempt_id` | 消费方身份；runtime 仅保存、绑定与校验，不计算 DAG |
| `spec_ref / input_ref / verification_plan_ref` | 不可变规格、输入/任务交接上下文、检查计划；不传完整聊天历史 |
| `workspace_ref` | 受信 provider 绑定的工作空间与输入快照关联 |
| `execution_policy` | `model_profile`、允许工具与路径、局部 `max_model_requests / max_tool_calls / deadline_ms`、所需批准方式；必须与 capabilities 兼容。密钥由 runtime 配置，不进入请求、事件或证据 |

消费方分配的预算只是上限请求。runtime 按有效宿主策略取更严格的上限；未实现的预算维度不能被忽略后宣称已受控。并行占用、全局预算和修复次数由 Veriflow 统筹。工具白名单与路径校验不等于 OS 级沙箱。

RunSnapshot 必须保留上述身份与摘要绑定，另含 `run_id / session_id / revision / runtime_status / stop_reason / cursor`，已产生的 `output_refs / receipt_refs`，以及局部检查状态（若有）。未产生的产物为空，不能伪造引用。`runtime_status` 与 `stop_reason` 合起来区分正常结束、工具/模型失败、局部预算耗尽、取消、等待批准和结果未知；各情况在 0.1 的支持范围见第 4 节。取消、结果未知、等待批准由 `runtime_status` 表达；`stop_reason` 只说明运行为什么不再调用模型。

## 3. 状态、事件与验证

`runtime_status` 仅描述执行生命周期：`running / waiting_approval / finished / cancelled / outcome_unknown`。`waiting_approval` 仅在能力声明并实施时可出现。`finished` 表示本次执行结束，包括预算耗尽或失败；不等于通过验收。`outcome_unknown` 保留已有 effect 的身份，阻止新派发直到受信核对。`cancelled` 是调度停止状态，不是“所有在途外部进程均已终止”的证明。

`local_verification` 若存在，只表示固定检查的 `passed / failed / undetermined` 和 receipt 引用；没有检查就缺省，不能当作 passed。Veriflow 另行维护 workflow 验收状态与证据有效性。运行退出 0、模型自报成功或 R1a 的 `Accepted` 都不能直接生成整体验收通过。

每个事件包含 `contract_version / event_id / cursor / run_id / session_id / attempt_id / revision / kind / payload_ref`；与工具或副作用有关时额外带 `effect_id`。payload 由受信运行层生成或保存其原始字节；模型消息只是 payload，不得伪装成 runtime 的权威状态事件。消费者按 event_id 去重，事件缺口先 get / events 补齐，不自行推进或重复执行。run/session 数据库保持 runtime 单一写入；任务图数据库保持 Veriflow 单一写入。

固定检查可以由 Rein 的 verifier primitive 或受信环境 provider 执行。回执必须绑定当前 plan、输入/候选 artifact、执行器/环境版本和原始结果；Veriflow 决定它是否满足 AcceptanceContract。若检查由 Veriflow 外部执行，只返回 VerificationReceipt 引用，不将外部结论冒充成 R1a 内部的 VerifierResult，也不直接写 `HarnessSession`。

## 4. 对已合并代码及目标类型的兼容映射与缺口

**以 RESULT-SPLIT-1 实施后的类型为准，实施前本表不能用于实现。** `RunStatus`、`StopReason`、`VerificationRecord` 是状态拆分后的目标类型，目前不能把旧状态猜成这些结果。停止原因只读取正式运行结果中保存的 `StopReason`；不从原始事件或回执重建停止原因。

下表描述 RESULT-SPLIT-1 实施后 0.1 可产生的组合，不表示当前已有 adapter。表内停止原因值引用源类型的枚举名；`stop_reason` 按保存的 `Option<StopReason>` 原样传递，有值不清空，无值不补造。`RunSnapshot.unreconciled_effects` 保留正式 `RunResult.unreconciled_effects` 中尚未核对的效果 ID（来自会话的 `unknown_effects`）。

| 情况 | RuntimePort 字段与值 | 拆分后的来源 | 0.1 是否会产生 |
| --- | --- | --- | --- |
| 正常结束 | `runtime_status = finished`；`stop_reason = FinalAnswer` | `RunStatus::Finished` + `StopReason::FinalAnswer` | 会产生；仅表示执行结束，不表示局部验证通过或最终接受 |
| 工具失败 | `runtime_status = finished`；`stop_reason = ToolFailed { call_id, error_ref }`，保留工具调用 ID 与错误引用 | `RunStatus::Finished` + `StopReason::ToolFailed` | 会产生 |
| 工具预算耗尽 | `runtime_status = finished`；`stop_reason = ToolBudgetExhausted` | `RunStatus::Finished` + `StopReason::ToolBudgetExhausted` | 会产生 |
| 取消 | `runtime_status = cancelled`；`stop_reason` 保留已有值或无值，不新增“取消”停止原因 | `RunStatus::Cancelled` + 已保存的 `Option<StopReason>` | 会产生 |
| 等待验证时取消 | `runtime_status = cancelled`；`stop_reason = FinalAnswer`；没有验证记录，`local_verification` 缺省 | `RunStatus::Cancelled` + `StopReason::FinalAnswer`；按规格转换表保留停止原因，取消时没有验证结果 | 会产生；呈现为“模型已给出最终回答，但运行在等待验证时被取消”，不能映射成 `finished` 或验证通过 |
| 结果未知 | `runtime_status = outcome_unknown`；`unreconciled_effects` 保留尚未核对的效果 ID；`stop_reason` 保留已有值或无值 | `RunStatus::OutcomeUnknown` + `RunResult.unreconciled_effects` + 已保存的 `Option<StopReason>` | 会产生；不据此猜测停止原因，不丢弃待核对效果 |
| 模型失败 | 0.1 无对应输出，不映射成工具失败或正常结束 | core 目前没有模型失败的观察，现有 `StopReason` 无对应变体 | 不产生；接入真实模型时在 core 增加停止原因并升级契约版本 |
| 等待批准 | 0.1 不产生 `runtime_status = waiting_approval`，适配层不声明批准能力 | 现有 `RunStatus` 无批准等待变体 | 不产生；批准实施时增加运行状态并升级契约版本 |

等待验证时取消的组合直接保留 [RESULT-SPLIT-1 转换表](../reports/2026-10-02-run-verification-acceptance-split.md#32-转换表) 的语义：`FinalAnswer` 说明模型调用已经结束，`cancelled` 说明之后的验证等待被取消，两个字段不能互相覆盖。迟到的验证结果仍被拒绝；没有检查记录不能生成 `local_verification.passed`。

| 已合并位置 / 待实施的目标类型 | 可复用的 primitive | adapter 必须补齐 / 避免的误用 |
| --- | --- | --- |
| `core/src/lib.rs`：HarnessSession、Observation、Intent、StepResult | run/session/attempt、revision、effect、纯步进和拒绝语义 | `workflow_id / task_id` 与规格摘要绑定属于 port envelope；不改成 core DAG |
| `Runtime::start / inspect / cancel / recover_explicitly` | 单会话执行、查询、取消及未知效果恢复 | CLI demo 的 fixture 参数不是通用 StartRequest；没有版本协商、请求去重或跨客户端 events API |
| `RunStatus`、`StopReason`（RESULT-SPLIT-1 待实施） | 独立的执行生命周期与持久化停止原因 | `RunStatus::Finished` 对应 `finished`；保留正式 `StopReason` 的原因与错误引用，不由验证结论推导运行状态，也不从事件猜测原因 |
| `VerificationRecord`（RESULT-SPLIT-1 待实施） | 与计划、被验证产物、原始回执绑定的独立局部检查记录 | `local_verification` 只映射该记录的检查状态与 receipt 引用；未执行检查不能生成 passed，不表示 workflow accepted |
| `ArtifactRef`、artifact store、FixedVerifier | 摘要产物与固定字节验证回执 | 不把固定字节 oracle 称为通用项目验证服务 |
| `contracts/runtime/schemas/{session,observation,intent,step-result}.json` | Rust DTO 生成的运行层 schema | 属于内部运行合同；没有 StartRequest / VerificationPlan / workflow schema。port wrapper 后续从同一个权威定义源派生 |

main 的 Python、TS、Rust 教学调用继续按原合同工作；这次没有包装或改名它们。未来 adapter 先复用上述 R1a 原语，不搬迁教学目录、不让 Veriflow 解析 SQLite 或依赖内部 Rust 私有布局。

## 5. 版本与实现验收

`0.1-draft` 只用于设计评审，不声称生产稳定性。首次实现应以固定提交的本文与正式生成 schema 一起发布兼容矩阵；未证明的能力不出现在 capabilities。消费者只引用固定版本或内容摘要。新增可选字段须明确缺省语义；必需字段、状态含义、预算或副作用行为变化须使用新的契约版本，不能静默覆盖。

首轮验证至少覆盖：同请求重发与冲突；旧 attempt / revision 拒绝；读取无副作用；重复事件与过期游标；越权与不支持的预算拒绝；取消后的迟到结果；未知效果不盲重放；artifact 字节损坏；局部 verifier 通过但 workflow 必需检查失败。以上均为**待运行的验收条件**，文档检查不能替代它们。

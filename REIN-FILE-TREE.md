# Rein 预期文件目录

**REIN — Runtime for Emergent Intelligence Networks**

版本：`0.3-draft`，2026-09-21。依据：[设计规格](REIN-DESIGN.md)、[模块设计](REIN-MODULES.md) `0.4-draft`、[整合规格](REIN-INTEGRATION-SPEC.md)、[实施顺序](REIN-IMPLEMENTATION-PLAN.md) 和 [D12、D13、D14](DECISIONS.md)。

这是目标文件布局，不是当前源码清单。产品部分展开到文件级；现有教学、章节快照和历史记录按目录保留，不逐项重复列出章节内容。本次只维护设计文档，没有创建产品源码或空目录。

`core/` 保留完整原生 Harness；Coordinator 是它的一种运行角色，使用同一纯步进状态机。core/harness 不 await 任何 I/O，runtime driver 执行已提交意图并将结果作为观察送回。`agentmux/` 接入外部 Harness。三个产品 crate 分阶段形成，不要求第一轮一次创建所有文件。文件名可在实现中按规模合并，职责与依赖方向应保持稳定。

## 1. 仓库总览

```text
Agent-Learning/
├── Cargo.toml                         # 新产品 workspace；members = core/agentmux/runtime
├── Cargo.lock                         # 新产品锁文件；旧 rust/Cargo.lock 独立保留
├── README.md                          # 产品与学习入口
├── DECISIONS.md                       # 架构决定；含完整 Harness 定位 D12
├── MIGRATIONS.md                      # 兼容迁移说明
├── REIN-DESIGN.md                      # 行为规格、接口与验收场景
├── REIN-MODULES.md                     # 功能归属、依赖与协作
├── REIN-FILE-TREE.md                   # 本文：目标文件布局
├── REIN-IMPLEMENTATION-PLAN.md         # 实施阶段与完成条件
├── REIN-INTEGRATION-SPEC.md           # 三项目职责、整合要求与首个纵切基准
├── .gitignore
├── .github/
│   └── workflows/
│       ├── test.yml                   # 保留教学检查，增加产品检查
│       ├── deploy.yml                 # 保留文档站部署
│       └── runtime-live.yml           # 拟新增：手动触发真实 Agent 验证
├── core/                              # rein-core：原生 Harness + 编排 + 控制规则
├── agentmux/                          # rein-agentmux：外部 Harness 适配
├── runtime/                           # rein-runtime：装配、基础设施与 CLI/daemon
├── contracts/                         # 版本化协议和规范
├── fixtures/                          # 教学夹具与新增产品故障输入
├── examples/                          # 教学示例与新增产品用例
├── scripts/                           # 现有书籍脚本与新增产品检查
├── docs/                              # 现有文档站；新增 runtime/ 产品说明
├── book/                              # 保留章节索引
├── chapter-snapshots/                 # 保留历史快照、补丁、清单
├── rust/                              # 现有独立 Rust 教学包与兼容入口
├── ts/                                # 现有 TypeScript 示例与 reference tool host
├── records/                           # 保留既有验收记录；新增记录另建条目
├── reports/                           # 调研、检查与分析产物
├── package.json                       # 现有 Node 文档/教学命令
└── package-lock.json
```

根 Cargo workspace 显式 `exclude = ["rust"]`。仓库目录继续使用 `Agent-Learning`；本目录设计不要求重命名本地 checkout。品牌为 AgentMux，文件夹和 Rust 模块使用小写 `agentmux`。

## 2. core：完整原生 Harness 与任务控制

```text
core/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs                         # 公共 API 与有意保留的兼容导出
│   ├── ids.rs                         # RunId、AttemptId、SessionId 等强类型 ID
│   ├── error.rs                       # 领域错误；不泄漏厂商/数据库内部错误
│   ├── harness/
│   │   ├── mod.rs
│   │   ├── agent_loop.rs              # 纯步进：状态+观察→意图+revision；不 await I/O；首轮合并 ModelRouter/ToolScheduler，后续再拆
│   │   ├── native_adapter.rs          # 原生意图/观察与 AgentAdapter 生命周期的语义映射；不持有 I/O 循环
│   │   ├── session.rs                 # HarnessSession 的引用、游标、revision 与等待/恢复元数据
│   │   ├── message.rs                 # 规范消息/turn 类型和摘要引用；消息体独立存储
│   │   ├── tool_call.rs               # 工具调用关联、执行阶段与未知结果
│   │   ├── tool_registry.rs           # 工具描述与可见性；具体 I/O 由端口提供
│   │   ├── checkpoint.rs              # 引用式会话检查点、在途调用与恢复语义；不存 Future
│   │   ├── usage.rs                   # 单会话/轮次用量；关联根预算账本
│   │   ├── interrupt.rs               # 会话内在途调用的中断规则；实际停止由 driver 执行
│   │   ├── context/
│   │   │   ├── mod.rs
│   │   │   ├── window.rs              # 当前会话上下文装配与长度约束
│   │   │   ├── retrieval.rs           # 局部检索策略与内容引用选择
│   │   │   └── compaction.rs          # 压缩策略、保留项与摘要版本
│   │   └── extensions/
│   │       ├── mod.rs
│   │       ├── protocol.rs            # 0.1/0.2 工具扩展类型与版本适配
│   │       ├── invocation.rs          # 单调用状态与请求/响应校验
│   │       └── approval_bridge.rs     # 工具候选映射至统一 policy 审批
│   ├── orchestration/
│   │   ├── mod.rs
│   │   ├── coordinator.rs             # Coordinator 角色装配与行为约束
│   │   ├── proposal.rs                # 模型提出的拆分/选择/调整提案
│   │   ├── delegation.rs              # DelegationLink、父子关联和调用幂等
│   │   ├── limits.rs                  # 深度、子并发和子预算的校验
│   │   ├── feedback.rs                # 子结果按游标进入父会话工具反馈
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── list.rs                # agents.list
│   │       ├── delegate.rs            # agents.delegate
│   │       ├── status.rs              # agents.status
│   │       ├── await_children.rs      # agents.await；持久等待与唤醒条件
│   │       ├── result.rs              # agents.result
│   │       └── cancel.rs              # agents.cancel
│   ├── task/
│   │   ├── mod.rs
│   │   ├── mission.rs                 # 用户目标与任务集合
│   │   ├── spec.rs                    # 冻结的 TaskSpec 与版本引用
│   │   ├── graph.rs                   # 数据依赖 DAG 与合法性
│   │   └── template.rs                # AgentTemplate；instructionsRef 引用 artifact 中的提示词摘要
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── run.rs                     # Run、NodeRun、Attempt 与 Binding
│   │   ├── lifecycle.rs               # 合法状态转换和终态条件
│   │   ├── cancellation.rs            # 任务/父子执行取消与停止证据
│   │   ├── recovery.rs                # 未知结果、重启核对与恢复判定
│   │   └── retry.rs                   # 重试、换资源与有界修复条件
│   ├── scheduling/
│   │   ├── mod.rs
│   │   ├── readiness.rs               # 依赖验收后的就绪计算
│   │   ├── admission.rs               # 能力/权限/预算/资源硬过滤
│   │   ├── rules.rs                   # 确定性候选排序与受控 fallback
│   │   └── decision.rs                # BindingDecision 与 DecisionTrace
│   ├── resource/
│   │   ├── mod.rs
│   │   ├── resource.rs                # 计算资源、模型资源与共享池归属
│   │   ├── quota.rs                   # QuotaPool、窗口和 unknown
│   │   ├── reservation.rs             # 容量/写 lease 预留与释放规则
│   │   ├── budget.rs                  # 根/父/子 BudgetLedger 与去重结算
│   │   └── health.rs                  # 健康事实的采纳与冷却规则
│   ├── identity/
│   │   ├── mod.rs
│   │   ├── identity.rs                # 账号主体、身份核对状态
│   │   └── profile.rs                 # Profile 版本、SecretRef 与所有权模式
│   ├── policy/
│   │   ├── mod.rs
│   │   ├── permissions.rs             # 工具、路径、执行主体与操作范围
│   │   ├── destinations.rs            # 内容发送目的地和上下文出域约束
│   │   ├── approval.rs                # 候选绑定、批准、撤销与消费
│   │   └── evaluation.rs              # allow/deny/await 及可解释原因
│   ├── handoff/
│   │   ├── mod.rs
│   │   ├── bundle.rs                  # 跨 Agent 的 ContextBundle 清单
│   │   ├── provenance.rs              # 来源、版本和引用关系
│   │   └── handoff.rs                 # 跨 Attempt/Agent 交接规则
│   ├── acceptance/
│   │   ├── mod.rs
│   │   ├── artifact.rs                # ArtifactRef、ChangeSet、IntegrationReceipt
│   │   ├── verification.rs            # VerificationPlan 与检查回执类型
│   │   ├── evidence.rs                # Evidence 绑定、采纳与 stale 原因
│   │   ├── decision.rs                # pass/fail/undetermined 与 Acceptance
│   │   └── repair.rs                  # 有界修复提案；状态推进交给 execution
│   ├── application/
│   │   ├── mod.rs
│   │   ├── service.rs                 # TaskApplication 统一命令入口
│   │   ├── command.rs                 # 任务、审批、委派、观察等 Command
│   │   ├── query.rs                   # 只读查询与一致性要求
│   │   ├── snapshot.rs                # 带 revision 的决策输入快照
│   │   ├── transition.rs              # 状态变化、事件、效果意图和期望版本
│   │   └── effect.rs                  # Prepare/Dispatch/Verify 等效果描述
│   ├── events/
│   │   ├── mod.rs
│   │   ├── envelope.rs                # 事件 ID、关联、顺序和版本
│   │   └── domain.rs                  # 任务、会话、委派、资源等领域事件
│   └── ports/
│       ├── mod.rs
│       ├── model.rs                   # ModelAdapter；唯一权威声明
│       ├── tool.rs                    # ToolExecutor；唯一权威声明
│       ├── agent.rs                   # AgentAdapter、能力及 AdapterObservation
│       ├── delegation.rs              # DelegationPort；回到同一应用入口
│       ├── resource.rs                # ResourceObserver
│       ├── store.rs                   # StateStore、原子提交与事件续读
│       ├── workspace.rs               # 工作区/产物输入边界与执行回执
│       ├── artifact.rs                # 不可变产物读写接口
│       ├── verifier.rs                # 验证计划执行接口
│       ├── credential.rs              # 凭证租约/句柄；领域状态不持有秘密字节
│       └── clock.rs                   # 可注入时间
└── tests/
    ├── support/
    │   └── mod.rs                     # 固定时间、会话快照和模型/工具观察序列；不执行 I/O
    ├── native_harness.rs              # 无外部 CLI 的原生闭环与恢复
    ├── coordinator.rs                 # 委派结果进入下一次模型输入
    ├── delegation_limits.rs           # 越权/预算/深度/循环拒绝
    ├── task_graph.rs
    ├── lifecycle.rs
    ├── scheduling.rs
    ├── resource_budget.rs
    ├── approvals.rs
    ├── context_handoff.rs
    ├── acceptance.rs
    └── extension_compatibility.rs
```

`core/harness/context` 管一个会话的模型输入；`core/handoff` 管跨执行交接。`core/harness/usage` 管会话用量，`core/resource/budget` 管根预算及父子结算。`core/harness/interrupt` 管会话内中断，`core/execution/cancellation` 管任务与父子生命周期的取消。端口类型只声明一次，通过重导出供使用者访问。

Coordinator 工具名仍为内部草案。它提出的委派或资源选择必须经过 application 调用 scheduling、policy 和 resource 校验；`coordinator.rs` 不实现第二个 Agent loop。`core/harness/agent_loop.rs` 与 `runtime/driver/native.rs` 组成步进机 + I/O driver，步进接口及先提交后执行的顺序见 [模块设计第 5.7 节](REIN-MODULES.md#harness-session-step)。

## 3. agentmux：外部 Harness 接入

```text
agentmux/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── registry.rs                    # adapter ID → 实现；不决定任务路由
│   ├── error.rs                       # 协议/版本/能力不支持等错误
│   ├── capabilities.rs                # 版本×平台×接入模式×授权模式能力报告
│   ├── ports/
│   │   ├── mod.rs
│   │   ├── process.rs                 # ProcessPort 与 LaunchRecipe
│   │   ├── transport.rs               # TransportPort 与有界通道
│   │   └── private_profile.rs         # PrivateProfilePort
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── correlation.rs             # 请求、会话、turn、tool call 关联
│   │   ├── framing.rs                 # 公共帧边界/尺寸规则
│   │   └── redaction.rs               # 对外观察的脱敏规则
│   ├── profiles/
│   │   ├── mod.rs
│   │   ├── recipe.rs                  # 私有运行配置配方；不直接写磁盘
│   │   ├── reconciliation.rs          # 配置/身份差异和核对结果
│   │   ├── codex.rs                   # Codex 原生配置格式
│   │   └── claude.rs                  # Claude Code 原生配置格式
│   └── adapters/
│       ├── mod.rs
│       ├── fake.rs                    # test-support 功能下的确定性假适配器
│       ├── codex/
│       │   ├── mod.rs
│       │   ├── adapter.rs             # AgentAdapter 实现
│       │   ├── probe.rs               # 实际版本/模式/能力探测
│       │   ├── protocol.rs            # 原生请求和事件编解码
│       │   ├── session.rs             # 原生会话控制与恢复引用
│       │   ├── events.rs              # 原生事件 → AdapterObservation
│       │   ├── approval.rs            # 原生批准往返及 unsupported 分支
│       │   └── usage.rs               # 用量/额度观测；不推测未知数据
│       └── claude/
│           ├── mod.rs
│           ├── adapter.rs
│           ├── probe.rs
│           ├── protocol.rs
│           ├── session.rs
│           ├── events.rs
│           ├── approval.rs
│           └── usage.rs
└── tests/
    ├── support/
    │   └── mod.rs                     # 内存进程/通道端口
    ├── adapter_contract.rs
    ├── codex_protocol.rs
    ├── claude_protocol.rs
    ├── capability_matrix.rs
    ├── profile_codec.rs
    ├── approval_roundtrip.rs
    └── malformed_stream.rs
```

上述文件表示要处理的能力边界，不承诺某外部 CLI 已支持完整恢复、批准或额度查询。选定接入模式后，实测不支持的能力必须明确返回 `unsupported/unknown`。`protocol.rs` 在单一模式足够时保持一个文件，多模式有实测需求时再拆。

Fake adapter 供合同和故障基准使用，默认产品构建不自动注册。原生 Rein 的 `NativeAgentAdapter` 在 `core/harness`，通过 runtime 装配接入执行入口。

## 4. runtime：装配、服务与具体 I/O

```text
runtime/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── bin/
│   │   ├── rein.rs                    # CLI 入口
│   │   └── reind.rs                   # 持久服务入口
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs                    # 命令行参数
│   │   ├── commands.rs                # 控制客户端命令；嵌入模式亦走同一应用入口
│   │   └── output.rs                  # 终端/JSON 输出；不判任务终态
│   ├── composition/
│   │   ├── mod.rs
│   │   ├── config.rs                  # 服务配置、路径和能力开关
│   │   ├── bootstrap.rs               # 数据迁移、恢复核对和启动顺序
│   │   ├── native.rs                  # 注入 ModelAdapter/ToolExecutor
│   │   ├── adapters.rs                # 外部适配器与低层端口装配
│   │   ├── delegation.rs              # DelegationPort → TaskApplication
│   │   └── shutdown.rs                # 停止接单、检查点与受管资源收尾
│   ├── driver/
│   │   ├── mod.rs
│   │   ├── dispatcher.rs              # 执行已持久化的效果意图
│   │   ├── native.rs                  # 执行已提交意图→送回观察；每步经 TaskApplication 提交会话进度
│   │   ├── observations.rs            # 接收观察、去重并送回 core
│   │   ├── reconciliation.rs         # 查询实际进程/会话/产物事实
│   │   └── timers.rs                  # 定时唤醒；判断规则由 core 拥有
│   ├── control/
│   │   ├── mod.rs
│   │   ├── server.rs                  # 本地服务监听与连接管理
│   │   ├── client.rs                  # CLI/其他客户端连接
│   │   ├── auth.rs                    # 控制主体与调用权限上下文
│   │   ├── dto.rs                     # 权威 Rust wire 信封/控制 DTO；引用 core 事件类型，不复制 payload
│   │   ├── handlers.rs                # 请求 → Command/Query
│   │   └── subscription.rs            # 事件订阅、游标过期和快照重连
│   ├── store/
│   │   ├── mod.rs
│   │   ├── sqlite.rs                  # 数据库连接与事务
│   │   ├── state.rs                   # 业务快照读写和 revision 比较
│   │   ├── sessions.rs                # 会话/检查点只存摘要引用、游标、revision 与控制元数据
│   │   ├── journal.rs                 # 持久事件及有序续读
│   │   ├── outbox.rs                  # 已提交效果的投递记录
│   │   ├── inbox.rs                   # 外部事件接收与去重
│   │   ├── idempotency.rs             # 命令/派发/调用幂等键
│   │   └── migrations.rs              # schema 版本与迁移执行
│   ├── process/
│   │   ├── mod.rs
│   │   ├── supervisor.rs              # 受管 spawn、回收和进程组控制
│   │   ├── identity.rs                # PID + 启动标识，防 PID 复用
│   │   ├── limits.rs                  # 输出/时间/受支持的 OS 资源限制
│   │   └── unix.rs                    # 首批 macOS/Linux 平台实现
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── stdio.rs                   # 子进程 stdio 通道
│   │   ├── local_socket.rs            # 本地控制通道
│   │   └── buffering.rs               # 有界缓冲、背压与断流回执
│   ├── credentials/
│   │   ├── mod.rs
│   │   ├── broker.rs                  # SecretRef 解析、受控句柄与租约
│   │   ├── environment.rs             # 显式环境凭证来源
│   │   ├── keychain.rs                # macOS 原生秘密后端，平台功能开关
│   │   ├── private_profile.rs         # PrivateProfilePort 的文件/权限实现
│   │   └── rotation.rs                # generation 比较与更新回执
│   ├── models/
│   │   ├── mod.rs
│   │   ├── registry.rs                # 已选模型配置 → ModelAdapter
│   │   ├── http.rs                    # 请求、超时和有界响应读取
│   │   └── openai.rs                  # 首个原生模型端口实现；兼容抽取已有路径
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── executor.rs                # 已获准调用 → 具体工具或宿主
│   │   ├── filesystem.rs              # 有界文件读取与元数据 I/O
│   │   ├── patch.rs                   # 按候选/基线约束执行写入并回读
│   │   ├── node_host.rs               # Node 单调用传输；经 process/supervisor 启动，不自建进程管理
│   │   └── host_locator.rs            # 显式宿主路径/版本，避免固定源码相对路径
│   ├── workspace/
│   │   ├── mod.rs
│   │   ├── manager.rs                 # 工作区生命周期
│   │   ├── snapshot.rs                # 输入版本快照
│   │   ├── locks.rs                   # OS 写锁与租约核对
│   │   ├── changeset.rs               # 实际文件差异收集
│   │   └── integration.rs             # 已授权整合和回读回执
│   ├── artifact/
│   │   ├── mod.rs
│   │   ├── store.rs                   # 按内容摘要保存不可变字节
│   │   ├── manifest.rs                # 产物清单编码与完整性核对
│   │   └── collection.rs              # 冻结边界、路径校验和产物封存
│   ├── verification/
│   │   ├── mod.rs
│   │   ├── runner.rs                  # 执行固定 VerificationPlan
│   │   ├── environment.rs             # 输入与验证环境指纹
│   │   └── receipt.rs                 # stdout/stderr/退出码/摘要回执
│   └── telemetry/
│       ├── mod.rs
│       ├── logging.rs                 # 运维日志与敏感值过滤
│       └── metrics.rs                 # 队列、耗时和资源指标；非预算权威
├── migrations/
│   └── 0001_initial.sql               # 首个冻结持久模型；后续升级只新增迁移
└── tests/
    ├── support/
    │   └── mod.rs                     # 临时状态目录、假进程和固定模型
    ├── native_standalone.rs
    ├── coordinator_roundtrip.rs
    ├── coordinator_recovery.rs
    ├── coordinator_liveness.rs
    ├── control_api.rs
    ├── event_resume.rs
    ├── state_transactions.rs
    ├── crash_recovery.rs
    ├── migration_compatibility.rs
    ├── profile_isolation.rs
    ├── credential_rotation.rs
    ├── shared_quota.rs
    ├── process_cancellation.rs
    ├── workspace_integration.rs
    ├── artifact_verification.rs
    ├── legacy_compatibility.rs
    ├── live_codex.rs                  # 显式启用、独立测试账号和能力报告
    └── live_claude.rs                 # 默认离线 CI 不运行
```

原生模型与外部 Harness 的选择是两种绑定：`models/registry` 不控制任务路由，`agentmux/registry` 也不决定账号调度。Coordinator 模型消耗必须进入根预算。

SQLite 是首批物理存储方案；`core/ports/store.rs` 不暴露 SQLite 类型。消息体与工具结果按内容摘要存入 `runtime/artifact`，`runtime/store/sessions.rs` 只保存引用、游标、revision 和必要控制元数据，避免每轮复制完整上下文。driver 在调用模型/工具时按需读取对应产物；先确保持久产物存在，再通过 TaskApplication 提交引用和下一意图，提交失败不得派发。

outbox 记录持久意图，外部效果无法确认时转入核对流程，不自动重放未知派发。`agents.await` 将父会话持久化为 waiting_children，提交后释放模型槽；重启从引用式检查点与结果游标继续同一 Session/Attempt，不序列化 Future，仍保留必要的进程事实、工作区锁与凭证 lease。

runtime 集成测试使用 agentmux 的 FakeAgent 时，须在拟建的 `runtime/Cargo.toml` 的 `[dev-dependencies]` 中为 `rein-agentmux` 显式开启 `test-support` feature；正常产品依赖不启用该 feature，也不依赖测试构建的 feature 合并来注册 FakeAgent。本轮不创建或修改 Cargo 文件。

## 5. 合同、夹具、示例、脚本与文档

以下列全新增产品支撑文件。既有文件/子树标明保留。

```text
contracts/
├── README.md                          # 已有，补充各协议入口
├── extension-v0.1.md                  # 已有，保留语义
├── extension-v0.2.md                  # 已有，保留语义
├── maintenance-v1.md                  # 已有，保留单文件维护约束
├── maintenance-session-v1.md          # 已有，保留会话维护约束
└── runtime/
    ├── README.md                      # 控制/Agent/工具协议的范围和权威来源
    ├── versioning.md
    ├── control-api.md
    ├── agent-adapter.md
    ├── native-harness.md
    ├── delegation.md
    ├── lifecycle.md
    ├── profile-isolation.md
    ├── evidence.md
    ├── schemas/
    │   ├── common.schema.json
    │   ├── task-spec.schema.json
    │   ├── execution.schema.json
    │   ├── harness-session.schema.json # 生成的引用式会话交换格式；不内嵌完整消息体
    │   ├── delegation.schema.json
    │   ├── capability-report.schema.json
    │   ├── profile.schema.json
    │   ├── resource.schema.json
    │   ├── approval.schema.json
    │   ├── context-bundle.schema.json
    │   ├── artifact.schema.json
    │   ├── evidence.schema.json
    │   ├── control.schema.json
    │   └── event.schema.json           # 由权威 Rust 事件/信封类型生成
    └── examples/
        ├── native-task.json
        ├── coordinator-task.json
        ├── delegation-result.json
        ├── external-task.json
        ├── capability-unknown.json
        ├── approval-request.json
        ├── outcome-unknown.json
        └── stale-evidence.json

fixtures/
├── cases/                             # 已有，保留
├── responses/                         # 已有，保留
├── workspaces/                        # 已有，保留
├── ch08-context/                      # 已有，保留
├── document-maintenance/              # 已有，保留
├── hybrid-marker.txt                  # 已有，保留
└── runtime/
    ├── README.md
    ├── manifest.json                  # 样本来源/版本/hash/脱敏及 synthetic 标记
    ├── models/
    │   ├── native-tool-roundtrip.json
    │   └── coordinator-delegation.json
    ├── agents/
    │   ├── codex-events.jsonl
    │   ├── claude-events.jsonl
    │   ├── approval-roundtrip.jsonl
    │   └── malformed-stream.jsonl
    ├── processes/
    │   ├── fixture-agent.mjs           # 受测试参数驱动的假 Agent 进程
    │   └── fixture-tool-host.mjs       # 延迟/崩溃/副作用后断流故障
    ├── profiles/
    │   ├── codex.template.toml         # 无真实账号与秘密
    │   └── claude.template.json
    ├── scenarios/
    │   ├── native-standalone.json
    │   ├── parent-child-roundtrip.json
    │   ├── parent-wait-restart.json
    │   ├── duplicate-child-result.json
    │   ├── dispatch-outcome-unknown.json
    │   ├── cancellation-tree.json
    │   ├── shared-quota-contention.json
    │   ├── invalid-delegation.json
    │   └── stale-acceptance.json
    ├── workspaces/
    │   └── single-file/
    │       ├── input.txt
    │       └── expected.txt
    └── migrations/
        └── v1-seed.sql

examples/
├── ch08-context-methods/              # 已有，保留完整子树
├── document-maintenance/              # 已有，保留完整子树
└── runtime/
    ├── README.md
    ├── rein.example.toml              # 产品配置示例；只引用秘密，不含秘密
    ├── native-agent/
    │   ├── README.md
    │   └── task.json
    ├── coordinator/
    │   ├── README.md
    │   └── task.json
    ├── external-agent/
    │   ├── README.md
    │   └── task.json
    └── verified-dag/
        ├── README.md
        ├── task.json
        └── verification.json

scripts/
├── book-check.mjs                     # 已有，保留
├── book-generate.mjs                  # 已有，保留
├── book-links.mjs                     # 已有，保留
├── book-toc.mjs                       # 已有，保留
├── ch05-compare.mjs                   # 已有，保留
├── ch06-compare.mjs                   # 已有，保留
├── ch07-compare.mjs                   # 已有，保留
├── ch08-compare.mjs                   # 已有，保留
├── ch08-verify.mjs                    # 已有，保留
├── check-book-links.mjs               # 已有，保留
├── maintenance-fixture.mjs            # 已有，保留
└── runtime/
    ├── check-boundaries.mjs           # crate/core/runtime 内部边界；harness/orchestration 禁直引 execution/acceptance 状态变更接口，只经 application 或 ports
    ├── check-contracts.mjs            # 只核对 Rust→schemars schema→TS 与样本的一致性，不手写第二套
    ├── check-compatibility.mjs        # 原有 Rust/TS 命令按基线复跑
    ├── run-offline.mjs                # 离线合同、故障与纵向检查
    └── run-live.mjs                   # 显式真实 Agent 测试入口和证据保存

docs/
├── .vitepress/                        # 已有，保留；后续增产品导航
├── chapters/                          # 已有，保留
├── readings/                          # 已有，保留
├── milestones/                        # 已有，保留
├── appendices/                        # 已有，保留
├── public/                            # 已有，保留
├── （现有页面文件继续保留）
└── runtime/
    ├── index.md
    ├── quickstart.md
    ├── architecture.md                # 面向使用者的概览，引用根目录设计
    ├── native-agent.md
    ├── coordinator.md
    ├── external-agents.md
    ├── profiles.md
    ├── approvals.md
    ├── verification.md
    ├── recovery.md
    ├── cli.md
    ├── configuration.md
    └── development.md
```

**Rust 类型为权威，最迟 R0 冻结生成合同：Rust → schemars → `contracts/runtime/schemas/*.json` → TypeScript 类型。** `core/events/domain.rs` 定义领域事件 payload；`runtime/control/dto.rs` 只定义 wire 信封/控制 DTO 并引用该 payload，不复制一套事件定义。`event.schema.json` 从这些 Rust 类型生成。schema 与 TypeScript 均为派生物，`scripts/runtime/check-contracts.mjs` 只核对重新生成结果、规范样本与现有产物的一致性，不手写第二套类型，也不负责修改权威定义。

`harness-session.schema.json` 交换的是消息/工具结果的摘要引用、游标、revision 与必要控制元数据；它不内嵌完整消息体，不公开全部数据库行或秘密字段。schemars 版本、生成入口和 schema → TypeScript 工具在 R0 冻结；这里固定方向，不宣称生成链路已实现。

真实 CLI 样本记录版本、平台、接入方式和脱敏过程；合成样本明确标为 synthetic。真实测试工作流仅在明确配置的账号/资源下触发，其结果不能用离线假进程测试替代。

## 6. 现有代码与教学资产怎样保留

```text
rust/                                 # 独立教学 package；根 workspace 排除
├── Cargo.toml
├── Cargo.lock
├── README.md
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── README.md
│   ├── bin/
│   │   └── hello.rs
│   └── rein/
│       ├── mod.rs                     # 后续按兼容合同逐项 wrapper/re-export
│       ├── loop.rs
│       ├── context_methods.rs
│       ├── stdio_executor.rs
│       ├── extension02.rs
│       ├── extension02_executor.rs
│       ├── maintenance.rs
│       └── maintenance_session.rs
├── examples/                          # 全部既有章节入口保留
└── tests/                             # 原基准保留；抽取后继续按原输入运行

ts/
├── package.json
├── README.md
├── src/
│   ├── README.md
│   ├── main.ts
│   ├── config.ts
│   ├── errors.ts
│   ├── chat.ts
│   ├── transport.ts
│   ├── hello.ts
│   ├── hello-safe.ts
│   ├── hybrid-host.ts
│   └── rein/
│       ├── adapters.ts
│       ├── contracts.ts
│       ├── loop.ts                    # 教学参考，不形成第二套产品状态权威
│       ├── readonly.ts
│       ├── extension02-host.ts        # 现有 Node 单调用 reference host
│       └── maintenance-tools.ts
├── examples/                          # 保留
├── scripts/                           # 保留
├── tests/                             # 保留
└── （现有配置及锁文件继续保留）

book/
└── chapters.json                      # 保留 slug、顺序与历史来源语义

chapter-snapshots/                     # 保留所有现存文件及其历史字节
records/                               # 保留历史基线/验收条目
reports/                               # 保留历史报告与证据；新检查另建报告
```

当前教学源码不是整体移动到 core。迁移按职责抽取：`loop.rs` 的 async 循环改为 core 纯步进机 + runtime driver；HTTP、文件和子进程实现进入 runtime；旧 async 入口暂通过兼容桥保持原命令与行为。双 Cargo.lock 的独立解析、版本偏差、重编译成本及 bridge 退出条件见 [MIGRATIONS：旧教学包兼容桥](MIGRATIONS.md#rein-core-bridge)。已固化的章节快照保持历史版本；维护扩展继续遵守当前单文件候选、批准消费和回读合同。

## 7. 跨项目接入与后续扩展目录

Web Studio 最小接入属于 R1c：本仓库维护 `contracts/runtime/`、runtime 控制与验证边界，Swift 客户端在 Web Studio 仓库；不在 Rein 新建 UI 包。Veriflow adapter 放在 `runtime/src/verification/` 下，复用固定版本规则包并保存不可变投影与原始回执，具体文件按首轮规模合并。服务独立持有任务状态，客户端不直接读取或修改数据库。首个网页 fixture 与 INT 基准的语义以整合规格为准，R0 冻结实际输入和路径后才创建。

以下额外能力不属于首批 P0 完成要求，也不预建空壳。开启具体能力时再展开文件、冻结协议并添加基准。

| 后续能力 | 预留位置 | 边界 |
| --- | --- | --- |
| pi 外部 Harness | `agentmux/src/adapters/pi/`、`agentmux/src/profiles/pi.rs` | 先核实版本、接入方式和能力；不把 pi 原生能力扩大解释为 Rein 已支持的多 Agent 路由 |
| Gemini CLI | `agentmux/src/adapters/gemini/`、`agentmux/src/profiles/gemini.rs` | 复用 AgentAdapter 合同，补独立协议样本与实测 |
| Anthropic/更多模型 | `runtime/src/models/anthropic.rs`、其他 provider 文件 | 原生 ModelAdapter；与 Claude Code Harness adapter 分开 |
| 本地模型 | `runtime/src/models/local.rs` | 注入原生 Harness；额度/能力照常建模 |
| 学习路由 | `core/src/scheduling/scoring.rs`、`evaluation.rs` | 保持硬准入、解释和确定性回退；需对照任务集 |
| MCP 工具客户端 | `runtime/src/tools/mcp/` | 原生 Harness 使用外部工具，遵守 ToolExecutor 边界 |
| Rein MCP 控制入口 | `runtime/src/control/mcp/` | 外部调用 Rein 的 Command/Query；与工具客户端是相反方向 |
| 扩展 SDK | `packages/extension-sdk/`、`packages/extension-host/` | Node/TS SDK 与可分发宿主；需另做包边界和发布设计 |
| 远程执行 | `agentmux/src/adapters/remote/`、`runtime/src/transport/remote/` | 协议/身份/连接和断线核对分别归属，不直接暴露数据库 |
| Windows | `runtime/src/process/windows.rs` 等平台实现 | 另验进程树、取消、凭证与路径行为 |

首轮不新增 `harness/`、`modelmux/`、`gateway/`、`ui/` 顶层产品包。未来只有独立依赖、分发或生命周期需要时再增加 crate/package。

## 8. 运行时数据目录：与源码分开

以下用 `<config-dir>` 和 `<state-dir>` 表示显式配置的路径；最终平台默认值与环境变量名在 CLI 合同实施时确定。它们不是本仓库要提交的目录。

```text
<config-dir>/
└── rein.toml                          # 用户配置；秘密使用引用

<state-dir>/
├── state.sqlite                       # 任务/会话/账本/journal/outbox/inbox
├── state.sqlite-wal                   # SQLite 启用 WAL 时生成
├── state.sqlite-shm                   # SQLite 启用 WAL 时生成
├── control.sock                       # Unix 本地控制监听；启动时生成
├── artifacts/
│   └── sha256/<prefix>/<digest>       # 不可变产物、清单和验证回执
├── workspaces/
│   └── <workspace-id>/                # 受管工作目录
├── private-profiles/
│   └── <profile-id>/<generation>/     # 按 lease 实例化的私有原生配置
├── processes/
│   └── <process-id>.json              # 进程身份/受管句柄核对信息
├── logs/
│   ├── runtime.jsonl                  # 运维日志；不替代领域 journal
│   └── agents/<attempt-id>/           # 受限的原始流和封存索引
└── tmp/                               # 可核对和清理的临时数据
```

秘密主存储使用声明的秘密后端；确需原生凭证文件的集成模式，由 credentials 在私有目录按权限与 lease 生成，不能写入仓库示例或通用日志。配置、原始流和产物的访问/保留策略由合同约束。原生会话检查点在持久 store 中保存，不再另建一套权威 session JSON 文件。

## 9. 依赖与分阶段创建规则

```text
rein-core                         # Harness + Coordinator + 领域控制
    ↑                 ↑
rein-agentmux         │           # 依赖 core 的外部适配器
    ↑                 │
rein-runtime ─────────┘           # 依赖 core 和 agentmux，注入具体 I/O
```

| 阶段 | 主要创建范围 | 结果 |
| --- | --- | --- |
| R0 | 步进/引用式检查点/正式 DTO 合同；Rust → schemars schema → TS；`contracts/runtime`、`fixtures/runtime` 的基准 | 明确权威类型、原生/外部能力、状态与验证输入 |
| R1a | workspace、`core/harness` 纯步进与最小领域/应用/端口、runtime I/O driver/存储/工具/CLI | 逐步提交的原生闭环及原教学兼容 |
| R1b | `core/orchestration`、DelegationPort、FakeAgent 和首个真实外部 adapter | 父委派、子反馈、父继续及故障恢复 |
| R1c | runtime 控制/验证 adapter、网页 fixture、任务交付；Web Studio 仓库内的最小消费者 | INT-S01–S07 / INT-B01–INT-B12，网页与原生 UI 分别验收 |
| R2 | 第二真实 adapter、多 Profile/共享池/并发预算相关文件 | 多主体隔离、资源竞争与根预算核算 |
| R3 | 完整 graph/handoff/acceptance/repair 与整合验证路径 | P0 适用 V01–V22 场景全部取得证据 |
| R4 | Web Studio 扩展体验及三产品打包、安装、兼容与发行路径 | INT-S08 及各产品安装/交互证据 |
| R5 | 第 7 节已立项的额外能力 | 独立合同与回归，按需求增加文件 |

目录表达所有权，验收依靠实际依赖图、接口检查和执行证据。**R1a 实际创建文件数目标约 25 个，文件树是职责地图而不是首轮清单。** 第一轮可以将紧密相关的小类型放在同一文件，随着实现规模再按本表拆分；不会为了符合文件树预先制造空模块。

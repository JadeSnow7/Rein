# 状态拆分规格：RunResult、VerificationResult、AcceptanceDecision

版本：`RESULT-SPLIT-1`（设计稿），2026-10-02。来源：边界审计的 P1 与[复核](2026-10-02-boundary-audit-review.md)第 3 节。状态：**设计稿，未实施**；类型草图没有编译过，基准没有运行过。行号对应分支 `codex/ch01-helloworld`、HEAD `1dfa82b` 的工作树。

本规格不依赖 [D19](../DECISIONS.md) 是否确认。它修的是实现与现有设计之间的偏差：`REIN-MODULES.md` §4.3 要求 Agent 输出、验证器结果、是否采纳是三个来源，R1a 首个增量把后两者合成了一个 `Accepted`。

## 1. 要解决的问题

| # | 现状 | 位置 |
| --- | --- | --- |
| 1 | 验证 `Passed` 直接把会话置为 `Accepted` | `core/src/lib.rs:351-354` |
| 2 | 预算耗尽、工具失败、验证失败、验证无法判定都落到 `Completed`，只看状态分不出来 | `core/src/lib.rs:300`、`:344`、`:353` |
| 3 | 工具失败的 `error_ref` 没进会话 | `core/src/lib.rs:339` |
| 4 | 验证计划必填，最终回答无条件进入验证 | `core/src/lib.rs:96`、`:284-296` |
| 5 | `acceptance` 表单列、无键、全表覆盖，存的是验证状态 | `runtime/src/lib.rs:108`、`:398-406` |
| 6 | `runtime::RunResult { answer, acceptance }`、`run_to_acceptance`、CLI 顶层 `"acceptance"` 键沿用同一混淆 | `runtime/src/lib.rs:432`、`:930`；`main.rs:86`、`:97`、`:109`、`:117` |
| 7 | 验证器写死为 `FixedVerifier`，恢复时也按它解析计划 | `runtime/src/lib.rs:557`、`:628`、`:719`、`:872` |
| 8 | `VerificationReceipt` 不在生成的 schema 里；会话与数据库都没有版本标记 | `runtime/src/lib.rs:1009-1056` |

## 2. 目标与非目标

目标：

- G1　Rein 的类型、数据库、CLI 输出和 schema 中不再出现“接受”语义。
- G2　只读 RunResult 就能区分运行为什么结束。
- G3　验证结果是一条带绑定的独立记录：哪次运行、哪份计划、哪个产物、哪份回执。
- G4　验证变为可选效果；没有计划时运行正常结束。
- G5　离线演示保留，但作为显式命名的验证 profile，通过端口装配。
- G6　对外类型带版本；旧 state 目录被明确拒绝，不被新代码误读。

非目标：真实模型接入；模型与工具的端口化（审计 P2）；工具失败回填模型后继续循环；修复循环；Control API；Veriflow 侧的实现。AcceptanceDecision 只给出 Rein 要求的绑定字段，类型本身归外层。

## 3. 类型设计

### 3.1 core

```rust
/// 取代 SessionStatus。没有 Accepted，也没有含义模糊的 Completed。
pub enum RunStatus {
    Running,         // 还可能产生效果，含等待验证
    Finished,        // 本次运行不会再产生效果
    Cancelled,
    OutcomeUnknown,  // 有效果结果未知，等待核对
}

/// 运行为什么不再继续调用模型。在模型给出最终回答或被迫停止时写入。
pub enum StopReason {
    FinalAnswer,
    ToolBudgetExhausted,
    ToolFailed { call_id: String, error_ref: ArtifactRef },
}

pub struct VerificationRecord {
    pub plan_ref: ArtifactRef,
    pub subject_ref: ArtifactRef,   // 被验证的产物，即当时的 final_ref
    pub receipt_ref: ArtifactRef,
    pub status: VerificationStatus, // Passed | Failed | Undetermined，沿用现有枚举
}

pub enum VerificationState {
    NotRequested,                 // 本次运行没有验证计划
    NotRun,                       // 有计划但没执行到（预算耗尽、工具失败、取消）
    Recorded(VerificationRecord),
}
```

`HarnessSession` 的字段变化：

| 字段 | 变化 |
| --- | --- |
| `status: SessionStatus` | 改为 `RunStatus` |
| `verification_plan_ref: ArtifactRef` | 改为 `Option<ArtifactRef>` |
| `stop_reason: Option<StopReason>` | 新增 |
| `verification: Option<VerificationRecord>` | 新增 |
| `unknown_effects: Vec<String>` | 新增，见第 7 节决定 D |

`RunResult` 是从会话算出来的只读视图，不单独存储，避免出现第二份状态：

```rust
pub struct RunResult {
    pub schema: String,             // "rein.run-result/1"
    pub session_id: SessionId,
    pub run_id: RunId,
    pub attempt_id: AttemptId,
    pub revision: u64,
    pub status: RunStatus,          // 不会是 Running
    pub stop_reason: Option<StopReason>,
    pub final_ref: Option<ArtifactRef>,
    pub tool_budget_remaining: u32,
    pub unreconciled_effects: Vec<String>,
    pub verification: VerificationState,
}

impl RunResult {
    /// 会话仍在 Running 时返回 None。
    pub fn from_session(s: &HarnessSession) -> Option<RunResult>;
}
```

### 3.2 转换表

只列语义变化的分支，其余（身份校验、去重、revision 检查、Start、ToolResult）不变。

| 观察 | 条件 | 现在 | 改后 |
| --- | --- | --- | --- |
| `ModelTurn` 最终回答 | 有验证计划 | 产生 `Verify`，保持 `Running` | 同左，并写入 `stop_reason = FinalAnswer` |
| `ModelTurn` 最终回答 | 无验证计划 | 不存在此路径 | `Finished`，`stop_reason = FinalAnswer`，不产生效果 |
| `ModelTurn` 工具调用 | 预算为 0 | `Completed` | `Finished`，`stop_reason = ToolBudgetExhausted` |
| `ToolFailed` | 调用匹配 | `Completed`，丢弃 `error_ref` | `Finished`，`stop_reason = ToolFailed { call_id, error_ref }` |
| `VerifierResult` | 任意结论 | `Passed` → `Accepted`，否则 `Completed` | 一律 `Finished`，写入 `verification` 记录；结论只体现在记录里 |
| `Cancel` | `Finished` 或 `Cancelled` | 拒绝（`Terminal`） | 不变 |
| `Cancel` | 等待验证中 | `Cancelled`，迟到的验证结果被拒绝 | 不变；`stop_reason` 保留 `FinalAnswer`，`verification` 为空 |
| `OutcomeUnknown` | 有活动效果 | `OutcomeUnknown` | 不变，并把效果 ID 记入 `unknown_effects` |

`VerifierResult` 观察需要多带 `plan_ref` 与 `subject_ref`，或由 core 从会话里取当前的计划引用和 `final_ref` 填入记录。建议后者：记录里的绑定以会话状态为准，不信任观察自报；回执内容与会话是否一致，继续由 runtime 在提交前校验（现 `runtime/src/lib.rs:715-742`）。

### 3.3 runtime

数据库：

```sql
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);  -- ('state_schema','2')
CREATE TABLE verifications(
  run_id TEXT NOT NULL, attempt_id TEXT NOT NULL, revision INTEGER NOT NULL,
  plan_hash TEXT NOT NULL, subject_hash TEXT NOT NULL, receipt_hash TEXT NOT NULL,
  status TEXT NOT NULL,
  PRIMARY KEY(run_id, attempt_id, receipt_hash)
);
```

- 删除 `acceptance` 表；`verifications` 只追加，与状态转换在同一事务写入（沿用现在回滚测试覆盖的保证）。
- `sessions.state` 的取值改为 `running`、`finished`、`cancelled`、`unknown`。
- 打开没有 `meta.state_schema = 2` 的目录时返回明确错误，不迁移、不修改原文件。理由见第 7 节决定 C。

接口：

- 删除 `runtime::RunResult` 与 `run_to_acceptance`，新增 `run_to_end() -> rein_core::RunResult`；读取最终回答字节另给一个辅助方法。
- `SqliteStore::acceptance()` 改为 `verification() -> Option<VerificationRecord>`；`RuntimeSnapshot.acceptance` 改为 `run: Option<RunResult>`。
- CLI 四个命令的输出去掉顶层 `"acceptance"`，改为 `"run": <RunResult 或 null>`；`session` 与 `effects` 保留。

验证器端口：

```rust
pub trait Verifier {
    fn profile(&self) -> &'static str;                     // 如 "fixed-bytes-v1"
    fn plan_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>>;
    fn run(&self, plan_ref: &ArtifactRef, subject_ref: &ArtifactRef, subject: &[u8])
        -> Result<VerificationReceipt, Box<dyn Error>>;
    fn validate_receipt(&self, receipt: &VerificationReceipt, plan_ref: &ArtifactRef,
        subject_ref: &ArtifactRef, subject: &[u8]) -> Result<(), Box<dyn Error>>;
}
```

- `FixedVerifier` 实现该 trait，作为 `fixed-bytes-v1` profile 保留；`Runtime::open` 接收 `Option<Box<dyn Verifier>>`。
- 恢复时按计划里的 `version` 字段查 profile 注册表来重建验证器，取代三处写死的 `FixedVerifier::from_plan_bytes`。未知 profile 是明确错误。
- `VerificationReceipt` 保留 `version` 作为 profile 判别字段，加入 schema 生成。

合同生成：在现有四份之外增加 `run-result` 与 `verification-receipt`，同步修改 `runtime/src/lib.rs` 的 `schema::generate`、`schema::check` 和 `scripts/runtime/check-contracts.mjs` 里的三处名单。

### 3.4 AcceptanceDecision（归外层，这里只定绑定）

Rein 不存储、不产生、不读取接受决定。为了让外层的决定能绑定到确定的版本，Rein 保证 RunResult 提供下列可引用的身份，外层决定必须引用它们：

```text
AcceptanceDecision（由 Veriflow 或其他外层定义）
  subject   : run_id, attempt_id, revision, final_ref      ← 取自 RunResult
  evidence[]: plan_ref, subject_ref, receipt_ref, status   ← 取自 VerificationRecord
  decision  : accepted | rejected | needs_repair | deferred
  decided_by: human 或 policy，及其标识与策略版本
  reason, decided_at, supersedes
```

有效性规则由外层执行：`evidence[].subject_ref` 必须等于 `subject.final_ref`；任一引用与当前 RunResult 不符则该决定过期，旧决定保留为历史。验证 `Passed` 而没有决定时，任务状态是“已验证、未接受”。

## 4. 受影响的现有断言

改实现之前先出基准修订，说明这是合同缺陷（把验证通过命名为接受），并保留原始结果。依据是 `records/REIN-RUNTIME-R1A-20260921/implementation-spec.md` 第 28 行。

| 文件 | 位置 | 现断言 | 改后 |
| --- | --- | --- | --- |
| `fixtures/runtime-r1a/baseline.json` | B01、B03、B04、B07 的 `expected` | `acceptance: passed / not_passed` 等 | 改用 `verification` 与 `status` 表述，出新版本 |
| `core/src/lib.rs` 单元测试 | `:585`、`:732` | `SessionStatus::Accepted` | `RunStatus::Finished` 加验证记录为 `Passed` |
| `runtime/tests/cli.rs` | `:34-35`、`:78`、`:105` | `status == "Accepted"`、`json["acceptance"]` | `run.status == "Finished"`、`run.verification` |
| `runtime/tests/control.rs` | `:53`、`:175`、`:229`、`:301-355` | `state() == "accepted"`、`acceptance()`、`acceptance` 表触发器 | `"finished"`、`verification()`、触发器改挂 `verifications` 表 |
| `runtime/tests/r1a_acceptance.rs` | `:57-62`、`:89`、`:126-136`、`:173`、`:316`、`:359-379` | `run_to_acceptance`、`Accepted`、`.acceptance` | `run_to_end`、`Finished`、`.verification` |
| `runtime/tests/durable_boundaries.rs` | `:44-45`、`:62-63`、`:84`、`:122-124` | 同上 | 同上 |
| `contracts/runtime/schemas/*` | 四份 JSON 与四份 TS | 含 `Accepted`、`Completed` | 重新生成，新增两份 |
| `runtime/README.md` | 命令输出与取消说明 | 按旧输出描述，取消说明里用“验收”指代验证通过 | 按新输出改写 |
| `architecture/decisions.json` | `DEC-CORE-BOUNDARY` | 当前摘要匹配 | 改代码后变为 `review_required`；按 D16 新增一版登记，不刷新旧摘要 |

## 5. 新增基准

实现前先写、先跑、先记录失败。每条都要能区分正确与错误实现。

| ID | 输入 | 预期 |
| --- | --- | --- |
| S1 | 验证通过的完整运行 | `status = Finished`，`stop_reason = FinalAnswer`，验证记录为 `Passed`；会话 JSON、CLI 输出、六份 schema 中都没有字符串 `Accepted` |
| S2 | 最终回答与预期不同；验证器无预期 | 两者都是 `Finished` 加 `FinalAnswer`，验证记录分别为 `Failed`、`Undetermined` |
| S3 | 预算为 0；工具读取越界路径 | 分别为 `ToolBudgetExhausted`、`ToolFailed`（带 `call_id` 与可读取的 `error_ref`）；`verification = NotRun`；不查事件表、只看 RunResult 就能把 S1、S2、S3 的五种结局互相区分 |
| S4 | 不提供验证计划 | `Finished`、`FinalAnswer`、`verification = NotRequested`；outbox 里 `Verify` 效果为 0 条 |
| S5 | 等待验证时取消，之后送入合法的 `Passed` 回执 | `Cancelled`；迟到观察被拒绝；`verifications` 表无行；计数不变 |
| S6 | 回执指向别的产物或别的计划；两个独立 state 目录各跑一次 | 前者提交被拒绝且无半条状态；后者两条记录的 run、计划、产物摘要各自对应，互不覆盖 |
| S7 | 用新程序打开旧版 state 目录（含 `acceptance` 表、无 `meta`） | `show`、`resume`、`cancel` 都返回明确的版本错误；目录内全部文件的摘要前后一致 |
| S8 | 生成并检查合同；人为改动 `run-result.json` 后再检查 | 正常检查通过；漂移检查非零退出；还原后通过；TS 类型可编译 |
| S9 | 效果被标为结果未知后取消（仅在采纳决定 D 时） | `status = Cancelled` 且 `unreconciled_effects` 含该效果 ID |
| S10 | `verification` 写入失败（触发器拒写） | 整个验证转换回滚，会话仍在等待验证，无半条记录（现 `control.rs:301` 的等价改写） |

“验证通过但尚未接受”在 Rein 侧由 S1 覆盖：结果里只有验证记录，没有任何接受字段。决定过期（引用了旧 `final_ref`）的检查属于外层，放进 Veriflow adapter 的基准，不在本规格内。

## 6. 实施顺序

每步一个可单独审阅的提交；第 1 步不含生产代码。

1. 基准修订说明、`baseline.json` 新版本、S1–S10 测试（预期全部失败），保存实现前结果。
2. core：新类型、`advance` 中第 3.2 节列出的分支、`RunResult::from_session`、单元测试。
3. runtime 存储：`meta` 与 `verifications` 表、版本检查、提交事务、`verification()`。
4. runtime 验证器端口：`Verifier` trait、`fixed-bytes-v1` profile、按计划版本重建。
5. runtime 接口与 CLI：`run_to_end`、快照、四个命令的输出。
6. 重新生成合同，更新 `check-contracts.mjs` 名单与 `runtime/README.md`。
7. 同一基准复跑；`architecture/decisions.json` 新增登记；在 `MIGRATIONS.md` 记录输出合同与 state 版本变化。

第 2 步之后 runtime 不能编译是预期内的，第 2–5 步需要在同一分支连续完成后才能跑通 workspace 测试。

## 7. 需要用户决定的四件事

| # | 问题 | 建议 | 理由 |
| --- | --- | --- | --- |
| A | 终态叫 `Finished` 还是沿用 `Completed` | `Finished` | `REIN-DESIGN.md` §8.2 的 `completed` 含义正是“执行结束、尚未验证”，但现代码里 `Completed` 的实际含义是“结束且未被接受”，已有测试和 schema 依赖它。换名加版本号，旧消费者会报错而不是悄悄读错 |
| B | 验证留在运行内作为可选效果，还是完全移到外层 | 留在运行内 | 验证也是一次有副作用风险的执行，需要 outbox 的派发计数、结果未知和取消语义；移出去等于让外层重做一遍。外层仍可不给计划、自己另行验证 |
| C | 旧 state 目录：拒绝还是迁移 | 拒绝 | 现有 state 只来自离线演示和测试临时目录，`runtime/README.md` 也只把它描述为首个离线纵切；写迁移的成本大于价值。若已有需要保留的 state，再改为只读迁移 |
| D | `unknown_effects` 是否纳入本次 | 纳入字段，不改 outbox 行为 | 现在取消会把“结果未知”覆盖成“已取消”（`core/src/lib.rs:186-215`、`runtime/src/lib.rs:390`），目前全是只读效果所以无害。加一个只追加的会话字段不改变已固定的测试行为（`control.rs:233`），又能让 RunResult 在出现写工具之前就带上这条信息，省一次 schema 升版 |

四项都按建议处理时，本规格可以直接进入第 6 节第 1 步。

# 边界审计复核

版本：`BOUNDARY-REVIEW-1`，2026-10-02。来源：用户提供的一份只读审计结论（三条执行路径、章节归属、P0–P2 调整），要求独立复核、起草 P0 决策、设计 P1 状态拆分。配套产出：[D19（拟议）](../DECISIONS.md)、[状态拆分规格](2026-10-02-run-verification-acceptance-split.md)。

方法与限度：在分支 `codex/ch01-helloworld`、HEAD `1dfa82b` 的当前工作树上静态阅读，行号对应本次读取。逐行读了 `core/src/lib.rs`、`runtime/src/{lib,verify,main}.rs`、`DECISIONS.md`、`REIN-INTEGRATION-SPEC.md`、`MIGRATIONS.md`；对 `rust/`、`python/`、`rust-hello-world/`、CI、章节清单只核对了被引用的入口和行号。没有运行测试、构建或真实模型；没有读 Veriflow 与 Web Studio 仓库。下面的“成立”只表示引用与源码一致，不表示行为已被运行验证。

## 1. 成立的结论

| 审计结论 | 复核结果 | 依据 |
| --- | --- | --- |
| 旧决策把任务状态与 Acceptance 放在 Rein | 成立 | `DECISIONS.md:138`（D12 第三段）、`:156`（D14）；`REIN-INTEGRATION-SPEC.md:20-21`、`:57`、`:64` |
| 产品内核把验证通过直接记为 Accepted | 成立 | `core/src/lib.rs:351-354`；最终回答无条件进入 Verify（`:284-296`）；`verification_plan_ref` 为必填（`:96`） |
| `acceptance` 表存的是验证状态 | 成立 | `runtime/src/lib.rs:108`（建表）、`:398-406`（写入）、`:295-305`（读出为 `VerificationStatus`） |
| 模型、工具、验证器是写死的具体类型 | 成立 | `runtime/src/lib.rs:501-509`、`:553-558`；`reopen_owned` 固定用 `OfflineModel` 与 `FixedVerifier`（`:628`、`:639`） |
| 三套 Rust 实现未统一 | 成立 | 根 `Cargo.toml` 排除 `rust/`；`rust-hello-world/Cargo.toml` 自带 `[workspace]`；`MIGRATIONS.md:50` 兼容桥仍为计划 |
| 旧维护会话通过回调取候选，不调用 Agent loop | 成立 | `rust/src/rein/maintenance_session.rs:163-175`，文件内无 `run_agent_loop` 调用；演示标注 `offline_fixture`（`rust/examples/document_maintenance.rs:113`） |
| 第 19 章身份歧义 | 成立 | `book/chapters.json` 中 19 为 `mcp-tools`；`docs/chapters/durable-recovery.md:1` 仍题为“19 持久化执行与崩溃恢复” |
| CI 未覆盖新版 Python 与 `rust-hello-world` | 成立 | `.github/workflows/test.yml` 的触发路径和三个 job 都不含这两个目录 |
| core 不依赖 Veriflow/Web Studio | 成立 | `core/Cargo.toml` 仅 `serde`、`schemars`；`runtime/Cargo.toml` 仅 core 与存储/哈希库 |

## 2. 需要更正的地方

1. `python/hello_world/cli.py:58` 是 `make_model`，`main` 在 `:92`。调用链描述本身没错，行号指错了函数。
2. 审计把“拆开三个状态”写成新边界带来的调整。实际上现有设计已经要求三者分开，是 R1a 首个增量把它们压到了一起：`REIN-MODULES.md` §4.3 写明“Agent 输出、Verifier 退出和用户是否采纳是三个来源”；`REIN-DESIGN.md` §8.2 的 `completed` 定义为“本次执行已结束并收集产物，随后才进行任务验证”；INT-S05 要求“一致性通过不能独立生成产品 Acceptance”。所以 **P1 不依赖 P0**：无论 D19 是否确认，这项拆分都该做，可以先行。
3. 审计称“采纳新假设，需要正式记录哪些旧决策被取代”，但没有指出 D12 的核心部分并不冲突。D12 的主体是“core 保留完整 Harness，并能调度外部 Agent”；与新边界冲突的只有第三段里“任务状态……与验收仍由确定性规则提交”这一句的归属，以及 D14 中“Rein 拥有……Acceptance”。D19 按此只取代这两处。

## 3. 审计遗漏的问题

按对后续工作的影响排序。

1. **`Completed` 一个状态承载了四种结局。** 工具预算耗尽（`core/src/lib.rs:300`）、工具失败（`:344`）、验证失败和验证无法判定（`:353`）都落到 `SessionStatus::Completed`。外层要决定“重试、修复还是放弃”时，只看会话状态分不出来。这比 `Accepted` 的命名问题更直接地妨碍外层接手，拆分时必须一起解决。
2. **工具失败的错误引用没有进入会话。** `ToolFailed { call_id, .. }` 丢弃了 `error_ref`（`core/src/lib.rs:339`）；错误只留在 `observations` 表。外层拿到的运行结果里没有失败原因。
3. **`runtime` 已经有一个 `RunResult`。** `runtime/src/lib.rs:432-435` 定义了 `RunResult { answer, acceptance: VerificationStatus }`，入口叫 `run_to_acceptance`（`:930`），CLI 四处输出顶层 `"acceptance"` 键（`main.rs:86`、`:97`、`:109`、`:117`）。P1 新增同名类型会直接冲突，改动面包含 CLI 输出合同。
4. **`acceptance` 表没有任何绑定键。** 单列、无主键，每次先 `DELETE` 全表再插入（`runtime/src/lib.rs:404-405`）。记录不带 run、attempt、计划或被验证产物的摘要，谈不上“绑定版本”。回执内容本身有绑定（`verify.rs:103-110`），但 `VerificationReceipt` 不在生成的四份 schema 里，没有版本化的对外类型。
5. **冻结基准断言了旧语义。** R1a 规格把“独立 verifier passed 才 acceptance passed”写成 B01 的预期（`records/REIN-RUNTIME-R1A-20260921/implementation-spec.md:32`），`fixtures/runtime-r1a/baseline.json` 的 B01、B03、B04、B07 预期都用 `acceptance` 字段；`runtime/tests/` 四个文件共有三十多行引用 `Accepted` 或 `acceptance`。该规格第 28 行要求“改断言必须说明合同缺陷并保留原始失败”，所以 P1 要先出基准修订，再动实现。
6. **取消会覆盖“结果未知”。** `Cancel` 的终态判断不含 `OutcomeUnknown`（`core/src/lib.rs:186-215`），outbox 的 `unknown` 也会被改写为 `cancelled`（`runtime/src/lib.rs:390`），并有测试固定这一行为（`runtime/tests/control.rs:233`）。当前效果全是只读的，没有实际危害；一旦有写工具，外层只看到 `Cancelled` 就可能重跑一个其实已经生效的动作，与 INT-S04“未知副作用不盲重放”冲突。
7. **决策登记表会被触发复审。** `architecture/decisions.json` 的 `DEC-CORE-BOUNDARY` 钉住了 `core/src/lib.rs` 与 `runtime/src/lib.rs` 的 SHA-256（本次核对与当前文件一致），来源写的是 D9、D12、D13。P1 改代码后它会变成 `review_required`，这是设计内的行为；按 D16 应新增一版登记，不刷新旧摘要。
8. **新边界对 Veriflow 的要求没有被估价。** 按 `REIN-INTEGRATION-SPEC.md:21`、`:64`，Veriflow 目前是规则包加 CLI（`validate_task.py`、`record_execution.py`、schema 1.3 记录），并被明确要求“不另起任务状态机”。让它持有 Workflow/Task，意味着要在 Veriflow 仓库新增持久任务状态、调度和修复策略。本次没有读 Veriflow 仓库，它现在离这一步有多远是未知项，应在确认 D19 前核实。

## 4. 对优先级的判断

审计的 P0→P1→P2 顺序把 P1 排在“先确立权威”之后。按第 2 节第 2 点，建议改为：**P1 立即开始，P0 并行等待确认**。理由是 P1 修的是现有设计与实现之间的偏差，改动集中在两个 crate、无跨仓依赖；P0 则取决于第 3 节第 8 点的未知项，以及 D19 里列出的五个待确认问题。P2（抽端口、处理三套 Rust 实现）与 P1 只在验证器端口上重叠，规格里已把这部分纳入 P1 的第四步。

审计的四点架构补充我都同意，其中“运行内依据测试失败继续行动仍属 Rein”目前在产品内核里并不存在：工具失败直接结束会话，没有回填给模型的路径。这是 R1a 增量的范围限制，不属于 P1，规格里列为非目标。

## 5. 本次操作记录

- 新增 `reports/2026-10-02-boundary-audit-review.md`、`reports/2026-10-02-run-verification-acceptance-split.md`；在 `DECISIONS.md` 末尾追加 D19。没有修改任何源码、测试、schema、章节清单或登记表。
- 复核开始时一次普通的 `git status` 在 `.git/` 里留下了零字节的 `index.lock`（本会话的 shell 不能删除文件）。已将它改名为 `.git/index.lock.stale-cowork-20261002`，不影响 git 使用，可以直接删除。此后只用 `git --no-optional-locks`。

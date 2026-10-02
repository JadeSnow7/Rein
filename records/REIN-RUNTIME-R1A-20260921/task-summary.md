# R1a 首批实现与 Apache-2.0 对齐

**R1A-SLICE-1 首批实现已完成定向审查及绑定执行复验，MET-001–004 均通过；整库 acceptance 仍为 undetermined。** 这是原生离线只读增量的结果，不是完整 R1a/P0 或三产品整合验收。实现范围见 [冻结合同](implementation-spec.md)，运行方式见 [runtime/README](../../runtime/README.md)。

## 已交付

- 新根 Cargo workspace 的 `rein-core` / `rein-runtime`，旧 `rust/` 保持独立。
- 纯状态机、真实 read_file 反馈、SQLite IMMEDIATE/CAS 事务与 outbox、引用式检查点、SHA-256 artifact、冻结的 fixture/验证计划。
- 独立 verifier 原始结构化回执；篡改 Passed、迟到结果、错误/缺失产物不能越过验收；工具错误持久化为 ToolFailed。
- driver 独占锁，独立只读 show 与取消；pending 可继续，遗留 claimed 转 outcome_unknown 且不自动重发。
- 实际 `demo/show/resume/cancel/schema` CLI；Rust DTO → JSON schema → TS 生成及只读漂移检查；CI 新路径触发。
- Rein 自有代码、文档、教材及包元数据统一 Apache-2.0。Web Studio/Veriflow 已有 Apache-2.0，当前文件哈希与本轮初始快照一致；本轮未写入这两个仓库。第三方许可保留。

## 最终证据

三份回执绑定同一源码 revision 和 R1A-SLICE-1 Spec digest；均为 recorder 实际执行结果，退出码 0、结果 passed，执行期间输入未变。

| 回执 | 执行与结果 |
| --- | --- |
| [运行时](evidence/final-runtime.json) | 54 项 Rust 测试、6 项生成器测试通过；真实 alpha/beta CLI 和 SQLite 派发次数通过；实际生成 schema/TS 篡改后检查失败，恢复后通过，仓库生成物未改写 |
| [旧工程回归](evidence/final-regression.json) | 原 TS 123 项、Rust 76 项通过；hybrid 与 ch05/ch06/ch07 原对比通过；64 个已捕获旧源码/测试/快照/书籍检查器文件字节未变 |
| [许可及文档](evidence/final-license.json) | 三仓许可与包元数据通过；book 索引仅 60 个开放字段变化，章节状态/快照不变；book:check 与 VitePress build 通过 |

预先保存的失败、过渡实现和输入纠正记录继续保留。无效 UTF-8 fixture 起初误写成 ASCII 转义文本，已留存旧 hash/内容说明后改成真实 ff fe fd 字节；原始通过/失败记录不被改写为最终验收。旧 TS 全量基线曾出现一项 deadline 偶发失败，后续全量、loop 连续 10 次及本次最终回归通过；旧测试稳定性问题未修复。

## Veriflow 记录门槛

[整库 acceptance 原始结果](evidence/acceptance-gate.json) 未通过：209 条路径的完整改动尚未纳入本轮审查，`overall_acceptance` 保持 `undetermined`。其中 129 条已与实施前的文件清单核对，hash 一致；另 80 条位于历史 Veriflow 记录目录，未被该清单覆盖，本轮未复核其内容完整性，保持未审查（见 [边界核对](evidence/preexisting-review-boundary.json)）。当前三份执行回执均有效，Spec 各条件的 MET 检查没有未满足项；本轮没有把旧工作重分类、改写基线或批量标为已审查来消除门槛。

## 范围与交付状态

本轮审查针对相对实施前工作区快照的增量；仓库原有大量未提交改动仍保留，不代表整体仓库已完成审查。文件权限和路径检查作用于当前用户控制的本地状态目录，不提供 OS 级沙箱；取消先于派发门槛时阻止执行，已经过门槛的调用可能完成但不能在取消后推进或验收。show 显示保存的状态，不重验 artifact 实时完整性。

后续仍需：旧教学入口兼容桥、真实模型、独立工作树与写入批准、daemon、Coordinator/外部 Agent、Veriflow 产品 adapter、Web Studio R1c 接入及规范发行。离线模型只证明协议、反馈和恢复闭环。

**本轮定向验证通过，整库验收未完成；未提交、未推送、未发布；远程 CI 未运行；未调用真实模型。**

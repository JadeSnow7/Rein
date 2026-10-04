# 设计决策与连续演化工具：首批增量完成

本轮 EVOLUTION-SLICE-1 已实现并通过限定范围的本地验收。设计记录现在可以检查其文件前提是否发生变化；连续演化工具可以继承上轮产物，持续执行旧要求，并记录新要求通过时发生的历史回归。

## 交付内容

- `architecture/decisions.json` 登记 core 边界示例，引用现有 D9/D12/D13；`scripts/architecture/decisions.mjs` 只读核对已声明文件摘要，区分 `applicable`、`review_required`、`undetermined`，保留人工生命周期状态。摘要匹配只说明前提文件未变，不证明架构正确。
- `scripts/evolution/` 提供单文档连续演化 runner、通用 CLI、离线 demo 和测试；`fixtures/evolution/` 提供确定性正负适配器与独立检查。每轮保存计划、任务、输入、产物、原始进程结果及摘要；只有明确退役理由才能移除历史要求。未知 token、费用和人工时间保持 null。
- 第 18 章 `docs/chapters/task-planning.md` 补充后续变更场景、设计取舍、决策记忆及复审流程；第 24 章 `docs/chapters/project-evaluation.md` 补充连续继承、旧要求回归、离线报告解读和未来真实模型三组实验方案。`DECISIONS.md` 新增 D16，实施计划 R5 补充工具入口。

## 验证证据

| 检查 | 结果 | 原始记录 |
| --- | --- | --- |
| 决策与演化测试 | 25/25 通过 | [accepted-tests.json](evidence/accepted-tests.json) |
| 当前登记表实际 CLI | 已采纳示例的四个文件前提匹配 | [accepted-decisions.json](evidence/accepted-decisions.json) |
| 离线校准 | 两组各 12 轮，正例通过；负例第 7 轮新要求通过、旧要求失败，后续持续失败 | [accepted-calibration.json](evidence/accepted-calibration.json)、[逐轮报告](evidence/calibration-final/report.json) |
| 通用 CLI | 新报告成功；重复报告路径拒绝且未覆盖原证据 | [accepted-cli.json](evidence/accepted-cli.json) |
| 逐轮证据与原文件保护 | 24 轮输入/任务/输出摘要与落盘记录一致；168 份既有源码、合同、索引、快照等受保护文件不变 | [accepted-review.json](evidence/accepted-review.json) |
| 教材检查与构建 | `book:check`、`build` 通过 | [accepted-book.json](evidence/accepted-book.json)、[accepted-build.json](evidence/accepted-build.json) |

最终执行回执绑定同一产品指纹 `838e2a1d1d9beb73e62715fc47c240460d75591b28e95c47ddf6fc5854710192` 和 Spec 摘要 `2852b42a3a8c9818d7681f28244ab8660e71f1dcc9a3382c0635747b997ac115`。源码缺失基准、独立审查反例和补充核查脚本的失败记录均保留，没有用成功结果覆盖历史失败。并行任务新增审查报告被确认为外部路径后，已按最终分类重新运行全部检查；本表引用重新绑定后的 accepted 回执。

## 验收与交付边界

主线程审阅了两个 coder 的最终代码、测试、文档及实际执行回执。本轮接受独立本地工具和对应教学增量；没有接入 Rein runtime，没有运行真实模型三组对照实验，也没有证明 AI 架构或长期维护能力提升。单文档 fixture 不能代表任意多文件项目；普通可信本地子进程不构成盲测或安全沙箱。教材构建通过不等于完整章节的教学验收。

工作区原本包含大量未提交工作，另有第一篇教材任务并行修改其他文档，详见 [并行变化观测](evidence/concurrent-foreign-observation.json)。本轮四份既有文档的增量以 `baseline/owned/` 保存的开始版本为比较依据；未覆写其他任务修改，也未把全部既有改动纳入整库验收。168 份受保护文件仍按最初基线核对。

交付停留在本地工作区，未提交、推送或发布。后续工作是选择真实维护序列、接入模型适配器并冻结等预算对照方案，再检验能力改善这一假设。

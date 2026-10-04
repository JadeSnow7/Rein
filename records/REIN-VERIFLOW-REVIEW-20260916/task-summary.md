# 《Rein》审查更新与 Veriflow 效果报告

已完成全书 43 个 Markdown 页面的审查，修复 18 项登记问题，形成 21 个文件的本次增量。本地验证通过；三条真实模型入口各尝试一次均失败，因此整体任务保留 `blocked`，完整验收未通过。未提交、推送、部署，也未修改原 Veriflow 或模型配置。

## 更新结果

| 问题 | 本次修复 |
| --- | --- |
| 第 01 章跟做链不完整 | 删除无效的独立项目支线；说明当前工作副本与本地标签；修复代码围栏和诊断入口 |
| 配置与凭据说明错误 | 模型名明确必填；删除未被忽略的 `.env.back` 备份命令；解释 API key 与 HTTPS 的区别 |
| 第 03–04 章承诺与实现不一致 | 统一文件名；不以正确问候单独证明读取；移除对未写章节的依赖；明确非流式、离线适配范围 |
| 路线、完成状态与导航过时 | 同步阅读指南、附录规划、组件 README 和阶段导航；主线进入 Rust core，保留历史 TS 对照 |
| live 失败仍返回进程成功 | TS/Rust live 入口仅在 `completed/final_answer` 时退出 0，其余结果退出 1；离线演示语义保留 |
| 第 08 章计时范围不准确 | 明确 `elapsedMs` 包含 Node 宿主启动、文件 I/O、协议处理和退出回收，不是供应商模型耗时 |

完整问题与处置见 [18 项清单](evidence/findings-resolved.json)，精确增量见 [本次补丁](evidence/task-only.patch)与[文件前后哈希](evidence/task-only-files.json)。未完成章节仍保持规划/草稿状态；未整体重写作者稿件。已有 178 个书稿及实现文件已在修改前封存，未删除基线文件，四个历史快照内容和执行位均与清单一致。

## 验证结果与边界

| 检查 | 当前结果 | 证据 |
| --- | --- | --- |
| TS 类型检查与测试 | 通过，102 项测试 | [本地回归](evidence/final-repo-r2.json) |
| Rust 格式、检查与测试 | 通过，53 项测试 | 同上 |
| 导航状态测试 | 5 项通过 | 同上 |
| 05–08 对照、合同与例程 | 通过，含零预算、取消、错误及 live CLI 正反例 | 同上 |
| 最终 Markdown 跟做 | 21 组步骤通过，故障步骤按预期退出 1 | [隔离跟做](evidence/final-markdown-r3.json) |
| 网站构建与站内链接 | 构建通过，2,207 个链接检查无失败 | [构建](evidence/final-site.json)、本地回归 |
| 浏览器实际交互 | 语言切换、共同锚点、主线/历史入口及阶段导航通过 | [浏览器记录](evidence/browser-review.md) |
| 冻结版 Veriflow 自测 | 85 项通过 | [自测](evidence/skill-self-tests-r2.json) |
| 真实模型调用 | TS hello、Rust hello、TS live loop 各一次，均失败 | [真实调用判定](evidence/live-assessment.json) |

跟做步骤从最终正文提取，在新临时副本中执行 `npm ci`，组合 03/04 章代码并运行离线练习，执行 05–08 关键命令、Plugin 故障场景和阅读 0 专项检查。交互编辑、远端 clone/fetch、所有历史归档的重新安装运行及任意跨平台环境，不在本次运行证明范围。浏览器验收是 macOS 桌面导航和渲染检查，不是全面移动端或无障碍验收。

真实调用使用既有 `opencode.ai/zen/go/v1` / `mimo-v2.5` 配置，仅发送 hello 或临时合成标记。live loop 明确返回 `OpenAI HTTP 400`；两个 hello 入口只返回通用失败诊断，具体原因未判定。三次原始记录保留在 `live-ts.json`、`live-rust.json`、`live-loop.json`。随后仅 Rust 格式发生变化，旧记录仍标为历史证据，没有伪造当前绑定，也没有为刷新指纹重复付费调用。

恢复完整验收需要确认该服务的端点、模型及账户可用性，并在配置问题明确修复后重新验证三条入口。当前没有成功的真实模型证据，不能用离线测试替代。

## Veriflow 的实际效果

本次验证的是显式使用 skill 后的一次交付流程：

- 先固定范围、基线和成功条件，使原有未提交修改与本次 21 文件增量可以分开复核。
- 逐页审查与源码对照发现实际问题；主线程的反例调用额外发现 live 失败退出码错误，原有 100 项 TS 测试未覆盖这一行为。
- 独立复核和主线程检查拦下实现初稿的问题，包括同步子进程阻塞模拟服务器、测试入口参数错误、TS 测试不应依赖 Cargo，以及修复措辞仍有歧义。
- recorder 检出了运行期间文件变化，旧结果未被当作当前通过；最终源码稳定后重新执行并绑定证据。
- 门槛正确保留未完成状态：[record](evidence/gate-record.json)与[implementation](evidence/gate-implementation.json)通过，[acceptance](evidence/gate-acceptance.json)因真实调用缺少通过证据而失败，没有把本地交付冒充完整验收。

流程也有实际开销和限制：最初受限沙箱禁止回环端口、Cargo 默认目录不可写；格式检查发现新增 Rust 格式问题；第一次跟做验证器误把教程明确要求的 exit 1 当成失败；旧预览实例在重建后出现暂时 404，使用最终构建的新预览实例完成复核；一次 skill 自测在并发验证期间超时，延长窗口后通过。原始失败与修正后结果均保留。机械门槛没有自动发现教学语义错误，也没有替代对验证器的审查。

这些证据支持“该流程能用于本次书稿交付、保留失败并阻止错误完成结论”。本次没有无 skill 对照、重复样本或自动触发实验，不能证明因果质量提升、自动触发可靠性、提速或省钱。可获得的命令起止时间见 [用量与时间记录](evidence/usage.json)；agent token、服务 token、费用与比较收益均为 `null`，并行命令时长不累加为总延迟。

## 版本与复跑入口

- 书稿基线 HEAD：`1dfa82b9d90196c2294f61914d2017449e5548e3`，连同当时的脏工作区冻结；[文件清单](baseline/files.json)、[原始归档](baseline/book.tar.gz)。
- 本地 skill 源：`/Users/huaodong/Documents/evidence-driven-development/skill/veriflow/`；读取到源仓库 HEAD `331db211b2b7e0ca88dd92df2ebb54a89b699b79`，源仓库 status 查询超时，未声称其工作区干净。
- skill 共冻结 29 个文件；SKILL.md SHA-256：`fc2ef4b580d3aface00c1e75a6d8231f6e2ae4e389b53fc3ec37ced3f953742c`；[完整冻结清单](baseline/skill-files.json)。全程使用冻结副本。
- [结构化状态及 Spec](task-state.json)、[覆盖清单](evidence/coverage.json)、[修复前发现](evidence/review-before.json)、[最终复核](evidence/review-after.md)。
- 当前验证环境：macOS，Node v26.5.0，Rust 1.98.0；Cargo 产物写入独立临时目录。

在仓库根目录复跑本地回归与跟做，可使用保留的验证脚本（会生成临时副本、安装依赖及构建产物；不会调用真实模型）：

```bash
python3 records/REIN-VERIFLOW-REVIEW-20260916/evidence/verify-repo-final.py
python3 records/REIN-VERIFLOW-REVIEW-20260916/evidence/verify-markdown-final.py
```

若作为新的验收证据，仍应按冻结 skill 的 `record_execution.py --repo --state --output <新文件>` 包装执行，禁止覆盖旧记录。真实调用脚本会使用现有密钥并产生 API 用量，未列入上述默认复跑。

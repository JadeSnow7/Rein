# 现有材料到六部分框架的迁移映射

> **历史迁移底稿，记录 2026-09-22 的迁移前状态。** 2026-09-23 已把六部分 00–29 设为现行清单和网站导航，并把旧五篇清单归档于 `book/history/chapters-v2.json`；新 04 为“Agent 的核心能力”规划页，Rust 实作尚未完成。以下“尚未执行”“未来工作包”等表述只对应原记录日期。当前状态与验收以[同步规格](../../reports/2026-09-23-six-part-ch04-sync.md)和[现行清单](../chapters.json)为准。

本表为正式迁移方案，尚未执行移动、改名、索引生成、导航更新或代码迁移。新编号仅代表 [框架阅读顺序](README.md)。检查基线为 2026-09-22 当前工作副本，HEAD `1dfa82b9d90196c2294f61914d2017449e5548e3`，含既有未提交修改；逐文件摘要见 [baseline.json](evidence/baseline.json)。

“保留”表示原路径和原义继续存在；“移动”仅指未来教材中主题位置调整；“拆分/合并/改写/补充/新增”指新框架的写作动作，不授权覆盖旧稿。已有正文不等于完整章节，已有实现不等于新章验收；本轮读取历史执行记录而没有复跑这些产品实验。

## 当前主题页逐项映射

下表覆盖当前 [book/chapters.json](../chapters.json) 的全部 25 个 slug。该索引 00–17 为 draft、18–24 为 planned，全部 `acceptance_status=not_run`；正文可能比索引有更多局部增量，不能以索引或页名替代内容检查。

| 现有路径 / 主题 | 新章 | 写作动作 | 当前材料、缺口与证据范围 |
| --- | --- | --- | --- |
| [task-map.md](../../docs/chapters/task-map.md) | 00 | 保留旧入口；改写 | 已有模型/工具/环境/Harness 分工与文档维护场景；改成六部分与 Python 建议器终点，旧主线仍保留 |
| [model-hello.md](../../docs/chapters/model-hello.md)、[model-hello-rust.md](../../docs/chapters/model-hello-rust.md) | 01、04 | 改写、补充 | 已有 TS 入门和 Rust 对照；Python hello 与其教学验收计划新增；04 复用 Rust 基础而非要求两次独立入门 |
| [task-spec.md](../../docs/chapters/task-spec.md) | 00、01、03、15 | 拆分、合并 | 任务边界与输入要求前置到 00–01，建议输出要求并入03，独立任务验收深化在15；不另设新“提示词工程”必修章 |
| [tool-roundtrip.md](../../docs/chapters/tool-roundtrip.md) | 02、03、05 | 改写、拆分 | 有 TS 单次工具读取教学与错误处理；02 改为 Python 并从直接读取过渡；03 新增程序定位的终端建议，05 抽内部协议 |
| [provider-adapter.md](../../docs/chapters/provider-adapter.md) | 05 | 合并、保留扩展 | 内部消息/工具/错误语义优先；第二服务商适配放可选补充，不作为 Python 小 demo 前置 |
| [rust-migration.md](../../docs/chapters/rust-migration.md) | 04 | 改写 | 原 TS→Rust 改为 Python→Rust；必须重新建立相同任务、文件、响应和失败样本的行为对照，不能引用旧 TS 对比通过代替 |
| [agent-loop.md](../../docs/chapters/agent-loop.md) | 06 | 保留、补充 | 循环与工具结果反馈可复用；与04–05新输入合同衔接待建 |
| [loop-budget.md](../../docs/chapters/loop-budget.md) | 07 | 保留、补充 | 步数、工具调用、重复动作等机制；精确默认值及计数边界在新例固定，不从旧结果推断新案例已通过 |
| [cancellation.md](../../docs/chapters/cancellation.md) | 08 | 保留、补充 | 取消、截止时间、资源回收；增加与CLI事件以及后续IDE状态的衔接 |
| [tool-host.md](../../docs/chapters/tool-host.md) | 05、08、18 | 拆分、移动 | 内部执行接口/错误到05，生命周期到08，外部宿主协议和声明能力到18；第二部分无需先建TS进程宿主 |
| [context-state.md](../../docs/chapters/context-state.md) | 09 | 保留、改写 | 围绕完整修改任务组织历史、工作状态和发送副本，不将内存上下文等同持久执行恢复 |
| [context-retrieval.md](../../docs/chapters/context-retrieval.md) | 10 | 合并 | 按需读取/检索与压缩同章，先取得来源再讨论摘要，不额外引入必需向量库 |
| [context-compression.md](../../docs/chapters/context-compression.md) | 10 | 合并、补充 | 来源、版本、丢失信息和预算合同复用；与固定任务回归结合，四方法独立实验保留原入口 |
| [task-verification.md](../../docs/chapters/task-verification.md) | 03、13、15、24 | 拆分、移动 | 03先检查定位与原文，13固定候选及验证计划，15讲实际验证及有限修复，24做系统评测；不能等15才第一次提出“成功标准” |
| [trust-permissions.md](../../docs/chapters/trust-permissions.md) | 11、12 | 拆分 | 11讲指令/资料/不可信输入，12讲程序执行准入；02已有基本读取边界，后续扩充而非之前完全无保护 |
| [patch-candidate.md](../../docs/chapters/patch-candidate.md) | 13 | 保留、改写 | 候选可审阅与可应用性检查；不在13实际写目标项目 |
| [approved-patch.md](../../docs/chapters/approved-patch.md) | 14、15 | 拆分 | 14批准绑定/写入/读回，15业务验证/失败反馈；旧维护示例与真实宿主有历史离线记录，仍需新章衔接 |
| [bounded-repair.md](../../docs/chapters/bounded-repair.md) | 15 | 合并 | 新尝试重新读基线/生成候选/批准，保留失败；未知副作用不得直接重放 |
| [task-planning.md](../../docs/chapters/task-planning.md) | 26、27；16仅题旨参考 | 拆分、重新归位、补充 | 当前正文实际是设计备选、决策前提及复审工具，不是可运行任务计划模式。16状态/依赖/修订/执行切换须新增；27复用已有决策检查 |
| [durable-recovery.md](../../docs/chapters/durable-recovery.md) | 08、21；深层恢复补充 | 移动、拆分、保留 | 保留独立主题入口；08区分取消与恢复，21区分保存、重连、进程中断恢复。完整通用恢复非IDE最小路线隐含前置 |
| [extension-sdk.md](../../docs/chapters/extension-sdk.md) | 18 | 合并、改写 | 以最小执行扩展接口说明版本/能力/错误；TS参考SDK可选，不让读者维护第三套完整Harness |
| [mcp-tools.md](../../docs/chapters/mcp-tools.md) | 19 | 保留、补充 | 有主题初稿入口，当前索引未实现；MCP适配、连接生命周期与预算权限回归计划新增 |
| [execution-hooks.md](../../docs/chapters/execution-hooks.md) | 18 | 合并、补充 | 在固定事件点触发受控动作，失败策略/超时/递归限制与核心权威一起讲 |
| [bounded-delegation.md](../../docs/chapters/bounded-delegation.md) | 20 | 保留、补充 | 从串行受限子任务到并行冲突；不能把协调器设计或本轮写作协作当作Rein运行时已实现 |
| [project-evaluation.md](../../docs/chapters/project-evaluation.md) | 24、28、29 | 拆分、改写、补充 | 当前实际已有单文档连续继承、回归与离线校准说明，主要归28–29；24新增IDE产品交付，保留四类维护案例评测思想 |

## 新增覆盖与证据等级

| 新章 | 当前能直接利用的材料 | 新框架仍需新增的关键内容 |
| --- | --- | --- |
| 00–03 | 旧入门正文、TS读取教学、任务说明 | Python入口、真实原文行表、候选定位与终端展示、固定离线响应，均计划新增 |
| 04–08 | Rust教学loop、预算/取消/上下文代码 | Python等价迁移与F03→F08连续快照；当前对照实验属于旧TS/Rust |
| 09–15 | 上下文、候选、批准、验证与有限修复旧实现 | 新案例来源合同与新阅读顺序衔接、逐章学习者验收 |
| 16–20 | 方法/扩展/协作旧提纲及扩展宿主 | 运行时计划模式、Skill加载、MCP与委派教学纵切及各分支实测 |
| 21–24 | [整合规格](../../REIN-INTEGRATION-SPEC.md)、[模块设计](../../REIN-MODULES.md)、[产品实施计划](../../REIN-IMPLEMENTATION-PLAN.md) | 教学IDE脚手架、编辑器上下文、运行面板/diff/授权、整合运行和打包验收；产品规格本身不是实现 |
| 25–27 | [设计决定](../../DECISIONS.md)、[设计登记](../../architecture/decisions.json)、[架构研究报告](../../reports/2026-09-20-rein-architecture-research.md)、task-planning正文 | 从代码取证的项目图、设计备选/小原型/变化检验、模型可消费的记忆生命周期；摘要匹配不能证明设计正确 |
| 28–29 | [演进工具](../../scripts/evolution/README.md)、[演进实施规格](../../records/REIN-EVOLUTION-20260922/implementation-spec.md)、[演进摘要](../../records/REIN-EVOLUTION-20260922/task-summary.md) | 多文件连续维护、兼容迁移、真实Agent等模型/权限/预算多序列重复实验；当前工具只接受单文档校准 |

## 代码、实验与原始记录

| 现有路径 | 去向与处理 | 本轮核查到的范围 |
| --- | --- | --- |
| [ts/src](../../ts/src/)、[tool-roundtrip正文](../../docs/chapters/tool-roundtrip.md)、[ts/examples](../../ts/examples/) | 01–03行为参考、04历史对照；原位保留 | TS hello源码已存在；单次读取在正文提供由读者创建的 `ts/ch03.ts`，当前工作副本没有该文件，不能标为现成可运行入口；Python新增目录及依赖待设计 |
| [rust/src](../../rust/src/)、[rust/examples](../../rust/examples/)、[rust/tests](../../rust/tests/) | 04–15主要实现参考；原位保留 | 独立Cargo教学工程；loop、context、maintenance等入口存在，不能与根产品workspace混为一套当前已迁移代码 |
| [ts/src/rein/extension02-host.ts](../../ts/src/rein/extension02-host.ts)、[maintenance-tools.ts](../../ts/src/rein/maintenance-tools.ts)、[rust/src/rein/extension02_executor.rs](../../rust/src/rein/extension02_executor.rs) | 14–15和18；拆分讲授、保留旧实验 | 现有Node宿主与Rust维护边界可作参考；新Rust主线可在进程内实现最小工具，不能隐含强制Node宿主 |
| [contracts/extension-v0.1.md](../../contracts/extension-v0.1.md)、[extension-v0.2.md](../../contracts/extension-v0.2.md)、[maintenance-v1.md](../../contracts/maintenance-v1.md)、[maintenance-session-v1.md](../../contracts/maintenance-session-v1.md) | 05语义参考、13–15维护、18版本兼容 | 保留版本和原义，不把教学新字段直接宣布为旧wire合同的兼容修改 |
| [core](../../core/)、[runtime](../../runtime/)、[contracts/runtime](../../contracts/runtime/) | 06、08、21、25–27的产品架构参考 | 原生离线只读纵切有纯步进/driver/SQLite/工件/验证入口；真实模型、写入审批、daemon和Web Studio完整整合仍是后续工作 |
| [R1a 原始回执](../../records/REIN-RUNTIME-R1A-20260921/evidence/final-runtime.json) | 21现状说明、25架构来源 | 已读取历史stdout、exit_code=0与绑定revision，支持当轮alpha/beta CLI及派发/漂移检查；本轮未复跑，不是IDE完成证据 |
| [维护原始回执](../../records/REIN-BOOK-V2-20260917/evidence/maintenance-runtime-final-v2.json) | 13–15参考证据 | 历史离线宿主测试含批准拒绝、基线变更、无须修改、两次修复上限；只支持当时输入与revision，不证明Python/Rust新章链已验收 |
| [fixtures/document-maintenance](../../fixtures/document-maintenance/)、[fixtures/cases](../../fixtures/cases/) | C01–C07与24的样本来源 | 文档维护案例可补充命令、参数、链接、无须修改；新统一案例与冻结响应尚待建立 |
| [examples/ch08-context-methods](../../examples/ch08-context-methods/)、[fixtures/ch08-context](../../fixtures/ch08-context/)、[scripts/ch08-compare.mjs](../../scripts/ch08-compare.mjs)、[scripts/ch08-verify.mjs](../../scripts/ch08-verify.mjs) | 10可选实验；原位保留 | 四种资料获取方法独立实验，不强迫所有新读者运行，旧实验数字“08”不重标为新取消章 |
| [scripts/architecture](../../scripts/architecture/)、[architecture/decisions.json](../../architecture/decisions.json) | 27可运行复审机制基础 | 已读检查器：声明前提摘要匹配/需复审/未知，人工状态不自动提升；读取前提变化不能自动刷新历史决定 |
| [scripts/evolution](../../scripts/evolution/)、[fixtures/evolution](../../fixtures/evolution/) | 28–29校准基础 | 已读runner的前轮文件复制与活动检查累积逻辑；普通子进程不是OS隔离或未来任务盲测 |
| [校准回执](../../records/REIN-EVOLUTION-20260922/evidence/accepted-calibration.json)、[逐轮报告](../../records/REIN-EVOLUTION-20260922/evidence/calibration-final/report.json) | 29解释区分性检查 | 历史两组12轮确定性夹具，正例通过、负例第7轮新增需求通过但旧要求失败；不是AI对照实验或能力提升证据 |
| [chapter-snapshots](../../chapter-snapshots/) | 历史恢复补充；保留 | pre-ch05/ch05/ch06/ch07包、manifest及相邻补丁保持旧TS/Rust主题，不作为新F04–F07输入输出 |

## 数字旧稿、阶段汇总与补充材料

所有旧数字页继续原位保留，不能因同名新编号而换内容：

| 旧材料 | 新阅读用途 |
| --- | --- |
| [00.md](../../docs/chapters/00.md)、[01.md](../../docs/chapters/01.md)、[01-ts.md](../../docs/chapters/01-ts.md)、[01-rust.md](../../docs/chapters/01-rust.md)、[01-ts-advanced.md](../../docs/chapters/01-ts-advanced.md) | 00–01概念和04语言迁移；HTTP深入内容保留可选，作者稿不覆写 |
| [02.md](../../docs/chapters/02.md)、[03.md](../../docs/chapters/03.md)、[04.md](../../docs/chapters/04.md) | 00/03任务要求、02工具往返、05内部协议；按主题拆解，原章号保留 |
| [05.md](../../docs/chapters/05.md)、[05-rust.md](../../docs/chapters/05-rust.md)、[06.md](../../docs/chapters/06.md)、[06-rust.md](../../docs/chapters/06-rust.md)、[06-plugin.md](../../docs/chapters/06-plugin.md) | 06–08循环控制及18宿主；原TS/Rust实验与Plugin M1保留 |
| [07.md](../../docs/chapters/07.md)、[07-rust.md](../../docs/chapters/07-rust.md)、[08.md](../../docs/chapters/08.md) | 09–10上下文与按需实验 |
| [09.md](../../docs/chapters/09.md)、[10.md](../../docs/chapters/10.md)、[11.md](../../docs/chapters/11.md)、[12.md](../../docs/chapters/12.md) | 11–15可信修改；区分已有提纲、正文和维护实现 |
| [13.md](../../docs/chapters/13.md)、[14.md](../../docs/chapters/14.md)、[15.md](../../docs/chapters/15.md)、[16.md](../../docs/chapters/16.md) | 16/21计划恢复题旨、20协作、18–19扩展、24/26领域设计与复盘；提纲不代表对应能力实现 |
| [evidence-qa.md](../../docs/milestones/evidence-qa.md) | 改写为S1的Python建议器检查；现有真实资料问答复查保留 |
| [controlled-harness.md](../../docs/milestones/controlled-harness.md) | S2可控Rust CLI；宿主内容移动到S4 |
| [grounded-answer.md](../../docs/milestones/grounded-answer.md)、[single-agent-maintainer.md](../../docs/milestones/single-agent-maintainer.md) | 合并贡献S3：上下文依据、权限、批准、验证和修复 |
| [project-defense.md](../../docs/milestones/project-defense.md) | 拆分至S5产品展示与S6研究复盘；S4需新增扩展阶段检查 |
| [旧小结01](../../docs/milestones/01.md)、[旧汇总02](../../docs/milestones/02.md)、[旧汇总03](../../docs/milestones/03.md)、[旧汇总04](../../docs/milestones/04.md) | 原位保留旧阶段意义；只提取适用练习，不复用旧号作为新S编号 |
| [readings/00.md](../../docs/readings/00.md)、[00-ts.md](../../docs/readings/00-ts.md)、[00-rust.md](../../docs/readings/00-rust.md) | Python基础补充计划新增；Rust必要知识放04–08正文，TS补充服务历史与可选宿主 |
| [readings/01.md](../../docs/readings/01.md)、[02.md](../../docs/readings/02.md)、[03.md](../../docs/readings/03.md) | AI学习、版本/历史恢复、外部Harness阅读继续可选；未复用内容保留原路径，不变成必修关卡 |
| [appendices/a1.md](../../docs/appendices/a1.md)、[a2.md](../../docs/appendices/a2.md)、[a3.md](../../docs/appendices/a3.md)、[a4.md](../../docs/appendices/a4.md)、[a5.md](../../docs/appendices/a5.md) | 分别支持05/12行为边界、04–09状态类型、08取消、21恢复、20并发；主线所需解释须回正文，完整深入讨论继续保留 |

## 正式迁移的后续工作包

1. **冻结新主题身份与状态字段。** 先确定30章的稳定slug和新版本manifest；当前 `book/chapters.json` 为五篇正式入口，不能只改 `order` 就完成迁移。已完成/规划/验收/快照分别登记，新章无执行证据时保持未验收。新slug方案尚待设计，不复用旧数字文件作新语义。
2. **逐章写作与教学实现。** 先00–03 Python纵切及04对照；固定F03、C01–C04和响应来源，再依次冻结Rust增量。旧TS与Node宿主按实际复用保留；选择教学Rust最小实现与产品 `core/runtime` 的适配边界，不能复制出三套同等维护负担的完整Harness。
3. **生成索引与网站入口。** 迁移时协调根README、`docs/index.md`、`docs/about.md`、`docs/toc.md`、`docs/history.md`、`docs/.vitepress/config.mts`、语言切换组件及其状态逻辑；核查 `scripts/book-check.mjs`、`scripts/book-toc.mjs` 的模式和链接约束。旧URL仍指原主题，新增入口使用明确slug；历史作者稿继续可达。
4. **迁移六个阶段成果。** 把旧五次汇总按上表重组为S1–S6，逐一重建输入、输出、异常与手工教学检查。旧汇总页面保留原义并解释新去向，不继承旧passed为新passed。
5. **分离历史证据与新快照。** `records/`、旧归档、标签、实验脚本原位保留；新执行记录绑定新章起始/结束清单、Spec、环境与真实输入输出。已有记录可引用，不通过补写新字段伪造旧事实。
6. **整体验收和分发。** 先目录、相对链接、旧URL回归，再在冻结快照上逐章运行，最后做人工教学验收和适用产品验收。提交、推送、发布另按当时用户授权执行，本轮不执行。

这些动作是未来迁移建议；本轮只完成独立框架。许可沿用当前 D15 的 Apache-2.0，自有成果开放状态与内容验收分开。旧历史页面内仍可能有过时开放措辞，应在正式迁移时增加当前说明而非改写冻结历史。

## 未复用材料的保留与待核实项

- 供应商专有细节、完整手写HTTP和全部TS对照不作为新主线必修；保留 `docs/history.md` 与原实现，后续作者按API版本重新核对。本轮未查询远程SDK/模型价格，也未把旧文中的版本推广为当前建议。
- 原生持久执行、Coordinator、AgentMux、产品发行等继续归现有产品设计与实施计划；新框架只取教学所需，完整P0与R1c依赖不能因裁剪而自动清零。
- 真实Web Studio工作副本与原生窗口未在本轮核查。第五部分仅以仓库整合规格为设计输入，其UI脚手架、编辑器支持和运行状态明确待实现/待现场核验。
- 未列为主线的研究报告、作者风格稿、历史审核记录和原测试保留原目录供追溯，不批量删除或重标。旧结论只适用于原版本；本框架不承担整库既有改动验收。

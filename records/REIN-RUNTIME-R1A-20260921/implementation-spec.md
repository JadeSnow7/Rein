# R1a 首批实现合同

版本：R1A-SLICE-1，2026-09-21。来源：用户要求按修订计划使用子代理开始实现，并统一采用 Apache-2.0。主线程拥有本合同；实现和最终证据不得反向降低它。

## 目标及范围

交付可实际运行的原生只读单任务纵切：确定性离线模型调用 read_file、接收真实工具结果、输出最终回答，再由独立 verifier 判定。纯 core 步进与 runtime I/O 分离，SQLite 原子保存检查点、观察、事件和 outbox；内容按 SHA-256 保存。产品代码采用 Apache-2.0。

这是 R0/R1a 的第一个增量，不是整个 R1a 的完成声明。旧教学 API 本轮保留原样并复跑原基准；旧入口改用新引擎的兼容桥、真实模型调用、写入批准、Coordinator、外部 Agent、独立工作树、多客户端 daemon、Veriflow 产品 adapter、Web Studio 原生 UI、规范发行仍未完成。离线模型只能证明协议和状态闭环；不得宣称真实模型验收。

## 实施边界

- 新根 Cargo workspace（排除旧 rust/），rein-core / rein-runtime；agentmux 若建立只提供有真实测试的能力合同，不声称实现外部 Agent。
- core 不做 I/O、取时、随机或异步操作。状态 + observation 产生拒绝、重复或带 expected_revision 的 transition。确定性 effect ID 至少包含 session/attempt/revision/种类。
- 唯一任务写入权威为 SQLite。Session、Run/Attempt 关联、observation 去重、events 和 effect outbox 同事务提交；持久检查点只存 artifact 引用和控制元数据。外部 message/tool body 不嵌入检查点。
- 一个 observation ID 重复必须幂等；不同 ID 对同一已完成 effect 的重复结果同样不推进、不重新派发。未知、错 attempt 或迟到结果不能覆盖当前状态。
- effect 先持久 claim 再调用 I/O；恢复时遗留 claimed effect 标为 outcome_unknown，阻止自动重试。pending 尚未派发的效果可恢复。实现不承诺外部 exactly-once。
- 每次派发前重新核对取消/终态；预算在 core 产生工具效果之前扣减。预算零时工具派发零次。Verifier failed/undetermined 不可产生 passed Acceptance。取消后 verifier 迟到不可验收。
- Runtime 在状态提交前核对 artifact 存在、内容 hash、长度；文件写入失败、事务失败或 revision 冲突时禁止派发下一效果。状态目录为明确参数，不推导源码绝对路径；单独建临时 fixture 副本作为只读 workdir，不写用户源 fixture。
- read_file 只接受根内普通 UTF-8 文件，拒绝绝对路径、..、符号链接逃逸及超限输入；固定最大字节数。首批无写工具，无隐含 shell/network 权限。
- OfflineModel 必须通过工具返回值生成最后回答，测试改变 fixture 能改变回答；不得把预置成功文本当作反馈闭环。独立 verifier 对比冻结的 expected bytes 与最终回答，记录 version/input/output/hash/status 原始回执。expected 不由模型输出替换。
- CLI 至少提供 demo、show、resume、cancel（具体参数由实现选定并写入 runtime/README.md）。重复 demo 不重建已有 session。查询与恢复不能无条件把 unknown 转为 pending。
- Rust serde/schemars 类型是 DTO 权威。生成 contracts/runtime/schemas/*.json，再从 schema 生成 TS；提供可复跑生成命令和不写文件的 check 命令。Rust 类型或生成物漂移时 check 失败；不手写第二套 TS DTO。摘要采用原始字节 SHA-256、小写 hex，JSON 哈希需明确采用具体存储字节，不自称跨语言 JCS。
- 当前自有代码、文档和教材统一 Apache-2.0；第三方许可和历史决策/回执保留原义。包元数据及当前开放提示一致，章节正文交付状态不因许可证调整而改变。

## 冻结基准

先编写固定 fixture 与验收测试，运行并保存新产品缺失导致的实现前失败；主线程审查后开始生产实现。后续按同一输入和断言复跑。改断言必须说明合同缺陷并保留原始失败。

| 编号 | 输入和故障 | 可观察预期 |
| --- | --- | --- |
| B01 | fixture 为 alpha；模型 read_file 后 final；再以 beta 独立运行 | 分别返回实际 alpha/beta；至少 model/tool/model/verify 四个效果及递增 revision；独立 verifier passed 才 acceptance passed |
| B02 | 相同 observation 重复；换 observation ID 重放同 effect | revision、事件数、工具派发计数不增加 |
| B03 | 工具预算 0 或耗尽 | 无额外 ExecuteTool；原输入文件 hash 不变；无错误 passed Acceptance |
| B04 | pending 模型/工具/verify 前取消；迟到 verifier passed | 无后续派发；持久 cancelled；迟到结果不改成 accepted |
| B05 | artifact 缺失/损坏；事务触发器拒写或等效真实事务失败；旧 revision | 事务不留半条状态或 outbox；零次未提交派发；旧 revision 拒绝 |
| B06 | 派发前 pending 恢复；claim 后模拟崩溃，再重新打开 SQLite | pending 正常续行；claimed -> outcome_unknown；同 effect 派发总次数不增加；show 可见原因 |
| B07 | final 与 expected 不同；verifier 不可执行 | 原始回执分别 failed/undetermined；无 passed Acceptance；模型正常结束不能越过 gate |
| B08 | 路径 ..、绝对路径、symlink 逃逸、非 UTF-8/超限 | 拒绝并保留工具错误观察；未读根外内容；输入根无写入 |
| B09 | 同 fixture 导出 schema/TS；故意更改生成物再 check | 正常 check 通过；漂移 check 非零；还原后通过 |
| B10 | 两个 driver/独立连接竞争同一 effect | 一个 claim 成功，最多一次 dispatch；无覆盖 revision；单进程限制若必要须明确拒绝第二 owner |
| B11 | 独立旧 rust/ts 原测试、ch05/ch06/ch07 对比、hybrid | 保留原语义；环境无法监听只记 undetermined，不改断言 |
| B12 | 三仓许可证、Rein 包元数据、当前 docs 索引 | Apache-2.0 一致；原章节 status/snapshot 不变；book:check/build 通过；第三方条款不被覆盖 |

新测试先行记录不等于产品已实现。最终命令经 Veriflow recorder 运行并绑定当前输入；保留最初环境受限日志。主线程检查实际 diff 和原始日志后才报告首批结果。

## 所有权

- runtime coder：新 Cargo.toml/Cargo.lock、core/、runtime/、agentmux/、contracts/runtime/、scripts/runtime/、.github/workflows/test.yml 产品检查、.gitignore 的 /target/，以及 evidence/runtime-*、handoffs/runtime-*。
- license coder：LICENSE、package 元数据、旧 Rust license 字段、book/chapters.json 开放字段、当前 docs 开放提示；独占 VitePress build。
- 主线程：本合同、task-state/summary/work-log、README、DECISIONS、实施计划阶段进度与最终审查。禁止覆盖其他作者既有改动；不提交、不推送、不发布。

# Migrations

## 2026-09-23：六部分书稿成为现行结构

`book/chapters.json` 现登记六部分、00–29 共 30 章；旧五篇 25 章清单原貌保存为 `book/history/chapters-v2.json`。网站现行目录、侧栏与章间导航读取六部分清单。新 04 为[Agent 的核心能力](docs/chapters/agent-capabilities.md)的规划页，05–29 指向明确标为规划中的部分页，六个新阶段成果各有独立入口。旧数字 URL 和旧五篇主题页面从[历史入口](docs/history.md)查阅，原文、锚点、快照与运行证据未迁作新章的已验收证明。

本次迁移删除了现行目录中重复生成的五篇章节块以及主线里“04 是服务商适配、第一部分全程只读”等失效描述；历史计划与研究报告保留原版本语义。此变更是书稿结构与导航同步，不包含第 04 章 Rust core 实现、真实模型调用、提交、推送或发布。规格和基线见[同步记录](reports/2026-09-23-six-part-ch04-sync.md)。以下旧条目按各自日期和当时编号阅读。

## ch08

第08章加入 Rust `context_methods` 四策略比较、真实 Node `read_file` 混合入口、独立示例与离线验收命令；不运行真实模型服务。

状态：正文与入口已提供；当前版本以最终源码、fixture 和实际执行记录核对四策略输出、独立练习、异常路径及文档构建。第 09 章仍未实现。

## ch05 loop contract（历史基线）

第05章新增可观察循环事件：`model_requested` 保存完整 `messages` 与 `tools`，`model_received` 保存模型消息与 `toolCallIds`，`tool_result` 保存原始 `call`、`toolCallId` 与结果；终态仍使用小写 snake_case 值。Rust 内部字段由 serde 映射为共享 camelCase wire。旧的 `runAgentLoop`/`run_agent_loop` 入口保持兼容。

工具 schema 现在明确声明 `read_file.path` 与 `search_files.needle` 为必需字符串。Rust 回放耗尽返回 `replay_exhausted`，不会 panic。真实入口必须显式 `--live`，最多四轮且不自动重试。

该历史基线：第05章的 TypeScript 与 Rust typed loop、只读工具路由、离线回放与受限真实入口处于本地实现核验中；Rust 的旧 `rein::run_agent_loop` 仍作为 OpenAI HTTP 兼容包装，核心实现位于 `rein::loop`。工程决策记录见 [DECISIONS.md](DECISIONS.md)。
## 混合路线后续任务（计划）

第 06 章混合运行时已有本地实现与测试材料，离线实验已核验：Rust core → Node `read_file` → 第二轮离线 fixture model；最终全站检查仍待维护者统一放行。第 13 章 journal 是后续交付物，不是当前流程的前置自循环；第 11 章的权限代理与隔离能力仍需按实际权限边界做独立验收。

以下项目描述交付物、依赖和验收条件；它们是路线计划，不表示当前已经实现。

| 任务 | 交付物 | 依赖 | 验收 |
| --- | --- | --- | --- |
| SDK 提炼 | 从实际调用抽出 TS SDK 接口 | core 调用稳定 | SDK 由真实调用驱动并保留 IDs/evidence |
| 08 context | 同一任务、同一预算下的四种上下文策略材料 | core 状态与消息快照 | 四策略输出可复算，并分别记录上下文估算单位与服务实际 token 的边界 |
| 09 validators | evidence binding 验证器 | 08 context | 按 task/call/targetVersion/rule 绑定证据；验证通过不等同任务成功 |
| 10 repair + unknown | 失败反馈、未知结果处理 | validators、断连结果 | 未知结果不自动重放，修复路径有明确终态 |
| 11 boundary | 参数、路径和信任边界；若需限制进程副作用，交付实际代理或隔离机制 | host 协议与工具权限 | 未授权路径和进程边界有真实失败证据；不把本轮 trusted Node 当作 sandbox |
| 12 approval baseline | patch approval 基线 | boundary、evidence | 补丁 hash 与文件基线绑定；基线变化使旧批准失效，拒绝状态不写入 |
| 13 persist recovery | 持久化、恢复和 unknown | approval、事件合同 | 交付 journal 后验证崩溃恢复不重放未知调用 |
| 14 bounded delegation | 受限委派合同 | recovery、权限边界 | 权限与预算不扩大；依赖满足、部分失败、取消传播和冲突均可验证 |
| 15 MCP/hooks | MCP stdio 正式协议与 hooks 兼容 | delegation、协议稳定 | 连接生命周期、取消和证据字段通过兼容测试 |
| 16 doc maintainer | 文档维护者流程与 12 个固定案例（命令过期、参数变化、相对链接失效、无须修改各 3 个） | 全部已实现能力 | 12 个案例均有可运行命令、结果和边界说明 |


## 2026-09-17：Rein 全书 v2

新版以主题 slug 为入口，见 `book/chapters.json` 与 `DECISIONS.md` D10。原数字页面及其历史含义保留。当前实施从包括未提交内容的工作区冻结副本开始，不是从 HEAD 重置。冻结清单、工作状态与回写证据位于 `records/REIN-BOOK-V2-20260917/`。

只有实际完成正文、实现和前驱快照复现的章节才能标为已验收；旧通过记录不自动适用于新版。运行时协议0.1保留历史含义，0.2另行定义通用调用信封。

<a id="rein-core-bridge"></a>

## 2026-09-20：旧教学包与 rein-core 的兼容桥（计划，尚未实施）

依据 [D12、D13](DECISIONS.md)，产品根 workspace 只纳入 core、agentmux、runtime，并排除 `rust/`。兼容抽取期间，旧教学包拟通过 path 依赖 rein-core 提供临时 wrapper/re-export；原 async 入口保持调用方式，内部改接纯步进机及受管 I/O driver。具体适配与依赖集合在实施前冻结，本轮未创建 Cargo workspace 或修改源码。

代价与约束：

- 根 `Cargo.lock` 和 `rust/Cargo.lock` 各自独立解析 core 的传递依赖。即使指向同一份 core 源码，也可能选出不同的依赖版本和 feature 组合；path 依赖不能让两份锁文件自动一致。兼容检查需分别记录两套锁文件和所用版本，不静默同步或覆盖旧锁文件。
- 从旧 `rust/` 包复跑原基准会在其独立构建上下文重新编译 core，不能假定复用产品 workspace 的构建缓存。依赖漂移、feature 差异和额外构建时间属于兼容桥成本，需要在两套入口上分别验证。
- core 改为步进机不改变历史 loop/工具/维护合同；旧 async 包装只承担调用适配，不另保留一套独立产品循环。章节快照和原测试输入保持历史含义。

**退出条件：原生产品闭环与历史命令均按同一冻结基准通过后，才移除临时 bridge。** 移除候选还须复跑同一基准，确认原命令、结果、退出码和异常边界继续成立；失败则不能宣称退出完成。退出不以“文件已搬完”或仅新产品测试通过为依据，不删除历史命令和快照。

待 R0 明确的兼容决定：移除临时 bridge 后，旧教学入口由哪种稳定方式承接。可选方式涉及保留薄兼容门面或使用固定教学版本，当前条目不替用户选定，也不据此扩大迁移范围。只有承接方式与基准明确且退出验证通过，才能执行移除。

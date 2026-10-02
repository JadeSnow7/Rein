# Rein

**REIN — Runtime for Emergent Intelligence Networks**

产品线采用 **Rein 作为基础设施、Web Studio 作为第一验证场景、Veriflow 作为方法论**，沿需求分析 → 规格拆解 → 基准先行 → 约束实现 → 可信验证 → 规范分发推进。三方职责、首个网页修复纵切及验收基准见 [整合规格](REIN-INTEGRATION-SPEC.md)；最小 Web Studio 客户端在 R1c 接入，多资源与 DAG 随后扩展。

产品演进方向见 [设计文档](REIN-DESIGN.md)、[功能与模块设计](REIN-MODULES.md)和[实施改造清单](REIN-IMPLEMENTATION-PLAN.md)；设计依据见 [CLI 与聚合平台源码调研](reports/2026-09-20-rein-architecture-research.md)。运行时设计和当前教学实现分别记录，文档规划不代表功能已交付。

Rein core 保留完整 Harness，既可独立执行任务，也可作为调度 Agent，经 AgentMux 委派外部 Agent；运行时统一约束任务、权限、预算和验收。架构决定见 [D12](DECISIONS.md)。首批离线只读运行时已有可运行入口，当前验收状态见 [R1a 记录](records/REIN-RUNTIME-R1A-20260921/task-summary.md)；Coordinator 与外部 Agent 委派仍待实施。

## 当前阅读：六部分框架

从 [00 我们要实现一个怎样的 Agent](docs/chapters/minimal-agent.md)开始。现行 [六部分目录](docs/toc.md)包含 00–29 共 30 章：第一部分 00–03 已有 Python 正文与配套程序，围绕 C++ Hello World 完成模型调用、报错读取、差异审查与接受后的编译验证；[04 Agent 的核心能力](docs/chapters/agent-capabilities.md)已有初稿：先分析代表性编码 Agent，再由人写设计、代码助手实现，把不依赖模型协议的行为迁移到 `rust-hello-world/`，并用对照脚本检查一致。05–29 新版正文尚未完成。当前配套代码在 `python/hello_world/`；旧 `python/part1/` 的 README 只读建议器保留历史用途。

`book/chapters.json` 是现行六部分清单；旧五篇 25 章清单已移至 `book/history/chapters-v2.json`。旧数字 URL、TS/Rust 正文和历史快照保持原义，入口见 [历史目录](docs/history.md)。旧验证记录不代表新章验收。


《Rein：从零构建 Agent》是一本面向 Agent 学习者与求职者的工程实践书。全书围绕 Agent Harness 展开：第一至五部分由人确定架构和设计、AI 编写代码，第六部分研究把设计也交给 Agent 的完全 vibe coding 需要哪些前提（见 [D18](DECISIONS.md)）。

## 仓库目录

书籍与配套代码共用本仓库：

| 目录 | 用途 | 当前状态 |
| --- | --- | --- |
| `docs/chapters/` | 当前正文与历史页面 | 新 00–03 为 Python 第一部分；04 为能力分析与 Rust 迁移；旧版页面保留原主题 |
| `docs/roadmap/` | 六部分后续章节规划 | 新 05–29 按部分提供可达入口，均不冒充正式正文 |
| `python/hello_world/` | 当前三章的终端代码修改助手 | 单文件、用户接受后写入、固定编译验证；默认离线 |
| `python/part1/` | 前一版 README 只读建议器 | 保留原实现、固定输入、测试和 S1 清单 |
| `rust-hello-world/` | 第 04 章：第一部分非模型行为的 Rust 迁移 | 独立 crate；`compare_parity.py` 在 25 个样本上对照 Python |
| `docs/readings/` | 阅读材料 0–3 | 阅读 0 与阅读材料 2 已提供；其余待撰写 |
| `docs/milestones/` | 六部分的阶段成果与历史汇总 | 新 S1–S6 入口分别标注现状和预期；旧汇总保留历史身份 |
| `docs/appendices/` | 实现对照与深入讨论 A.1–A.5 | A.2 已提供；其余待撰写 |
| `core/` | 无 I/O 的产品状态机 | 首批 R1a：会话、观察、意图、预算与验收状态转换 |
| `runtime/` | 原生离线只读运行时 | 首批 R1a：SQLite、artifact、固定 verifier 与 CLI；使用方式见 [运行时说明](runtime/README.md) |
| `ts/` | TypeScript 入门、Node host、领域插件与历史 loop 对照 | 第 01 章实现、host 与测试材料已落地 |
| `rust/` | 历史 Rust 教学实现、M1 stdio host 与测试 | 旧 05–08 与 M1 材料可作机制对照；不是新 04 的 Rust 迁移（见 `rust-hello-world/`） |
| `contracts/` | provider、扩展进程及产品运行时合同 | `runtime/` 子目录由 Rust 权威 DTO 生成 schema 和 TS；章节协议保留 |
| `fixtures/` | 共享验收输入、模型响应与 hybrid marker | 现有录制、工具输入和 hybrid marker 可按章节使用 |

影响长期结构的决定记录在 [DECISIONS.md](DECISIONS.md)；迁移记录入口为 [MIGRATIONS.md](MIGRATIONS.md)。

现行章节顺序、标题、状态与路由由 `book/chapters.json` 管理，目录由 `npm run book:generate` 生成；规划页与正式正文的状态须分开。新增章节同时核对侧栏、章间链接、历史入口和 [内容开放说明](docs/access.md)。从 04 起，每章正式正文末尾附[仅供参考的提示词模板](docs/prompt-examples.md)，不能把模板写成实际开发记录。

## 本地阅读

```bash
npm ci
npm run dev
```

打开终端提示的本地地址即可预览。

## 运行代码

新第一部分从仓库根运行，不需要模型密钥：

```bash
python3 python/hello_world/cli.py hello
python3 python/hello_world/cli.py generate
python3 -m unittest discover -s python/hello_world/tests -v
```

完整编译、诊断、接受/放弃流程见三章正文，先创建临时练习目录，不直接编辑仓库样本。Python 环境和可选 SDK 安装见 [第 01 章](docs/chapters/python-model-call.md)。下列 TS/Rust 命令用于已有工程和历史材料，不能代替新第一部分的验证。

`ts/` 是根仓库的 npm workspace，与文档共用一次安装：

```bash
npm ci
npm test        # vitest
npm run typecheck
npm run hybrid:demo
npm run hybrid:verify
```

模型密钥复制 `ts/.env.example` 为 `ts/.env` 后填写，不入库。`tsx` 不会自动加载 `.env`，运行真实请求或录制命令时需显式传入 `--env-file=.env`；当前测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

旧教学 Rust 工程继续独立验证：

```bash
cargo test --locked --manifest-path rust/Cargo.toml
```

产品运行时使用独立的根 Cargo workspace，先运行 `cargo test --locked --workspace --all-targets`，再按 [运行时说明](runtime/README.md) 执行离线 demo。它不需要模型密钥；完整 R1a 和三产品集成仍按 [实施计划](REIN-IMPLEMENTATION-PLAN.md) 推进。

历史真实调用步骤见 [TypeScript 正文](docs/chapters/01.md)和 [Rust 正文](docs/chapters/01-rust.md)。本地测试使用受控响应，不证明真实端点可用。

## 取某一章的代码

`ts/src/` 与 `rust/src/` 随正文演进。默认使用当前交付的工作副本；需要复现旧 TypeScript 第 01 章时，可使用快照 `ch01-helloworld`：

```bash
git switch --detach ch01-helloworld
```

该命令会进入 detached HEAD；请先保存当前改动或在副本中操作。原 `ch01` 标签保留手写 HTTP 客户端、测试和两份录制，不包含新版 SDK 教程。`ch01-helloworld` 在本地交付；尚未推送时，新克隆的远程仓库取不到它，请使用交付的本地仓库。其余章节快照随章节完成并核验后建立。

阅读 0 已提供 [TypeScript 知识补充](docs/readings/00-ts.md)与 [Rust 知识补充](docs/readings/00-rust.md)，对应 `ts/examples/reading-00/` 与 `rust/examples/reading-00/` 中的可选示例。它们按需补充正文所需的基础知识，不是进入第 01 章的前置条件；安装依赖后无需密钥即可运行，具体命令见各版材料。专项测试需按材料中的命令单独运行，根目录 `npm test` 仍检查 TS 正式调用代码。

计划标注"可独立阅读"的后续章节会按模块职责在 `ts/examples/` 或 `rust/examples/` 提供自包含最小示例，不承诺两套完整 Harness，也不依赖主线累积状态。

## 构建与发布

```bash
npm run build
npm run preview
```

GitHub Pages 由 `.github/workflows/deploy.yml` 自动发布。推送到 `main` 或手动运行工作流都会构建 `docs/.vitepress/dist` 并部署。仓库设置中需要将 Pages 来源设为 **GitHub Actions**。代码测试由 `.github/workflows/test.yml` 在 `ts/`、`rust/`、`contracts/`、`fixtures/` 等相关路径变更时运行。

网站：https://jadesnow7.github.io/Rein/

当前默认入口为六部分主线的 Python 00–03，04 为初稿，05–29 为后续规划入口。已有 TS/Rust 材料仍可独立阅读；`ch08:compare`、`ch08:verify` 属于旧第 08 章，输出范围是离线上下文合同与本地进程记录。

本仓库当前自有代码、文档与教材统一采用 [Apache-2.0](LICENSE)，永久免费开放；第三方内容保留各自的许可声明。旧快照、原稿及历史决策中的开放安排仅作为历史记录，当前安排以 [内容开放说明](docs/access.md) 为准。许可证变更不表示规划中的章节或运行时能力已经完成。

## 旧版混合路线与历史入口

下列说明只解释旧版编号：01–03 章先用 TypeScript 建立调用直觉，Rust hello 是可选准备；从旧 05 章起，Rust core 负责权威运行时，TypeScript 负责宿主、SDK 和领域插件。旧 04 章的 provider adapter contract 与后续 plugin stdio 是两种不同边界。当前新路线见 [阅读指南](docs/about.md)与[完成状态](docs/toc.md)。

每个主题只在侧栏出现一次。旧版语言按钮只出现在阅读 0 与旧第 01 章这两个确实拥有 TS/Rust 对应正文的主题；05–07 的 Rust core 页面与旧 TS 页面是主线和历史对照，不伪装成语言变体。核心小节和练习编号对齐，语言特有知识放在相关主题下。公共内容、正文、实现和验证进度分别说明，不用本地预备练习代替正式章节交付。历史 `ch01` 快照保持不变，新版使用单独标签。

# Rein

Rein 的产品职责是 **单 Agent Harness / runtime 与 CLI**：模型适配、上下文构造、工具执行循环、执行权限与预算、取消、会话恢复、事件及原始产物。书稿与教学实现继续保留在本仓库。

**当前 main 交付的是教学实现，还没有统一的产品 runtime API。** 《Rein：从零手写一个 Agent Harness》及配套示例仍是当前阅读入口；下面的产品边界和接口候选不表示能力已经实现。

## 产品职责与实现状态

| 项目 | 拥有的职责 | 交接方式 |
| --- | --- | --- |
| Rein | 一个 Agent / 一次执行尝试的模型、上下文、工具循环、权限执行、局部预算、取消、恢复、事件和产物 | 接收有界执行请求，返回运行状态与原始回执 |
| [Veriflow](https://github.com/JadeSnow7/Veriflow) | 验收契约、任务依赖、跨 Agent 调度、验证计划、失败诊断与修复安排、证据组织及整体验收 | 通过版本化 RuntimePort 驱动 Rein；不直接写 Rein 会话存储 |
| [Web Studio](https://github.com/JadeSnow7/Web-Studio) | Web 工作空间、浏览器与终端环境、CDP、页面操作、截图、日志、运行时状态观测、调试及人工审阅界面 | 向执行器提供环境能力与原始观测；不替 Veriflow 判定任务通过 |

Rein 保留执行一个固定检查并返回回执的能力，也可独立运行自己的单 Agent 示例；项目验收条件、跨任务门槛及修复策略由 Veriflow 管理。目标项目自己的业务契约仍是验收依据。跨 Agent 的资源选择、依赖就绪、全局预算、重试与整合不进入 Rein core。

| 可核验位置 | 实际范围 |
| --- | --- |
| main：`python/hello_world/` | `hello / generate / diagnose / edit / check`，有界读取、局部工具循环、单文件确认写入与固定 C++ 检查 |
| main：`python/part1/API-CONTRACT.md`、`ts/src/`、`rust/src/` | 历史只读建议器合同、模型调用与教学接口；不能当作跨项目运行 API |
| 未合并的 [PR #3](https://github.com/JadeSnow7/Rein/pull/3)，审查提交 `44e3454` | R1a 纯步进状态机、SQLite/outbox、artifact、固定 verifier 和 `demo / show / resume / cancel / schema`；尚未进入 main，不能据此宣称通用模型或调度已交付 |
| 本轮 [RuntimePort 0.1 候选](contracts/runtime-port-v0.1.md) | 文档契约；没有新增命令、服务、SDK 或跨 Agent 执行器 |

本轮决定、与 PR #3 旧分工的冲突及后续实现切片见 [职责边界同步](DECISIONS.md#职责边界同步2026-10-04)。合并在途 PR 时必须沿用这个更新后的分工，不能恢复“Rein 负责 Coordinator / DAG、Veriflow 仅为方法论”的旧定位。

## 仓库目录

书籍与配套代码共用本仓库：

| 目录 | 用途 | 当前状态 |
| --- | --- | --- |
| `docs/chapters/` | Python 00–03 章正文与旧版章节 | Python 主线 00–03 已发布；旧版 TS/Rust 与其余规划页面保留 |
| `docs/readings/` | 阅读材料 0–3 | 阅读 0 公共导读、TS 与 Rust 两版及练习已提供；其余待撰写 |
| `docs/milestones/` | 第一阶段小结与三次阶段汇总 | 已建入口，待撰写 |
| `docs/appendices/` | 实现对照与深入讨论 A.1–A.5 | 已建入口，待撰写 |
| `ts/` | TypeScript 实现：源码、测试与独立示例 | 第 01 章实现与测试已落地 |
| `rust/` | Rust 实现与测试 | 第 01 章 SDK 调用与测试、阅读 0 独立练习已落地 |
| `python/hello_world/` | 当前三章 Hello World 生成、修复与验证 | 包含 CLI、样本和离线 / SDK 模拟测试 |
| `python/part1/` | 前一版 README 只读建议器 | 保留实现和历史样本 |
| `contracts/` | 教学共享合同与跨项目运行接口候选 | 教学合同仍按原路线推导；新增 RuntimePort 文档候选，尚无实现 |
| `fixtures/` | 共享验收输入与预期结果 | 已有两份模型响应录制；工具验收用例待第 03 章 |

影响长期结构的决定记录在 [DECISIONS.md](DECISIONS.md)；迁移记录入口为 [MIGRATIONS.md](MIGRATIONS.md)。

章节写作直接编辑对应 Markdown 文件。新增页面时同步更新 `docs/toc.md` 和 `docs/.vitepress/config.mts`；免费类型须与 [内容开放说明](docs/access.md) 一致。

## 本地阅读

```bash
npm ci
npm run dev
```

打开终端提示的本地地址即可预览。

当前推荐阅读路线是 Python 00–03 章，从 [最小 Agent](docs/chapters/minimal-agent.md) 开始；配套代码位于 `python/hello_world/`。三章围绕同一个 C++ Hello World，依次完成生成、报错诊断、全文与差异审查、接受后备份及编译验证。修改结构见[三章方案](reports/2026-09-22-hello-world-three-chapters-plan.md)。

```bash
python3 python/hello_world/cli.py hello
python3 -m unittest discover -s python/hello_world/tests -v
```

## 运行代码

`ts/` 是根仓库的 npm workspace，与文档共用一次安装：

```bash
npm ci
npm test        # vitest
npm run typecheck
```

模型密钥复制 `ts/.env.example` 为 `ts/.env` 后填写，不入库。`tsx` 不会自动加载 `.env`，运行真实请求或录制命令时需显式传入 `--env-file=.env`；当前测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

Rust 正式工程的验证在仓库根目录执行：

```bash
cargo test --locked --manifest-path rust/Cargo.toml
```

两版的真实调用步骤见 [TypeScript 正文](docs/chapters/01-ts.md)和 [Rust 正文](docs/chapters/01-rust.md)。本地测试使用受控响应，不证明真实端点可用。

## 取某一章的代码

`ts/src/` 与 `rust/src/` 随正文演进。新版 SDK 教程使用第 01 章快照 `ch01-helloworld`：

```bash
git switch --detach ch01-helloworld
```

原 `ch01` 标签保留手写 HTTP 客户端、测试和两份录制，不包含新版 SDK 教程。`ch01-helloworld` 在本地交付；尚未推送时，新克隆的远程仓库取不到它，请使用交付的本地仓库。其余章节快照随章节完成并核验后建立。

阅读 0 已提供 [TypeScript 知识补充](docs/readings/00-ts.md)与 [Rust 知识补充](docs/readings/00-rust.md)，对应 `ts/examples/reading-00/` 与 `rust/examples/reading-00/` 中的可选示例。它们按需补充正文所需的基础知识，不是进入第 01 章的前置条件；安装依赖后无需密钥即可运行，具体命令见各版材料。专项测试需按材料中的命令单独运行，根目录 `npm test` 仍检查 TS 正式调用代码。

计划标注"可独立阅读"的后续章节也将在 `ts/examples/` 提供自包含最小示例，不依赖主线累积状态。

## 构建与发布

```bash
npm run build
npm run preview
```

GitHub Pages 由 `.github/workflows/deploy.yml` 自动发布。推送到 `main` 或手动运行工作流都会构建 `docs/.vitepress/dist` 并部署。仓库设置中需要将 Pages 来源设为 **GitHub Actions**。代码测试由 `.github/workflows/test.yml` 在 `python/`、`ts/`、`rust/`、`contracts/`、`fixtures/` 等相关路径变更时运行。

网站：https://jadesnow7.github.io/Rein/

当前仓库包含首页、阅读指南、规划目录、第 01 章公共导读与双语言版本、阅读 0 的公共导读与双语言预备材料，以及其余章节占位页。

本书目前限时免费。在持续更新过程中，部分限时免费章节将逐步转为收费；标注"永久免费"的章节将保持免费开放。具体收费范围、时间和价格以届时公告为准。GitHub 与 Pages 的公开历史会保留；政策说明不构成付费墙，未来付费交付将使用独立系统。

## 双语言阅读结构

全书保留共同章节编号。正文按主线连续推进，阅读材料在需要时补充知识。阅读 0 使用公共导读与 TS / Rust 两版，第 01 章使用公共导读与 TS / Rust SDK 调用两版。当前入口见 [阅读指南](docs/about.md)与[完成状态](docs/toc.md)。

每个主题只在侧栏出现一次，语言按钮切换已有版本并尽量保留相同知识点。核心小节和练习编号对齐，语言特有知识放在相关主题下。公共内容、正文、实现和验证进度分别说明，不用本地预备练习代替正式章节交付。历史 `ch01` 快照保持不变，新版使用单独标签。

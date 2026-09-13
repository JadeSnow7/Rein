# Rein

《Rein：从零手写一个 Agent Harness》是一本面向 Agent 学习者与求职者的工程实践书。

## 仓库目录

书籍与配套代码共用本仓库：

| 目录 | 用途 | 当前状态 |
| --- | --- | --- |
| `docs/chapters/` | 00–16 章正文 | 第 01 章公共导读与 TS 版已提供；Rust 待补齐，其余待撰写 |
| `docs/readings/` | 阅读材料 0–3 | 阅读 0 公共导读、TS 与 Rust 两版及练习已提供；其余待撰写 |
| `docs/milestones/` | 第一阶段小结与三次阶段汇总 | 已建入口，待撰写 |
| `docs/appendices/` | 实现对照与深入讨论 A.1–A.5 | 已建入口，待撰写 |
| `ts/` | TypeScript 实现：源码、测试与独立示例 | 第 01 章实现与测试已落地 |
| `rust/` | Rust 实现与测试 | 阅读 0 独立练习已落地；正式 Harness 实现待建设 |
| `contracts/` | 两条路线重叠能力的共享合同 | 第 04 章定义 |
| `fixtures/` | 共享验收输入与预期结果 | 已有两份模型响应录制；工具验收用例待第 03 章 |

影响长期结构的决定记录在 [DECISIONS.md](DECISIONS.md)；迁移记录入口为 [MIGRATIONS.md](MIGRATIONS.md)。

章节写作直接编辑对应 Markdown 文件。新增页面时同步更新 `docs/toc.md` 和 `docs/.vitepress/config.mts`；免费类型须与 [内容开放说明](docs/access.md) 一致。

## 本地阅读

```bash
npm ci
npm run dev
```

打开终端提示的本地地址即可预览。

## 运行代码

`ts/` 是根仓库的 npm workspace，与文档共用一次安装：

```bash
npm ci
npm test        # vitest
npm run typecheck
```

模型密钥复制 `ts/.env.example` 为 `ts/.env` 后填写，不入库。`tsx` 不会自动加载 `.env`，运行真实请求或录制命令时需显式传入 `--env-file=.env`；当前测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

## 取某一章的代码

`ts/src/` 只保留一份随正文演进的代码。已建立第 01 章初版快照 `ch01`，包含调用代码、测试和两份响应录制：

```bash
git checkout ch01
```

第 01 章后续修订补充了教学步骤并修正 429 错误提示，原 `ch01` 快照保持不变；跟随修订正文时请使用包含这些修订的版本。其余章节快照随章节完成并核验后建立。

阅读 0 已提供 [TypeScript 知识补充](docs/readings/00-ts.md)与 [Rust 知识补充](docs/readings/00-rust.md)，对应 `ts/examples/reading-00/` 与 `rust/examples/reading-00/` 中的可选示例。它们按需补充正文所需的基础知识，不是进入第 01 章的前置条件；安装依赖后无需密钥即可运行，具体命令见各版材料。专项测试需按材料中的命令单独运行，根目录 `npm test` 仍检查 TS 正式调用代码。

计划标注"可独立阅读"的后续章节也将在 `ts/examples/` 提供自包含最小示例，不依赖主线累积状态。

## 构建与发布

```bash
npm run build
npm run preview
```

GitHub Pages 由 `.github/workflows/deploy.yml` 自动发布。推送到 `main` 或手动运行工作流都会构建 `docs/.vitepress/dist` 并部署。仓库设置中需要将 Pages 来源设为 **GitHub Actions**。代码测试由 `.github/workflows/test.yml` 在 `ts/`、`contracts/`、`fixtures/` 变更时运行。

网站：https://jadesnow7.github.io/Rein/

当前仓库包含首页、阅读指南、规划目录、第 01 章公共导读与 TS 版、阅读 0 的公共导读与双语言预备材料，以及其余章节占位页。

本书目前限时免费。在持续更新过程中，部分限时免费章节将逐步转为收费；标注"永久免费"的章节将保持免费开放。具体收费范围、时间和价格以届时公告为准。GitHub 与 Pages 的公开历史会保留；政策说明不构成付费墙，未来付费交付将使用独立系统。

## 双语言阅读结构

全书保留共同章节编号。正文按主线连续推进，阅读材料在需要时补充知识。阅读 0 使用公共导读与 TS / Rust 两版，第 01 章使用公共导读与 TS 版，Rust 正式调用待补齐。当前入口见 [阅读指南](docs/about.md)与[完成状态](docs/toc.md)。

每个主题只在侧栏出现一次，语言按钮切换已有版本并尽量保留相同知识点。核心小节和练习编号对齐，语言特有知识放在相关主题下。公共内容、正文、实现和验证进度分别说明，不用本地预备练习代替正式章节交付。章节重组不改变现有代码快照。

# Rein

《Rein：从零手写一个 Agent Harness》是一本面向 Agent 学习者与求职者的工程实践书。

## 仓库目录

书籍与配套代码共用本仓库：

| 目录 | 用途 | 当前状态 |
| --- | --- | --- |
| `docs/chapters/` | 00–16 章正文 | 第 01 章已完成，其余已建入口待撰写 |
| `docs/readings/` | 阅读材料 0–3 | 已建入口，待撰写 |
| `docs/milestones/` | 第一阶段小结与三次阶段汇总 | 已建入口，待撰写 |
| `docs/appendices/` | Rust 对照附录 A.1–A.5 | 已建入口，待撰写 |
| `ts/` | TypeScript 主线：源码、测试与独立示例 | 第 01 章实现与测试已落地 |
| `rust/` | Rust 对照源码与测试 | 目录说明，尚未实现 |
| `contracts/` | 两条路线重叠能力的共享合同 | 第 04 章定义 |
| `fixtures/` | 共享验收输入与预期结果 | 目录约定已冻结，尚无用例 |

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

`ts/src/` 只保留一份随正文演进的代码。章节快照 tag 计划在对应章节完成并核验后建立，当前尚未建立：

```bash
# 计划中的章节完成并核验后再建立快照 tag
# git checkout ch03
```

计划标注"可独立阅读"的章节将在 `ts/examples/` 提供自包含最小示例；当前仅有目录说明，不依赖主线累积状态。

## 构建与发布

```bash
npm run build
npm run preview
```

GitHub Pages 由 `.github/workflows/deploy.yml` 自动发布。推送到 `main` 或手动运行工作流都会构建 `docs/.vitepress/dist` 并部署。仓库设置中需要将 Pages 来源设为 **GitHub Actions**。代码测试由 `.github/workflows/test.yml` 在 `ts/`、`contracts/`、`fixtures/` 变更时运行。

网站：https://jadesnow7.github.io/Rein/

当前仓库包含首页、阅读指南、规划目录、第 01 章正文与其余章节占位页。

本书目前限时免费。在持续更新过程中，部分限时免费章节将逐步转为收费；标注"永久免费"的章节将保持免费开放。具体收费范围、时间和价格以届时公告为准。GitHub 与 Pages 的公开历史会保留；政策说明不构成付费墙，未来付费交付将使用独立系统。

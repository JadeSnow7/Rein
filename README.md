# Rein

《Rein：从零手写一个 Agent Harness》是一本面向 Agent 学习者与求职者的工程实践书。

## 仓库目录

书籍与配套代码共用本仓库：

| 目录 | 用途 | 当前状态 |
| --- | --- | --- |
| `docs/chapters/` | 00–16 章正文 | 已建入口，待撰写 |
| `docs/readings/` | 阅读材料 0–3 | 已建入口，待撰写 |
| `docs/milestones/` | 第一阶段小结与三次阶段汇总 | 已建入口，待撰写 |
| `docs/appendices/` | Rust 对照附录 A.1–A.5 | 已建入口，待撰写 |
| `ts/src/`、`ts/tests/` | TypeScript 主线源码与测试 | 目录说明，尚未实现 |
| `rust/src/`、`rust/tests/` | Rust 对照源码与测试 | 目录说明，尚未实现 |
| `contracts/` | 两条路线重叠能力的共享合同 | 尚未定义合同 |
| `fixtures/` | 共享验收输入与预期结果 | 尚未添加用例 |

章节写作直接编辑对应 Markdown 文件。新增页面时同步更新 `docs/toc.md` 和 `docs/.vitepress/config.mts`；免费类型须与 [内容开放说明](docs/access.md) 一致。迁移记录入口为 [MIGRATIONS.md](MIGRATIONS.md)。

## 本地阅读

```bash
npm ci
npm run dev
```

打开终端提示的本地地址即可预览。新增章节时，在 `docs/` 下创建 Markdown 文件，并同步更新 `docs/.vitepress/config.mts` 中的导航或侧栏。

## 构建与发布

```bash
npm run build
npm run preview
```

GitHub Pages 由 `.github/workflows/deploy.yml` 自动发布。推送到 `main` 或手动运行工作流都会构建 `docs/.vitepress/dist` 并部署。仓库设置中需要将 Pages 来源设为 **GitHub Actions**。

网站：https://jadesnow7.github.io/Rein/

当前发布首页、阅读指南、规划目录与章节占位页，尚无章节正文或 Agent 运行时代码。`ts/`、`rust/`、`contracts/` 与 `fixtures/` 仅记录未来实现边界；后续计划使用 TypeScript 主线与 Rust 对照 track。

本书目前限时免费。在持续更新过程中，部分限时免费章节将逐步转为收费；标注“永久免费”的章节将保持免费开放。具体收费范围、时间和价格以届时公告为准。GitHub 与 Pages 的公开历史会保留；政策说明不构成付费墙，未来付费交付将使用独立系统。

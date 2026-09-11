# Rein

《Rein：从零手写一个 Agent Harness》是一本面向 Agent 学习者与求职者的工程实践书。

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

当前仅发布首页、阅读指南与规划目录，尚无章节正文或 Agent 运行时代码。后续计划使用 TypeScript 主线与 Rust 对照 track；正式书版 tag 和 `MIGRATIONS.md` 将随实现与发布建立。

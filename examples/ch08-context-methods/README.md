# 第08章独立示例

这是一个可复制的最小资料集。入口从仓库根目录运行，通过 `scripts/ch08-compare.mjs` 进入 Rust core，再由 core 通过既有协议启动真实 Node `read_file` 宿主。

```bash
node examples/ch08-context-methods/entry.mjs
```

它复制本目录自有的 `input/index.json`、`input/tasks.json` 和 `docs/`，输出四种策略的可检查结果，并验证 JSON Lines、来源 `doc-demo`、quality 1 以及真实进程 records。示例只验证离线回答，不调用真实模型服务；Rust 到 Node 的 `read_file` 进程边界仍会实际运行。正文中的 mutation 练习另行展示旧 oracle 与新资料不一致的情况。

# Rust track

Rust 是本书的权威运行时路线。当前已提供[阅读 0 · Rust 版](../docs/readings/00-rust.md)与[第 01 章 Rust 版](../docs/chapters/01-rust.md)；05–08 章的 Rust core、Node 宿主边界与测试也已提供，后续章节仍按规划推进。

- `src/`：第 01 章入口与 05–08 Rust core
- `tests/`：正式工程的验收用例
- `examples/reading-00/`：独立 Cargo 项目，练习本地响应解析、错误处理与异步调用；运行和验证命令见材料正文

`rust/` 根目录的 Cargo manifest 与锁文件属于正式工程；`examples/reading-00/` 仍是独立预备练习，`examples/ch05_loop.rs`、`ch06_loop.rs`、`ch07_context.rs` 和 `hybrid_stdio.rs` 提供章节入口。测试使用本地 HTTP mock，不请求真实模型服务；真实调用由读者自行配置并记录。

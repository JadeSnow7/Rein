# Rust track

Rust 是本书两种实现路线之一。当前已提供[阅读 0 · Rust 版](../docs/readings/00-rust.md)与[第 01 章 Rust 版](../docs/chapters/01-rust.md)。根目录工程用于一次 OpenAI 兼容 Chat Completions 调用，目录中的后续设计对照主题与附录仍为规划。

- `src/`：第 01 章最小调用与安全入口
- `tests/`：正式工程的验收用例（当前测试放在库模块中）
- `examples/reading-00/`：独立 Cargo 项目，练习本地响应解析、错误处理与异步调用；运行和验证命令见材料正文

`rust/` 根目录的 Cargo manifest 与锁文件属于第 01 章正式工程；`examples/reading-00/` 仍是独立预备练习。测试使用本地 HTTP mock，不请求真实模型服务；真实调用由读者自行配置并记录。

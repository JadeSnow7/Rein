# rust-hello-world：第 04 章的 Rust 迁移

第一部分 Python 终端助手中**不依赖模型协议**的行为，等价迁移到 Rust：受限读取与三个只读工具、全文审查与 diff、确认后写入（复查、备份）、固定编译运行检查。模型调用和工具往返协议留到第 05 章；这里的候选来自 Python `diagnose` 输出的 `proposal.json`。

这是独立的教学 crate，不属于仓库根目录的产品 workspace。

```bash
# 在仓库根目录
cargo build --manifest-path rust-hello-world/Cargo.toml
cargo test  --manifest-path rust-hello-world/Cargo.toml
python3 rust-hello-world/compare_parity.py        # 在 25 个冻结样本上对照 Python 与 Rust
```

命令行：

```bash
rein-hello check --workspace DIR
rein-hello tool  --workspace DIR --call '{"id":"t1","name":"read_file","arguments":{"path":"hello.cpp"}}'
rein-hello edit  --workspace DIR --proposal proposal.json [--color auto|always|never]
```

| 模块 | 职责 | 对应 Python |
| --- | --- | --- |
| `error.rs` | 错误类别枚举，`code()` 与 Python 的错误字符串一致 | `HelloError.code` |
| `workspace.rs` | 只读 `hello.cpp`、`compiler.log`，限制大小与编码 | `safe_read` |
| `tools.rs` | `read_file`、`read_environment`、受限 `bash` 的检查与执行 | `dispatch_tool`、`run_bash` |
| `review.rs` | 行号全文与 unified diff（按 `difflib` 的匹配规则） | `render_review` |
| `apply.rs` | `Decision`、`ApplyOutcome`；唯一写入 `hello.cpp` 的地方 | `apply_candidate` |
| `check.rs` | 固定命令编译、只运行本轮产物、判定输出 | `check_cpp` |
| `process.rs` | 不经 shell 启动程序，带超时并收集输出 | `subprocess.run` |

实现记录与验收输出见 `records/REIN-CH04-RUST-20260923/`。

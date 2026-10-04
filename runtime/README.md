# Rein Runtime CLI

Runtime 使用 `--state-dir` 保存 SQLite 会话、outbox、artifact 引用和 verification plan。命令输出一行 JSON；退出码只表示命令执行成功或失败，不代表 verification 已 Passed。

```bash
# 使用一个尚未创建过任务的新目录
cargo run --locked -p rein-runtime -- demo --state-dir /tmp/rein-demo --fixture fixtures/runtime-r1a/alpha.txt
cargo run --locked -p rein-runtime -- show --state-dir /tmp/rein-demo

# 单独演示先准备、再恢复
cargo run --locked -p rein-runtime -- demo --state-dir /tmp/rein-resume-demo --fixture fixtures/runtime-r1a/alpha.txt --prepare-only
cargo run --locked -p rein-runtime -- resume --state-dir /tmp/rein-resume-demo

# 在另一个准备状态上验证取消
cargo run --locked -p rein-runtime -- demo --state-dir /tmp/rein-cancel-demo --fixture fixtures/runtime-r1a/alpha.txt --prepare-only
cargo run --locked -p rein-runtime -- cancel --state-dir /tmp/rein-cancel-demo
node scripts/runtime/check-contracts.mjs --check
```

`demo` 只创建一次 state。重复执行会失败且不覆盖已有数据库或 workdir。`--prepare-only` 只创建会话、workdir 内的只读文件副本和固定 verification plan，不派发后续 effect。首次运行会把 fixture 复制到 workdir/fixture.txt 并将副本设为只读；恢复时继续读取 state 副本，即使原始文件被修改或删除也不改变 oracle。

`demo` 可选 `--expected FILE` 指定冻结的验证基准、`--budget N` 指定工具调用预算；默认 expected 为输入 fixture，预算为 10。工具只读取根内普通 UTF-8 文件，最多 1 MiB，不提供 shell、网络或写工具。状态目录由当前用户控制，文件权限与路径检查不构成 OS 级沙箱。

`show` 是只读查询，不创建 state 目录；它显示已保存的状态快照，不重新验证 artifact 的实时完整性。`resume` 会显式处理遗留的 claimed effect：它被标为 `OutcomeUnknown`，不会自动重放；仍处于 pending 的效果才会继续。`cancel` 持久保存取消状态；取消先于派发事务提交时，效果不会执行。每个运行 driver 持有 state 目录的独占 owner 文件锁，第二个 driver 会被拒绝；已通过派发门槛的调用可能完成，但取消后其结果不能继续推进或验收。

`schema --check` 只校验 Rust 生成的四份 JSON schema。`node scripts/runtime/check-contracts.mjs --check` 额外校验对应 TypeScript 生成物，并使用仓库本地的 TypeScript 编译器；检查模式不会生成或改写文件。Rust 要求 1.89 或更高版本。离线验证使用已缓存 Cargo 依赖和 `CARGO_TARGET_DIR=/private/tmp/rein-r1a-target`。

这是 R1a 的首个离线纵切：它证明确定性模型、真实 read_file 结果、SQLite durable state 和独立 verifier 的闭环，不代表完整产品（真实模型、写入批准、外部 Agent、daemon、多客户端、Web Studio 与发行流程仍在后续范围）。源码和正文按 Apache-2.0 开放，见仓库根目录 `LICENSE`。

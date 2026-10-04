# Rein 章节快照

这些 portable 包对应已经冻结的历史章节。它们只用于恢复当时的文件树和测试，不包含当前 Rust core + TypeScript plugin 主线后来增加的扩展协议。下面每段恢复命令都从原书仓库根目录单独开始；请把包解压到新目录，不要覆盖原仓库：

```sh
REIN_BOOK_ROOT="$PWD"
REIN_RESTORE_DIR="$(mktemp -d)"
tar -xzf "$REIN_BOOK_ROOT/chapter-snapshots/rein-ch05.tar.gz" -C "$REIN_RESTORE_DIR"
cd "$REIN_RESTORE_DIR/rein-ch05"
# TypeScript 示例：
npm ci --workspace ts
npm test --workspace ts
# Rust 示例：
cargo build --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml
```

需要从 pre-ch05 逐步恢复到 ch05 时，从仓库根目录另建一个临时副本并使用相邻补丁：

```sh
REIN_BOOK_ROOT="$PWD"
REIN_PRE_DIR="$(mktemp -d)"
tar -xzf "$REIN_BOOK_ROOT/chapter-snapshots/rein-pre-ch05.tar.gz" -C "$REIN_PRE_DIR"
cd "$REIN_PRE_DIR/rein-pre-ch05"
git init
git add -A
git apply --binary "$REIN_BOOK_ROOT/chapter-snapshots/pre-ch05-to-ch05.patch"
```

这里的 `git init` 只用于临时副本，不创建 commit。补丁是可选的 Git binary patch；相邻 manifest 记录文件 hash 和 Git 执行位。包内容是对应冻结章节的事后归档，后续章节源码会继续演进。


恢复到 ch06 时，从仓库根目录另建临时副本并解包：

```sh
REIN_BOOK_ROOT="$PWD"
REIN_RESTORE_DIR="$(mktemp -d)"
tar -xzf "$REIN_BOOK_ROOT/chapter-snapshots/rein-ch06.tar.gz" -C "$REIN_RESTORE_DIR"
cd "$REIN_RESTORE_DIR/rein-ch06"
```

从已接受的 ch05 快照应用到 ch06 时，在 ch05 文件树的临时副本中使用：

```sh
git init
git add -A
git apply --binary "$REIN_BOOK_ROOT/chapter-snapshots/ch05-to-ch06.patch"
```

对应清单是 `rein-ch06.manifest.json`。恢复到 ch07 时使用新的临时目录：

```sh
REIN_BOOK_ROOT="$PWD"
REIN_RESTORE_DIR="$(mktemp -d)"
tar -xzf "$REIN_BOOK_ROOT/chapter-snapshots/rein-ch07.tar.gz" -C "$REIN_RESTORE_DIR"
cd "$REIN_RESTORE_DIR/rein-ch07"
npm ci --workspace ts
npm test --workspace ts
```

`rein-ch07.manifest.json` 记录包内文件的 SHA-256 和执行位。若已有 ch06 临时副本，也可以初始化临时 Git 工作区后应用 `ch06-to-ch07.patch`：

```sh
git init
git add -A
git apply --binary "$REIN_BOOK_ROOT/chapter-snapshots/ch06-to-ch07.patch"
```

勘误：ch06 归档中的 `ToolObserver` 旧注释只是可信的本地闭包说明，并不提供文件写入限制。实际运行边界以正文和代码中的工具分派规则为准。历史包不应被描述成当前 plugin 实现；当前主线命令在整合工作区中运行。

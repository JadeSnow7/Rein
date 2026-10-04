# Rein 基线验证记录 — 2026-10-02

验证提交：`3d2e4ec74a2699c444d0cedc97e47242a571ed34`（六组工作树基线提交完成点）。
分支：`wip/rein-runtime-20261002`。本报告提交本身不属于被测源码提交。
执行时间（UTC）：`2026-10-02T13:48:21.214480+00:00` 至 `2026-10-02T13:49:40.109517+00:00`。

11项实际执行命令均退出0。每项结束后，436个已批准候选文件的SHA均保持不变，Git工作树状态为空。未修复源码、测试、格式或锁文件，未安装依赖，未调用真实模型。

## 环境

- 本机为 macOS，Darwin 25.6.0，arm64。
- CI 使用 Ubuntu 和 Node 22；本次本机结果不表示完整复现CI环境。
- Rust：`rustc 1.98.0 (88d9e12ae 2026-08-18)`；Cargo：`cargo 1.98.0 (797e8a9bc 2026-08-05)`。
- rustfmt：`rustfmt 1.9.0-stable (88d9e12ae1 2026-08-18)`。
- Node：`v26.5.0`；npm：`11.17.0`。
- Xcode：Xcode 27.0；Build version 27A266a。
- Swift：swift-driver version: 1.168.6 Apple Swift version 6.4 (swiftlang-6.4.0.34.1 clang-2100.3.34.1)；Target: arm64-apple-macosx26.0。

## 执行约束与未原样执行的CI命令

- Cargo命令使用 `--locked` 和离线模式；格式检查只使用 `--check`。嵌套Cargo也继承 `CARGO_NET_OFFLINE=true`，仓库脚本本身已带 `--locked`。
- `CARGO_TARGET_DIR=/private/tmp/rein-phase1-20261002-task6/target`；临时文件与缓存也放在该临时根目录。
- npm使用现有依赖，设置离线、禁audit、禁update notifier。未运行 `npm ci`、安装或工具链下载步骤。
- 测试进程不继承真实服务凭据或模型端点相关环境变量；未输出变量值。
- **原CI命令 `npm test` 未原样执行**：默认Vite配置会加载 `ts/.env`，不符合本次凭据隔离约束。
- **替代调用**：使用 `/private/tmp/rein-phase1-20261002-task6/vitest.config.mjs` 执行相同TS测试集合，保持 `test.include=["tests/**/*.test.ts"]`、`environment="node"`，只隔离配置位置、`envDir`、`cacheDir`并禁缓存。替代调用的通过结果不记作原命令已通过。
- GitHub Actions的checkout、setup-node、Rust下载和 `npm ci` 未执行；Python、新 `rust-hello-world/`、文档构建和UI测试不属于本次CI命令范围，未扩展执行。

## 实际结果

| 编号 | 实际命令 | 退出码 | 通过 / 失败 / 忽略 | 秒 |
|---|---|---:|---|---:|
| `01-root-build` | `cargo build --workspace --locked --offline` | 0 | 不适用（构建或静态检查） | 8.223 |
| `02-root-test` | `cargo test --workspace --locked --offline` | 0 | 54 / 0 / 0 | 6.868 |
| `03-ts-typecheck` | `npm run typecheck` | 0 | 不适用（构建或静态检查） | 0.961 |
| `04-ts-test-isolated` | `npm run test --workspace ts -- --config /private/tmp/rein-phase1-20261002-task6/vitest.config.mjs` | 0 | 123 / 0 / 0（12个测试文件） | 1.529 |
| `05-old-rust-format` | `cargo fmt --manifest-path rust/Cargo.toml -- --check` | 0 | 不适用（构建或静态检查） | 0.178 |
| `06-old-rust-check` | `cargo check --manifest-path rust/Cargo.toml --locked --offline` | 0 | 不适用（构建或静态检查） | 16.061 |
| `07-old-rust-test` | `cargo test --manifest-path rust/Cargo.toml --locked --offline` | 0 | 76 / 0 / 0 | 41.809 |
| `08-root-format` | `cargo fmt --all -- --check` | 0 | 不适用（构建或静态检查） | 0.107 |
| `09-root-all-targets` | `cargo test --locked --offline --workspace --all-targets` | 0 | 54 / 0 / 0 | 1.142 |
| `10-schema-node-test` | `node --test scripts/runtime/schema-to-ts.test.mjs` | 0 | 6 / 0 / 0 | 0.727 |
| `11-contract-check` | `node scripts/runtime/check-contracts.mjs --check` | 0 | 不适用（构建或静态检查） | 0.919 |

失败用例名：无。上述实际执行测试套件均为0失败；未原样执行的 `npm test` 不计为通过或失败。
根workspace测试54项通过；all-targets是另一次54项通过的执行记录，两者不相加称为108个唯一用例。旧Rust测试76项、隔离TS集合123项、Node Schema测试6项通过。

## 日志校验

原始日志及逐命令退出码、起止时间、版本和执行计划保存在仓库外：
`~/Backups/phase1-20261002/Agent-Learning/commit-review-20261002/tests/`。
其中 `results.json`、`versions.json`、`plan.json` 保存元数据；以下SHA指向原始完整日志，不把日志内容整体入库。

| 日志文件 | 字节 | SHA-256 |
|---|---:|---|
| `01-root-build.log` | 1542 | `78517e12dfe0249742c98d199d79ec88bfe44701ab2125445c1cce290a87ce68` |
| `02-root-test.log` | 6144 | `c961611cde3ed9492f2e557b9fd7516afc18baacfb866d08eb07890d88419e4b` |
| `03-ts-typecheck.log` | 109 | `e9222c5c8bc472066e74fed6a14c31737be8541bba27ebfacda32e551f5e1752` |
| `04-ts-test-isolated.log` | 1128 | `4af979a161fd3642b6c6d4816afd7c127ac42edffa500b1274776956693d8d59` |
| `05-old-rust-format.log` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `06-old-rust-check.log` | 4608 | `9b1e38369f77b1d9a1e332aa093fcb61900405ad077d0a52ede6283af477adbd` |
| `07-old-rust-test.log` | 12798 | `e33f70b29f8d4648a0a716f41cf1e8f589eab1698e7a20b65c1cf24193e8279a` |
| `08-root-format.log` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `09-root-all-targets.log` | 5300 | `1bcfc6749f0ac1aa582cc2d9957303d6b1c8b71f0f2c323542b5c1b80b642fc5` |
| `10-schema-node-test.log` | 583 | `ec566ccb7370b6f67fff3fa566df8d2dfe06673bb244602f84849b733bac1c2c` |
| `11-contract-check.log` | 116 | `e893f24f535f10bade99a5b7e33c6c16aadf39a4ae31976f2d07c83e99fecbed` |

日志及本报告已按指定模式预扫描，无真实敏感命中；提交前另对暂存报告完整内容再次扫描。

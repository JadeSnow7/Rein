# Rein 合并后基线验证记录 — 2026-10-03

验证提交：`594da2b05b71236d9570933e81cca1a31e695d88`。分支：`integrate/main-20261002`。
本报告提交仅保存证据，不属于被测源码提交。
执行时间（UTC）：`2026-10-02T17:03:34.170413+00:00` 至 `2026-10-02T17:04:41.032793+00:00`。

16 条实际命令均退出 0。每条命令后 HEAD 不变，工作树状态为空。没有安装依赖、调用真实模型或修复产品源码。

## 环境与隔离

- 本机 macOS 26.6.2、arm64；Xcode 27.0（27A266a）。本机通过不等于 GitHub Actions 已运行。
- rustc：`rustc 1.98.0 (88d9e12ae 2026-08-18)`。
- cargo：`cargo 1.98.0 (797e8a9bc 2026-08-05)`。
- node：`v26.5.0`。
- npm：`11.17.0`。
- python：`Python 3.14.6`。

- 前 11 条 argv 与第一阶段 plan.json 一致。Cargo 使用 --locked/--offline，子进程继承 CARGO_NET_OFFLINE=true；target 复用 `/private/tmp/rein-phase1-20261002-task6/target`。
- npm 使用现有 node_modules，并设置 offline、禁 audit、禁 update notifier。进程环境只允许基础系统变量，未继承模型凭据或端点。
- TS 使用第一阶段隔离配置 `/private/tmp/rein-phase1-20261002-task6/vitest.config.mjs`，envDir 为已核对的空目录，cache=false，仍覆盖 tests/**/*.test.ts、environment=node。没有运行会加载 ts/.env 的默认 npm test。
- 已先读取 python/hello_world/README.md，使用现有 .venv 和固定本地/模拟响应测试；未安装依赖或运行真实请求入口。

## 实际结果

| 编号 | 实际命令 | 退出码 | 通过 / 失败 / 忽略 | 秒 |
|---|---|---:|---|---:|
| 01-root-build | `cargo build --workspace --locked --offline` | 0 | 不适用（构建或静态检查） | 0.209 |
| 02-root-test | `cargo test --workspace --locked --offline` | 0 | 54 / 0 / 0 | 2.321 |
| 03-ts-typecheck | `npm run typecheck` | 0 | 不适用（构建或静态检查） | 0.905 |
| 04-ts-test-isolated | `npm run test --workspace ts -- --config /private/tmp/rein-phase1-20261002-task6/vitest.config.mjs` | 0 | 123 / 0 / 0 | 1.481 |
| 05-old-rust-format | `cargo fmt --manifest-path rust/Cargo.toml -- --check` | 0 | 不适用（构建或静态检查） | 0.135 |
| 06-old-rust-check | `cargo check --manifest-path rust/Cargo.toml --locked --offline` | 0 | 不适用（构建或静态检查） | 8.703 |
| 07-old-rust-test | `cargo test --manifest-path rust/Cargo.toml --locked --offline` | 0 | 76 / 0 / 0 | 37.229 |
| 08-root-format | `cargo fmt --all -- --check` | 0 | 不适用（构建或静态检查） | 0.138 |
| 09-root-all-targets | `cargo test --locked --offline --workspace --all-targets` | 0 | 54 / 0 / 0 | 1.127 |
| 10-schema-node-test | `node --test scripts/runtime/schema-to-ts.test.mjs` | 0 | 6 / 0 / 0 | 0.77 |
| 11-contract-check | `node scripts/runtime/check-contracts.mjs --check` | 0 | 不适用（构建或静态检查） | 0.891 |
| 12-book-check | `npm run book:check` | 0 | 不适用（构建或静态检查） | 0.185 |
| 13-site-build | `npm run build` | 0 | 不适用（构建或静态检查） | 4.549 |
| 14-python-hello-test | `python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v` | 0 | 42 / 0 / 0 | 4.926 |
| 15-source-version-test | `node --import tsx --test docs/.vitepress/theme/sourceVersionState.test.ts` | 0 | 7 / 0 / 0 | 0.1 |
| 16-book-links | `node scripts/book-links.mjs` | 0 | 不适用（构建或静态检查） | 0.243 |

根 workspace 54、教学 Rust 76、TypeScript 123（12 个文件）、schema 6，与第一阶段完全一致。all-targets 再次执行同一组 54，不加总为 108 个唯一用例。新增 Python 42、主题状态 7，均 0 失败、0 忽略。没有待归因的测试失败。

book:check 确认 6 部分、30 章、6 个里程碑，历史清单存在，生成目录同步。VitePress 构建成功，有 >500 kB chunk 体积提示；未为该非失败提示改构建策略。额外 book-links 检查 93 个 HTML、5,539 次链接/锚点，failures=[]。

## 主题测试的常规覆盖状态

`node --import tsx --test docs/.vitepress/theme/sourceVersionState.test.ts`：7 通过，0 失败，0 跳过。

当前没有 npm 脚本或 CI 作业运行这份主题测试。根 package.json 的 test 只转到 ts workspace，ts/vitest.config.ts 只收集 tests/**/*.test.ts；.github/workflows/test.yml 的 Node 单测为 scripts/runtime/schema-to-ts.test.mjs，deploy.yml 构建文档但不运行主题测试。本轮显式执行不能当作已建立持续覆盖。未擅自新增 CI 作业。

## 合并与忽略规则核对

A3 合并提交 `5c77332275141637f7f0606c5a71f0b7a567cd4c`，两父提交为 `6c34a5b66b53f310bf914ba43c6c0ca17c5b005f` 和 `93fd7203428962ae741cc86b8d7e87657341df64`；32 项处理为取本地 12、取上游 0、内容合并 20。

相对 wip 的 A3 变更为 57 个文件：20 个内容合并文件、36 个上游独有文件、用户新增授权的 reports/2026-10-03-python-published-behavior-differences.md。12 个取本地文件逐字节保持；core/runtime/contracts 树对象与 wip 完全一致。

A4 独立提交 `594da2b05b71236d9570933e81cca1a31e695d88` 只改 .gitignore，1,094 行改为 50 行。clean 提交前后 git status --porcelain --ignored 均为 400 行，集合及字节完全一致，SHA-256 为 `a7962fb3167f99de803da14e6ba3628cdfcedc38afca74cd3997f1e49a958696`；展开全部文件后的 17,999 条状态也一致。比较期间唯一正常 dirty 项为 .gitignore，提交后完整输出恢复一致。

## 日志校验

完整计划、版本、逐条元数据和日志位于仓库外：`/Users/huaodong/Documents/Codex/2026-10-02/files-pasted-by-the-user-main/work/rein-phase2/a5-594da2b`。

| 日志 | 字节 | SHA-256 |
|---|---:|---|
| 01-root-build.log | 72 | `d7dae192715495fa89ccbe279af3efb9792c1dae1768228e369035e68a1df673` |
| 02-root-test.log | 5575 | `683ac8903ff3ed1aaaea5d635dfd2672aff6acd89ef63b030484b5778892662b` |
| 03-ts-typecheck.log | 109 | `e9222c5c8bc472066e74fed6a14c31737be8541bba27ebfacda32e551f5e1752` |
| 04-ts-test-isolated.log | 1128 | `690e83f98388e16c8be19860c8b4956dc66de291ed04970c68011a3b135c45b0` |
| 05-old-rust-format.log | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 06-old-rust-check.log | 426 | `220360c8f0703a8606debf04b70a1659c0be1441b40d6cdd6f30dbbd7c068bfc` |
| 07-old-rust-test.log | 9626 | `620b4e3923889d217f1868eca7ba15cbb6a33b07073d4917731aa41fb1551a88` |
| 08-root-format.log | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 09-root-all-targets.log | 5300 | `cdda4c03602d899061ca2dc06db19bb9f56124926e55001cb414e13500804ef8` |
| 10-schema-node-test.log | 585 | `6410caa8282992a89a5084fd8cb4ba96f6653c7fc78fbc9048078ba8c5140221` |
| 11-contract-check.log | 116 | `e893f24f535f10bade99a5b7e33c6c16aadf39a4ae31976f2d07c83e99fecbed` |
| 12-book-check.log | 243 | `3e5493858d427f1f9146fda95c0697bbdaeeadbe8a33faacc79693ac0461eba4` |
| 13-site-build.log | 567 | `10e53b658b979b5613b310131a640deef7a666938a944f7729465d7a6df2a37e` |
| 14-python-hello-test.log | 7776 | `15a807515c1b99e832e05d7fb41ecc73eca69d2f651f9d2a096da44962c316f8` |
| 15-source-version-test.log | 664 | `02d36efd42ca1dd92af705f52452053633e64afee3ae20a580932b9771b55931` |
| 16-book-links.log | 80 | `d75f61fa046f4f08489b267e28c3a95c42743f4d9b148a55ef0d41839566706e` |

## 证据边界

本轮证明列出的离线样本、静态检查及本地站点构建结果；真实模型、读者跟做、章节快照教学验收与线上部署未运行。两个 main 均未更新。A6 推送集成分支及创建 PR 尚待用户确认；PR 合并由用户操作。

# ch01 双语言交付验收

验证日期为 2026-09-13。工作分支为 `codex/ch01-helloworld`，新版本地快照为 `ch01-helloworld`，历史 `ch01` 保留不变。未推送 GitHub，未部署 Pages，未请求真实模型服务。

## 教学与文稿

已按用户确认的路线完成公共导读、TypeScript 与 Rust 正文。正文均以「在ChatGPT中」起笔，使用指定标题「HelloWorld——从模型调用开始」。从环境、源码切换、API key、hello 调用推进到逐步错误处理、结果记录和四项练习。补全判断见 [写作路线](ch01-writing-plan.md)，四层编辑质检见 [TS](ch01-ts-quality.md)与 [Rust](ch01-rust-quality.md)。

旧 TS 手写 HTTP 教程保存到 `docs/chapters/01-ts-advanced.md`，作为有明确历史背景的延伸阅读。原客户端、录制和样本保留。

## 代码验证

| 范围 | 结果 |
| --- | --- |
| `npm run typecheck` | 通过 |
| `npm test` | 6 个文件、65 个测试通过，其中新 SDK 测试 18 项 |
| `node --import tsx --test docs/.vitepress/theme/sourceVersionState.test.ts` | 3 项通过 |
| `cargo fmt --manifest-path rust/Cargo.toml -- --check` | 通过 |
| `cargo check --manifest-path rust/Cargo.toml --locked` | 通过 |
| `cargo test --manifest-path rust/Cargo.toml --locked` | 7 项库测试通过；二进制与 doc-test 无单独测试项 |
| `cargo build --manifest-path rust/Cargo.toml --locked --bins` | 两个 Rust 入口成功构建 |

本地 Node 为 26.5.0，Rust/Cargo 为 1.98.0。TS 依赖锁定 `openai 4.104.0`，Rust 锁文件解析为 `async-openai 0.29.6`。本地 Rust 验证设置 `CARGO_TARGET_DIR=/tmp/rein-cargo-target`，不改变读者的 Cargo 使用方式。CI 配置使用 Node 22 和 Rust stable；本轮未运行远程 CI，不把本机结果冒充 CI 结果。

既有 TS transport 测试和 Rust HTTP 测试需要本机回环监听。沙箱初次限制监听、Rust 初次限制依赖下载，后在获准的命令权限下完成；这些环境限制没有作为源码失败或成功证据。

主线程审查时修正了 SDK 默认重试、错误文本误分类、空回答误判、整体超时、凭据错误回显、源码路径与文稿片段不匹配、站点全局标题误改等问题。Rust SDK 丢失部分 HTTP 状态的限制在正文中明确，未用文本猜测补造状态码。

## 独立命令行进程验证

主线程另用 Python 本地 HTTP 模拟服务运行四个真实 CLI 进程。TS 两个入口和 Rust 两个已构建二进制各执行 6 项，共 24 项通过。输入仅为 hello、虚构本地模型 ID 和虚构 key，Rust 进程在临时空目录启动，TS 不传 env-file。

| 输入 | 检查 |
| --- | --- |
| HTTP 200，非空文本 | stdout 为本地模拟回答，退出码 0 |
| HTTP 200，空 choices | 退出码 1 |
| HTTP 401，凭据错误 | 退出码 1，不回显错误正文中的虚构 key |
| HTTP 429，额度不足 | 退出码 1，不自动重试 |
| HTTP 400，未知且含虚构 key 的 code | 退出码 1，诊断不回显该 code |
| 本地空 key | 退出码 1，服务收到 0 次请求 |

前五项逐次检查服务收到恰好 1 次请求，并核对路径、Bearer 头、model 和单条 user hello 消息。首次运行复核脚本时，Rust 二进制路径按默认目录猜测有误；确认子代理实际临时构建目录后，Rust 两入口全部重跑通过。此问题只涉及复核脚本定位，不涉及教材中的 Cargo 运行命令。

## 书籍页面

`npm run build` 通过。构建输出 40 个 HTML 页面，内部文件链接与锚点检查 0 错误。

在本地预览中打开 TS 正文，并从 `01-ts.html#failures` 点击 Rust 语言按钮，实际导航到 `01-rust.html#failures`，选中状态与侧栏同步。截图检查正文和代码块可读。整体视觉样式保持既有书籍设计，新增 span 锚点参与同位置语言切换。

正文引用的源码随构建读取，最终构建在源码修订完成后执行。真实调用记录仍为未验证；获取新标签的远程命令要等标签发布后才适用于新克隆仓库，正文已说明。

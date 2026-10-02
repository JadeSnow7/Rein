# 01 HelloWorld：从模型调用开始（Rust 对照）

**状态：文字修订稿，待教学验收｜永久免费 · Apache-2.0**

在聊天应用中输入 `hello` 后，界面替我们处理了地址、凭据、网络等待和回答展示。本篇把同一个动作放到终端：用 Rust 发出一条用户消息，取得非空回答，并在失败时说明下一步检查方向。

本篇是 [TypeScript 主线](./model-hello.md)的可选对照，可以独立跟做。接口同样采用 OpenAI 兼容的非流式 Chat Completions。Rust 使用社区维护的 `async-openai`，它不是 OpenAI 官方 Rust SDK；本工程锁定版本为 `0.29.6`，接口可对照[该版本文档](https://docs.rs/async-openai/0.29.6/async_openai/)。

<span id="from-reading-0"></span>
<span id="first-run"></span>
<span id="setup"></span>

## 1、准备仓库与 Rust 环境

需要 Git 和 Rust。尚未安装时，查看 [Git 安装页](https://git-scm.com/downloads)与 [Rust 安装指南](https://www.rust-lang.org/tools/install)。安装后重新打开终端，检查工具：

```bash
git --version
rustc --version
cargo --version
```

Cargo 是 Rust 的构建和依赖管理工具，后面用它编译、运行和测试。若依赖提示编译器版本不足，按安装指南更新工具链。

在准备存放项目的目录克隆仓库。已有工作副本时，直接进入其仓库根目录，再进入 `rust/`。

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
cd rust
cargo build --locked
```

本篇使用当前配套工作副本中的 `rust/src/bin/hello.rs`、`rust/src/main.rs` 与 `rust/src/lib.rs`。新版逐章快照尚未冻结，远程版本也可能与本地修订稿不同；缺少入口时应先核对配套版本。旧 `ch01-helloworld` 标签不包含新版书籍索引，不作为本篇的必需步骤。

`--locked` 要求使用锁文件中的依赖。首次构建可能需要下载和编译 SDK；下载或编译失败发生在模型调用之前，应先处理 Cargo 报错。本篇使用独立的教学 `rust/` 工程，不需要构建根目录的产品运行时。

<span id="live-call"></span>
<span id="api-key"></span>

## 2、填写调用配置

访问模型服务需要 API key、根地址和模型名。可以在 [DeepSeek 官方平台](https://platform.deepseek.com/)创建 key，并在[模型与价格页](https://api-docs.deepseek.com/quick_start/pricing/)确认计费方式；也可以使用已准备好的兼容服务。网页聊天账号和 API 凭据需要分别核对。

仍在 `rust/`，首次配置时复制模板。已有 `.env` 时跳过复制，直接编辑原文件。

```bash
cp .env.example .env
```

用编辑器填入三项配置。下方根地址与模型名来自 2026-09-22 查阅的 [DeepSeek 首次调用说明](https://api-docs.deepseek.com/)，实际运行前仍需核对当前可用配置。

```dotenv
REIN_BASE_URL=https://api.deepseek.com
REIN_API_KEY=替换为自己在控制台创建的密钥
REIN_MODEL=deepseek-flash
```

三项应来自同一服务。SDK 会在根地址后请求 `/chat/completions`，不要重复填写完整接口路径；服务要求根地址包含 `/v1` 时照其文档保留。

程序通过 `dotenvy` 加载 `rust/.env`，不会读取 `ts/.env`。终端已有的同名环境变量优先于文件；配置改了却未生效时，先检查这一点。`.env` 已被 Git 忽略，但没有加密，key 不应写进源码、截图或运行记录。

<span id="request-response"></span>

## 3、让程序发出一句 hello

仍在 `rust/` 执行：

```bash
cargo run --locked --bin hello
```

`hello` 是最简程序的名称，源文件为 `rust/src/bin/hello.rs`，固定发送 `hello`。命令会请求配置的服务，产生相应 API 用量。成功时可能看到下面这样的回答，文字只是示意，不是运行记录。

```text
Hello! How can I help you today?
```

Cargo 的编译与启动信息不是模型回答。在 macOS、Linux 或 WSL 的 shell 中，紧接着运行 `echo $?` 查看退出码；PowerShell 使用 `$LASTEXITCODE`，CMD 使用 `%ERRORLEVEL%`。成功应为 `0`，失败为 `1`。取得非空文本后，还需要自己判断它是否回应了问候。

<span id="implementation"></span>

## 4、看懂这一段 SDK 调用

下面直接引用仓库中可以运行的最简程序：

<<< ../../rust/src/bin/hello.rs

先看请求的构造。`ChatCompletionRequestUserMessageArgs` 建立用户消息，`.content("hello")` 放入问候；`CreateChatCompletionRequestArgs` 再把模型名和消息装进请求。`.build()` 检查这些值能否构造成 SDK 所需的类型，随后由 `client.chat().create(request)` 发出调用。

客户端来自 `lib.rs` 的 `build_client()`。下面是函数内部的配置，`config` 保存已经读取的地址、key 和模型名：

```rust
Client::with_config(
    OpenAIConfig::new()
        .with_api_key(&config.api_key)
        .with_api_base(&config.base_url),
)
.with_backoff(backoff::ExponentialBackoff {
    max_elapsed_time: Some(Duration::ZERO),
    ..Default::default()
})
```

`with_api_key` 和 `with_api_base` 配置请求去向与凭据；零重试等待预算用于关闭这个锁定版本 SDK 的自动重试。一次方法调用背后仍然经过网络：

```text
hello → SDK 构造 JSON → POST /chat/completions
      → 服务返回响应 → SDK 解析
      → choices[0].message.content → 终端回答
```

`choices` 是候选列表。Rust 的 `.first()` 不假设列表一定有元素，后续 `Option` 操作继续检查消息内容；全空白文本也会被拒绝。这些检查能帮助程序取得文本，但不能判断回答本身是否正确。

<span id="failures"></span>

## 5、从请求经过的位置理解错误

最简程序在失败时只给出一条诊断提示。需要更清楚的错误分类时，使用完整入口，它支持接收自己的提示词：

```bash
cargo run --locked --bin rein-ch01-helloworld -- hello
```

`--bin` 选择要运行的程序，后面的 `--` 分隔 Cargo 参数和程序参数。这一次会重新请求服务，可能产生用量；应先判断错误原因再决定重试。

第一层是配置。`src/main.rs` 先调用 `load_config()`，只有配置成功才继续网络请求：

```rust
let result = match load_config() {
    Ok(config) => chat(&config, &prompt).await,
    Err(error) => Err(error),
};
```

`Ok` 携带可以继续使用的值，`Err` 携带错误。三项配置不能为空，根地址还要通过 URL 检查；但格式合法不意味着地址一定可达。

第二层是等待与传输。在 `chat_with_timeout()` 中，SDK 调用被放进 30 秒的等待期限：

```rust
let response = tokio::time::timeout(timeout, client.chat().create(request))
    .await
    .map_err(|_| AppError::Timeout)?
    .map_err(classify_error)?;
```

外层 `map_err` 把等待超时转成 `Timeout`；内层把 SDK 失败交给错误分类函数。问号 `?` 在错误时交回调用者，不继续读取文本。网络问题可以从 DNS、代理、TLS 和端点查起；超时只说明本地没有及时完成，不能证明服务端没处理过请求。

第三层是服务拒绝。锁定的 `async-openai 0.29.6` 在部分 API 错误中保留服务商 `code/type`，没有保留原始 HTTP 状态码，所以程序不从错误文字猜数字。`insufficient_quota` 提示检查额度或余额，`rate_limit_exceeded` 提示检查请求频率；未知错误码会使用固定诊断，仍需查看服务商资料。

第四层是响应内容。SDK 成功返回后，程序还要取出非空文本：

```rust
response
    .choices
    .first()
    .and_then(|choice| choice.message.content.as_deref())
    .filter(|text| !text.trim().is_empty())
    .map(str::to_owned)
    .ok_or(AppError::ResponseFormat)
```

候选为空、文本缺失或全是空白，都会成为 `response-format`。坏 JSON 或缺少 SDK 必需字段，可能更早就在解析阶段失败。完整入口把这些错误打印为短诊断，并以退出码 `1` 结束；成功则打印回答并退出 `0`。

<span id="verification"></span>

## 6、先检查本地行为

暂时没有可用 key 时，可以在 `rust/` 运行：

```bash
cargo fmt --check
cargo check --locked
cargo test --locked --lib
```

这组命令检查教学工程的格式、编译和库测试。测试用本地 HTTP 服务提供受控响应，不读取你的 `.env`，也不请求真实模型；测试环境需要允许回环端口监听。Cargo 首次获取依赖仍可能需要网络。

查看 `rust/src/lib.rs` 的 `tests` 模块，可以找到请求体、超时、响应结构和服务错误的用例。它们与真实账户能否连通是两项检查，不能互相代替。

<span id="exercises"></span>

## 7、完成四个小练习

<span id="exercise-01"></span>

**练习 1：改变输入。** 仍在 `rust/`，用完整入口发送另一句问候，指出请求中哪个字段变化。这是一次真实调用。

```bash
cargo run --locked --bin rein-ch01-helloworld -- '你好，请用一句话回应我'
```

<span id="exercise-02"></span>

**练习 2：制造配置错误。** 在其他配置已正确填写的基础上，给这次进程一个空模型名。以下采用 macOS、Linux 或 WSL 的 shell 写法：

```bash
REIN_MODEL='' cargo run --locked --bin rein-ch01-helloworld -- hello
echo $?
```

预期诊断为 `config`，退出码为 `1`，不会发送模型请求。前缀赋值只影响这次命令，不会修改 `.env`。

<span id="exercise-03"></span>

**练习 3：解释失败用例。** 重跑第 6 节的库测试，在 `lib.rs` 中找出超时、服务拒绝和坏响应的测试，各写一句排查方向，再说明为什么 HTTP 成功仍可能得不到可用回答。

<span id="exercise-04"></span>

**练习 4：整理调用记录。** 记录实际调用的时间、请求的模型名、输入、回答和退出码；没有调用就写未运行。CLI 未打印服务端回报的模型标识，所以配置值只作为请求模型名，不视为已核实的服务端元信息。记录中不要包含 key。

<span id="comparison"></span>
<span id="transport"></span>
<span id="recording"></span>

对照 TypeScript 时，先看同一条消息如何进入请求，再看失败如何返回。两种语言的错误写法和 SDK 保留的信息不同，但都要检查实际结果。想进一步观察手写 HTTP 与历史回放，可阅读 [TS 进阶材料](./01-ts-advanced.md)；它是历史补充，本篇 Rust 没有对应的录制命令。

<span id="next"></span>

接下来进入[第 02 章：任务与成功标准](./task-spec.md)。主线仍用 TypeScript 建立工具调用直觉，Rust 迁移安排在[第 05 章](./rust-migration.md)。

## 提示词示例

```text
我正在学习 Rein 的 Rust hello 对照，使用当前工作副本中的独立 rust/ 工程。
请结合 hello.rs、main.rs 和 lib.rs 解释配置、SDK 请求、响应解析和错误出口。
不升级依赖，不添加工具或循环，保留一次请求与关闭自动重试的边界。
先运行本地库测试，再协助我解释本章四个练习。
真实调用由我准备配置后执行，请区分本地测试、实际回答和尚未运行的部分。
```

“独立工程”明确命令位置；“一次请求”保持本章范围；“区分结果”避免用离线响应替代真实调用。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。

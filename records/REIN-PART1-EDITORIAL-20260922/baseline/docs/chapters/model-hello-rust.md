
# HelloWorld——从模型调用开始

在ChatGPT中，你输入 hello，按下发送，等一会儿，一句问候就出现在屏幕上。你有没有好奇过，输入框里的文字发到了哪里，回答又是怎么回来的？

你想想看，平时我们只需要关心对话。地址、凭据、网络、等待，都藏在界面后面。

这一章，把输入框换成终端。

就说一句 hello。

<span id="from-reading-0"></span>

我觉得，学 Rust 做模型调用，可以从这么一个很小的结果开始。准备配置，用 SDK 发出一条消息，拿到非空回答，再亲自读一下，它是不是回应了这句问候。

这就是本章任务。接口使用 OpenAI 兼容的非流式 Chat Completions，消息只有一条 `user`，默认内容是 `hello`。成功退出码是 `0`，失败是 `1`。我们还会逐步补上错误处理，让终端在没收到回答时给出下一步线索。

本篇与 [TypeScript 正文（修订中）](./01.md)做同一件事。Rust 使用社区维护的 `async-openai`，它并非 OpenAI 官方 Rust SDK，来源与用法可查[项目文档](https://docs.rs/async-openai/0.29.6/async_openai/)。不用先读完另一种语言。

<span id="first-run"></span>
<span id="setup"></span>

1、准备仓库与 Rust 环境

电脑上需要 Git 和 Rust。Git 可从[官方安装页](https://git-scm.com/downloads)取得，Rust 按 [rustup 官方指南](https://www.rust-lang.org/tools/install)安装。装好后重新打开终端，检查工具是否能找到。

```bash
git --version
rustc --version
cargo --version
```

本次在 Rust 1.98.0 环境验证，项目按 `Cargo.lock` 固定依赖。使用较旧的编译器时，如果依赖提示版本不足，先通过 rustup 更新工具链。Cargo 是 Rust 的构建和依赖管理工具，后面编译、运行、测试都会用到它。

在准备存放项目的目录执行。已经有本次交付的工作副本，直接进入它即可。

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

这里先说清源码版本。旧标签 `ch01` 仍保存此前的手写 HTTP 版本，新版使用 `ch01-helloworld`。新版标签目前在本地交付，尚未推送到远程时，新克隆仓库取不到它。预览本次交付请使用这份本地仓库；远程发布后，新克隆仓库才能按下面的命令获取它。

```bash
git fetch --tags
git switch --detach ch01-helloworld
cd rust
cargo build --locked
```

`git switch` 切换整个仓库的版本，不是运行第一章。已有修改先保存，想在快照上继续写代码，可以用 `git switch -c my-ch01` 建立自己的练习分支。

`--locked` 要求 Cargo 使用仓库锁定的依赖。首次构建要下载并编译 SDK，可能比你预期久一些。说真的，这时如果出现下载错误，我们还没开始调用模型，先处理依赖网络问题就好。

<span id="live-call"></span>
<span id="api-key"></span>

2、取得 key，把调用配置填完整

很多朋友可能不知道，网页聊天账号与 API 凭据需要分别准备。程序请求服务时，要带上控制台创建的 API key。

想从成本较低的选择开始，可以打开 [DeepSeek 官方平台](https://platform.deepseek.com/)注册或登录，在控制台创建 key 并查看余额。准备充值前，打开[模型与价格页](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/)，比较输入和输出单价，按这次练习的实际需要准备余额即可。

价格和模型会调整，所以这里保留官方入口，不写永久有效的低价承诺。你如果已经有其他兼容服务，也可以使用自己的配置。

回到这次 hello，要找齐三个值。`REIN_BASE_URL` 决定发往哪里，`REIN_API_KEY` 提供身份凭据，`REIN_MODEL` 指定模型。三项来自同一家服务，别把不同平台的配置拼在一起。

仍在 `rust/` 目录，首次配置复制模板。已经有 `.env` 时直接编辑，不要覆盖。

```bash
cp .env.example .env
```

Windows CMD 使用 `copy .env.example .env`。在编辑器里填入自己的值。

```dotenv
REIN_BASE_URL=https://api.deepseek.com
REIN_API_KEY=替换为自己在控制台创建的密钥
REIN_MODEL=deepseek-flash
```

上面的地址和模型来自 2026-09-16 查阅的 [DeepSeek 首次调用说明](https://api-docs.deepseek.com/zh-cn/)，运行时再核对当时的文档。SDK 会请求根地址下面的 `/chat/completions`，不要重复填写完整路径。

这块需要注意一下，本章从 `rust/` 启动程序，加载这里的 `.env`。已有进程环境变量可能优先于文件中的同名值，修改 `.env` 后仍调用旧配置，就检查终端里是否曾设置过变量。这里不读取 `ts/.env`。

key 留在本机即可，`.env` 已被 Git 忽略，源码和练习记录里都不用放它。

<span id="request-response"></span>

3、让 SDK 发出第一声 hello

仍在 `rust/` 目录执行。

```bash
cargo run --locked --bin hello
```

`hello` 是本章最简示例的程序名，源文件位于 `rust/src/bin/hello.rs`。它固定发送 `hello`。这条命令会请求配置的服务，并产生相应 API 用量。

成功时，终端可能出现下面这样的问候。这只是输出示意，并非本次运行的真实记录。

```text
Hello! How can I help you today?
```

Cargo 还可能打印编译或启动信息，那些不是模型回答。在 macOS / Linux shell 中，紧接着用 `echo $?` 查看退出码；PowerShell 查看 `$LASTEXITCODE`，CMD 查看 `%ERRORLEVEL%`。成功应为 `0`。

收到问候了？先停一秒。

这次是你的程序组织了请求，从网络另一头读回一段文字。它不必和示意逐字相同，只要确实有非空回答，再由你确认它回应了问候。

要是没有收到，继续往下看，我们会把问题拆小。

<span id="implementation"></span>

4、把这一小段 SDK 调用看清楚

下面直接引用可以运行的 `hello.rs`，代码和仓库保持一致。

<<< ../../rust/src/bin/hello.rs

Rust 的类型名可能显得长一点，先别急着背。`ChatCompletionRequestUserMessageArgs` 构造用户消息，`.content("hello")` 填入内容，`CreateChatCompletionRequestArgs` 再把模型名与消息装进请求。`.build()` 检查能否构造这个对象。

真正发出请求的是 `client.chat().create(request)`。

客户端由 `lib.rs` 中的 `build_client()` 创建，核心配置如下。这里是函数内部片段，`config` 是已经读取的三项配置。

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

它把地址和 key 交给 SDK，并把重试等待预算设为零。我们已经核对这个版本的实现，并用本地失败响应检查禁用重试的行为。

其实吧，方法看起来像本地函数，背后走的仍是 HTTP。

```text
hello → SDK 构造 JSON → POST /chat/completions
      → 服务返回响应 → SDK 解析
      → choices[0].message.content → 终端回答
```

本章的请求只需要 `model` 和一条 `user` 消息。响应里，`choices` 是候选列表，程序取第一个候选的消息内容。

Rust 的 `.first()` 不假设列表一定有元素，后面的 `Option` 操作继续检查内容存在。全空白文本也会被拒绝。SDK 的类型帮助我们读取结构，但不会替我们判断问候是否有意义。

<span id="failures"></span>

5、从网络的角度，逐步写出错误处理

怎么说呢，第一次看到长报错，难免会想，难道只是打声招呼也要这么麻烦？？？

先问一个小问题，最后确认成功的是哪一步？

第一层，可能根本没走到网络。`load_config()` 读取三项环境变量，检查空值和地址。主入口用 `match` 把这个结果接住，配置失败就直接保留错误。

```rust
let result = match load_config() {
    Ok(config) => chat(&config, &prompt).await,
    Err(error) => Err(error),
};
```

这段位于 `src/main.rs`。`Ok` 才继续请求，`Err` 则进入错误出口。URL 检查能发现格式问题，却不能证明远程地址今天一定能连上。

如果连 `async-openai` 依赖都没下载成功，或者编译器提示接口用错，还没有进入这段运行逻辑。先回到 Cargo 的构建错误，检查工具链和锁文件。程序里的网络异常处理接不到编译失败。

第二层，配置正确，等待却没有结束。在 `chat_with_timeout()` 中，把整个 SDK 调用放进等待期限。

```rust
let response = tokio::time::timeout(timeout, client.chat().create(request))
    .await
    .map_err(|_| AppError::Timeout)?
    .map_err(classify_error)?;
```

这里为什么有两次错误转换？

外层是 Tokio 的等待结果，超过期限就变成 `Timeout`。内层是 SDK 的调用结果，连接失败、服务拒绝或者响应无法解析，会再交给 `classify_error()`。问号 `?` 把失败交给调用者，不继续取文本。

顺着上面的路径，连接阶段可以这样匹配 SDK 的真实错误类型。下面摘自分类器的一个分支。

```rust
OpenAIError::Reqwest(error) => {
    if error.is_timeout() {
        AppError::Timeout
    } else {
        AppError::Network
    }
}
```

`Reqwest` 是 SDK 使用的 HTTP 客户端。网络错误可以从 DNS、代理、TLS、地址和服务状态查起。30 秒超时则只说明没有及时完成，不能断定服务端没处理过请求。这个我也没法隔着终端替你猜，先查服务商记录，再决定是否重发。

第三层，服务已经回复，但拒绝请求。如果拿到了原始 HTTP 状态，401 通常让你检查凭据，403 检查权限，404 核对路径和模型，5xx 查看服务状态。

这里有个 Rust SDK 的实际边界需要讲清。锁定的 `async-openai 0.29.6` 在部分 `ApiError` 中保留服务商的 `code/type`，没有保留原始 HTTP 状态码。所以程序不从英文错误文字里猜一个数字，而是识别已知的服务商错误码，未知值使用固定诊断。

`insufficient_quota` 指向额度或余额，需要检查控制台；`rate_limit_exceeded` 指向请求频率限制，后续动作不同。两者都可能对应 429，但看到 SDK 错误类型或者一个状态码，还不足以替你做完整判断。缺少线索时就查服务商控制台，不无限重发。

第四层，拿到响应，还要确认能取出回答。下面是 `chat_with_timeout()` 在 SDK 成功返回后的检查。

```rust
response
    .choices
    .first()
    .and_then(|choice| choice.message.content.as_deref())
    .filter(|text| !text.trim().is_empty())
    .map(str::to_owned)
    .ok_or(AppError::ResponseFormat)
```

没有候选、没有文本、全是空白，都会变成 `response-format`。坏 JSON 或缺少 SDK 必需字段，则可能更早在 SDK 解析阶段失败。HTTP 成功之后仍然要检查数据，这个顺序挺重要。

这些分支合起来，是 `lib.rs` 中的调用和错误分类。主入口负责输出短诊断，并在失败时退出 `1`。想对照全部实现，可以展开查看。

<details>
<summary>查看完整调用与错误处理实现</summary>

<<< ../../rust/src/lib.rs

</details>

仍在 `rust/` 中，运行有分类诊断的入口。这里 `--` 后面的内容是交给程序的参数。

```bash
cargo run --locked --bin rein-ch01-helloworld -- hello
```

这个入口支持自己的提示词，不传参数时仍为 hello。最简程序帮助我们看 SDK 请求，完整入口把失败接到更清楚的诊断上。

<span id="verification"></span>

6、总结这次调用的结果

有成功回答，就记下调用时间、配置中的模型 ID、输入 hello、回答摘要和退出码。CLI 没有打印服务端回报的模型标识，因此配置值只记成请求的模型名，别写成已经核实的服务端元信息。

暂时没有可用 key，可以先在 `rust/` 运行本地验证。

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
```

测试通过本地 HTTP 服务提供受控响应，检查请求和失败处理，不读取你的 `.env`，也不请求真实模型。Cargo 首次下载依赖仍需要网络，安装完成后的测试不依赖外部模型服务。

说实话，这里最容易混淆的是两种成功。代码在本地样例上通过测试，与今天的账户和模型能连通，需要分别记。没执行真实调用，就写「未验证」。

<span id="exercises"></span>

7、课后练习

<span id="exercise-01"></span>

练习 01，仍在 `rust/`，把 hello 换成另一句简短问候。入口已经支持参数，直接使用它即可。读一下回答，再指出请求中哪个字段发生变化。

```bash
cargo run --locked --bin rein-ch01-helloworld -- '你好，请用一句话回应我'
```

<span id="exercise-02"></span>

练习 02，用一个空 key 制造配置错误。macOS / Linux shell 可以在 `rust/` 执行下面的命令，进程环境的空值覆盖 `.env` 同名配置，不会发出模型请求。

```bash
REIN_API_KEY= cargo run --locked --bin rein-ch01-helloworld
echo $?
```

PowerShell 可在练习终端设置 `$env:REIN_API_KEY=''` 后运行。检查退出码是 `1`，提示指向 key，练习后恢复自己终端的环境设置。

<span id="exercise-03"></span>

练习 03，运行 `cargo test --locked`，找到超时、服务拒绝和坏响应的用例。每种写一句下一步检查方向，并说明 SDK 为什么不能凭空补出一个已丢失的 HTTP 状态码。

<span id="exercise-04"></span>

练习 04，整理一次真实调用记录，或者注明尚未执行。再解释一下，本地测试通过，为什么不能保证今天的 key 仍然有效？key 本身不要放进答案。

<span id="comparison"></span>

两版比较时，先看同一条 hello 怎么进去，再看失败怎么回来。TS 的异常捕获和 Rust 的 `Result` 写法不同，SDK 保留的现场信息也不同。把边界讲清楚，才知道两份程序各自确认了什么。

<span id="transport"></span>
<span id="recording"></span>

想继续研究手写 HTTP 和历史回放，可以看 [TS 进阶材料](./01-ts-advanced.md)。它展示另一条观察收发的路线；本章 Rust 没有录制入口，不能把 TS 的录制命令当作 Rust 命令。

<span id="next"></span>

再回到开头的 ChatGPT 输入框。

同样是一句问候，现在你能从终端发出它，也能解释回答从哪里取出、等待何时结束、失败怎样传回程序。我一直觉得，Hello World 作为入门题的那种朴素劲儿，在这里刚好合适，先让自己写的程序完成一件小事。

带着这次交换继续往前。计划中的[第 02 章](./02.md)，再讨论怎样把任务说清楚。

# 01 HelloWorld：从模型调用开始

**状态：新版初稿，未完成逐章衔接验收｜永久免费**

本页正在迁移为新版主题入口。下面保留可检查的已有教学内容；其中数字命令与代码入口仍指历史主题，不能把运行当前工程等同于完成新版前驱快照验收。


今天我们几乎每天都在使用 Agent 应用，但你有没有想过，在你向 ChatGPT 发送一句简单的“你好”时，背后经过了哪些流程呢？

像 ChatGPT、DeepSeek 这一类的 Agent 应用帮我们实现了中间复杂的过程，我们在日常对话时只会感受到“发送消息——等待回答”这一层抽象。而要学习 Agent 开发，就必须了解背后的流程。

本章，我们将会在终端完成一次最简单的模型调用，了解从一句问候到收到回复之间我们需要完成的工作。

<span id="from-reading-0"></span>

本章只处理最简单的 OpenAI 兼容的非流式请求。

<span id="first-run"></span>
<span id="确认环境-先完成一次离线检查"></span>

1、环境准备

本书需要用到 Git 和 Node.js 22 或更高版本。如果你尚未安装，可以分别从 [Git 安装页](https://git-scm.com/downloads)和 [Node.js 下载页](https://nodejs.org/en/download)安装。Node.js 通常会一起安装 npm，装好后重新打开终端。

```bash
git --version
node --version
npm --version
```

看到三个版本号即代表已经安装完成。

2、拉取代码

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

现在我们就已经成功拉取源码并进入仓库根目录了。

接下来在当前交付的工作副本继续本章代码；如果需要复现本章快照，先保存当前改动，再在副本中切换到本地交付的 `ch01-helloworld` 标签。远程新克隆的仓库可能不包含这个本地标签。

```bash
npm ci
```

当前工作副本已经包含完整工程和依赖配置，建议先在自己的副本或分支中沿本章完成流程；如果当前目录有自己的改动，请先保存，再切换快照。单独新建一个项目需要自行补齐工程、依赖和配置，本章不把它作为可直接复制的替代路径。

`npm ci` 按锁文件安装依赖，包括本章使用的 `openai` SDK。安装的过程中可能遇到网络环境问题，可以按需查看[阅读 0 的安装说明](../readings/00-ts.md#setup)。当然在实际调用模型时也可能遇到网络问题，我们稍后会讨论这一点。

<span id="live-call"></span>
<span id="配置-尽早指出哪里没填好"></span>

3、准备 API key

就像你访问 ChatGPT  App 需要登录一样，在终端调用模型同样需要对应的凭据来判断请求来自哪个账户，这就是 API key。

本章用 DeepSeek 作为配置示例。你可以在 [DeepSeek 官方平台](https://platform.deepseek.com/)注册或登录，在控制台创建自己的 key，查看账户余额，再打开[模型与价格页](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/)核对可用模型的输入和输出单价。先按本章的少量调用需要准备余额即可。

请按服务商当前的官方文档自选兼容的 OpenAI 格式端点、模型和计费方案；本章只依赖这些配置项的接口形状，不对具体服务商或价格作推荐。

4、配置环境变量

仅仅有 API key 还不够，我们还需要 baseURL，它代表了我们将会向哪个站点发送信息；还必须填写本次请求使用的模型 `model`。本章程序会检查三项配置，缺少 `REIN_MODEL` 时直接报错。

截至 2026-09-16 查阅的 [DeepSeek 首次调用说明](https://api-docs.deepseek.com/zh-cn/)，OpenAI 格式的根地址为 `https://api.deepseek.com`，示例模型为 `deepseek-flash`。填写时请核对当时可用的模型名。

从仓库根目录进入 `ts/`，首次配置时复制模板。

```bash
cd ts
cp .env.example .env
```

如果仓库中已经有了 `.env`，直接编辑它，不要用模板覆盖现有文件。

```bash
nano .env
```

Windows CMD 用 `copy .env.example .env`。在编辑器里打开 `.env`，填写三项。

```dotenv
REIN_BASE_URL=https://api.deepseek.com
REIN_API_KEY=替换为自己在控制台创建的密钥
REIN_MODEL=deepseek-flash
```

**由于 Windows 不是 POSIX 兼容的，我们强烈建议 Windows 用户在 WSL 下进行开发**

由于 SDK 会在根地址后请求 `/chat/completions`，因此不要把完整接口路径填进 `REIN_BASE_URL`。

**注意，为了避免 API key 泄露造成不必要的损失，`.env` 已经被 Git 忽略。把 key 留在本机环境变量里即可，不要向任何人提供。**

如果你是在完整工程之外自行创建项目，请把 `.env` 加入 `.gitignore`。API key 是凭据，`.env` 的 Git 忽略只防止误提交，不能把 key 当成加密数据。

<span id="request-response"></span>
<span id="调用-先写清楚一种协议"></span>

5、发出第一句问候

正如我们学习编程时从 helloworld 开始一样，我们将会从一句最简单的 hello 请求开始。

仍在 `ts/` 目录，执行这一条。

```bash
node --import tsx --env-file=.env src/hello.ts hello
```

`tsx` 负责运行 TypeScript，`--env-file=.env` 指定加载的配置，末尾的 `hello` 就是我们要交给模型的话。不写最后一个参数，程序也会默认使用 hello。

这条命令会请求配置的服务，并产生相应 API 用量。成功时，终端可能出现这样一句话。不过大模型的输出是随机的，因此每次回复的内容会有些许变化，这点我们稍后会谈到。

```text
Hello! How can I help you today?
```

<span id="implementation"></span>
<span id="解析-http-成功之后还要检查什么"></span>

6、SDK 为我们做的工作

SDK（Software Development Kit，软件开发套件） 可以理解为一组已经写好的调用方法。本章安装的是 OpenAI 官方维护的 `openai` 包，参照 [SDK 说明](https://developers.openai.com/api/docs/libraries)；本项目锁定版本，可以直接由 `npm ci` 复现安装。

接下来我们打开 `ts/src/hello.ts`，并理解下面的代码，`config` 来自本章的配置读取，`prompt` 来自命令行；它是解释用片段，完整入口已经在刚才运行的文件中。

```ts
const client = new OpenAI({
  apiKey: config.apiKey,
  baseURL: config.baseUrl,
  maxRetries: 0,
  timeout: 30_000,
})

```

`new OpenAI` 将会新建一个客户端实例，并填入请求地址和凭据。后面两个参数是最大重试次数和超时时长，用来应对异常情况。

```ts
const response = await client.chat.completions.create({
  model: config.model,
  messages: [{ role: 'user', content: prompt }],
  stream: false,
})
```

接下来我们可以通过刚建立的客户端示例发起请求，`model` 选择模型，`messages` 即我们要发送的信息，`role: 'user'` 表示这句话来自用户。`prompt`参数来自我们之前的终端输入，`await` 等待请求返回，`stream: false` 表示非流式请求。

7、SDK 背后的 HTTP 响应过程

```text
命令行 hello → SDK 组织 JSON → POST /chat/completions
             → 服务返回响应 → 读取 choices[0].message.content
             → 打印回答
```

（网络请求模型图）

我们调用 SDK 发出一条完整的 HTTP 请求-响应。`baseURL` 应填写服务商的 OpenAI 兼容根地址，SDK 会在后面拼接 `/chat/completions`；`API key` 是用于鉴权的凭据，会随请求发送。使用 HTTPS 可保护传输中的内容；`.env` 和 Git 忽略都不是加密。

<span id="failures"></span>

8、异常处理

在实际运行时，我们难免会因为各种问题导致运行失败，如网络波动、配置缺失或服务端错误等。用已有的安全入口先诊断配置和请求错误：

```bash
node --import tsx --env-file=.env src/hello-safe.ts hello
```

这条命令可能真实调用配置的端点并产生用量。若要检查缺少模型配置时的本地失败路径，可运行：

```bash
REIN_MODEL='' node --import tsx --env-file=.env src/hello-safe.ts hello
```

预期进程以退出码 1 结束，并报告配置错误。


## 提示词示例

```text
请在当前工程完成一次非流式模型调用，显式关闭自动重试。
先检查配置，区分配置错误、传输错误、服务拒绝和响应结构错误。
不要把示意回答或离线回放写成真实服务结果。
```

“非流式”限定本章接口；“区分错误”保留排查线索；“真实服务结果”限制验收结论。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。

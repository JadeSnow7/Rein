# 01 HelloWorld：从模型调用开始

**状态：文字修订稿，待教学验收｜永久免费 · Apache-2.0**

今天我们几乎每天都在使用聊天应用，但你有没有想过，在向 ChatGPT 发送一句简单的“你好”时，输入与回答之间经过了哪些步骤？平时，界面替我们处理了地址、凭据、网络等待和响应展示；自己编写调用程序时，我们需要把这些事情接起来。

本章，我们将在终端完成一次最简单的模型调用。接口选用 OpenAI 兼容的非流式 Chat Completions：发送一条用户消息，等待完整响应，再把回答打印出来。这里是用公开 API 学习请求与响应，不是在复现某个聊天产品的内部实现。

<span id="from-reading-0"></span>

主线使用 TypeScript；想对照另一种语言，可以阅读 [Rust 版本](./model-hello-rust.md)，不必同时完成两版。学完本章，你应能发出一句问候，解释回答从哪里取出，并在失败时找到下一步检查方向。

<span id="first-run"></span>
<span id="确认环境-先完成一次离线检查"></span>

## 1、准备环境

需要 Git 和 Node.js 22 或更高版本。尚未安装时，分别查看 [Git 安装页](https://git-scm.com/downloads)和 [Node.js 下载页](https://nodejs.org/en/download)。Node.js 通常会一起安装 npm，装好后重新打开终端。

```bash
git --version
node --version
npm --version
```

三条命令都应输出版本号，并确认 Node.js 的主版本至少为 22。后面的终端命令采用 macOS、Linux 或 WSL 的 shell 写法；Windows 用户可以使用 WSL 跟做。

## 2、准备代码与依赖

在准备存放项目的目录执行；已有工作副本时，直接进入其仓库根目录，不必重复克隆。

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

本章使用当前配套工作副本中的 `ts/src/hello.ts` 和 `ts/src/hello-safe.ts`。新版教材尚未冻结逐章快照，远程克隆与本地修订稿也可能不同；如果缺少这两个入口，先核对取得的配套版本，不要把任意旧版当作本章代码。旧 `ch01-helloworld` 标签用于历史复现，不是新版教材的起始快照。

读源码时沿着一条短路径走即可：先看 `ts/src/hello.ts` 的入口怎样取得命令行输入并调用 `loadConfig`，再看 `requestHello` 怎样接收配置并调用 SDK，最后看 `readHelloText` 怎样从响应中取出文本；需要分类错误时，再打开 `ts/src/hello-safe.ts`，沿着它调用的配置、请求和诊断函数回看。先找这几个入口和函数名，再阅读函数内部的防御性检查，可以避免一开始就在整个工程里来回跳转。

在仓库根目录安装锁定的依赖，再做本章的本地检查：

```bash
npm ci
npm run typecheck --workspace ts
npm test --workspace ts -- tests/hello.test.ts tests/config.test.ts
```

`npm ci` 安装依赖，包括 OpenAI 官方维护的 `openai` SDK。它可能需要网络；后两条命令检查 TypeScript 类型，以及 hello 和配置读取的受控测试，不会请求真实模型。测试通过后，我们知道本地样例能够按预期处理，账户和模型是否可用还要在后面实际调用。

这里的 `ts/` 是根工程中的一个 npm workspace，依赖统一在根目录安装。阅读 0 的[安装说明](../readings/00-ts.md#setup)可以补充终端和依赖知识，不是本章的必读前置。

<span id="live-call"></span>
<span id="配置-尽早指出哪里没填好"></span>

## 3、准备 API key

程序访问模型服务时，需要用 API key 表明请求来自哪个账户。网页聊天账号与 API 凭据通常需要分别准备，不要把网页登录凭据填入程序。

本章用 DeepSeek 的配置作示例。可以在 [DeepSeek 官方平台](https://platform.deepseek.com/)创建 key，并在[模型与价格页](https://api-docs.deepseek.com/quick_start/pricing/)核对可用模型和计费方式。已有其他兼容服务时，也可以使用自己的配置；先确认它支持本章使用的 Chat Completions 接口。

一次真实请求会产生相应 API 用量。没有可用 key 时，可以先完成本地检查和代码阅读，把真实调用结果留为未验证。

## 4、填写环境变量

除了 key，还需要根地址 `baseURL` 和模型名 `model`。它们分别决定请求发到哪里、使用哪个模型。三项要按同一家服务的文档填写，程序不会替你选择默认模型。

从仓库根目录进入 `ts/`。首次配置时复制模板；**已有 `.env` 时跳过复制，直接编辑原文件**。

```bash
cd ts
cp .env.example .env
```

用编辑器打开 `.env`，填写下面三项。2026-09-22 查阅的 [DeepSeek 首次调用说明](https://api-docs.deepseek.com/)列出了下方根地址和模型名；运行前仍应核对当前可用配置。

```dotenv
REIN_BASE_URL=https://api.deepseek.com
REIN_API_KEY=替换为自己在控制台创建的密钥
REIN_MODEL=deepseek-flash
```

SDK 会在根地址后请求 `/chat/completions`，因此不要把完整接口路径填进 `REIN_BASE_URL`。某些服务的根地址包含 `/v1`，应照其文档保留。

本章通过 Node.js 的 `--env-file=.env` 显式加载这个文件；`tsx` 不会自动加载它。终端中已经设置的同名环境变量优先于文件，修改 `.env` 后配置仍未变化时，可以先检查这一点。

`.env` 已被 Git 忽略，key 应留在本机，不放进源码、截图或运行记录。Git 忽略只降低误提交风险，不会加密文件。

<span id="request-response"></span>
<span id="调用-先写清楚一种协议"></span>

## 5、发出第一句问候

仍在 `ts/` 目录执行：

```bash
node --import tsx --env-file=.env src/hello.ts hello
```

`tsx` 让 Node.js 运行 TypeScript 文件，`--env-file=.env` 加载配置，末尾的 `hello` 是交给模型的话。不写最后一个参数时，程序也默认使用 `hello`。运行 TypeScript 和执行类型检查是两件事，所以前面仍保留了 `typecheck`。

成功时，终端可能出现下面这样的回答。这只是输出示意，模型不必逐字返回相同内容。

```text
Hello! How can I help you today?
```

紧接着在当前 shell 运行 `echo $?` 查看退出码：成功为 `0`，失败为 `1`。先确认返回了非空文本，再读一下它是否回应了问候；退出成功只能证明程序取到了文本，不能替你判断回答内容。

<span id="implementation"></span>
<span id="解析-http-成功之后还要检查什么"></span>

## 6、看懂 SDK 发出的请求

SDK（Software Development Kit，软件开发套件）提供已经封装好的调用方法。本章使用的 `openai` 包由 OpenAI 维护，仓库锁定为 `4.104.0`，安装时以锁文件为准；官方入口见 [SDK 说明](https://developers.openai.com/api/docs/libraries)。

打开 `ts/src/hello.ts`，找到 `requestHello`。下面摘出理解调用所需的配置；它是函数内部的解释用片段，完整程序已经在刚才运行的文件中。

```ts
const client = new OpenAI({
  apiKey: config.apiKey,
  baseURL: config.baseUrl,
  maxRetries: 0,
  timeout: 30_000,
})
```

`config` 来自配置读取。`new OpenAI` 创建客户端，`maxRetries: 0` 关闭 SDK 自动重试，`timeout` 的单位是毫秒。源文件还用 `AbortController` 为整次调用设置等待期限，并在请求结束后清理计时器。

请求的核心字段如下。实际源码在第二个参数传入取消信号；此处先看发送给服务的内容：

```ts
const response = await client.chat.completions.create({
  model: config.model,
  messages: [{ role: 'user', content: options.prompt ?? 'hello' }],
  stream: false,
})
```

`model` 选择模型，`messages` 是消息列表，`role: 'user'` 表示内容来自用户。入口将终端参数交给 `options.prompt`；`?? 'hello'` 在未提供参数时使用默认问候。`await` 等待请求返回，`stream: false` 表示一次取得完整响应，而不是逐段处理输出。

每次 CLI 运行只把本次命令得到的输入放进这一条 `user` 消息；程序没有把上一次输入写入文件、缓存或下一次请求，所以它不会自动记住上一次运行的内容。想延续上下文，必须在程序中明确保存并重新传入消息；本章的入口没有这层行为。

## 7、沿着 HTTP 响应找到回答

看起来像一次方法调用，背后仍然是网络请求：

```text
终端参数 → SDK 组织 JSON → POST /chat/completions
         → 服务返回响应 → 读取 choices[0].message.content
         → 检查非空文本 → 打印回答
```

API key 随请求用于鉴权，HTTPS 保护传输中的内容。配置加载、网络传输和响应解析各有自己的失败方式，不能只看“请求已经发出”就认为任务完成。

`choices` 是服务返回的候选列表，本章取第一个候选中 `message.content` 的文本。`readHelloText()` 依次检查响应是否为对象、候选是否存在、消息是否存在，以及文本是否为空。HTTP 成功并不保证这些条件都满足；即使取得了文本，也还需要读者检查它的意思。

可以把本章需要读取的响应层级压缩成下面这个示意。它只展示取值路径，其他响应字段省略：

```json
{
  "choices": [
    {
      "message": {
        "content": "模型返回的文本"
      }
    }
  ]
}
```

因此，程序读取的是 `response.choices[0].message.content`，不是响应顶层的 `content`。只要其中一层缺失、类型不对或文本为空，解析就应停下来并进入诊断路径。

<span id="failures"></span>

## 8、按发生位置排查失败

最简入口在失败时只提示检查调用。要得到分类诊断，仍在 `ts/` 运行安全入口：

```bash
node --import tsx --env-file=.env src/hello-safe.ts hello
```

这会重新发起一次请求，可能产生 API 用量。先根据错误判断是否需要重试，不要把诊断入口当成无成本的本地检查。

| 诊断 | 表示什么 | 下一步检查 |
| --- | --- | --- |
| `config` | 三项配置缺失、为空或根地址格式不符 | `.env`、终端同名变量与根地址 |
| `network`、`timeout` | 连接失败，或请求未在 30 秒内完成 | DNS、代理、TLS、地址及服务状态 |
| `unauthorized`、`not-found` | 请求被拒绝，或接口/模型不存在 | key 权限、根地址和模型名 |
| `quota`、`rate-limit`、`http` | 响应指出额度、频率或其他 HTTP 错误 | 服务商错误码与控制台；429 本身不足以区分原因 |
| `server`、`response`、`sdk` | 服务端失败、回答结构不符，或本地调用异常 | 服务状态、响应格式与本地依赖/参数 |

超时只说明本地没有及时得到结果，不能证明服务端没有处理过请求。两种入口都关闭了自动重试，是否再次调用需要根据现场信息判断。

## 9、完成四个小练习

<span id="exercises"></span>
<span id="exercise-01"></span>

**练习 1：改变输入。** 在 `ts/` 把问候换成一句自己的话，指出请求中的哪个字段随之变化。这是一次真实调用。

```bash
node --import tsx --env-file=.env src/hello-safe.ts '你好，请用一句话回应我'
```

<span id="exercise-02"></span>

**练习 2：制造配置错误。** 在其他配置已正确填写的基础上，给这次进程一个空模型名。它会覆盖 `.env` 中的同名值，程序应在请求之前失败。

```bash
REIN_MODEL='' node --import tsx --env-file=.env src/hello-safe.ts hello
echo $?
```

预期诊断为 `config`，退出码为 `1`。这条前缀赋值只影响本次命令，不会改写 `.env`。

<span id="exercise-03"></span>

**练习 3：解释失败用例。** 回到仓库根目录，重跑第 2 节的 hello/config 测试。打开 `ts/tests/hello.test.ts`，找到超时、429 和响应结构错误的用例，各写一句下一步该检查什么。解释为什么“HTTP 成功”和“有可用回答”不是同一个条件。

<span id="exercise-04"></span>

**练习 4：整理调用记录。** 记录一次实际调用的时间、请求的模型名、输入、回答和退出码；没有调用则注明未运行，不填示意结果。CLI 未打印服务端回报的模型标识，所以配置中的模型名只代表请求值。记录里不要包含 key。

下一章[任务与成功标准](./task-spec.md)会把问候换成具体的文档检查任务。我们仍然使用这个调用入口，重点转向怎样说明目标，以及如何判断回答是否有依据。

## 提示词示例

```text
我已准备 Rein 当前工作副本，正在学习 ts/src/hello.ts 和 hello-safe.ts。
请沿配置读取、请求发送、响应解析和错误分类解释一次非流式 hello 调用。
使用现有接口，不升级依赖，不添加工具或多轮对话。
先检查 hello/config 的本地测试，再协助我解释本章四个练习；
真实调用由我提供配置后执行，记录中不包含密钥。
请分别说明实际通过的检查和尚未运行的部分。
```

“使用现有接口”限定本章范围；“本地测试”让解释有代码依据；“分别说明”避免把受控响应当成真实服务结果。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。

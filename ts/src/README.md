# TypeScript source

主线实现的落点，随正文逐章演进。第 01 章先用官方 `openai` SDK 完成一次 `hello`，再保留手写 transport 供进阶阅读：

| 文件 | 职责 |
| --- | --- |
| `config.ts` | 从环境变量读 `REIN_BASE_URL` / `REIN_API_KEY` / `REIN_MODEL`，缺失即报错，不提供默认值 |
| `transport.ts` | 可替换的 transport 层：`createFetchTransport` 走网络，`createReplayTransport` 读录制样本 |
| `chat.ts` | 一次模型调用：构造请求、判定状态码、逐层落地响应形状、取出回答文本 |
| `errors.ts` | 五类失败：配置、网络、非 2xx、超时、响应形状 |
| `main.ts` | 最小可运行入口 |
| `hello.ts` | 官方 SDK 最小非流式 `hello` 入口，单次请求、30 秒 deadline、非空文本校验 |
| `hello-safe.ts` | 官方 SDK 安全入口，分类配置、网络、超时、HTTP 和响应错误 |

```bash
cd ts
npx tsx --env-file=.env src/hello.ts hello
npx tsx --env-file=.env src/hello-safe.ts hello
```

`main.ts` 和手写 transport 是同一章的进阶实现；它们用于观察请求构造、错误路径和本地回放，不替代两个 SDK 入口。

尚未引入 provider 适配器与统一消息协议——那是第 04 章从具体麻烦里推导的内容。

# TypeScript source

主线实现的落点，随正文逐章演进。第 01 章建立的部分：

| 文件 | 职责 |
| --- | --- |
| `config.ts` | 从环境变量读 `REIN_BASE_URL` / `REIN_API_KEY` / `REIN_MODEL`，缺失即报错，不提供默认值 |
| `transport.ts` | 可替换的 transport 层：`createFetchTransport` 走网络，`createReplayTransport` 读录制样本 |
| `chat.ts` | 一次模型调用：构造请求、判定状态码、逐层落地响应形状、取出回答文本 |
| `errors.ts` | 五类失败：配置、网络、非 2xx、超时、响应形状 |
| `main.ts` | 最小可运行入口 |

```bash
cd ts
npx tsx --env-file=.env src/main.ts "用一句话说明什么是 Agent Harness"
```

尚未引入 provider 适配器与统一消息协议——那是第 04 章从具体麻烦里推导的内容。

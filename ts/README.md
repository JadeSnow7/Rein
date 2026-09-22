# TypeScript track

TypeScript 是本书两种实现路线之一。`src/` 随正文逐章演进。第 01 章 SDK 教程使用 `ch01-helloworld` 快照；旧 `ch01` 保留手写 HTTP 版本。新版标签只在本地交付时，远程新克隆仓库尚不可获取，具体见[第一章](../docs/chapters/01-ts.md#first-run)。策略与理由见仓库根 [DECISIONS.md](../DECISIONS.md)。

- `src/`：主线实现，逐章累积
- `tests/`：与合同、fixture 对应的验收用例
- `examples/`：阅读 0 独立练习已落地；后续标注"可独立阅读"章节的自包含最小示例待建设

## 运行

本目录是根仓库的 npm workspace，在仓库根执行一次安装即可：

```bash
npm ci
npm run test       # vitest
npm run typecheck  # tsc --noEmit
```

模型密钥经 `.env` 提供，复制 `.env.example` 填写；`.env` 不入库。`tsx` 不会自动加载 `.env`，运行真实请求或录制命令时需显式传入 `--env-file=.env`。当前测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

## 当前状态

第 01 章包含两个官方 `openai` SDK 入口，`src/hello.ts` 是最小 `hello` 调用，`src/hello-safe.ts` 负责配置、网络、超时、HTTP 和响应错误分类。`src/transport.ts`、`src/chat.ts`、`src/errors.ts` 与 fixtures 保留为手写协议和离线回放进阶材料。阅读 0 使用自己的专项测试命令，见[材料正文](../docs/readings/00-ts.md)；根目录 `npm test` 仍检查主线测试。

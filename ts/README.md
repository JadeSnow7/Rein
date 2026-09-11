# TypeScript track

TypeScript 是本书的完整叙事主线。`src/` 随正文逐章演进，始终只保留一份最新代码；章节快照 tag 会在对应章节完成并核验后建立，当前尚未建立。策略与理由见仓库根 [DECISIONS.md](../DECISIONS.md)。

- `src/`：主线实现，逐章累积
- `tests/`：与合同、fixture 对应的验收用例
- `examples/`：计划提供给标注"可独立阅读"章节的自包含最小示例（当前仅有目录说明）

## 运行

本目录是根仓库的 npm workspace，在仓库根执行一次安装即可：

```bash
npm ci
npm run test       # vitest
npm run typecheck  # tsc --noEmit
```

模型密钥经 `.env` 提供，复制 `.env.example` 填写；`.env` 不入库。`tsx` 不会自动加载 `.env`，运行真实请求或录制命令时需显式传入 `--env-file=.env`。当前测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

## 当前状态

第 01 章运行时代码与测试已落地；后续章节能力仍在规划中。

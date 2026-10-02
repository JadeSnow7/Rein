# SC-EVOLUTION 离线连续维护工具

这是一个单文档、离线校准用的连续演化 runner。它把每轮任务写入该轮的 `task.txt`，只把显式参数数组传给普通子进程：`--input-file`、`--output-file`、`--task-file` 和 `--round-id`。适配器退出 0 只表示进程成功，独立 validator 和活动检查仍必须通过。

模块接口位于 `runner.mjs`：

```js
validatePlan(plan)             // 同步校验，非法计划在创建子进程前抛 EvolutionPlanError
await runEvolution({ plan })   // 返回逐轮 JSON 证据
```

计划必须包含序列身份 `sequenceId`、绝对路径 `seedFile`、全新的 `outputDir`、正整数 `timeoutMs`、`adapter`、`validator` 和非空 `rounds`。command 是单个可执行文件，args 是字符串数组；runner 不使用 shell。round 与 check id 只允许字母、数字、`.`、`_`、`-`。同一活动检查会自动带入后续轮次；旧检查只有在先前已活动、提供 `retirementReason` 的情况下才能退役，替代检查应使用新 id。

最小计划形状如下：

```json
{
  "sequenceId": "offline-sequence-01",
  "seedFile": "/absolute/seed.md",
  "outputDir": "/absolute/new-evidence",
  "timeoutMs": 1000,
  "adapter": { "command": "/usr/bin/node", "args": ["/absolute/adapter.cjs"] },
  "validator": { "command": "/usr/bin/node", "args": ["/absolute/validator.cjs"] },
  "rounds": [{ "id": "r01", "task": "Update the document", "checks": [] }]
}
```

CLI 接受一个 JSON 计划和一个全新的报告路径，不会覆盖已有报告：

```bash
node scripts/evolution/run.mjs --plan /absolute/plan.json --report /absolute/new-report.json
```

`inputRoot` 可选，用于把包含种子及其相关输入的目录显式声明给路径重叠检查；输出父级的符号链接会先规范化再检查。如果未声明，runner 仍拒绝报告/输出直接覆盖 `seedFile`。报告写入后包含 `sequenceId`、总状态和逐轮证据。demo 也支持 `--output /absolute/new-directory`，该目录必须事先不存在；不传时使用全新的临时目录。

每轮结果保留 adapter、validator 和 check 的原始 stdout/stderr、完整 argv、exitCode/signal/error、输入/输出 SHA-256、状态、从上一轮通过变为失败的回归列表、仍未修复的 `stillFailing` 列表，以及实际测得的 `elapsedMs` 和单文件 `modifiedFiles`。token、cost、人工时间没有可靠来源时为 `null`。runner 在每轮完成后以 `wx` 写入该轮 `result.json`，并在根目录保存冻结的 `plan.json` 与 `summary.json`。适配器错误、输入/任务被改写、输出缺失或输出被改写会中止轨迹并将剩余轮次标为 `notRun`；普通 validator/check 非零会保留证据并使当前轮失败，按当前实现不会单独因此中止后续轮次。普通子进程没有对抗式隔离，也不能隐藏未来任务，demo 只用于确定性校准。

先运行基准测试：

```bash
node --test scripts/evolution/runner.test.mjs
```

运行两组 12 轮离线校准，结果写入全新临时目录：

```bash
node scripts/evolution/demo.mjs
```

`adapter-good.cjs` 应 12 轮全部通过；`adapter-regressing.cjs` 在第 7 轮保留新要求通过，同时使先前通过的 `legacy-contract` 失败，从而产生历史回归。demo 只有在这两个条件都成立时退出码才为 0，并在报告写入 `calibrationPassed: true`。两组都是确定性 fixture，不是 AI 对照实验，也不能证明架构设计或长期维护能力提升。

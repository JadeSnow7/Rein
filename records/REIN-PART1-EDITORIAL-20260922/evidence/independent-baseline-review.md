# 第一部分独立初审与离线基线记录

记录时间：2026-09-22（Asia/Shanghai）。记录者：独立事实审查代理 `part1_fact_audit`。

## 适用边界

下文“初审”及其代码组装、测试输出发生在第一部分正文修订之前。它们保留当时观察到的事实、问题及运行结果，**不能作为修订后终稿的验证证据**。原始终端输出从同一次代理会话的工具结果转录；它们不是 Veriflow recorder 的绑定执行回执。初审时没有为全部 Markdown 另建不可变快照，因此不补造当时的 Markdown hash、revision 或输入稳定性证明。后面记录的临时文件 hash 是保存本记录时对仍保留的初审抽取物计算的值。

主线程另有 `baseline-book.json`，其执行与 revision 绑定保持原样；本记录不宣称上述独立命令属于该回执。测试、离线 mock、真实模型、章节快照与正式发布分别判断。

初审读取的范围：`book/chapters.json`、第一部分四章及 Rust 支线、阶段汇总 1、README、实施计划、R1a 任务摘要、旧教学 hello/config 源码和相关测试。没有修改源文件，没有读取模型密钥，没有请求真实模型。除本记录外，本代理只曾在系统临时目录创建抽取与验证文件。

## 初审发现（原位置，修订后行号可能变化）

| 优先级 | 初审位置 | 当时问题 | 建议 |
| --- | --- | --- | --- |
| P1 | `model-hello-rust.md:42–48` | 主流程要求切换仅在本地交付的旧 `ch01-helloworld`；与 TS 的当前工作副本起点不一致，且不是 v2 快照。 | 两种入口统一当前工作副本；旧标签仅保留历史复现语义。 |
| P1 | `tool-roundtrip.md:264,330`；`model-hello-rust.md:18,303` | 新版正文跳向旧数字稿；旧 05 循环章与新版 05 Rust 迁移、06 循环章错位，旧 02 仍标待撰写。 | 改为对应稳定 slug，历史进阶阅读显式注明。 |
| P1 | `evidence-qa.md:5` | 要求使用该章结束快照，但正文未声明且索引快照为空，读者无法确定正式复查起点。 | 当前工作副本练习与待冻结快照验收分开。 |
| P2 | `model-hello.md:158` | 存在“（网络请求模型图）”占位文字。 | 删除占位或交付成品。 |
| P2 | Rust 支线及 `:62` | 缺少新版状态说明，仍带“成本较低”服务商评价。 | 统一状态，并让读者核对当前兼容端点与计费方案。 |
| P2 | `tool-roundtrip.md:275` | 仅检查 truthiness，没有复用第一章的 trim、URL 和嵌入凭据校验。 | 复用配置读取，或明确简化边界。 |

初审同时确认：R1a 新增 `core/`、`runtime/`，旧教学 `rust/` 保持独立；R1a 记录没有将章节快照或验收升级。Rust 锁定 `async-openai 0.29.6`，零重试预算、30 秒外层 timeout、非空文本检查与源码一致；03 原文已说明 lstat/readFile 的文件替换窗口。

## 初审代码组装检查

cwd 为 `/Users/huaodong/workspace/Agent-Learning`。使用 Node 读取当时的 `docs/chapters/tool-roundtrip.md`，按 `/^```ts\n([\s\S]*?)^```/gm` 顺序抽取 7 个 TypeScript 代码块，以换行拼接为临时 `ch03.mts`；临时目录的 `node_modules` 链接至本仓库安装依赖。未修改正文或仓库源码。

严格类型检查以 Node 子进程执行以下 argv：

```text
node <repo>/node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target ES2022 --module NodeNext --moduleResolution NodeNext /var/folders/87/gyhx13hs45351j7vwkdrr4sr0000gn/T/rein-part1-audit-O5pawg/ch03.mts
```

原始结构化输出：

```json
{"dir":"/var/folders/87/gyhx13hs45351j7vwkdrr4sr0000gn/T/rein-part1-audit-O5pawg","blocks":7,"typecheckExit":0,"stdout":"","stderr":""}
```

随后在该临时目录执行 `node --import tsx check.mts`。验证实际写入的临时 `hello.md` 可读取，三种负例分类准确，并用受控 client 替身确认工具调用编号和真实读取内容进入第二次请求。这是本地 mock，不是服务商验收。

原始结构化输出：

```json
{"checkExit":0,"stdout":"{\"ok\":true,\"path\":\"hello.md\",\"content\":\"task: Hello audit!\"}\n{\"executorCodes\":[\"invalid_arguments\",\"invalid_path\",\"not_found\"],\"requests\":2,\"feedbackMatched\":true,\"output\":\"Hello audit!\"}\n","stderr":""}
```

临时测试脚本原文：

```ts
import { executeTool, parseToolArguments, run } from './ch03.mts';
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
const dir=process.cwd();
await fs.writeFile('hello.md','task: Hello audit!');
const bad=['{"path":"hello.md"','{"path":"../.env"}','{"path":"missing.txt"}'];
const codes=[]; for(const raw of bad){const r=await executeTool(parseToolArguments(raw));codes.push(r.ok?'unexpected-ok':r.error.code)}
assert.deepEqual(codes,['invalid_arguments','invalid_path','not_found']);
const good=await executeTool({path:'hello.md'}); assert(good.ok && good.content==='task: Hello audit!');
let calls=[];
const client={chat:{completions:{create:async(input)=>{calls.push(input);return calls.length===1?{choices:[{finish_reason:'tool_calls',message:{content:null,tool_calls:[{id:'audit-call',type:'function',function:{name:'read_file',arguments:'{"path":"hello.md"}'}}]}}]}:{choices:[{finish_reason:'stop',message:{content:'Hello audit!'}}]}}}}};
const output=await run(client as any,'read hello.md'); assert.equal(output,'Hello audit!'); assert.equal(calls.length,2);
assert.equal(calls[1].messages.at(-1).tool_call_id,'audit-call'); assert.equal(JSON.parse(calls[1].messages.at(-1).content).content,'task: Hello audit!');
console.log(JSON.stringify({executorCodes:codes,requests:calls.length,feedbackMatched:true,output}));
```

保存本记录时计算的抽取物 SHA-256：

- `/var/folders/87/gyhx13hs45351j7vwkdrr4sr0000gn/T/rein-part1-audit-O5pawg/ch03.mts`：`ecddf89859e6623d9842b8e6e5a36cd8b908f582e73c732bb069d6e6269ca431`
- `/var/folders/87/gyhx13hs45351j7vwkdrr4sr0000gn/T/rein-part1-audit-O5pawg/check.mts`：`fba7cc0d9bbc1ad5dfa400048f49571cce568b848e0f5e09046f6696e1a5c681`

## 初审 hello/config 定向测试

原始命令（仓库根目录）：

```bash
npm test --workspace ts -- tests/hello.test.ts tests/config.test.ts
```

原始终端合并输出（执行退出码 0）：

```text
> @rein/ts@0.0.0 test
> vitest run --passWithNoTests tests/hello.test.ts tests/config.test.ts

 RUN  v2.1.9 /Users/huaodong/workspace/Agent-Learning/ts

 ✓ tests/config.test.ts (13 tests) 8ms
stderr | tests/hello.test.ts > hello SDK entry points > 安全 CLI 缺配置时返回退出码 1，且不打印密钥
失败（config）：REIN_BASE_URL 未设置。参考 ts/.env.example，复制为 ts/.env 后填写。

 ✓ tests/hello.test.ts (18 tests) 35ms

 Test Files  2 passed (2)
      Tests  31 passed (31)
   Start at  14:29:26
   Duration  673ms (transform 130ms, setup 0ms, collect 233ms, tests 43ms, environment 1ms, prepare 207ms)
```

以上 31 项只覆盖被选择的 hello/config 测试，并不是全库或章节验收。

## 初审版本事实

以下命令分别执行；在组合读取中，最后一个 `git show` 预期失败，不能把该组合调用说成全部成功。

```text
node --version
rustc --version
git rev-parse ch01-helloworld
git show ch01-helloworld:book/chapters.json
```

对应原始输出摘录：

```text
v26.5.0
rustc 1.98.0 (88d9e12ae 2026-08-18)
630d4ef4dd21d453152f01a3a24f7a0a01e14d52
fatal: path 'book/chapters.json' exists on disk, but not in 'ch01-helloworld'
```

这证明本地旧标签不含新版索引；没有据此推断远程当前标签状态。

## 补充：DeepSeek 请求体方案核对

这是针对拟修正文案的独立兼容性调查，同样不能代替最终章稿组装验证或真实请求。2026-09-22 通过官方文档搜索结果核对：

- [Chat Completions API](https://api-docs.deepseek.com/api/create-chat-completion/)：thinking 默认开启；thinking 下 required/指定具体工具会 400，应先关闭 thinking。
- [Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode/)：带 tools 的后续请求需要保留 reasoning_content。
- [Responses 兼容说明](https://api-docs.deepseek.com/guides/responses_api/) 的 parallel_tool_calls ignored 结论属于 Responses，不能移用于本章 Chat Completions。未取得 Chat Completions 对关闭并行的明确保证。

建议两轮对官方 api.deepseek.com 显式发送 thinking disabled，其他端点不发送该扩展字段；仍由本地检查限制恰好一次工具请求。锁定 SDK 4.104.0 可从 create 第一参数对象展开扩展字段。不能只在第二参数传 `{ body: { thinking: ... } }`，因为本地 `completions.mjs` 的 `{ body, ...options }` 会覆盖整个请求体。

用真实 SDK + fake fetch 验证后，四种端点均保留 model/messages；两个 DeepSeek 根路径发送 thinking disabled，OpenAI 与相似域名不发送。首次检查脚本因为 DOM Response 与 node-fetch Response 类型不符，类型检查退出 2；这是临时测试脚本问题。改为导入 node-fetch Response 后，严格类型检查与运行均退出 0。没有将首次失败改写成通过。

最终运行输出：

```text
{"baseURL":"https://api.deepseek.com","thinkingSent":true,"requests":2,"modelAndMessagesPreserved":true}
{"baseURL":"https://api.deepseek.com/v1","thinkingSent":true,"requests":2,"modelAndMessagesPreserved":true}
{"baseURL":"https://api.openai.com/v1","thinkingSent":false,"requests":2,"modelAndMessagesPreserved":true}
{"baseURL":"https://api.deepseek.com.example/v1","thinkingSent":false,"requests":2,"modelAndMessagesPreserved":true}
```

测试脚本 SHA-256：`43f6e4faeb745afaa2aa1b2d03ac6d19d2e867a3e3818c5bb741505649889f2e`。路径为 `/var/folders/87/gyhx13hs45351j7vwkdrr4sr0000gn/T/rein-deepseek-options-LZhjL5/check.mts`。

## 三篇文字修订稿独立复审

本节与前面的初审执行分开。2026-09-22 14:35（Asia/Shanghai）只读复审了主线程已修订的 00、02、阶段汇总 1；没有为这些修订运行真实模型。此次读取文件的 SHA-256：

- `docs/chapters/task-map.md`：`04b3c83cc241a8e5e87be034066445e26dd9955aa1b3a6b63838b106ce90f1d8`
- `docs/chapters/task-spec.md`：`1f7fe66bb54f92fd97e51bca7f4e661cec82fd4b411a55c36f1e79df8aa2346d`
- `docs/milestones/evidence-qa.md`：`06ef0d516ce9a516289795a29afb5ad643664bb04ee661e73c600804a7585f64`

复审结论：三篇没有发现新的发布阻断。02 已明确同版本、完整 scripts 清单、没有其他启动入口；相对链接案例以完整目录清单为依据，并要求清单不完整时保持依据不足；提示词要求与 hello-safe.ts 没有开放读写工具的实际能力边界已分清。阶段汇总明确当前工作副本练习不是统一章节快照验收，保留代码、输入、修改和原始输出要求合理。

一项非阻断措辞建议：当时 `task-map.md:57` 的“第一部分不执行文档里的命令，也不修改项目文件”可收窄为“本部分的助手不执行待分析资料中的命令，也不修改待维护的项目文件”，避免与读者手动安装、运行、创建练习代码和文件混淆。该建议不改变能力或验收结论。

最终 01、Rust 支线、03 尚待 coder 完成后另行复审。本文件不将三篇文字复审、初审代码检查或请求体方案检查合并成第一部分正式验收。

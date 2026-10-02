# 第一部分终稿独立文字复审

记录时间：2026-09-22T14:45:39+08:00。审查者：`part1_fact_audit`。

## 范围与证据性质

本次只读复审 01 TypeScript 正文和 Rust 对照，核对终端路径、选定检查命令、现有源码语义、解释片段、四项练习和历史快照边界。未修改这两篇或其源码，未访问密钥，未请求真实模型，也没有将初审运行结果改写成终稿证据。本记录是静态文字与源码一致性审查，不是本轮执行回执、章节跟做验收或发布回执。

审查输入 SHA-256：

- `docs/chapters/model-hello.md`：`69458f16d034c8b0986c1781be06c92065608c05bc985c14b7ff6f737a4dcb56`
- `docs/chapters/model-hello-rust.md`：`d1f7adac20e4fa3e06db28b173a9e72d9463c9b29a9a0b5ad456e0ebaec7f02d`

## 结论

**两篇未发现发布阻断。** 初审所列必需切换旧标签、错误数字链接、占位图、旧状态口径等问题已在此次读取的正文中消除。两篇继续标注文字修订稿、待教学验收，符合现有证据边界。

| 复审项 | 核对结果与依据 |
| --- | --- |
| TS 命令路径 | 安装及 typecheck/选定测试从仓库根目录执行；随后明确 `cd ts`，配置和两个运行入口均使用该 cwd。练习 3 明确返回根目录。根 npm workspace 和 ts/package.json 支持所列脚本与测试参数。 |
| TS 检查范围 | `typecheck` 覆盖 tsconfig 指定的教学工程；hello/config 测试文件实际包含请求、配置、429、超时和响应结构检查。正文没有把这些本地检查写成账户或服务可用证明。 |
| TS 最简/安全入口 | hello.ts 固定默认问候、非空响应检查及失败退出 1，与文字一致；hello-safe.ts 增加分类诊断，确实可能重新请求服务。maxRetries 为 0，外层 AbortController 与计时器清理存在。 |
| TS 解释片段 | 配置和请求片段明确是函数内部的解释用片段；正文说明实际源码另传取消信号，未把省略后的片段伪装成完整程序。 |
| Rust 命令路径 | 克隆后明确进入 rust/，Cargo build/run/check/test 均面向独立教学工程；根 Cargo workspace 的 exclude 包含 rust，不需要先构建 core/runtime。 |
| Rust 选定测试 | cargo test --locked --lib 选择教学库测试，避开需要其他章节宿主环境的 integration tests；库内 hello 测试使用回环 HTTP，并有本地合同测试，未读取 dotenv。正文已说明回环监听条件与依赖下载网络。 |
| Rust 最简/完整入口 | src/bin/hello.rs 固定发送 hello，只打印通用失败提示；src/main.rs 接收一个用户参数并调用分类处理。正文将两者分开讲，没有把最简程序内部所有 SDK 失败映射成 Network 的细节描述为完整分类。 |
| Rust 类型与代码 | build_client 的 backoff、chat_with_timeout 的两层错误处理、Option 非空检查均对应现有 lib.rs；Cargo.lock 确认 async-openai 0.29.6。引用 hello.rs 为已有完整入口。 |
| 四项练习 | 两篇均包含改变输入、单次进程空模型配置、解释受控失败用例、保存实际调用记录；引号确保多字提示作为一个参数，空 REIN_MODEL 在配置层被拒绝，且前缀赋值不修改 .env。 |
| 旧快照与进度 | 旧 ch01-helloworld 明确只保留历史复现语义；当前工作副本可能不同于远程，缺少入口需核对配套版本；没有声称新版快照已冻结或真实模型已验证。 |
| 导航与补充阅读 | 当前主线链接使用 model-hello、task-spec、rust-migration；01-ts-advanced 明确称为历史补充，未将其录制命令套用于 Rust。 |

## 限制与后续

没有要求这两篇顺带完成正式运行验收；文本交付与运行验收仍分别记录。01 的请求模型名、账户配置和回答需由实际调用确认，命令可读不等于真实服务已通过。最终 03 仍由 coder 修订，未纳入本记录本次结论。00、02、阶段汇总 1 的独立复审见 independent-baseline-review.md 末节，其输入 hash 与本节两篇分开保存。

## Rust 配置错误原命令的隔离本地负例

从此次最终 Rust 正文中核对并提取命令，在新建的临时 rust/ 副本运行；只复制 Cargo.toml、Cargo.lock、src/，没有复制真实 .env。临时 .env 使用 loopback 端点、占位 key 和占位模型；同名进程端点与 key 也限定为这些占位值。正文的 REIN_MODEL 空值赋值保留原样。设置 CARGO_NET_OFFLINE=true 禁止 Cargo 下载，并按主线程指定复用 /private/tmp/rein-r0-target-20260921。

实际执行为 `/bin/zsh -c` 加下列正文原命令，未执行正文后续的 echo，因此记录的是 Cargo 命令自身退出码：

```bash
REIN_MODEL='' cargo run --locked --bin rein-ch01-helloworld -- hello
```

原始执行结果：

```json
{
  "cwd": "/private/tmp/rein-part1-rust-cli-uagnf6fs/rust",
  "command": "REIN_MODEL='' cargo run --locked --bin rein-ch01-helloworld -- hello",
  "page_sha256": "d1f7adac20e4fa3e06db28b173a9e72d9463c9b29a9a0b5ad456e0ebaec7f02d",
  "source_manifest_sha256": "5c4a1fff2d7f3216011bdb6fcc522801f998848f7d865f2d2c2b2c43f8b56355",
  "source_files": 14,
  "exit_code": 101,
  "stdout": "",
  "stderr": "error: Cargo.toml: can't find `ch05_loop` example at `examples/ch05_loop.rs` or `examples/ch05_loop/main.rs`. Please specify example.path if you want to use a non-default path.\nerror: could not parse `rein-ch01-helloworld` (manifest) due to 1 previous error\n",
  "timed_out": false,
  "duration_seconds": 0.107
}
```

复制源文件的清单 hash 由排序后的相对路径到 SHA-256 映射以 JSON（sort_keys=True, separators=(',', ':')）序列化后计算，排除临时 .env。源码入口 hash：

- `Cargo.toml`：`7def46a969df2a8998fafc103b651bb1380785baa22dceae4d44e1ef0de55c99`
- `Cargo.lock`：`145141f65f3b80ce39ae254befe756a59f3bc4a516ada0758097495dfe8c0aa5`
- `src/main.rs`：`0299ccf72449f18087ecae8d39f002e1e3aba1b6247d1198818ec2d9147066b7`
- `src/lib.rs`：`5d24a462295c6676ab83ba0b29dd63d3b462ba319226881484b90c25cf52ef38`

结果：未取得符合全部预期的负例结果；以上保留实际输出，不将其改记为通过。

### 补齐 manifest 声明的原始 example 后复跑

第一次退出 101 发生在 Cargo manifest 解析阶段：隔离副本未包含 Cargo.toml 明确声明的 examples/ch05_loop.rs，未进入 CLI。主线程明确授权后，仅额外原样复制此文件，未改 manifest；继续保持离线 Cargo、临时占位 .env 和空 REIN_MODEL。此次仍执行相同正文 Cargo 命令。

原始结果：

```json
{
  "cwd": "/private/tmp/rein-part1-rust-cli-uagnf6fs/rust",
  "command": "REIN_MODEL='' cargo run --locked --bin rein-ch01-helloworld -- hello",
  "source_manifest_sha256": "27879a3875566beab74486620cd1c258b1b79582ec41ae0d770ed85e1b38ab75",
  "source_files": 15,
  "extra_example_sha256": "a475ec7f396b491608c5b4380e175bcb7089c18da9a6566916be49d5908f4f33",
  "exit_code": 1,
  "stdout": "",
  "stderr": "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.32s\n     Running `/private/tmp/rein-r0-target-20260921/debug/rein-ch01-helloworld hello`\n失败（config）：缺少 REIN_MODEL\n",
  "timed_out": false,
  "duration_seconds": 2.126
}
```

本次本地负例符合预期：真实 Cargo 命令退出 1，诊断为 config、缺少 REIN_MODEL。load_config 在建立请求前返回错误，未发送模型请求。此前退出 101 保留为隔离夹具准备失败，不代表正文在完整工作副本中的命令失败。本项仍不证明正常调用或真实服务可用。

## 03 最终文字与实现一致性复审

复审时间：2026-09-22T14:53:03+08:00。本节接续前面 03 尚在修订的历史记录，现已读取最终交付稿；未修改正文或源代码。

输入 `docs/chapters/tool-roundtrip.md` 的 SHA-256：`9c7ff03895b5cac77337219aa77b6156dfb633005bd18aa25f98596b6ff21534`，与主线程指定值和 `final-tool.json` 内记录的 `markdown_sha256` 一致。

结论：**本次复审未发现残留发布阻断。**

- 第 262–268 行已允许第二轮 `tool_calls` 省略、null 或空数组；仅在该字段非空值时要求数组且长度为 0，继续要求 finish_reason 为 stop、回答为非空文本。此前会错误拒绝正常省略字段的阻断已修正。
- 第 276 行已将 30 秒收窄为 SDK 的 timeout 配置，明确不等于整个任务完成期限、也不保证覆盖所有响应体读取阶段，与锁定 SDK 的行为边界相符。
- 两轮都显式接收入口传入的 model，并调用 modelOptions；该函数只按官方 api.deepseek.com 主机名发送 thinking disabled，其他端点不加此字段。文本没有把 OpenAI 兼容格式当作所有参数支持的保证。
- 请求、参数检查、真实读取、工具结果回传的先后顺序一致；两次请求、第一轮恰好一次工具调用、本地路径白名单和大小限制、失败结果可回传且 CLI 可能退出 0，均与目前代码解释一致。
- 章节链接分别指向 agent-loop、阶段汇总 1、provider-adapter 与 rust-migration，未重新引入旧数字稿错位；固定 ts/ 命令起点及逐段组装说明可跟随。

另已只读检查主线程 `final-tool.json`：其记录为退出 0、result passed，源码 hash 与上面一致；输出包含 7 块正文抽取、typecheck_exit=0、三个执行器错误码、alpha/beta 往返、DeepSeek thinking disabled 和坏响应拒绝等。该执行属于主线程 recorder，本代理没有重复冒领为独立执行；本节的新增工作为终稿静态独立复审。真实服务调用、教学快照验收及正式发布仍未由这些检查证明。

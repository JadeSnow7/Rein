# 六部分框架第一部分重写

版本 PYTHON-PART1-1。用户要求按最新六部分框架重写，并确认范围为 00–03 与必要入口。权威大纲为 `book/framework-six-parts/part-01.md`；此轮开始时的大纲属于规划，不作为执行证据。

## 范围与设计

新正文使用独立 slug：`minimal-agent`、`python-model-call`、`python-file-read`、`python-suggestions`，避免把旧 TS/Rust 页面变成不同主题。首页、阅读指南、目录、历史入口和侧栏将新四章作为当前第一部分；保留五篇索引与历史正文。六部分总览明确后五部分未随本轮重写，不创建 04–29 的伪正文。侧栏和章末导航不把新 03 自动接到旧 04。

配套代码只在 `python/part1/` 新建教学实现与测试。标准库支持离线流程；真实服务使用固定 `openai==2.26.0`，安装到独立虚拟环境。Python 基线 3.14.6，最低语法要求 3.11。正文解释最短 SDK 调用和逐步增量，不要求先学 Rust。

CLI 合同：从仓库根执行 `python3 python/part1/rein.py hello|read|suggest`，默认离线。参数 `--workspace` 默认该目录中的 `fixtures/outdated`，`--path README.md`，`--read-mode direct|tool` 默认 tool，`--mode offline|live` 默认 offline，`--timeout 30`，`--response <JSON回放文件>` 为可选故障/hello 回放。配置读取 REIN_BASE_URL/REIN_API_KEY/REIN_MODEL；真实调用只在用户主动选 live 时发生。退出 0 为 opinion/read/suggestion/no_change，1 为结构化失败。输出 JSON 含脱敏 record 和结果，suggest 同时提供可审查字段（可选 human 渲染而不污染 JSON）。无输出文件写入。

模块接口保持简单：Task(goal,workspace,allowed_paths,acceptance)，safe_read，ModelAdapter.request(task,messages,deadline)，ModelResponse，RunRecord。hello 一次请求不读目标；read direct 先程序读取后一次模型调用；read tool/suggest 最多两次请求与一次 read_file，保留工具调用 ID 和实际消息。响应解析先传输、再结构、再内容。预算覆盖整个运行，迟到结果不采纳，SDK 重试关闭。原始请求/响应可供测试检查但默认记录不能含密钥。

safe_read 拒绝绝对路径、..、非授权路径、逐组件符号链接、目录、无效 UTF-8 和大于 4096 字节文件；有界读取 4097 字节后复查大小。适用于自控目录，说明检查/打开间竞争边界。直接读取和工具调用复用同函数。

模型只返回 path/original/suggested/reason（以及 status），程序从完整行片段唯一匹配生成 start_line/end_line/source_digest。不得相信模型行号；带模型行号的字段视为 response_invalid。重复匹配 ambiguous_match 并给候选行；找不到 original_missing；空文件不造建议；no_change 必须有可定位原文且 suggested 与 original 相同。CRLF 仅在匹配/展示中规范化，摘要始终来自原字节。展示前重读并比摘要，变化 source_changed。目标文件从不写入。

离线适配器是显式标注的确定性教学替身：hello 回放固定响应；工具首轮返回调用；末轮消费当次工具内容后按 start/dev 规则产生候选，不能读取 expected 文件或用固定终句冒充数据流。固定 outdated 第 8 行、duplicate 第 3/9 行、correct 与故障响应，形成 S1 交接 manifest；没有真实服务/新手试读验收声明。

## 先行基准与验收

- MET-001 正文：四章依次完成任务卡、最短调用、direct→tool、定位建议；每个主要动作给操作目录、命令、输出、失败与练习；必需知识在正文解释。主线程逐章审阅。
- MET-002 Python：先编写测试并保存未实现结果，主线程确认后实现。覆盖 C01–C04，config/非 JSON/空 choices/空文本/超时/过期、真实工具消息配对、文件变化影响末轮、读边界、LF/CRLF/多行/空/重复/跨文件/模型行号、只读哈希和 SDK 本地 mock。预算 30 秒默认，用短预算慢响应测试校准，4 KiB 精确边界测试；不宣称对任意真实服务最佳。
- MET-003 入口：首页/指南/目录/侧栏/前后章一致；旧页面与索引原文保留；book/check/build/links、必要 UI 检查。
- MET-004 教程：最终 Markdown 提取关键离线命令，从所声明初始目录执行，检查结果/退出码与练习。S1 清单列输入、响应、代码摘要和未验证范围。

## 分工及交付

主线程负责 Spec、四篇正文、入口 Markdown、自审与证据。coder 负责 python/part1 与必要 VitePress 配置源改动；先提交测试基准供审阅，再实现。保留既有脏工作。本轮仅本地重写和核验；上一轮提交发布已完成，不能用它声称新稿已发布。

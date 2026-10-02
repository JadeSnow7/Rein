# 全书目录

本书现行结构为**六部分、00–29 共 30 章**。00–04 已有正文与配套程序：00–03 是第一部分的 Python 终端助手，04 分析 Agent 的核心能力，并把不依赖模型协议的行为迁移到 Rust；05–29 先提供逐部分规划入口。目录中的“规划中”不是实现或教学验收通过。

第一部分围绕 C++ Hello World 建立终端代码修改助手。第二部分先分析 Agent 的核心能力并迁移既有行为，再增加 Rust 的统一消息、循环、停止和取消。第三部分建立可信修改；第四部分按需接入扩展；第五部分把 Harness 用在轻量 IDE；第六部分研究架构与持续演进。第一至五部分由人确定设计和验收条件、AI 编程助手编写代码，第六部分研究完全 vibe coding 的前提（见[阅读方式](./chapters/minimal-agent.md#division)）。从第 04 章起，正式正文末尾附交给代码助手的提示词，并标明来源。

<!-- book:toc:start -->

## 极简 Harness

- 00 [我们要实现一个怎样的 Agent](/chapters/minimal-agent.md) · 初稿 · 永久免费 · Apache-2.0
- 01 [用 Python 完成第一次模型调用](/chapters/python-model-call.md) · 初稿 · 永久免费 · Apache-2.0
- 02 [让 AI 读取代码与日志](/chapters/python-file-read.md) · 初稿 · 永久免费 · Apache-2.0
- 03 [做一个终端代码修改助手](/chapters/python-suggestions.md) · 初稿 · 永久免费 · Apache-2.0
- [阶段汇总：终端代码修改助手](/milestones/part-01.md)

## 核心循环

- 04 [Agent 的核心能力](/chapters/agent-capabilities.md) · 初稿 · 永久免费 · Apache-2.0
- 05 [统一模型消息与工具协议](/roadmap/part-02.md#ch05) · 规划中 · 永久免费 · Apache-2.0
- 06 [写出核心 Agent Loop](/roadmap/part-02.md#ch06) · 规划中 · 永久免费 · Apache-2.0
- 07 [让循环适时停止](/roadmap/part-02.md#ch07) · 规划中 · 永久免费 · Apache-2.0
- 08 [取消任务与观察运行过程](/roadmap/part-02.md#ch08) · 规划中 · 永久免费 · Apache-2.0
- [阶段汇总：可控 Rust Agent Loop](/milestones/part-02.md)

## 可信 Harness

- 09 [管理上下文与运行状态](/roadmap/part-03.md#ch09) · 规划中 · 永久免费 · Apache-2.0
- 10 [按需获取与压缩资料](/roadmap/part-03.md#ch10) · 规划中 · 永久免费 · Apache-2.0
- 11 [区分指令、资料与不可信输入](/roadmap/part-03.md#ch11) · 规划中 · 永久免费 · Apache-2.0
- 12 [控制工具权限与执行范围](/roadmap/part-03.md#ch12) · 规划中 · 永久免费 · Apache-2.0
- 13 [把修改建议变成候选补丁](/roadmap/part-03.md#ch13) · 规划中 · 永久免费 · Apache-2.0
- 14 [审查、批准与应用修改](/roadmap/part-03.md#ch14) · 规划中 · 永久免费 · Apache-2.0
- 15 [验证结果与有限修复](/roadmap/part-03.md#ch15) · 规划中 · 永久免费 · Apache-2.0
- [阶段汇总：可审查的单 Agent 修改助手](/milestones/part-03.md)

## Agent 扩展

- 16 [计划模式：先组织任务，再执行](/roadmap/part-04.md#ch16) · 规划中 · 永久免费 · Apache-2.0
- 17 [Skill：加载可复用的工作方法](/roadmap/part-04.md#ch17) · 规划中 · 永久免费 · Apache-2.0
- 18 [扩展接口与执行 Hooks](/roadmap/part-04.md#ch18) · 规划中 · 永久免费 · Apache-2.0
- 19 [接入 MCP 工具](/roadmap/part-04.md#ch19) · 规划中 · 永久免费 · Apache-2.0
- 20 [多智能体与受限委派](/roadmap/part-04.md#ch20) · 规划中 · 永久免费 · Apache-2.0
- [阶段汇总：可选扩展能力](/milestones/part-04.md)

## 实际应用

- 21 [设计 IDE 与 Harness 的连接方式](/roadmap/part-05.md#ch21) · 规划中 · 永久免费 · Apache-2.0
- 22 [把编辑器上下文交给 Agent](/roadmap/part-05.md#ch22) · 规划中 · 永久免费 · Apache-2.0
- 23 [完成一次真实开发任务](/roadmap/part-05.md#ch23) · 规划中 · 永久免费 · Apache-2.0
- 24 [评测、打包与项目复盘](/roadmap/part-05.md#ch24) · 规划中 · 永久免费 · Apache-2.0
- [阶段汇总：集成 Harness 的轻量 IDE](/milestones/part-05.md)

## 架构设计与持续演进

- 25 [让 Agent 理解项目的架构](/roadmap/part-06.md#ch25) · 规划中 · 永久免费 · Apache-2.0
- 26 [让 Agent 做出有依据的设计](/roadmap/part-06.md#ch26) · 规划中 · 永久免费 · Apache-2.0
- 27 [让 Agent 记住设计理由](/roadmap/part-06.md#ch27) · 规划中 · 永久免费 · Apache-2.0
- 28 [让 Agent 持续修改而不破坏系统](/roadmap/part-06.md#ch28) · 规划中 · 永久免费 · Apache-2.0
- 29 [如何判断 Agent 真的进步了](/roadmap/part-06.md#ch29) · 规划中 · 永久免费 · Apache-2.0
- [阶段汇总：可复审的持续演进实验](/milestones/part-06.md)

<!-- book:toc:end -->

## 阅读状态与历史资料

章节状态、开放属性和导航由仓库中的 `book/chapters.json` 统一维护。第一部分正文、程序检查、真实模型表现和新手独立跟做分别判断；规划页不冒充已完成章节。六部分的详细写作边界见 `book/framework-six-parts/README.md`。

旧五篇 25 章清单、数字 URL、TypeScript/Rust 对照与旧阶段汇总从[历史与补充入口](./history.md)访问。旧页面的编号仍代表原主题，不在这里重复生成第二套现行目录。

[阅读材料](./readings/00.md)、[深入讨论附录](./appendices/a1.md)、[开放说明](./access.md)与[提示词示例说明](./prompt-examples.md)按需阅读。

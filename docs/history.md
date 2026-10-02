# 历史与补充入口

当前阅读从六部分框架的 [00 极简 Harness](./chapters/minimal-agent.md)进入。六部分中的 01 使用 Python，[04 Agent 的核心能力](./chapters/agent-capabilities.md)先分析产品机制，再规划 Rust 迁移；下列旧页的数字仍按各自旧版解释。

## 五篇 25 章版本

以下按旧五篇清单 `book/history/chapters-v2.json` 列出原主题。链接仍指向原内容；旧版状态与新六部分章节状态分开。

### 让模型完成一个小任务

- 旧 00 [从一次文档维护任务认识 Rein](./chapters/task-map.md)
- 旧 01 [HelloWorld：从模型调用开始](./chapters/model-hello.md)
- 旧 02 [把任务说清楚：提示词与成功标准](./chapters/task-spec.md)
- 旧 03 [工具调用：让模型读取真实文件](./chapters/tool-roundtrip.md)

### 建立可控的混合运行时

- 旧 04 [模型接口：隔离服务商差异](./chapters/provider-adapter.md)
- 旧 05 [从 TypeScript 原型走向 Rust core](./chapters/rust-migration.md)
- 旧 06 [核心 Agent Loop：让工具结果推动下一轮](./chapters/agent-loop.md)
- 旧 07 [循环预算与重复动作](./chapters/loop-budget.md)
- 旧 08 [取消、截止时间与资源生命周期](./chapters/cancellation.md)
- 旧 09 [工具宿主与声明式扩展点](./chapters/tool-host.md)

### 让回答有依据、可检查

- 旧 10 [上下文结构：历史、状态与发送预算](./chapters/context-state.md)
- 旧 11 [获取外部资料：按需读取与检索](./chapters/context-retrieval.md)
- 旧 12 [将资料压缩接入循环](./chapters/context-compression.md)
- 旧 13 [验收条件、验证器与证据](./chapters/task-verification.md)

### 让修改可审查、可验证

- 旧 14 [信任边界与工具权限](./chapters/trust-permissions.md)
- 旧 15 [从修改意图到可审查补丁](./chapters/patch-candidate.md)
- 旧 16 [批准、应用与修改后验证](./chapters/approved-patch.md)
- 旧 17 [失败反馈与有限修复](./chapters/bounded-repair.md)

### 可选能力分支与结项

- 旧 18 [计划与任务审查](./chapters/task-planning.md)
- 旧 19 [持久化执行与崩溃恢复](./chapters/durable-recovery.md)
- 旧 20 [扩展协议与 TypeScript SDK](./chapters/extension-sdk.md)
- 旧 21 [接入 MCP：复用外部工具能力](./chapters/mcp-tools.md)
- 旧 22 [Hooks：在执行节点加入受控扩展](./chapters/execution-hooks.md)
- 旧 23 [受限委派与多智能体协作](./chapters/bounded-delegation.md)
- 旧 24 [文档维护助手：评测与复盘](./chapters/project-evaluation.md)

旧阶段汇总仍在原页面：[1](./milestones/evidence-qa.md)、[2](./milestones/controlled-harness.md)、[3](./milestones/grounded-answer.md)、[4](./milestones/single-agent-maintainer.md)、[5](./milestones/project-defense.md)。

## 数字 URL 版本

这里的数字章号按旧版主题解释，旧锚点与归档保持原义。现行 00–29 的主题和状态统一从[全书目录](./toc.md)查阅。

- [00 绪论：一次完整的任务](./chapters/00.md)
- [01 HelloWorld——从模型调用开始](./chapters/01.md)
- [02 写一份合格的提示词](./chapters/02.md)
- [03 工具调用——Harness 的骨架](./chapters/03.md)
- [04 统一协议——隔离模型服务商差异](./chapters/04.md)
- [05 让工具结果推动下一轮](./chapters/05.md)
- [06 循环控制](./chapters/06.md)
- [07 上下文管理](./chapters/07.md)
- [08 上下文管理方法：同一份资料的四种取法](./chapters/08.md)
- [09 验收条件与验证器](./chapters/09.md)
- [10 失败反馈与循环工程](./chapters/10.md)
- [11 防御性编程——AI 的矛与盾](./chapters/11.md)
- [12 权限管理：把修改变成可审查的差异](./chapters/12.md)
- [13 计划与任务审查](./chapters/13.md)
- [14 多智能体协作](./chapters/14.md)
- [15 外部扩展](./chapters/15.md)
- [16 垂直领域设计方法论](./chapters/16.md)

## 历史实现与独立实验

- [01 Rust hello](./chapters/01-rust.md)与[HTTP深入材料](./chapters/01-ts-advanced.md)。
- [旧05 Rust循环](./chapters/05-rust.md)、[旧06 Rust控制](./chapters/06-rust.md)、[旧06 Plugin M1](./chapters/06-plugin.md)、[旧07 Rust上下文](./chapters/07-rust.md)。
- [四策略上下文独立实验](./chapters/08.md)保留限时免费属性，其运行与数据继续使用原入口；不承担新版主线的必需步骤。
- [旧小结](./milestones/01.md)、[旧汇总1](./milestones/02.md)、[旧汇总2](./milestones/03.md)、[旧汇总3](./milestones/04.md)。

## 补充阅读

[阅读0](./readings/00.md)、[阅读1](./readings/01.md)、[阅读2](./readings/02.md)、[阅读3](./readings/03.md)，以及[A.1](./appendices/a1.md)、[A.2](./appendices/a2.md)、[A.3](./appendices/a3.md)、[A.4](./appendices/a4.md)、[A.5](./appendices/a5.md)继续可达。

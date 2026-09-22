# 全书目录

旧版规划为五篇、00–24 共 25 章，下面保留其历史编号与目录语义；Python 00–03 是当前发布的独立主线，旧版 04–24 不表示 Python 主线已经接续到这些章节。

当前 Python 主线开放 00–03 章及配套代码。Rust 迁移尚未完成；旧版五部分目录和历史页面仍保留，后续新版章节仅列出规划。

## Python 主线：从最小请求到建议

- [00 最小 Agent](./chapters/minimal-agent.md)
- [01 Python 模型调用](./chapters/python-model-call.md)
- [02 Python 读取文件](./chapters/python-file-read.md)
- [03 Python 生成建议](./chapters/python-suggestions.md)

## 旧版第一部分：让模型完成一个小任务

- [00 从一次文档维护任务认识 Rein](./chapters/task-map.md) · 文字已开放，待教学验收
- [01 HelloWorld：从模型调用开始](./chapters/model-hello.md) · 文字已开放，待教学验收
- [02 把任务说清楚：提示词与成功标准](./chapters/task-spec.md) · 文字已开放，待教学验收
- [03 工具调用：让模型读取真实文件](./chapters/tool-roundtrip.md) · 文字已开放，待教学验收
- [01 Rust 可选对照](./chapters/model-hello-rust.md) · 文字已开放，待教学验收
- [阶段汇总 1：真实资料问答原型](./milestones/evidence-qa.md) · 复查步骤已开放，阶段项目待验收

## 建立可控的混合运行时

- [04 模型接口：隔离服务商差异](./chapters/provider-adapter.md) · 后续章节待发布
- [05 从 TypeScript 原型走向 Rust core](./chapters/rust-migration.md) · 后续章节待发布
- [06 核心 Agent Loop：让工具结果推动下一轮](./chapters/agent-loop.md) · 后续章节待发布
- 07 循环预算与重复动作 · 后续章节待发布
- 08 取消、截止时间与资源生命周期 · 后续章节待发布
- 09 工具宿主与声明式扩展点 · 后续章节待发布

## 让回答有依据、可检查

- 10 上下文结构：历史、状态与发送预算 · 后续章节待发布
- 11 获取外部资料：按需读取与检索 · 后续章节待发布
- 12 将资料压缩接入循环 · 后续章节待发布
- 13 验收条件、验证器与证据 · 后续章节待发布

## 让修改可审查、可验证

- 14 信任边界与工具权限 · 后续章节待发布
- 15 从修改意图到可审查补丁 · 后续章节待发布
- 16 批准、应用与修改后验证 · 后续章节待发布
- 17 失败反馈与有限修复 · 后续章节待发布

## 可选能力分支与结项

- 18 计划与任务审查 · 后续章节待发布
- 19 持久化执行与崩溃恢复 · 后续章节待发布
- 20 扩展协议与 TypeScript SDK · 后续章节待发布
- 21 接入 MCP：复用外部工具能力 · 后续章节待发布
- 22 Hooks：在执行节点加入受控扩展 · 后续章节待发布
- 23 受限委派与多智能体协作 · 后续章节待发布
- 24 文档维护助手：评测与复盘 · 后续章节待发布

## 阅读材料与历史入口

[阅读 0](./readings/00.md)、[TS 基础](./readings/00-ts.md)和 [Rust 基础](./readings/00-rust.md)按需取用。历史章节、其余阅读材料与附录见[历史目录](./history.md)。章末提示词另见[使用说明](./prompt-examples.md)。当前自有内容统一按 [Apache-2.0 永久开放](./access.md)。

## 旧目录链接

下面保留旧目录的锚点，链接按当时的主题解释，不套用新版同号章节。

<span id="ch00"></span>
- [旧版：00 绪论，一次完整的任务（待撰写·永久免费）](./history.md#ch00)
<span id="chread0"></span>
- [旧版：阅读 0 基础知识补充（公共 / TS / Rust 已提供·永久免费）](./history.md#chread0)
<span id="ch01"></span>
- [旧版：01 HelloWorld——从模型调用开始（公共 / TS / Rust 已提供·永久免费）](./history.md#ch01)
<span id="ch02"></span>
- [旧版：02 写一份合格的提示词（待撰写·永久免费）](./history.md#ch02)
<span id="chread1"></span>
- [旧版：阅读材料 1　Vibe Coding 方法论（待撰写·永久免费）](./history.md#chread1)
<span id="ch03"></span>
- [旧版：03 工具调用，Harness 的骨架（待撰写·永久免费）](./history.md#ch03)
<span id="chsum1"></span>
- [旧版：小结，Rein 第一次获得环境信息（待撰写·永久免费）](./history.md#chsum1)
<span id="ch04"></span>
- [旧版：04 统一协议，隔离模型服务商差异（待撰写·限时免费）](./history.md#ch04)
<span id="ch05"></span>
- [旧版：05 核心 Agent Loop（待撰写·限时免费）](./history.md#ch05)
<span id="ch06"></span>
- [旧版：06 循环控制（待撰写·限时免费）](./history.md#ch06)
<span id="chread2"></span>
- [旧版：阅读材料 2　版本管理，为自己的编码助手保存可靠基线（待撰写·永久免费）](./history.md#chread2)
<span id="chread3"></span>
- [旧版：阅读材料 3　Pi，极简 Harness 设计思路（待撰写·永久免费）](./history.md#chread3)
<span id="chsum2"></span>
- [旧版：阶段性汇总 1　极简 Harness 的完成（待撰写·限时免费）](./history.md#chsum2)
<span id="ch07"></span>
- [旧版：07 上下文管理（待撰写·限时免费）](./history.md#ch07)
<span id="ch08"></span>
- [旧版：08 上下文管理方法，四种做法的横向对比（待撰写·可独立阅读·限时免费）](./history.md#ch08)
<span id="ch09"></span>
- [旧版：09 验收条件与验证器（待撰写·可独立阅读·限时免费）](./history.md#ch09)
<span id="ch10"></span>
- [旧版：10 失败反馈与循环工程（待撰写·限时免费）](./history.md#ch10)
<span id="ch11"></span>
- [旧版：11 防御性编程，AI 的矛与盾（待撰写·永久免费·可独立阅读）](./history.md#ch11)
<span id="ch12"></span>
- [旧版：12 权限管理，把修改变成可审查的差异（待撰写·可独立阅读·限时免费）](./history.md#ch12)
<span id="chsum3"></span>
- [旧版：阶段性汇总 2　一个能够审查修改的编码助手（待撰写·限时免费）](./history.md#chsum3)
<span id="ch13"></span>
- [旧版：13 计划与任务审查（待撰写·限时免费）](./history.md#ch13)
<span id="ch14"></span>
- [旧版：14 多智能体协作（待撰写·可独立阅读·限时免费）](./history.md#ch14)
<span id="ch15"></span>
- [旧版：15 外部扩展（待撰写·限时免费）](./history.md#ch15)
<span id="ch16"></span>
- [旧版：16 垂直领域设计方法论（待撰写·限时免费）](./history.md#ch16)
<span id="chsum4"></span>
- [旧版：阶段性汇总 3　完成可以展示与答辩的项目（待撰写·限时免费）](./history.md#chsum4)
<span id="附录-rust-track"></span>
- [旧版补充入口](./history.md#附录-rust-track)

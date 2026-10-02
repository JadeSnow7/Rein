---
layout: home
hero:
  name: 'Rein'
  text: '从零构建 Agent'
  tagline: '人定设计，AI 编码：从一次模型调用，到可审查、可验证的 Agent Harness。'
  image:
    src: /mark.svg
    alt: Rein mark
  actions:
    - theme: brand
      text: 开始阅读
      link: /chapters/minimal-agent.html
    - theme: alt
      text: 查看全书目录
      link: /toc.html
features:
  - title: 一个连续项目
    details: 围绕同一个 Rein 工作区助手，沿共同章节逐步完成模型调用、工具执行与结果验证。
  - title: 工程证据
    details: 每个阶段都对应运行记录、失败测试、架构图与可展示的成果。
  - title: 分阶段实现
    details: 先用 Python 做出终端代码修改助手，再进入 Rust 核心循环、可信修改、Agent 扩展、轻量 IDE 与持续演进。
---

<div class="home-note">
  <span class="home-note__label">建设中 / 内容开放说明</span>
  <p>本书当前自有源码与正文统一按 Apache-2.0 永久开放。未完成章节仍处于规划、撰写、实现或验收阶段，开放许可不代表内容已经完成或发布；本页不承诺未发布的未来内容已经完成。<a href="/Rein/access.html">查看完整说明 →</a></p>
</div>

## 现在可以读什么

从[00 我们要实现一个怎样的 Agent](./chapters/minimal-agent.md)开始，再用 Python 生成 C++ Hello World，通过受限工具读取源码和真实报错，在终端审查并接受修改，最后编译验证。第一部分 00–03 已有正文和离线示例；你可以不配置模型服务先跟做。文字发布、程序检查、真实模型表现和读者教学验收分别判断。接着读[04 Agent 的核心能力](./chapters/agent-capabilities.md)：从这一章起，由你写设计、让代码助手实现，把第一部分的行为迁移到 Rust。05–29 的目标和状态见[六部分目录](./toc.md)。

<span id="先从一次完整任务开始"></span>

## 这条路线要走到哪里

Rein 最终要完成的事情很朴素：理解一个工作区任务，读取相关文件，提出一份人类可以审查的差异，等待批准，执行修改，并用验证器确认结果。全书从这个终点倒推，每一章只增加一个主要难点。

## 写给谁

如果你准备应聘 Agent 应用、AI 应用工程或开发者工具相关岗位，这本书会把“会调用模型”推进到“能解释一个可靠系统为什么这样工作”。随着章节推进，你可以积累能够运行、演示、评测并在面试中展开讨论的项目材料。

<div class="home-links"><a href="/Rein/about.html">阅读指南 →</a><a href="/Rein/toc.html">完整目录 →</a><a href="/Rein/access.html">开放说明 →</a><a href="https://github.com/JadeSnow7/Rein">GitHub →</a></div>


## 新版主线

六部分依次是极简 Harness、核心循环、可信 Harness、Agent 扩展、实际应用、架构设计与持续演进。当前正式章节清单为 00–29；Rust 等价迁移安排在第 04 章，后续规划页会明确标出尚未实现的部分。旧五篇、TS/Rust 对照和数字编号从[历史与补充入口](./history.md)查阅，不占用现行目录。

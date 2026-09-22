---
layout: home
hero:
  name: 'Rein'
  text: '从零手写一个 Agent Harness'
  tagline: '一条面向 Agent 学习者与求职者的工程实践路线：从模型调用，到可审查、可验证的工作区助手。'
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
  - title: 一条可运行的 Python 主线
    details: 从最小 Agent、模型调用、读取文件到生成建议，正文与 python/part1 配套代码同步推进。
---

<div class="home-note">
  <span class="home-note__label">第一部分已开放 / Apache-2.0</span>
  <p>当前自有正文与配套源码按 Apache-2.0 永久开放。文字发布与教学验收分别记录，后续章节继续更新。<a href="/Rein/access.html">查看完整说明 →</a></p>
</div>

## 现在可以读什么

当前阅读主线从[00 最小 Agent](./chapters/minimal-agent.md)开始，依次阅读[01 Python 模型调用](./chapters/python-model-call.md)、[02 Python 读取文件](./chapters/python-file-read.md)、[03 Python 生成建议](./chapters/python-suggestions.md)。旧版 TypeScript/Rust 页面仍可从[历史目录](./history.md)进入，Python 主线之后的 Rust 迁移尚未完成；后续规划见[全书目录](./toc.md)。

<span id="先从一次完整任务开始"></span>

## 这条路线要走到哪里

Rein 最终要完成的事情很朴素：理解一个工作区任务，读取相关文件，提出一份人类可以审查的差异，等待批准，执行修改，并用验证器确认结果。全书从这个终点倒推，每一章只增加一个主要难点。

## 写给谁

如果你准备应聘 Agent 应用、AI 应用工程或开发者工具相关岗位，这本书会把“会调用模型”推进到“能解释一个可靠系统为什么这样工作”。随着章节推进，你可以积累能够运行、演示、评测并在面试中展开讨论的项目材料。

<div class="home-links"><a href="/Rein/about.html">阅读指南 →</a><a href="/Rein/toc.html">完整目录 →</a><a href="/Rein/access.html">开放说明 →</a><a href="https://github.com/JadeSnow7/Rein">GitHub →</a></div>

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
      link: /chapters/task-map.html
    - theme: alt
      text: 查看全书目录
      link: /toc.html
features:
  - title: 一个连续项目
    details: 围绕同一个 Rein 工作区助手，沿共同章节逐步完成模型调用、工具执行与结果验证。
  - title: 工程证据
    details: 每个阶段都对应运行记录、失败测试、架构图与可展示的成果。
  - title: 两种语言
    details: TypeScript 主线建立模型调用与工具执行，第一章另有 Rust 对照；每篇分别说明实现与验收进度。
---

<div class="home-note">
  <span class="home-note__label">第一部分已开放 / Apache-2.0</span>
  <p>当前自有正文与配套源码按 Apache-2.0 永久开放。文字发布与教学验收分别记录，后续章节继续更新。<a href="/Rein/access.html">查看完整说明 →</a></p>
</div>

## 现在可以读什么

第一部分已经开放：从[00 从一次文档维护任务认识 Rein](./chapters/task-map.md)开始，依次阅读[01 HelloWorld](./chapters/model-hello.md)、[02 任务与成功标准](./chapters/task-spec.md)、[03 工具调用](./chapters/tool-roundtrip.md)，最后用[阶段汇总 1](./milestones/evidence-qa.md)复查。想对照 Rust 时，可在 HelloWorld 页面切换到 [Rust 版](./chapters/model-hello-rust.md)。需要补充终端、HTTP、配置、类型、异步或错误处理知识时，再按需查阅[阅读 0](./readings/00.md)及对应语言版本。第二部分从[模型接口](./chapters/provider-adapter.md)开始，当前仅提供待发布承接页。

<span id="先从一次完整任务开始"></span>

## 这条路线要走到哪里

Rein 最终要完成的事情很朴素：理解一个工作区任务，读取相关文件，提出一份人类可以审查的差异，等待批准，执行修改，并用验证器确认结果。全书从这个终点倒推，每一章只增加一个主要难点。

## 写给谁

如果你准备应聘 Agent 应用、AI 应用工程或开发者工具相关岗位，这本书会把“会调用模型”推进到“能解释一个可靠系统为什么这样工作”。随着章节推进，你可以积累能够运行、演示、评测并在面试中展开讨论的项目材料。

<div class="home-links"><a href="/Rein/about.html">阅读指南 →</a><a href="/Rein/toc.html">完整目录 →</a><a href="/Rein/access.html">开放说明 →</a><a href="https://github.com/JadeSnow7/Rein">GitHub →</a></div>

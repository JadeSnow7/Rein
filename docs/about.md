<span id="关于本书"></span><span id="书的目标"></span>

# 阅读指南

Rein 从一个工作区文档维护任务出发，逐步学习模型调用、工具执行、任务状态与结果验证。每增加一项能力，就检查它在什么输入下有效，以及失败时怎样停止或反馈。

<span id="如何阅读"></span>

## 从第一部分开始 {#start}

从[00 最小 Agent](./chapters/minimal-agent.md)进入，依次阅读[01 Python 模型调用](./chapters/python-model-call.md)、[02 Python 读取文件](./chapters/python-file-read.md)、[03 Python 生成建议](./chapters/python-suggestions.md)。

当前主线使用 Python；后续 Rust 迁移尚未完成。旧版 TypeScript/Rust 内容与原五部分目录历史语义保留，可从[历史目录](./history.md)独立访问。遇到终端、Git、HTTP、配置、类型或异步知识缺口时，再按需查阅[阅读 0](./readings/00.md)及 [TS](./readings/00-ts.md) / [Rust](./readings/00-rust.md) 材料。

<span id="两条-track"></span>

## 使用已有的语言对照 {#languages}

旧版配对内容的页面显示 TS / Rust 按钮；这些是历史语言对照页面，不是当前 Python 主线的语言切换。第一章以外的后续迁移、循环与扩展主题按[新版目录](./toc.md)推进，不能由旧版两版 hello 推断其他章节已经实现。

## 取得配套代码 {#code}

跟随 Python 主线取得当前仓库后，按正文准备依赖和配置：

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

`python/part1/` 提供 00–03 章配套代码；`ts/` 与 `rust/` 保留旧版教学实现。第 00 章提及的产品 `core/`、`runtime/` 属于另一条开发进度，不需要在当前克隆中查找或构建这些目录。

旧 `ch01`、`ch01-helloworld` 标签保留历史意义。切换标签会改变整个仓库，不是启动章节或切换网页语言。新版章节输入和输出快照仍待建立，当前工作副本不能冒充已封存的逐章快照。

## 区分检查与验收 {#evidence}

类型检查说明源码满足类型约束；本地测试说明选定样本符合预期；真实模型记录用于检查实际服务与回答；读者跟做用于检验教学步骤。它们不能互相替代。

Python 00–03 章正文与配套代码已发布，本地离线示例和测试已检查。真实模型、读者跟做与逐章快照仍需分别验收；运行练习时保存输入、命令、输出与失败，不以模型声称完成代替任务结果。

<span id="参与建设"></span>

## 开放与更新 {#access}

当前自有正文和配套源码按 Apache-2.0 永久开放，见[开放说明](./access.md)。全书基础路线为 **00–17 → 24**，18–23 为可选分支；后续章节的规划见[全书目录](./toc.md)，已有数字页面见[历史入口](./history.md)。

欢迎通过 [GitHub 仓库](https://github.com/JadeSnow7/Rein)提出问题与改进。

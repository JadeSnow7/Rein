<span id="关于本书"></span><span id="书的目标"></span>

# 阅读指南

Rein 从同一个 C++ Hello World 的生成与修复任务出发，用 Python 逐步学习模型请求、读取源码与报错、展示差异、接受修改和编译验证。每增加一项能力，就检查它在什么输入下有效，以及失败时怎样停止或反馈。

<span id="如何阅读"></span>

## 从第一部分开始 {#start}

从[00 最小 Agent](./chapters/minimal-agent.md)进入，依次阅读[01 用 Python 完成第一次模型调用](./chapters/python-model-call.md)、[02 根据报错修正 Hello World](./chapters/python-file-read.md)、[03 做一个终端代码修改助手](./chapters/python-suggestions.md)。

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

`python/hello_world/` 提供当前三章配套代码，`python/part1/` 保留前一版 README 只读建议器；`ts/` 与 `rust/` 保留旧版教学实现。第 00 章提及的产品 `core/`、`runtime/` 属于另一条开发进度，不需要在当前克隆中查找或构建这些目录。

旧 `ch01`、`ch01-helloworld` 标签保留历史意义。切换标签会改变整个仓库，不是启动章节或切换网页语言。新版章节输入和输出快照仍待建立，当前工作副本不能冒充已封存的逐章快照。

## 区分检查与验收 {#evidence}

类型检查说明源码满足类型约束；本地测试说明选定样本符合预期；真实模型记录用于检查实际服务与回答；读者跟做用于检验教学步骤。它们不能互相替代。

Python 00–03 章正文与配套代码已发布，本地离线示例和测试已检查。真实模型、读者跟做与逐章快照仍需分别验收；运行练习时保存输入、命令、输出与失败，不以模型声称完成代替任务结果。

<span id="参与建设"></span>

## 开放与更新 {#access}

当前自有正文和配套源码按 Apache-2.0 永久开放，见[开放说明](./access.md)。新框架计划在第04章进行 Rust 等价迁移，当前未交付；原 **00–17 → 24** 路线与18–23可选分支属于旧版规划；后续章节的规划见[全书目录](./toc.md)，已有数字页面见[历史入口](./history.md)。

欢迎通过 [GitHub 仓库](https://github.com/JadeSnow7/Rein)提出问题与改进。

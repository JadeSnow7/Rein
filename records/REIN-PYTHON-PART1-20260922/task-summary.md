# 六部分框架 00–03 重写

本轮范围完成：新第一部分以 Python 文件修改建议器为主线，四章正文为 `minimal-agent`、`python-model-call`、`python-file-read`、`python-suggestions`。同步首页、阅读指南、目录、README 与导航；旧五篇索引及历史 TS/Rust 正文保留。新 03 的下一页为目录，不冒接旧 04。

配套代码在 `python/part1/`，默认离线、只读；提供一次请求、直接读取、一次工具往返、程序定位与展示前来源复查。S1-manifest.json 固定17份源码/输入文件与10组实际运行产物，供未来Rust迁移使用，不是Git标签。

## 本地验证

- 17 项 Python 测试在 Python3.14.6 / openai2.26.0 隔离环境下通过。
- 主线程独立CLI复验24种输入；实际SDK经过本地HTTPX MockTransport验证4条流程，最短SDK正文另测一次。
- 从最终Markdown提取20段离线命令，按顺序在仓库根执行；正常与预期失败退出码均符合说明，fixture字节不变。
- 6项历史语言/导航检查通过；旧book索引检查、VitePress构建、81页面4540条站内链接与新页导航检查通过。
- 本地浏览器走通01→02→03→目录，检查当前/历史侧栏、代码布局、无Python语言误切换与无console错误。
- 主线程完成正文与源码自审。修复前失败回执保留为历史诊断，不作为当前验收。

## 边界与交付

真实模型服务、模型建议质量和新手试读未验证。新第04–29章未重写，Rust等价迁移未执行。此轮新稿为本地交付，未提交、推送或发布；上一轮已发布的TS四章与ff322245提交是历史交付。

最终证据：evidence/final-review.json、final-checks.json、markdown-checks.json、browser-review.json。原入口基线在baseline/。复验入口为checks/final.sh与checks/verify_markdown.py；完整SDK测试使用本轮隔离环境解释器，可在按正文安装固定依赖后替换解释器路径复跑。

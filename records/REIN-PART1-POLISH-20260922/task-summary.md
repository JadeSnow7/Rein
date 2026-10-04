# 00–03 第二轮精修

本轮完成四篇新版正文精修。范围限于教学表达和离线可跟做性；保留旧数字原稿、Rust对照、阶段汇总及章节草稿状态。

## 精修结果

- [00](../../docs/chapters/task-map.md)：以同一个维护案例解释建议、修改和运行检查的区别；先建立问题，再引入Harness，简化产品进度插叙。
- [01](../../docs/chapters/model-hello.md)：补源码阅读顺序、响应JSON层级和单次CLI的上下文范围。
- [02](../../docs/chapters/task-spec.md)：提供可复制的完整调用命令，增加格式正确却事实错误的反例；说明每次从原始输入重做对照，并消除材料版本矛盾。
- [03](../../docs/chapters/tool-roundtrip.md)：增加七段代码组装表、两次请求与本地读取的消息流，准确区分协议错误、参数错误和文件错误。

## 验证与边界

- [终稿审读](evidence/final-review.json)：主线程审查全部基线差异及coder产物。01/03 TS代码块逐字未改；保护路径哈希一致。
- [02离线检查](evidence/ch02-offline.json)：从最终Markdown提取完整命令，经shell核对参数，再使用真实调用模块与注入fetch验证消息；空模型配置在请求前返回1。没有调用真实服务。
- [最终回归](evidence/final-checks-v3.json)：七段TS提取与边界/往返验证、31项hello/config测试、类型检查、book/目录检查、VitePress构建及4012项链接检查。v3绑定补齐后的记录范围；v2对应最终文字，但记录声明随后补齐，保留为历史回执。

本轮未做真实模型调用、初学者试读或浏览器视觉检查，未冻结章节快照，未提交、推送或部署。文字细修与离线检查不等于正式教学验收。Rust对照和阶段汇总未改变操作契约，本轮不重复运行其不受影响的测试。

## 历史证据适用性

[上一轮审查](../REIN-PART1-EDITORIAL-20260922/task-summary.md)及其原始回执保留，支持上一轮稿件；本轮四篇的新字节以本目录Spec、基线、审读与最终回归为准。首次检查环境和临时核验的调整见[执行记录](work-log.md)。

接口参考于2026-09-22核对：[OpenAI工具调用](https://developers.openai.com/api/docs/guides/function-calling)、[DeepSeek首次调用](https://api-docs.deepseek.com/)、[DeepSeek Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/)。当前源码负责解释本地行为，官方接口文档不代替端点实测。

## 后续交付

用户随后授权“提交并发布”。本轮四篇已随 `ff32224587c150c0b73b9888c09b1ebdc04d72ec` 发布至main及GitHub Pages，详见[发布回执](../REIN-PART1-POLISH-RELEASE-20260922/task-summary.md)。此前“未提交、未发布”为精修阶段结束时的历史状态。

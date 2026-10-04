# 第一章精修结果

已完成用户2026-09-23批准方案；仅本地修改，未提交、推送或发布。

交付文件：
- `docs/chapters/python-model-call.md`：保留六节和作者手动稿的开篇场景；补三平台安装与成功标志、三家服务申请路径；全文引用首次问候与CLI代码；通过多轮对话生成C++并手动保存编译；第四节收缩为七类错误表和缺配置实验；展示及作业随之更新。
- `python/hello_world/hello.py`：本地启动问候、缺配置准确列项且不导入SDK、不联网、退出1；有效响应带模型标签；响应结构检查与安全诊断。
- `python/hello_world/chat.py`：新增自包含非流式交互入口、当前会话历史、成功问答才加入上下文、失败继续；空输入跳过、退出指令/EOF/Ctrl+C正常处理，无自动重试。
- `python/hello_world/README.md`：安装→隐藏配置→两个入口→离线练习→测试说明。
- `python/hello_world/tests/test_chat.py`：13项新增测试；与原23项配套测试共36项通过。

## 验证

- `evidence/unit-final-v2.json`：主线程最终全量36项通过，无跳过，覆盖现有第二三章代码路径。
- `evidence/walkthrough-final.json`：从最终Markdown提取命令与C++样本，真实SDK经MockTransport完成四轮对话并验证上下文；将返回源码写入临时文件模拟手动复制，真实编译运行得到Hello, world!与退出0；固定样本替代路径同样通过；删分号得到真实编译错误；缺配置无请求。
- `evidence/build-final.json`：站点构建通过。
- `evidence/book-check.json`：目录结构与同步检查通过。
- `evidence/links-final.json`：81页、4547项内部链接检查通过。
- `evidence/browser.json`：最终重建后浏览器核对完整代码、六节内容与后续章节入口。
- `final-source-hashes.json`：5份本轮交付文件哈希；验证回执绑定的输入逐项与当前文件复核一致。

已保留原始失败证据：agent-baseline为实现前基线；hello-shape-baseline暴露首次请求空候选分类缺口，修复后同一基准通过。首轮walkthrough夹具将SDK注入传给无SDK的系统Python配置检查，产生额外提示；修复验证夹具后重新完整演练，最终回执不含该错误。中间构建/测试回执只代表当时版本，最终结论使用上述final记录。

## 边界

安装命令和服务申请路径对照官方资料，未实际重装macOS工具、Linux或WSL，未注册/充值账户。未调用真实模型服务，未进行新手独立试读。受控响应不能证明服务可用性或真实建议质量。

第二三章正文和既有request/core/model/cli文件与本轮起始快照一致。工作区其他既有改动保留，本结果仅覆盖声明范围，不表示整个仓库验收。代码由coder实施，主线程已读diff、原始回执、复验输入哈希并审查。

## 后续文字补充：询问运行命令

按用户追加要求，在第三节取得C++源码后加入可直接输入的提示词：交代操作系统与hello.cpp，让AI给出创建目录、保存、检查编译器、编译、运行和退出码检查步骤，并区分编辑器操作和终端命令。同步第2项作业。仅正文改变，配套代码哈希保持一致。新增提示词未进行真实模型实测；构建与4547项链接检查重新通过，证据为run-instructions-build.json与run-instructions-links.json。之前final-source-hashes.json中正文哈希及正文演练绑定的是补充前版本，保留为历史记录。本轮仍未提交发布。

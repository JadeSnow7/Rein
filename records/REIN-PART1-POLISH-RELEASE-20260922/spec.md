# 00–03 精修提交发布

来源：用户“提交并发布”。将本轮精修四篇新版正文发布到 JadeSnow7/Rein main 与既有GitHub Pages。

仅将 docs/chapters/task-map.md、model-hello.md、task-spec.md、tool-roundtrip.md 的已审字节整合到最新main；不带入原工作区其他草稿、产品实现或导航修改。远程四文件与精修前基线完全一致，保留远程其他文件。

提交前：四文件与已审终稿哈希一致，独立发布副本的工具提取、hello/config测试、类型、构建和站内链接检查通过。先在远程基线构建检查，再应用正文。

提交与推送采用正常非force main更新，main若有变化先重新整合验证。推送触发现有Deploy book，等待绑定发布SHA的build/deploy成功；HTTPS核对四篇页面各自新增文本和章号，浏览器抽查03消息流。后续阶段仍待教学验收。

授权：commit、push、deploy，来源为本轮用户指令。无需变更接口、依赖、工作流或仓库设置。原工作区保持不变；动作与后续回执本地保存，不为日志追加再次部署。

# 精修执行记录

- 从当前四篇正文建立基线，不沿用旧稿回执证明新稿。书籍结构检查通过。
- 首次 record_execution 缺 Spec authority 的 contracts 引用，执行前被拒绝；补齐后开始基线。
- 首次工具检查误用 node 而未加 --import tsx，临时 config.ts 的无扩展名导入不能解析。原回执 baseline.json 保留；按正文相同的 tsx 加载器重跑，baseline-v2.json 通过。未修改源程序来迁就检查。
- 主线程负责00/02，coder负责01/03；先运行基线，再开始改正文。
- 02第一次临时核验写错预期行数，在进入程序前断言失败；去掉无关的硬编码行数，改为完整shell参数及输入内容相等检查。最终核验通过，见 ch02-offline.json。
- 主线程审查 coder diff，要求修正配置读取主体、拒绝读取时的流程图、arguments字符串与解析失败的区分，并将七段组装导引改成表格。
- 实时核对官方 OpenAI function-calling 的消息回传示例，以及 DeepSeek 根地址、模型名与 thinking 参数。引用保持在正文相应位置；不据此宣称真实端点已通过。
- 最终记录校验发现 receipt_paths 纳入Spec指纹，补登回执后须重验；同时补全本轮开始已存在的foreign_paths，并将02临时检查原始产物按artifact记录。保留旧回执，在完整声明下执行v3。

# 主线程审查

第一轮发现并交回coder：首次请求空choices应判响应不可用；补真实EOF/中断/失败后继续等行为检查；移除chat入口的测试注入全局与重复配置辅助函数以便完整阅读；README使用隐藏输入密钥；429提示与正文同步。

正文保留六节、作者开篇场景和跟做语气。环境命令先安装工具再克隆，三服务商各自申请路径与统一环境变量；CLI完整代码引用，退出后再执行shell命令；第四节没有core函数源码与测试框架讲解。第三节手动复制时显式排除模型标签和围栏。

官方资料核对（2026-09-23）：
- OpenAI https://developers.openai.com/api/docs/quickstart ：创建密钥与SDK入门；项目模型选择由账户可用性决定。
- DeepSeek https://api-docs.deepseek.com/ ：基础地址https://api.deepseek.com，当前示例deepseek-flash，OpenAI兼容。
- MiMo https://mimo.mi.com/docs/zh-CN/quick-start/summary/first-api-call ：小米登录、按量API Keys、基础地址https://api.xiaomimimo.com/v1，示例mimo-v2.6-pro；Token Plan凭据不同。
- Homebrew https://docs.brew.sh/Installation 与 Language-Runtimes-and-Packages ：安装命令、Next steps、brew python、venv。
- Microsoft https://learn.microsoft.com/en-us/windows/wsl/install ：管理员PowerShell安装WSL，随后进入Ubuntu。
- Ubuntu https://ubuntu.com/developers/docs/howto/python-setup/ ：Python包和虚拟环境。
- Apple安装文档正文抓取受JS限制，相关命令将只作为文档核对，不宣称本轮重装验证。

验证边界：不为教程改动安装操作系统/系统工具、不注册或充值服务账户、不调用真实服务。验证用现有macOS开发环境与受控SDK响应，三平台安装和账户开通未亲自执行。

首轮演练walkthrough.json主流程通过，但审查stdout发现验证夹具的PYTHONPATH泄漏给系统python3配置检查，产生sitecustomize缺httpx提示；这是验证夹具问题，修复为仅对SDK入口注入，并增加stderr断言后复跑，不改产品代码或放宽行为断言。

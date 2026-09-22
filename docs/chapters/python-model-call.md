---
title: 用 Python 完成第一次模型调用
---

# 01 用 Python 完成第一次模型调用

在聊天窗口里输入“你好”，很快就能得到一句回答。现在我们把这件事搬到自己写的 Python 程序里：准备配置，发送请求，取出文本，再把问候换成一个真正的编码任务。

这一章的终点是：你能用 Python 请求模型生成 C++ Hello World，亲自保存、编译并运行它。Python 是我们编写的调用程序，C++ 是这次任务的产物；不用同时学完整的两门语言。下一章会故意让这个小程序报错，再让模型根据报错修正它。

## 1、环境准备：Python 与环境变量

以下命令使用 macOS、Linux 或 WSL 的终端。先取得包含 `python/hello_world/` 的 Rein 工作副本，并进入仓库根目录。还没有仓库时执行：

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

已有副本时直接进入它，不必重新克隆。本轮修订尚未发布到远程时，新克隆可能没有配套目录；应使用与本章一起交付的副本，不要切换旧 `ch01` 标签寻找 Python 示例。

检查 Python，然后建立一个独立的依赖环境：

```bash
python3 --version
python3 -m venv python/hello_world/.venv
python/hello_world/.venv/bin/python -m pip install -r python/hello_world/requirements.txt
```

使用 Python 3.11 或更新版本；本轮本地检查使用 3.14.6。虚拟环境相当于本教程自己的依赖目录，后面用它的完整解释器路径运行真实请求，不必反复判断是否已经激活。配套依赖固定为 `openai==2.26.0`，这是教学基线，不表示最新版本。SDK 是一组封装好的接口调用方法，Python 安装方式可对照 [OpenAI 官方 SDK 文档](https://developers.openai.com/api/docs/libraries)。

要访问模型服务，需要准备三项信息：

| 配置 | 作用 | 从哪里取得 |
| --- | --- | --- |
| `REIN_BASE_URL` | 请求发送到哪个 API 基础地址 | 服务商的兼容接口文档 |
| `REIN_API_KEY` | 证明程序使用哪个账户的凭据 | 服务商 API 控制台 |
| `REIN_MODEL` | 本次请求使用的模型标识 | 账户可用模型清单 |

三项必须属于同一家服务。本章选择非流式 Chat Completions 接口，因此要确认模型支持它；第二章还会使用工具调用。使用 OpenAI 时可从 [API keys 页面](https://platform.openai.com/api-keys)管理凭据，基础地址为 `https://api.openai.com/v1`。兼容服务的地址可能不同，不要自行删掉 `/v1`，也不要填完整的 `/chat/completions` 路径。

在当前终端填写地址和模型。密钥通过 Python 的隐藏输入读取，避免明文进入命令历史：

```bash
export REIN_BASE_URL='https://api.openai.com/v1'
export REIN_MODEL='替换为账户可用的模型标识'
export REIN_API_KEY="$(python3 -c 'import getpass; print(getpass.getpass("API key: "))')"
```

环境变量是终端传给子程序的配置；新开终端需要重新设置。本章程序**不会自动加载 `.env`**。不要把密钥写进 Python 源码、日志或截图。真实请求会产生 API 用量，具体计费和可用性以所选服务为准。

下面只检查是否填写，不显示密钥内容：

```bash
python3 - <<'PY'
import os
for name in ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL"):
    print(name, "已填写" if os.getenv(name, "").strip() else "未填写")
PY
```

没有 API 配置时，可以使用本章的离线命令继续观察程序流程。离线替身返回预先约定的内容，不代表远端模型已经被调用。

## 2、发送请求：你好，请介绍一下自己

先写出下面这份完整程序。配套文件是 `python/hello_world/hello.py`；你可以在编辑器中逐行输入，与文件对照，而不是一开始就阅读整个项目。

<<< ../../python/hello_world/hello.py

程序按顺序做了四件事：从环境变量取得配置，创建 SDK 客户端，将一条 `user` 消息交给模型，再取出响应文本。`stream=False` 表示等待完整回答，`max_retries=0` 表示不自动重试。这里先保留最短成功路径，友好的失败处理在第四节补齐。

从仓库根目录运行，**这条命令会访问配置的真实服务**：

```bash
python/hello_world/.venv/bin/python python/hello_world/hello.py
```

回答可能类似下面这样；这是输出示意，不要求逐字相同：

```text
你好！我是一个 AI 助手，可以帮助你解释问题、整理文字和编写代码。
```

先检查有没有回应问候、有没有简短介绍能力。模型的自我介绍是一段生成文本，不能据此验证实际部署的模型身份。

没有配置时，用下面的离线命令观察同样的文本输出入口：

```bash
python3 python/hello_world/cli.py hello
```

离线模式会标明来源。即使看到了“你好”，也只说明本地程序处理了教学响应。

沿着刚才的代码看一次数据流：

```text
用户输入 → messages 中的 user.content → SDK 发送请求
        → 服务返回 response → choices[0].message.content → 终端
```

`messages` 是消息列表，`role` 表示这条消息来自谁，`content` 是消息内容。返回的 `choices` 是候选列表，本章取第一个候选的回答；它不是响应顶层的 `content`。每次运行都重新构造消息，因此程序不会自动记住上次问了什么。

## 3、一次完整任务：生成并运行 C++ Hello World

固定问候只能演示一次调用。接下来把任务作为命令行参数传入，让同一个程序处理不同请求。完整入口是 `python/hello_world/request.py`：

<<< ../../python/hello_world/request.py

入口负责接收输入、调用请求函数、打印回答，并把失败转换为非零退出码。请求函数仍然沿着上一节的 SDK 路径执行；`core.py` 集中处理配置、响应和错误，避免后面的读取工具重复写一遍网络代码。第四节会沿着失败路径读它。

先给模型一个可以检查的任务，而不只说“写个 hello world”：

```bash
python/hello_world/.venv/bin/python python/hello_world/request.py '请用 C++17 编写一个完整程序。正常运行时只输出 Hello, world!，末尾有一个换行，并返回 0。只返回源码，不加 Markdown 围栏或解释。'
```

这里改变的是消息中的任务文本，发送请求和读取响应的方法没有改变。创建一个独立练习目录，把终端返回的**源码部分**保存为 `hello.cpp`：

```bash
hello_workspace=$(mktemp -d)
printf '%s\n' "$hello_workspace"
```

在编辑器中打开这个目录，新建 `hello.cpp` 并粘贴回答。记住目录位置，后两章会继续使用。不要把解释文字或三个反引号一起保存为 C++。

一种合格的源码如下。`#include` 引入输出能力，`main` 是程序入口，`std::cout` 打印文本，`return 0` 表示正常结束：

```cpp
#include <iostream>

int main() {
    std::cout << "Hello, world!\n";
    return 0;
}
```

没有模型配置时，下面的命令把离线教学源码保存到这个刚创建的空目录：

```bash
python3 python/hello_world/cli.py generate > "$hello_workspace/hello.cpp"
```

这与前面的手工粘贴是两条替代路径，选择一种即可。`>` 会写文件，所以只对自己的练习副本使用；离线来源提示与源码分开，不会写进 `hello.cpp`。

现在才需要 C++ 编译器。先检查：

```bash
c++ --version
```

没有这个命令时，macOS 可以安装 Xcode Command Line Tools；Linux/WSL 可以通过发行版的开发工具包安装 C++ 编译器。先让 `c++ --version` 正常输出版本，再继续。编译器缺失是本地环境问题，此时让模型修改源码不能解决它。

读过生成的源码后，手工编译，再运行产物：

```bash
c++ -std=c++17 "$hello_workspace/hello.cpp" -o "$hello_workspace/hello"
```

只有上一步成功才执行：

```bash
"$hello_workspace/hello"
echo $?
```

预期第一条输出 `Hello, world!` 并换行，第二条输出 `0`。模型返回了源码、编译器接受了源码、程序符合要求，是三个不同的检查。本章由你完成后两个动作，模型并没有自动执行这些命令。

## 4、分析失败，并补全处理

最短程序在成功时很好理解，但缺少配置、网络超时或空回答时，读者可能只看到一长串异常。安全入口把这些失败分开处理，让错误指向下一步动作。

| 发生位置 | 常见现象与原因 | 程序应做什么，读者接着检查什么 |
| --- | --- | --- |
| 配置 | 缺 key、地址或模型，地址格式错误 | 发请求前停止；指出配置项，不打印 key |
| 请求 | 认证失败、连接失败、超时或服务异常 | 保留错误类别；检查账户、网络及服务状态 |
| 额度与频率 | 429 或相应服务错误码 | 根据错误详情区分额度和频率；不要把所有 429 都当成等一会就能解决 |
| 响应 | 没有候选、结构错误、回答为空 | 拒绝把不可用响应当成功文本 |
| 任务 | 有文本，但 C++ 编译失败或输出错误 | 保留编译/运行反馈；下一章把这些证据提供给模型 |

API 错误类型可对照 [OpenAI 官方错误说明](https://developers.openai.com/api/docs/guides/error-codes)；兼容服务可能采用不同错误码。本程序遇到无法确定的 HTTP 错误会保留通用分类，不根据状态码猜测具体账户情况。

打开配套的 `core.py`，沿 `_config`、`hello` 和 `_extract_response` 读下去：配置先检查，SDK 请求放在异常处理里，回答取出后还要检查类型与非空。入口捕获本章自己的错误对象，将简短诊断写到错误输出，并返回 `1`；正常回答写到标准输出，返回 `0`。区分两种输出，是为了让保存源码时不会混入错误提示。

安全版实际发送请求的函数如下。它仍然做“配置→发送→取文本”三步；可注入客户端的参数用于本地测试，跟做时不需要填写。辅助配置和响应检查函数在同一个 `core.py` 中。

<<< ../../python/hello_world/core.py#live_request

先制造一个确定的本地错误，不必等待网络碰巧出问题：

```bash
REIN_MODEL='' python/hello_world/.venv/bin/python python/hello_world/request.py '你好'
echo $?
```

预期为配置错误和退出码 `1`，不会访问模型。这条前缀赋值仅影响本次进程，不会删除终端原来的配置。

然后运行配套测试，观察受控的空响应、超时和 HTTP 失败如何被分类：

```bash
python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v
```

测试使用离线响应和本地模拟，不消耗模型额度。最短入口和安全入口都关闭自动重试；超时表示本地没有及时取得结果，不证明服务端没有处理请求。第一章先学会诊断，不增加自动反复请求。

## 5、效果展示与检查

完成后，你的终端应能展示三种结果：

| 操作 | 合格表现 | 能证明什么 |
| --- | --- | --- |
| 发送问候与自我介绍请求 | 得到回应任务的非空文本 | 配置与请求路径可用；离线运行则只证明替身路径 |
| 生成 `hello.cpp` 并编译运行 | 编译成功、输出正确、正常退出 | 这份源码通过这次本地验证 |
| 临时清空模型配置 | 明确配置诊断、退出码 `1` | 程序能在请求前拦住这个错误 |

记录自己的输入、实际回答、编译命令、输出和退出码；没有完成真实请求就写“未运行”。本章示意回答、配套固定源码和实际模型回答应分别标明，不能互相替代。

保留 `hello.cpp` 和练习目录。接下来会在这份程序里主动制造一个错误，观察仅仅“给模型一句任务”为什么还不够。

## 6、后续任务布置

1. 把问候改成“用两句话介绍你能怎样帮助我学习编程”，指出请求的哪个字段改变了，并检查实际回答是否符合两句话的要求。
2. 再请求生成 Hello World，先写好预期输出，再编译和运行；如果失败，保存真实报错，不替模型填一份成功结果。
3. 临时移除一项配置，记录失败发生在网络请求之前还是之后。
4. 复制一份已经能编译的 `hello.cpp`，删除输出语句末尾的分号。先自己编译一次，把报错留给下一章。

下一章[根据报错修正 Hello World](./python-file-read.md)，我们会从手工粘贴源码和报错开始，逐步让程序读取文件、日志和必要环境信息。

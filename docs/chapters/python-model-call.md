---
title: 用 Python 完成第一次模型调用
---

# 01 用 Python 完成第一次模型调用

当我们在 ChatGPT 等应用的聊天窗口里输入“你好”，很快就能得到一句回答。这一问一答间发生了什么呢？本章我们先用一个 Python 程序走一遍这个过程：发送 HTTP 请求，接收回复，再把终端变成一个可以连续提问的聊天窗口，最后完成一次真正的编码任务。

完成本章后，你应当能在终端中请求模型编写 C++ Hello World，并手动在本地保存、编译和运行它。这也是全书分工的第一次预演：模型写代码，你确定要求并亲自验证结果。Python 是第 01—03 章示例所用的语言，C++ 是这次任务的产物；本书关注的是这些程序如何协作，你不需要提前熟练掌握它们，跟做时遇到的必要语法会随代码解释。

::: info 本章设计
配套的 `hello.py` 和 `chat.py` 按下面的约定写成：

- 配置只从 `REIN_BASE_URL`、`REIN_API_KEY`、`REIN_MODEL` 三个环境变量读取；缺任何一项都不发请求。
- 每次请求等待完整回答（非流式），失败时不自动重试，只显示简短的错误类别。
- 密钥和服务返回的原始错误内容都不会打印出来。
- 对话记录只保存在内存中，只有成功的问答才进入记录。
:::

## 1、环境准备：Python 与环境变量

先准备运行程序的 Python，以及下载配套代码的 Git。下面按操作系统展开对应的一段即可；已经安装的工具可以直接检查版本。Windows 用户建议使用 WSL，这样后续命令就可以与 Linux 保持一致。

### 安装 Python 和 Git

::: details macOS
在“终端”应用中执行。如果还没有 Homebrew，先按 [Homebrew 官方安装说明](https://docs.brew.sh/Installation)安装：

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

安装过程会提示你确认操作，并可能要求安装 Apple 命令行开发工具。完成后，照终端末尾的 `Next steps` 提示执行配置命令，再运行 `brew --version`；能显示版本就可以继续。已有 Homebrew 的读者直接安装 Python 和 Git：

```bash
brew install python git
```

依赖稍后装进本项目的虚拟环境，不需要使用 `sudo pip` 修改系统 Python。[Homebrew 的 Python 使用说明](https://docs.brew.sh/Language-Runtimes-and-Packages)也采用这种做法。
:::

::: details Windows（通过 WSL）
先在管理员 PowerShell 中安装 WSL：

```powershell
wsl --install -d Ubuntu
```

按提示完成安装，必要时重启，然后从开始菜单打开 Ubuntu，设置 Linux 用户名和密码。看到 Ubuntu 的命令提示符后，切换到下面的 Ubuntu 步骤；后面的 `export`、`python3` 等命令都在 Ubuntu 中执行，不再放进 PowerShell。系统要求和安装问题可查 [微软 WSL 安装说明](https://learn.microsoft.com/zh-cn/windows/wsl/install)。
:::

::: details Ubuntu / Linux
在 Linux 终端或 WSL 的 Ubuntu 中执行。以下包管理命令以 Ubuntu 24.04 或更新版本为例，环境说明可对照 [Ubuntu 官方 Python 指南](https://ubuntu.com/developers/docs/howto/python-setup/)；其他发行版应使用自己的包管理器安装对应工具。

```bash
sudo apt update
sudo apt install -y python3 python3-venv python3-pip git
```
:::

不论选择哪条路径，最后都检查版本：

```bash
python3 --version
git --version
```

看到 Python 和 Git 的版本号，就说明终端能够找到它们。本章要求 Python 3.11 或更新版本；若显示更早版本，先升级 Python 或所用的 Ubuntu 版本，再继续。

### 下载代码，建立独立环境

在你准备存放练习项目的目录执行：

```bash
git clone https://github.com/JadeSnow7/Rein.git
cd Rein
```

已有仓库副本的读者直接进入它，不必重复克隆。接下来都从仓库根目录执行命令；这里能找到 `python/hello_world/` 和 `docs/` 两个目录。

```bash
python3 -m venv python/hello_world/.venv
python/hello_world/.venv/bin/python -m pip install -r python/hello_world/requirements.txt
python/hello_world/.venv/bin/python -c 'import openai; print(openai.__version__)'
```

最后一条输出 `2.26.0`，说明本书固定版本的 SDK 已经装好。SDK 是一组封装好的接口调用方法，它会替我们组织 HTTP 请求；本章仍能从代码中看见要发送给模型的内容。后续使用这条虚拟环境中的 Python 路径，不需要另做环境激活。

### 选择一家模型服务，申请 API Key

就像在聊天应用里需要登录一样，程序访问模型服务也需要凭据，这就是 API Key。除此之外，程序还需要知道请求发往哪里、使用哪个模型：

| 环境变量 | 填写内容 |
| --- | --- |
| `REIN_BASE_URL` | 服务商提供的 API 基础地址 |
| `REIN_API_KEY` | 在该服务商控制台创建的密钥 |
| `REIN_MODEL` | 该账户可用的模型标识，保持官方拼写 |

任选一家支持非流式 Chat Completions 接口的服务即可，不需要都注册。三项配置必须属于同一家服务：

| 服务 | 基础地址 | 查阅时官方示例中的模型标识 |
| --- | --- | --- |
| OpenAI | `https://api.openai.com/v1` | 以[模型文档](https://developers.openai.com/api/docs/models)中账户可用的为准 |
| DeepSeek | `https://api.deepseek.com` | `deepseek-flash` |
| MiMo（按量付费） | `https://api.xiaomimimo.com/v1` | `mimo-v2.6-pro` |

注册、开通计费和创建密钥的逐步说明见[阅读 0：申请模型服务](../readings/00.md#providers)，以上信息核对于 2026-09-23。模型目录和账户权限会变化，以你的控制台为准。密钥创建后妥善保存，不要复制进教程、截图或聊天记录。

### 把配置交给程序

在当前 macOS 或 Ubuntu 终端执行下面的命令。**把地址和模型占位文字替换成你刚取得的实际值**，地址只填写基础地址，不填写完整的 `/chat/completions` 路径。

```bash
export REIN_BASE_URL='替换为所选服务的基础地址'
export REIN_MODEL='替换为账户可用的模型标识'
export REIN_API_KEY="$(python3 -c 'import getpass; print(getpass.getpass("API key: "))')"
```

看到 `API key:` 后粘贴密钥并回车，输入不会显示出来，也不会把密钥明文放进命令历史。环境变量就是终端传给程序的配置；下面只检查是否填写，不显示密钥：

```bash
python3 - <<'PY'
import os
for name in ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL"):
    print(name, "已填写" if os.getenv(name, "").strip() else "未填写")
PY
```

三项都是“已填写”，表示程序能读到配置；能不能获得模型回答，还要在下一节实际请求后才能知道。API 请求会产生用量，不要把密钥写入源码、日志或截图。

新开终端后需要重新设置这些变量，本章程序不会自动读取 `.env`。若希望保留地址和模型，可以把这两条 `export` 放入所用 shell 的启动文件：zsh 通常是 `~/.zshrc`，交互式 bash 通常是 `~/.bashrc`；macOS 登录式 bash 还需由 `~/.bash_profile` 加载它。密钥仍可每次通过隐藏输入填写。第一遍跟做时先保持当前终端打开即可。

如果暂时没有 API Key，也可以先继续：下一节程序会打印本地问候，并明确告诉你尚未发送模型请求。HTTP 和环境变量的补充解释见[阅读 0](../readings/00.md#http)，不必先读完才开始。

## 2、发送请求：你好，请介绍一下自己

先写出下面这份完整程序，保存为 `python/hello_world/hello.py`。仓库里已有同名文件，你可以逐行对照阅读，也可以直接运行它：

<<< ../../python/hello_world/hello.py

从仓库根目录运行：

```bash
python/hello_world/.venv/bin/python python/hello_world/hello.py
```

程序先打印一条本地问候，再检查配置。假如只填写了基础地址，另外两项没有填写，会看到：

```text
你好！程序已启动。
API 尚未正确配置，本次没有发送模型请求。
缺少配置：REIN_API_KEY、REIN_MODEL
```

缺少哪项就列出哪项。这里的“你好”由 Python 程序打印，说明它已经运行到配置检查这一步；程序不会因此伪造一个模型回答。

配置完整时，程序才会创建 SDK 客户端并发送请求。正常结果可能如下，模型文字不要求逐字相同：

```text
你好！程序已启动。
模型：你好！我是一个 AI 助手，可以帮助你解释代码、排查错误和编写程序。
```

回到代码，先看从创建客户端到打印回答的几行：`messages` 是消息列表，`role: "user"` 表示用户发言，`content` 放入我们要问的话。服务返回 `response` 后，`choices[0].message.content` 取出第一个候选的回答文本。`stream=False` 表示等待完整回答；`max_retries=0` 表示失败时不自动重试。

```text
问候文字 → messages 中的 user.content → SDK 发送请求
        → 服务返回 response → 取出回答 → 终端显示“模型：……”
```

这份程序每次都从同一句问候开始，还不能连续聊天。程序能够运行，也可能在配置、请求或响应阶段遇到问题；我们先让问题以文字显示出来，第四节再集中认识这些提示。

## 3、一次完整任务：生成并运行 C++ Hello World

如果每问一句都要改源码中的提示词，用起来就很麻烦。我们先做一个命令行界面，也就是 CLI：在终端输入问题，等待模型回答，然后继续输入下一句。完整程序是 `python/hello_world/chat.py`：

<<< ../../python/hello_world/chat.py

这里新增的核心动作只有三个：`input()` 接收终端输入，`while` 循环让程序继续等待下一句，`messages` 保存本次会话中已经成功完成的问答。调用 SDK 的位置仍然写在这个文件里，可以与上一节直接对照。

从仓库根目录启动它：

```bash
python/hello_world/.venv/bin/python python/hello_world/chat.py
```

配置正确时会出现输入提示。下面是一段对话示意，其中“你：”后面的文字是你在程序中输入的内容，不是 shell 命令：

```text
终端对话已启动，输入 /exit 退出。
你：你好，请简单介绍一下自己。
模型：你好！我可以帮助你解释代码、编写程序和分析报错。
你：请把刚才的介绍缩短成一句话。
模型：我可以帮助你学习编程和解决代码问题。
```

第二次请求把前一次问答一起发送，模型才有机会理解“刚才”指什么。当前程序等待完整回答后再显示，不做逐字输出；输入空行不会发请求，输入 `/exit`、按 Ctrl+D 或 Ctrl+C 可以结束会话。退出后记录就清空，不会保存到磁盘。对话越长，下一次携带的内容也越多；本章用几轮简短对话练习即可。

### 小实验：模型只看得见 `messages`

模型本身不会记住上一次请求。它“记得”刚才的话，只是因为 `chat.py` 每次都把之前成功的问答放进 `messages` 一起发送。可以直接验证：先在 CLI 里说“我的练习文件叫 hello.cpp”，再问“我的练习文件叫什么？”，模型能答出来。然后输入 `/exit` 退出，重新启动 CLI，直接问同一个问题：这一次 `messages` 里只有新问题，模型无从知道答案。

程序每次发给模型的这组消息，就是模型这一次判断能依据的**上下文**。上下文由程序组装，模型看不见上下文之外的东西，包括你的电脑、文件和终端输出。这句话会贯穿后面所有章节。

### 在同一次对话中提出编码任务

保持 CLI 打开，输入下面这段任务，回车发送：

```text
请用 C++17 编写一个完整程序。正常运行时只输出 Hello, world!，末尾有一个换行，并返回 0。
```

如果回答里还带有讲解，可以接着输入：

```text
请只返回刚才程序的完整源码，不加解释和 Markdown 围栏。
```

这就是连续对话带来的变化：补充要求时不必修改 Python 程序，也不必重新描述整个任务。留意你刚才写的任务：语言标准、输出内容、退出码，它其实就是一份最小的设计，后面也要用它来检查结果。拿到源码后，还可以接着问 AI：“这段程序应该怎样运行？”提问时，把自己的操作系统和准备使用的文件名一起告诉它。例如，把下面的“macOS”替换为你的实际环境，再在同一次对话中输入：

```text
我使用 macOS，准备把刚才的程序保存为 hello.cpp。请给出从保存源码到运行程序的完整步骤：先创建一个独立的临时目录，并用 hello_workspace 变量保存目录路径；再说明如何保存源码、检查并按需安装 C++ 编译器、以 C++17 编译、运行程序，以及查看退出码。请分别列出终端命令和需要在编辑器中完成的操作，简要解释每条命令，并说明编译失败时应该停在哪一步。
```

这样，我们不仅让 AI 编写代码，也让它帮助补齐运行步骤。注意模型并不知道你的系统和已安装的工具：它只能根据你写进消息的“macOS”来回答，下一章会让程序主动把这类环境信息交给它。它列出的命令也不会自动执行。先核对回答中的系统、文件名和目录，再输入 `/exit` 回到 shell，按顺序操作。下面的步骤可以用来对照它的回答。首先创建独立的练习目录：

```bash
hello_workspace=$(mktemp -d)
printf '%s\n' "$hello_workspace"
```

终端会打印新目录的位置。在编辑器中打开这个目录，新建 `hello.cpp`，只粘贴模型回答里的 **C++ 源码**，不要包含“模型：”、解释文字或三个反引号。保留这个终端和目录，后两章还会用到；如果重新打开终端，要把 `hello_workspace` 设置为刚才记下的目录路径。

一种合格的源码如下。`#include` 引入输出能力，`main` 是入口，`std::cout` 打印文字，`return 0` 表示正常结束：

```cpp
#include <iostream>

int main() {
    std::cout << "Hello, world!\n";
    return 0;
}
```

没有 API 配置的读者，可以在刚创建的空目录中放入配套样本，继续练习编译：

```bash
cp python/hello_world/fixtures/hello.cpp "$hello_workspace/hello.cpp"
```

这个文件是固定的教学样本，不是本次模型响应；它与手工粘贴是两条替代路径，选择一种即可。

### 安装编译器，运行这份源码

Python 能运行不代表电脑已经能编译 C++。先在 shell 中检查：

```bash
c++ --version
```

如果找不到命令，macOS 在终端执行下面的命令，并在弹出的安装窗口完成 [Xcode Command Line Tools 安装](https://developer.apple.com/documentation/xcode/installing-the-command-line-tools/)：

```bash
xcode-select --install
```

Ubuntu 或 WSL 的 Ubuntu 则执行：

```bash
sudo apt update
sudo apt install -y build-essential
```

其他 Linux 发行版使用自己的开发工具包。安装后再次执行 `c++ --version`，看到编译器版本再继续。编译器没装好时，修改 C++ 源码不能解决这个问题。

确认目录中的源码就是准备运行的内容，然后编译：

```bash
c++ -std=c++17 "$hello_workspace/hello.cpp" -o "$hello_workspace/hello"
```

编译没有报错，才执行下一组命令：

```bash
"$hello_workspace/hello"
echo $?
```

第一条应输出 `Hello, world!` 并换行，第二条应输出 `0`。`$?` 是上一条命令的退出码，因此两条要紧接着执行。此时我们才完成“取得源码 → 保存文件 → 编译 → 运行”的全过程；如果编译报错，就先保留原文，不执行旧的产物。

## 4、分析失败，记录结果

前面的完整程序已经把常见失败转换为简短提示。本章先认识错误类型和对应文字；看到提示时，知道请求停在什么地方即可。

| 错误类型 | 返回信息示例 |
| --- | --- |
| 配置缺失 | API 尚未正确配置，本次没有发送模型请求。缺少配置：REIN_MODEL |
| 认证失败 | 认证失败：请检查 API Key |
| 连接失败 | 无法连接模型服务 |
| 请求超时 | 请求超时，未取得完整回答 |
| 额度或频率限制 | 请求受限：额度不足或请求过于频繁，请检查服务商控制台（HTTP 429） |
| 服务异常 | 服务请求失败：HTTP 503 |
| 响应不可用 | 未收到有效的回答文本 |

我们可以临时清空一项配置，观察一个确定的错误，无需等待网络碰巧失败：

```bash
REIN_MODEL='' python/hello_world/.venv/bin/python python/hello_world/hello.py
echo $?
```

假如另外两项配置已填写，程序会提示缺少 `REIN_MODEL`，最后显示退出码 `1`，本次不会发送请求。前缀赋值只影响这一次运行，不会清空终端原来的配置。对话程序遇到请求失败时会显示诊断，仍可继续输入；它不会把错误文字当作模型回答加入会话，也不会自动重试。

### 记录你实际得到的结果

完成后，应能分辨并如实记录下面几种结果：

| 你看到的 | 记录为 |
| --- | --- |
| “API 尚未正确配置……缺少配置：……” | 程序运行了，配置未完成，真实请求未运行 |
| 认证、连接、超时、限流等提示 | 真实请求失败，并写下错误类别 |
| 模型回答了源码 | 真实请求成功；代码是否可用尚未验证 |
| 本地编译通过，运行输出 `Hello, world!`，`echo $?` 为 `0` | 源码经过本地验证 |
| 使用配套样本编译运行成功 | 样本编译运行成功（不是本次模型的结果） |

模型返回了文字，还需要我们保存、编译并核对输出。不要用书中的示意回答代替你自己的实际结果。

## 5、后续任务布置

1. 启动 CLI，请模型介绍自己，再接着说“把刚才的介绍缩短成一句话”，观察是否能接续前文。
2. 在同一会话里提出 C++ Hello World 任务，再补充“只返回完整源码”。接着告诉 AI 你的操作系统，请它给出保存、编译和运行步骤；核对后逐步执行，记录输出和退出码。
3. 用第四节的命令临时清空模型配置，记录提示文字；重新正常运行，确认原来的终端配置仍在。
4. 复制一份已经能编译的 `hello.cpp`，删除输出语句末尾的分号。自己编译一次，把真实报错留给下一章。

## 本章边界卡

| 角色 | 本章结束时 |
| --- | --- |
| 模型 | 根据 `messages` 回答问题、写出源码和运行步骤 |
| 程序 | 读取配置，发送请求，显示回答或错误类别；保存本次会话的成功问答 |
| 用户 | 写下要求，保存源码，编译、运行并判断结果 |
| 还不能 | 让模型看到本地文件、编译报错和系统环境 |

下一章[让 AI 读取代码与日志](./python-file-read.md)，我们会从手工粘贴源码和报错开始，让模型通过受限工具请求本地资料，再提出有依据的修复候选。

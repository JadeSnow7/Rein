# Hello World 三章配套程序

本目录对应第一部分前三章。第一章先用两个可直接阅读的程序认识请求和对话：

- `hello.py`：直接使用 `openai==2.26.0` 发出一次真实请求；没有配置时会先打印本地问候和缺少的配置，不会访问网络。
- `chat.py`：在终端连续提问，成功回答才会加入当前会话；空输入跳过，输入 `/exit` 退出。
- `request.py`：把命令行任务发送给真实模型，并分类报告配置、超时和服务错误。
- `cli.py`：默认使用明确标注的离线替身，支持 `hello`、`generate`、`diagnose`、`edit` 和 `check`。
- `fixtures/hello.cpp` 与 `fixtures/broken.cpp`：正确和缺少分号的练习样本。

真实模式只从 `REIN_BASE_URL`、`REIN_API_KEY` 和 `REIN_MODEL` 读取配置；没有完整配置时不会创建网络客户端。离线模式只修复本目录约定的缺少分号样本，不代表通用自动修复能力。

从仓库根目录安装依赖：

```sh
python3 -m venv python/hello_world/.venv
python/hello_world/.venv/bin/python -m pip install -r python/hello_world/requirements.txt
```

真实请求前，在当前终端选择一个兼容 OpenAI SDK 的服务，依次填写地址、模型名和密钥。密钥用隐藏输入读取：

```sh
export REIN_BASE_URL="https://api.example.com/v1"
export REIN_MODEL="服务提供的模型名"
export REIN_API_KEY="$(python3 -c 'import getpass; print(getpass.getpass("API key: "))')"
```

没有配置时，两个真实模式入口都会显示 `API 尚未正确配置，本次没有发送模型请求。`，并以退出码 `1` 结束。示例使用当前终端临时环境变量；不要把 API Key 提交到仓库。真实调用依赖账户权限、额度、网络和服务商的模型名称。

配置完成后运行第一章入口：

```sh
python/hello_world/.venv/bin/python python/hello_world/hello.py
python/hello_world/.venv/bin/python python/hello_world/chat.py
```

一个完整的离线修复练习：

```sh
hello_workspace=$(mktemp -d)
WORK="$hello_workspace"
python/hello_world/.venv/bin/python python/hello_world/cli.py generate > "$WORK/hello.cpp"
python/hello_world/.venv/bin/python python/hello_world/cli.py check --workspace "$WORK"
cp python/hello_world/fixtures/broken.cpp "$WORK/hello.cpp"
python/hello_world/.venv/bin/python python/hello_world/cli.py check --workspace "$WORK"
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$WORK" --read-mode direct
python/hello_world/.venv/bin/python python/hello_world/cli.py diagnose --workspace "$WORK" --read-mode tool
python/hello_world/.venv/bin/python python/hello_world/cli.py edit --workspace "$WORK" --color never
```

如果已经安装目录内的固定依赖，也可以使用 `python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v` 运行全部测试。真实 SDK 连接需要显式配置三个 `REIN_*` 环境变量；没有配置时教程入口不会访问网络。

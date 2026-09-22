# Hello World 三章配套程序

本目录对应第一部分前三章：

- `hello.py`：直接使用 `openai==2.26.0` 发出一次真实请求。
- `request.py`：把命令行任务发送给真实模型，并分类报告配置、超时和服务错误。
- `cli.py`：默认使用明确标注的离线替身，支持 `hello`、`generate`、`diagnose`、`edit` 和 `check`。
- `fixtures/hello.cpp` 与 `fixtures/broken.cpp`：正确和缺少分号的练习样本。

真实模式只从 `REIN_BASE_URL`、`REIN_API_KEY` 和 `REIN_MODEL` 读取配置；没有完整配置时不会创建网络客户端。离线模式只修复本目录约定的缺少分号样本，不代表通用自动修复能力。

从仓库根目录运行：

```sh
python3 python/hello_world/cli.py hello
python3 python/hello_world/cli.py generate
python3 -m unittest discover -s python/hello_world/tests -v
```

一个完整的离线修复练习：

```sh
hello_workspace=$(mktemp -d)
WORK="$hello_workspace"
python3 python/hello_world/cli.py generate > "$WORK/hello.cpp"
python3 python/hello_world/cli.py check --workspace "$WORK"
cp python/hello_world/fixtures/broken.cpp "$WORK/hello.cpp"
python3 python/hello_world/cli.py check --workspace "$WORK"
python3 python/hello_world/cli.py diagnose --workspace "$WORK" --read-mode direct
python3 python/hello_world/cli.py diagnose --workspace "$WORK" --read-mode tool
python3 python/hello_world/cli.py edit --workspace "$WORK" --color never
```

如果已经安装目录内的固定依赖，也可以使用 `python/hello_world/.venv/bin/python -m unittest discover -s python/hello_world/tests -v` 运行全部测试。真实 SDK 连接需要显式配置三个 `REIN_*` 环境变量；没有配置时教程入口不会访问网络。

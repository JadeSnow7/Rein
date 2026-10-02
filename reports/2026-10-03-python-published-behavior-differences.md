# Python 配套程序：发布版与合并工作副本的行为差异

日期：2026-10-03

本文对比已发布 `origin/main`（`93fd720`）与本次合并前的本地工作副本（`6c34a5b`）。下列差异已经存在于工作副本，本次 A3 只保留它们；没有改动 `python/hello_world/` 的产品行为。

1. `bash` 工具移除 `cat compiler.log`。`python/hello_world/core.py:24-28,136-145` 的白名单现在只有 `pwd` 与 `ls -1`；日志由 `dispatch_tool` 的 `read_file` 分支读取（`core.py:320-326`）。测试在 `tests/test_behavior.py:205-215` 断言 `cat compiler.log` 被拒绝，在 `:224-229` 断言两次 `read_file` 的离线顺序。

2. 资料不齐时先提示补读，预算仍是六次。`core.py:354-393` 记录 `seen_evidence`，候选过早出现时追加补读提示并继续循环，最终仍以 `max_requests=6` 抛出 `request_budget`；`tests/test_behavior.py:289-307` 覆盖先候选、再补读后成功，`:177-194` 覆盖预算耗尽。

3. 拒绝先返回，不读取或比较当前源文件。`core.py:431-444` 的 `apply_candidate` 在 `accept` 为假时立即返回 `ApplyResult("rejected")`；`tests/test_behavior.py:244-253` 验证拒绝无副作用，随后才覆盖接受和源变更检查。

4. 接受围栏 JSON。`python/hello_world/model.py:41-43,120-127` 通过 `strip_code_fence` 去掉一层 Markdown 围栏再解析候选 JSON；`tests/test_behavior.py:309-320` 覆盖真实客户端响应被围栏包裹的情形。

5. Ctrl+C 退出码为 130。`python/hello_world/cli.py:124-126` 捕获 `KeyboardInterrupt` 并返回 `130`，由入口 `raise SystemExit(main())` 传给 shell。

这些是工作副本相对 `origin/main` 的五处行为差异；本说明随合并提交保存差异边界，不能作为新增运行时合同。

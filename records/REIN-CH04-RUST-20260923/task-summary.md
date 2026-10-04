# REIN-CH04-RUST-20260923：第 04 章 Rust 非协议部分迁移

日期：2026-09-23 ｜ 依据：[D18](../../DECISIONS.md)、第 04 章正文第 5–6 节的设计卡与验收样本。

## 范围

把 `python/hello_world/` 中不依赖模型协议的行为迁移到独立 crate `rust-hello-world/`：受限读取与三个只读工具、全文审查与 diff、确认后写入（复查、备份）、固定编译运行检查。模型调用留给第 05 章，候选来自 Python `diagnose` 的 `proposal.json`。

## 谁做了什么

| 部分 | 执行者 | 说明 |
| --- | --- | --- |
| 设计卡、验收样本 | Claude（Cowork 会话）据第一部分 Python 实现整理 | 属于“据设计反推”，**待作者审定** |
| Rust 实现、测试、对照脚本 | Claude（Cowork 会话，配置模型 claude-opus-5-5） | 按设计卡实现；并非用 Claude Code 或 Codex |
| 作者审查 | 未进行 | 合入前需作者审查 diff 并在 macOS 上复跑 |

第 04 章末尾的提示词没有原样交给代码助手运行，因此标为“据设计反推”，不标“已试用”。

## 环境

Linux 云端容器：rustc 1.95.0、cargo 1.95.0、c++ (Ubuntu 13.3.0)、Python 3.11.15。macOS 上尚未运行。

## 命令与结果

| 命令 | 结果 | 原始输出 |
| --- | --- | --- |
| `cargo test --manifest-path rust-hello-world/Cargo.toml` | 7 passed | [cargo-test.txt](cargo-test.txt) |
| `cargo clippy --all-targets` | 无警告 | — |
| `python3 rust-hello-world/compare_parity.py` | 25/25，退出码 0 | [parity-output.txt](parity-output.txt) |
| 反向实验（恢复旧的放弃顺序） | 24/25，报告 `stale source, reject` 不一致；还原后 25/25 | [mutation-check.txt](mutation-check.txt) |

同时，第一部分 Python 测试在本机 VM（Python 3.10 + openai 2.26.0）运行 42 个测试全部通过。

## 未验证项

- macOS 上的构建与对照结果。
- 真实模型模式（本轮没有 API 配置）。
- 新手按第 04 章独立完成的教学验收。

源文件摘要见 [source-hashes.txt](source-hashes.txt)。

# 设计决策失效检查器

`decisions.mjs` 是一个只读的 Node 内置库工具。它读取登记表和仓库文件，比较登记的 SHA-256 前提，并输出逐条、逐前提的机器结果。工具不会改写登记表、仓库文件或 Rein runtime 状态。

```bash
node scripts/architecture/decisions.mjs \
  --root . \
  --registry architecture/decisions.json
```

退出码为 0 只表示至少有一条 `accepted` 决定，且该决定的所有声明前提都匹配。文件变更是 `review_required`；缺失、不可读、非普通文件或没有前提是 `undetermined`。任意未知都不能成为通过。`proposed`、`superseded` 等人工状态会原样保留，机器检查不会自动把它们提升为 `accepted`。空登记表也不会通过。

登记表的每个决定包含 `id`、`title`、`status`、`rationale`、`scope`、`alternatives`、`prerequisites`、`reviewWhen` 和 `sources`。前提路径必须是仓库内的相对普通文件，不能经过符号链接；SHA-256 必须是 64 位小写十六进制摘要。登记表中的来源只用于追溯，不会被当作指令执行。

```bash
node --test scripts/architecture/decisions.test.mjs
```

`applicable` 只说明本次声明的文件前提仍与摘要一致，不证明架构决定本身正确。复审时应生成新版本登记并保留旧结论，而不是由检查器刷新历史摘要。

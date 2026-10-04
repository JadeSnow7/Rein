# 05 从 TypeScript 原型走向 Rust core

**状态：旧五篇历史正文，未作为现行 04 验收｜永久免费 · Apache-2.0**

上一章把服务商响应转换成统一工具请求。换语言时先保持这个约定：同一个调用ID、工具名、参数和错误不能因迁移而改变。我们先迁移数据表达，下一章才让它反复运行。

## 把对象变成拥有型数据

TypeScript的 `ToolCall` 包含 `id`、`name` 和 `arguments`。Rust同样需要这三个字段；`String` 拥有字符串内容，`serde_json::Value`保存JSON参数。结构定义位于 `rust/src/rein/mod.rs`，对应TS定义位于 `ts/src/rein/contracts.ts`。

```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
```

这是结构节选；实际定义还派生序列化与比较能力。拥有数据意味着结构离开创建函数之后，字符串仍然有效。借用 `&ToolCall` 则让检查函数读取同一份调用，不获得释放或移动它的权利。这里先掌握读取与拥有的差别，不提前引入异步生命周期。

## 成功和失败都必须保留

读取成功需要内容；读取失败需要错误代码和原因。`Result<T, E>`用不同分支表达两种结果。调用者通过 `match` 分别处理，不能为了拿到成功值而无条件 `unwrap()`。业务失败是可返回的数据，协议错误则可能要求停止整个调用过程。

`Option<String>`表达字段缺失。序列化时缺字段与显式null未必相同，迁移不能凭肉眼看起来一致就跳过检查。共享样本 `fixtures/cases/prerequisites.json`提供文本、工具调用、多个结果和非法响应，TS与Rust都读取它。

## 对照同一份样本

在仓库根目录执行以下检查。它们检查当前整合工程的既有类型与行为，尚不等于从04输出快照完成了本章迁移。

```bash
npm test --workspace ts -- tests/rein-prerequisites.test.ts
cargo test --locked --manifest-path rust/Cargo.toml --test prerequisites
```

核对调用ID、工具顺序、空文本处理、未知工具、参数和越界路径错误。测试通过说明这些样本没有发生行为漂移；不能推出所有服务商字段都兼容。

练习：给共享样本增加一个缺少工具名称的响应，分别说明两种实现在哪里拒绝它。再解释为什么把所有缺失字段补成空字符串会丢失信息。本章离开时应能用拥有型结构表达一轮请求，下一章再引入循环调用所需的async/await。

## 提示词示例

```text
请对照当前TS调用结构和共享样本迁移Rust数据类型。
保持调用ID、字段缺失和错误语义；先展示类型与序列化差异。
本步不引入循环调度或取消。用同一组合法与非法样本检查行为。
```

“字段缺失和错误语义”使迁移有具体比较对象；“不引入循环调度或取消”限制本次增量；“同一组样本”避免两份测试各自通过却互不对应。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。

现行六部分的 04 入口是 [`agent-capabilities`](./agent-capabilities.md)；本页保留旧五篇迁移正文，不能替代现行逐章验收。

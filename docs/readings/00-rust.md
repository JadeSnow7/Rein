---
prev:
  text: 阅读指南
  link: /about.html
next:
  text: 01 公共导读 · Rust 实现待补齐
  link: /chapters/01.html
---

<span id="阅读材料-0-·-rust-版"></span>

# 阅读 0 基础知识 · Rust 版

知识补充与可选示例已提供 · 永久免费

把示例里的 `text` 从一句话改成数字，程序会发生什么？

JSON 的语法没有坏，但程序需要的是字符串。Rust 编译器检查过源码，也不会提前知道这个输入里装着什么。我们就从这里出发，用一个本地解析程序，练习数据、检查和失败的传递。

先运行，再一点点拆开。看到所有权、枚举和 `.await` 挤在一起时，不用急着全部记住。沿着一份响应往下追，弄清数据交给了谁、错误从哪里回来，就有了阅读入口。

可选示例代码在 `rust/examples/reading-00/`，是独立 Cargo 项目，不读取密钥，也不发送请求。终端、HTTP、配置和 Git 见[公共导读](./00.md)。[TS 版](./00-ts.md)使用相同输入和自测要求，无需先学 TS 也能按需阅读本篇。

## 准备工程并运行 {#setup}

### 安装与检查

按照 [Rust 官方安装指南](https://doc.rust-lang.org/book/ch01-01-installation.html)安装稳定工具链；指南也说明各系统需要的链接器或构建工具。安装后重新打开终端，检查，

```bash
rustc --version
cargo --version
```

`rustc` 是编译器，Cargo 负责项目依赖、编译、运行与测试。本材料的配套项目在 Rust 1.98.0 上核验；如果已有工具链，先检查版本和项目能否编译，无需为练习修改其他工程的设置。

从仓库根目录进入练习，

```bash
cd rust/examples/reading-00
cargo run --locked
cargo test --locked
cargo check --locked
```

首次执行可能下载依赖并编译，之后运行主程序应打印，

```text
你好，Rein
```

测试应全部通过，检查命令应以退出码 0 结束。`cargo run` 编译并启动程序；`cargo test` 执行包含断言的测试；`cargo check` 检查代码而不生成用于正常运行的最终可执行文件。编译通过不等于外部数据正确，仍要使用测试检查行为。命令细节见 [cargo check](https://doc.rust-lang.org/cargo/commands/cargo-check.html) 与 [cargo run](https://doc.rust-lang.org/cargo/commands/cargo-run.html) 官方说明。

`--locked` 要求沿用已有 `Cargo.lock`，若构建需要更改锁文件，Cargo 会报错。依赖准备好后，可以给上述命令追加 `--offline`，禁止访问网络；首次使用离线模式却缺少缓存依赖时，会失败，这不是解析程序的错误。

### 认出项目里的文件

| 文件 | 作用 |
| --- | --- |
| `Cargo.toml` | 包信息与依赖声明 |
| `Cargo.lock` | 已解析的具体依赖版本 |
| `src/main.rs` | 数据结构、解析函数、入口与测试 |
| `.gitignore` | 忽略本项目生成的 `target/` |

本项目使用 Serde 和 `serde_json` 处理 JSON，用 Tokio 提供异步运行时。Cargo 配置直接取自配套文件，

<<< ../../rust/examples/reading-00/Cargo.toml

`features` 选择依赖提供的可选能力，Serde 的 `derive` 允许从结构体定义生成反序列化实现，Tokio 的 `macros` 与 `rt` 提供本篇用到的入口宏和运行时。根目录 `rust/` 尚未建立正式工程，所有运行命令都应在这个独立练习目录执行。使用它无需安装 Node.js。

## 结构体，把状态与正文放在一起 {#types}

以下片段取自 `src/main.rs`，先阅读定义，不要把它们重复添加到文件中，

```rust
#[derive(Debug)]
struct LocalResponse {
    status: u16,
    body: String,
}
```

`struct` 定义具有命名字段的数据。`u16` 是 16 位无符号整数，足够保存本练习的状态码；`String` 保存拥有自身存储空间的 UTF-8 文本。`#[derive(Debug)]` 是属性，让编译器生成调试格式化所需的实现，之后可用 `{:?}` 打印结构。

构造它时逐一填写字段，

```rust
let response = LocalResponse {
    status: 200,
    body: r#"{"text":"你好，Rein"}"#.to_owned(),
};
```

这是需要放在函数中的语法片段。`r#"..."#` 是原始字符串字面量，方便在里面写 JSON 的双引号；`.to_owned()` 从借用的字符串创建一个拥有数据的 `String`。原始字符串语法只影响 Rust 源码怎样表示文字，不会自动解析 JSON。

Rust 的 `let` 默认创建不可重新赋值的绑定。需要修改时使用 `let mut`，例如，

```rust
let mut text = String::from("你好");
text.push_str("，Rein");
println!("{text}");
```

`println!` 末尾的感叹号表示宏调用。这里的分号结束一条语句；后面函数结尾没有分号的表达式，则经常用来产生返回值。

### 所有权与借用，谁保存数据，谁暂时读取 {#ownership}

看函数签名之前，先分清 `String` 与 `&str`。前者拥有文本，后者是对一段有效 UTF-8 字符串的借用视图。可以把 `&String` 传给接收 `&str` 的函数，让它读取文字而无需复制。

下面是一个完整的小程序，仅演示语法，无需替换配套项目入口，

```rust
fn show(text: &str) {
    println!("{text}");
}

fn main() {
    let text = String::from("你好，Rein");
    show(&text);
    println!("{text}");
}
```

`show(&text)` 借用文本；调用后，`main` 仍然拥有 `text`，因此可以再次使用。相比之下，`let moved = text;` 会把这个 `String` 的所有权转给 `moved`。此后再使用 `text`，编译器会报告值已被移动。这里的赋值没有自动复制整段文本。所有权规则见 [Rust Book，所有权](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)。

配套代码中的两个签名体现了不同选择，

```rust
fn extract_text(response: &LocalResponse) -> Result<String, ReadingError>
async fn read_text(response: LocalResponse) -> Result<String, ReadingError>
```

上面仅展示签名，省略了函数体。`Result` 同时表达成功与失败，后面的[错误小节](#errors)会展开它，此时先关注参数有没有借用符号。`extract_text` 借用响应；`read_text` 接收并拥有响应，再把它借给内部函数。调用者把 `response` 交给 `read_text` 后，不能再次使用原来的非 `Copy` 值。

这是本练习的数据传递选择，并不意味着异步函数必须取得所有权。异步函数也可以借用，只要被借用的数据存活得足够久。以后将任务交给调度器独立执行时，再根据它的要求决定如何持有数据。

可变借用写作 `&mut T`，允许通过该引用修改值。对同一数据，同时有效的借用需要满足相应排他规则；这帮助编译器发现读写冲突。更完整的规则见 [Rust Book，引用与借用](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)。本例只有只读借用，无需先学习复杂生命周期标注。

**小练习**，阅读入口，设想在 `read_text(response).await` 之后再打印 `response.body`。这为什么不能直接通过检查？先说明数据被谁取得，再考虑是借用接口、调整使用顺序，还是确实需要复制。`.clone()` 适合需要独立副本的情况，不应成为每次编译失败后的默认处理。

## JSON，解析语法，再检查形状 {#json}

JSON 是文本格式，`Payload` 是程序想得到的结构，

```rust
#[derive(Debug, Deserialize)]
struct Payload {
    text: String,
}
```

`Deserialize` 来自 Serde，配合 `serde_json` 在运行时把 JSON 转为 Rust 数据。它并不意味着编译器能预先知道远程响应的内容。序列化反方向使用 `Serialize` 与 `serde_json::to_string`；本练习只需解析，因此没有给 `Payload` 添加不需要的派生。框架分工见 [Serde 文档](https://serde.rs/)。

解析函数按三个阶段工作，

1. 检查 HTTP 状态。500 直接返回 `Http(500)`，即使正文是坏 JSON 也不继续解析。
2. 用 `serde_json::from_str` 得到 `Value`。`Value` 表示通用 JSON 值；语法不完整的 `{` 在这里失败，归类为 `Json`。
3. 确认顶层是对象，再把它转为 `Payload`。字段缺失、类型不对或文本全为空白，归类为 `Shape`。

先解析通用值，让「不是合法 JSON」和「合法但不合要求」有明确分界。顶层数组、字符串、数字和 `null` 都不符合本练习的对象要求。Serde 派生默认允许未声明的额外字段；这里保留这个行为，也不删除合格文本两侧的空白。空白检查只用于拒绝全空白结果。

得到 `Payload` 后，`Ok(payload.text)` 将文本所有权交给调用者。不能直接返回借用这个局部 `Payload` 的字符串引用，因为局部数据会在函数结束时离开作用域，而返回值还要继续使用。

## 枚举、Option 与 Result {#errors}

Rust 用 `enum` 表达互斥的可能性。练习中的失败有三种，

```rust
#[derive(Debug, PartialEq)]
enum ReadingError {
    Http(u16),
    Json,
    Shape,
}
```

`Http` 附带状态码，另外两种不带额外数据。`PartialEq` 让测试可以比较这些错误值。使用 `match` 处理枚举时，必须覆盖所有可能性；也可以使用通配分支，但通配分支会把未来新增的情况一起接住。

标准库中的 `Option<T>` 同样是枚举，用 `Some(value)` 表示有值，用 `None` 表示没有值。下面是可以放进函数中的语法示意，

```rust
let usage: Option<u32> = None;
match usage {
    Some(tokens) => println!("tokens={tokens}"),
    None => println!("用量未回报"),
}
```

这里 `Some(0)` 与 `None` 含义不同，前者确实有一个 0，后者没有统计值。未来解析可缺省字段时，也需要保留这种差别。`if let Some(tokens) = usage` 可以只处理有值分支，但其余情况是否可以忽略，要由业务规则决定。

`Result<T, E>` 则表示成功或失败，`Ok(T)` 保存结果，`Err(E)` 保存错误。`Result<String, ReadingError>` 因此把正常文本与预期失败都放进函数返回类型中。

在返回 `Result` 的函数中，`?` 可以提取成功值，遇到错误则提前返回；必要时通过相应的错误转换机制转换类型。它没有重试或吞掉错误。练习先用 `map_err` 将解析库错误变成自己的类别，再用 `?` 向上传递。相关机制见 [Rust Book，使用 Result 处理可恢复错误](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)。

`Vec<T>` 是可增长的顺序集合，例如 `Vec<LocalResponse>` 可以保存多份响应。读取 `items[0]` 越界会 panic；`items.get(0)` 返回 `Option<&T>`，让你显式处理列表为空的情况。此时先知道「列表」和「可能缺失」怎样组合，后面再用它承载多轮消息。

## async、.await 与运行时 {#async}

`async fn` 调用产生 Future，函数体会在 Future 被轮询时推进；仅仅创建它，不等于里面的工作已经执行完毕。`.await` 等待它产生结果，当前 Future 暂时不能继续时，可以把执行机会交回运行时。

练习用下面的宏启动异步入口，

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 配套文件中的入口逻辑
}
```

这是入口形式示意。宏负责创建 Tokio 运行时并驱动异步代码；`current_thread` 选择单线程调度器，不需要为本地小练习启用多线程运行时。Tokio 的角色和用法见[官方教程](https://tokio.rs/tokio/tutorial)。

本例的 `read_text` 没有真正等待网络，保留异步形式是为了让调用位置与后续收发过程衔接。它不会因为加了 `async` 就把同步解析搬到另一个线程，也不会自动变快。

`#[tokio::test]` 同样让测试函数可以使用 `.await`。测试等待真实的异步入口，再比较它返回的 `Ok` 或 `Err`，而不是只检查「函数能调用」。

## 读懂完整源码与模块 {#source}

`src/main.rs` 的实现如下。首次阅读先沿着 `main → read_text → extract_text` 走一遍，再读底部测试。

::: details 展开配套源码（含测试）
<<< ../../rust/examples/reading-00/src/main.rs
:::

文件顶部的 `use` 把其他路径中的名字引入当前作用域；`impl ReadingError` 为错误类型定义方法，`&self` 表示方法借用这个错误。`kind()` 返回固定字符串字面量，所以签名使用 `&'static str`，这些字符串可以在整个程序运行期间保持有效。

`mod tests` 定义子模块，`use super::*` 引入父模块中的名称。`#[cfg(test)]` 使这个模块只在测试构建时参与编译。这里的实现项默认私有，子模块仍可以访问父模块中的私有项。以后拆为多文件或库接口时，再用 `mod` 声明模块、用 `pub` 明确公开哪些项；本练习不必先建立复杂目录。

入口按命令行参数选择本地场景，然后 `match` 调用结果，`Ok(text)` 打印正常文本；`Err(error)` 打印 `kind=...`，并用退出码 1 表示失败。`http/json/shape` 是供两版练习比较的分类，不是完整的正式错误协议。

## 可选自测与结果 {#exercises}

下面三组内容是可选自测。先写预期，再运行；需要比较另一种表达时，按编号进入 TS 版对应位置。

<span id="先运行现成失败场景"></span>

### 练习 01，运行正常与失败场景 {#exercise-01}

在 `rust/examples/reading-00/` 中逐条执行。每次运行后先查看退出码，再执行下一条。

```bash
cargo run --locked
cargo run --locked -- http
cargo run --locked -- json
cargo run --locked -- shape
```

正常场景打印 `你好，Rein`，退出码为 0。其余三次分别打印 `kind=http`、`kind=json` 和 `kind=shape`，退出码为 1。`--` 后的参数交给示例程序，Cargo 自身也可能打印编译或启动信息，注意分开看。

macOS / Linux shell 用 `echo $?`，PowerShell 用 `$LASTEXITCODE`，CMD 用 `echo %ERRORLEVEL%` 查看退出码。

打开入口，比较 `http` 和 `json` 场景的状态码与正文，沿解析顺序说明为什么一个报告 HTTP 失败，另一个报告 JSON 失败。两版内置的 `http` 场景正文不同，下一组练习会统一改成 `{`，再检查状态失败是否仍然优先。

<span id="修改正常场景的输入"></span>

### 练习 02，只改输入，预测结果 {#exercise-02}

打开 `main` 中默认分支 `_ => LocalResponse { ... }`，只修改其中的状态或正文，再执行不带场景参数的 `cargo run --locked`。每次验证表中的一行。

| 状态与正文 | 预期 |
| --- | --- |
| 500，`{` | `http` |
| 200，`{` | `json` |
| 200，`{"text":null}` | `shape` |
| 200，`{"text":"   "}` | `shape` |
| 200，`{"text":"  你好  ","extra":true}` | 返回原文，保留两端空格 |

正文用 `r#"..."#` 包住，再调用 `.to_owned()`。修改 JSON 内容时，保留 Rust 字符串的边界。测试另有自己的输入，入口数据改变后，测试仍可能通过；先确认各自检查的是什么。

观察后恢复正常输入。能解释为什么失败，也能解释为什么原文中的空格被保留，这组练习才算走完。

<span id="制造一次编译错误"></span>

### 练习 03，分清类型检查和行为测试 {#exercise-03}

把默认场景里的 `status: 200` 改成 `status: "200"`，运行 `cargo check --locked`。编译器应指出字符串不符合 `u16`。恢复为数字以后，再把 JSON 中的 `text` 改成 `null`，此时编译检查可以通过，运行程序却应报告 `shape`。

类型检查能看到 `body` 是一个 `String`，正文中的字段则要在运行时检查。把这个差别说明白，恢复正常输入，再运行测试和编译检查。

```bash
cargo test --locked
cargo check --locked
```

在 `mod tests` 中找一个成功断言和一个失败断言。`assert_eq!` 比较实际值与预期值；预期为 `Err(ReadingError::Shape)` 时，测试通过表示失败处理符合要求。读到 `.await` 时，也别跳过，它等待的是异步入口给出的实际结果。

记录一份输入、一条预期和一个检查结果。然后回到[所有权小练习](#ownership)，解释调用后为什么不能直接继续使用被移走的 `response`。这是 Rust 版额外需要掌握的一步。

## 和 TypeScript 版放在一起看 {#comparison}

回到开头那个数字 `text`。Rust 先把正文读成通用 JSON 值，再检查对象与字段；TS 让未知值经过实际判断后收窄。两边都需要对输入做检查，编译通过不能替代这一步。

| 同一个问题 | Rust 这份练习 | TypeScript 对应位置 |
| --- | --- | --- |
| 状态和正文怎么放在一起？ | `LocalResponse` 结构体拥有正文 | [对象类型与只读字段](./00-ts.md#types) |
| JSON 通过语法检查以后呢？ | `Value` 转成 `Payload`，再检查空白 | [`unknown` 与字段检查](./00-ts.md#json) |
| 失败怎样离开函数？ | `Result<String, ReadingError>`、`?` | [异常与错误类别](./00-ts.md#errors) |
| 调用方怎样等到结果？ | Future、`.await`，入口 `match` 结果 | [`Promise` 与 `await`](./00-ts.md#async) |
| 怎样认出代码的职责？ | 单个文件含入口、解析与测试模块 | [入口与解析模块分开](./00-ts.md#source) |

这份程序为了区分 `json` 与 `shape`，保留了先解析 `Value` 的阶段。它还把 `read_text` 的输入按值接收，再借给内部解析函数。阅读时可以追问每一步为什么这样写，无需把这个小程序当作所有 Rust 项目都该采用的模板。

两版共同的输入与结果见[示例输入与结果表](./00.md#acceptance)。当前 Rust 正式模型调用还未提供；需要补充时可打开[第 01 章公共导读](../chapters/01.md)，继续理解从本地响应到真实收发会增加哪些问题，无需先完成本页练习。

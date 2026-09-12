# Fixtures

`ts/` 与 `rust/` 两条 track 共用的验收输入与预期结果。现在冻结的是**目录布局与命名**，不是消息形状——消息形状属于合同，由第 04 章从正文推导，见 [contracts/](../contracts/README.md)。

## 布局

```
fixtures/
  workspaces/<case>/        被 Agent 读取的样例工作区（普通文件，原样提交）
  cases/<case>.json         一个验收用例：输入、期望结果、期望失败
  responses/<provider>/     录制的模型响应（已有两份）
```

## 命名

- `<case>` 用小写连字符，动词在前，说明被验收的行为：`read-existing-file`、`read-missing-file`、`invalid-args`。
- `cases/<case>.json` 与 `workspaces/<case>/` 同名配对；多个用例复用同一工作区时，在 case 文件里显式写出 `workspace` 字段。
- `responses/<provider>/` 下按 `<场景>-<序号>.json` 录制，provider 目录名取小写厂商标识。

## 录制响应

正文使用的真实请求与响应应将脱敏录制落盘到 `responses/`。当前已有 `opencode-go/hello-1.json`（200 文本回答）与 `openai/insufficient-quota-1.json`（429 额度不足），由 `ts/tests/recorded.test.ts` 离线回放。它们记录历史响应，不代表提供方当前的服务状态。

现有测试使用录制样本、本地桩、内存数据及回环 HTTP 服务，不访问外网；回环服务测试需要允许监听本地端口。

录制脚本替换已列出的敏感请求头、过滤响应头，并替换正文中的配置密钥字面量；它不识别所有私人内容。录制文件还需人工检查并去除其他敏感信息后再提交。

## 当前状态

`responses/` 已有第 01 章的两份录制。`cases/` 与 `workspaces/` 尚无工具验收用例，第一批计划随第 03 章的只读工具落地：读取成功、文件不存在、参数错误。

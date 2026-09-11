# Fixtures

`ts/` 与 `rust/` 两条 track 共用的验收输入与预期结果。现在冻结的是**目录布局与命名**，不是消息形状——消息形状属于合同，由第 04 章从正文推导，见 [contracts/](../contracts/README.md)。

## 布局

```
fixtures/
  workspaces/<case>/        被 Agent 读取的样例工作区（普通文件，原样提交）
  cases/<case>.json         一个验收用例：输入、期望结果、期望失败
  responses/<provider>/     计划存放的录制模型响应
```

## 命名

- `<case>` 用小写连字符，动词在前，说明被验收的行为：`read-existing-file`、`read-missing-file`、`invalid-args`。
- `cases/<case>.json` 与 `workspaces/<case>/` 同名配对；多个用例复用同一工作区时，在 case 文件里显式写出 `workspace` 字段。
- `responses/<provider>/` 下按 `<场景>-<序号>.json` 录制，provider 目录名取小写厂商标识。

## 录制响应

后续章节把真实请求与响应纳入正文时，应将脱敏录制落盘到 `responses/`。当前尚无真实录制样本；现有测试使用本地桩、内存数据及回环 HTTP 服务，不访问外网。

录制文件需去除密钥、账号标识与其他敏感字段后再提交。

## 当前状态

尚无用例。第一批随第 03 章的只读工具落地：读取成功、文件不存在、参数错误。

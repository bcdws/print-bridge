# TdPrint 发行版说明

基于 [PrintBridge](https://github.com/vergil-lai/print-bridge) 二次开发，定制逻辑放在独立文件，便于合并上游更新。

## 与上游的差异

| 项 | 位置 |
| --- | --- |
| 产品名、identifier、更新器公钥与端点 | `tauri*.conf.json` |
| 配置目录 `com.td.printbridge` | `crates/core/src/config.rs` |
| 内置网站白名单、固定访问令牌 | `tdprint_auth.rs`，调用点 `server.rs` |
| 禁用 Office 打印 | `tdprint_policy.rs`，调用点 `office.rs` |
| 日志限制、诊断日志导出 | `src-tauri/src/tdprint.rs`，调用点 `lib.rs` |
| 界面隐藏远程任务、命令行工具、GitHub 跳转、内置白名单及相关状态检测项 | `tdprint.ts`，调用点 `App.vue`、`StatusDoctorSheet.vue` |
| 不附带命令行工具，附带许可证文件 | `tauri.windows.conf.json` |
| 发版与试构建 | `release-windows.yml`、`build-windows.yml`、`scripts/tdprint-*.mjs`、`tauri.test-build.conf.json` |
| 中文 README | `README.md` |

## 内置网站白名单

`https://oms.tongdacdsc.com`、`https://pt.kuasuda.com` 始终放行，不读配置，用户无法删除。修改时同步改 `tdprint_auth.rs` 的 `BUILTIN_ORIGINS` 和 `tdprint.ts` 的 `TDPRINT_BUILTIN_ORIGINS`。

## 禁用 Office 打印

docx / xlsx / pptx 任务直接失败。恢复：`tdprint_policy.rs` 的 `OFFICE_PRINTING_ENABLED` 改为 `true`，并取消 `tdprint.ts` 对 `office.` 检查项的隐藏。

## 固定访问令牌

供局域网设备（如 PDA）和本机调试页面免配白名单。令牌正确且 Origin 为空或指向本机（`localhost`、`127.0.0.1`、`[::1]`）时跳过 IP 和 Origin 白名单；公网 Origin 仍按白名单处理。

传递方式：查询参数 `ws://<打印电脑IP>:17890/ws?token=<令牌>`，或请求头 `Authorization: Bearer <令牌>`。`print-bridge-sdk` 需注入自定义 WebSocket 来追加参数：

```js
class TokenWebSocket extends WebSocket {
  constructor(url, protocols) {
    super(`${url}?token=${encodeURIComponent(TOKEN)}`, protocols)
  }
}
const client = new PrintBridgeClient({ host: '192.168.1.20', WebSocket: TokenWebSocket })
```

令牌编译时由 `TDPRINT_AUTH_TOKEN` 注入（CI 取同名 Secret），未设置则令牌认证关闭；发版缺失会失败。更换后需重新发版，并同步更新 ERP 和 PDA。

## 日志

- 位置 `%LOCALAPPDATA%\com.td.printbridge\logs\TdPrint.log`，单文件 1 MB，保留 3 个归档。
- 任务页「下载日志」导出环境信息、白名单、最近任务和日志，不含远程 Token。

## 签名

- 更新签名：公钥在 `tauri.conf.json`，私钥与密码在 Secret `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。**私钥需备份，丢失后旧客户端无法自动更新。** 根目录 `printbridge.key.pub` 是上游的，与本发行版无关。
- Windows 代码签名未配置，安装时提示“未知发布者”。需要时购买证书，配置到 `tauri.windows.conf.json` 的 `bundle.windows`。

## 构建与发版

命令见 [AGENTS.md](../AGENTS.md#building)。

- 试构建 `build-windows.yml`：只出安装包（运行产物，保留 14 天），安装后会覆盖已装的 TdPrint。
- 发版 `release-windows.yml`：递增版本、打 `tdprint-v*` tag、发布 Release 和 `latest.json`，**现有客户端随即可以更新**。
- ERP「下载打印插件」固定链接：`https://github.com/bcdws/print-bridge/releases/latest/download/TdPrint_x64-setup.exe`

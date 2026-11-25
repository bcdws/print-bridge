# TdPrint 发行版说明

TdPrint 是基于 [PrintBridge](https://github.com/vergil-lai/print-bridge) 的二次开发版本。定制改动尽量集中在独立文件，上游文件只保留少量调用点，便于继续合并上游更新（`git fetch upstream && git merge upstream/main`）。

## 与上游的差异

| 项 | 位置 |
| --- | --- |
| 产品名 TdPrint、identifier、更新器公钥与端点 | `apps/desktop/src-tauri/tauri*.conf.json` |
| 配置目录 `com.td.printbridge` | `crates/core/src/config.rs` |
| 内置网站白名单、固定访问令牌（见下） | `crates/runtime/src/tdprint_auth.rs`，调用点在 `crates/runtime/src/server.rs` |
| 日志级别与大小限制、诊断日志导出（见下） | `apps/desktop/src-tauri/src/tdprint.rs`，调用点在 `src-tauri/src/lib.rs` |
| 界面：隐藏远程任务 Tab、关于页 GitHub / 更新日志按钮、内置白名单 | `apps/desktop/src/tdprint.ts`，调用点在 `App.vue`、`StatusDoctorSheet.vue` |
| Windows 发版流程 | `.github/workflows/release-windows.yml`、`scripts/tdprint-*.mjs` |

## 内置网站白名单

`https://oms.tongdacdsc.com`、`https://pt.kuasuda.com` 由插件后端**始终放行**，不读配置文件；界面上不显示，导入配置或手改 `config.json` 也删不掉。修改时 `tdprint_auth.rs` 的 `BUILTIN_ORIGINS` 与 `tdprint.ts` 的 `TDPRINT_BUILTIN_ORIGINS` 同步改。IP 白名单的 `127.0.0.1` 同样在界面隐藏（上游本就固定放行）。

## 固定访问令牌

上游只按 IP 白名单 + Origin 白名单放行：局域网设备（PDA 等 App 内嵌页面的 Origin 是 `http://localhost`）、本机调试页面（`http://localhost:端口`）都要逐台配置。TdPrint 额外内置一个固定令牌：

- 请求携带正确令牌，且 Origin **为空或指向本机**（`localhost`、`127.0.0.1`、`[::1]`，协议 http / https / capacitor）时，跳过 IP 白名单和 Origin 白名单；
- 公网网站即使拿到令牌，Origin 不是本机，仍然被拒绝；
- 不带令牌时行为与上游完全一致，网站白名单里的 Origin 照常可用。

携带方式（浏览器 WebSocket 不能自定义请求头，用查询参数）：

```text
ws://<打印电脑IP>:17890/ws?token=<令牌>
```

非浏览器客户端也可以用请求头 `Authorization: Bearer <令牌>`。

`print-bridge-sdk` 拼接地址时不带查询参数，可注入自定义 WebSocket 构造函数：

```js
import { PrintBridgeClient } from 'print-bridge-sdk'

const TOKEN = '<令牌>'
class TokenWebSocket extends WebSocket {
  constructor(url, protocols) {
    super(`${url}?token=${encodeURIComponent(TOKEN)}`, protocols)
  }
}

const client = new PrintBridgeClient({ host: '192.168.1.20', WebSocket: TokenWebSocket })
```

### 令牌来源

令牌在**编译期**通过环境变量 `TDPRINT_AUTH_TOKEN` 注入，不写进源码（本仓库公开）：

- CI 取自仓库 Secret `TDPRINT_AUTH_TOKEN`，缺失时发版流程直接失败；
- 本地构建不设置该变量时令牌认证关闭，行为与上游一致；本地调试令牌：`TDPRINT_AUTH_TOKEN=xxx pnpm --dir apps/desktop tauri dev`。

更换令牌：修改 Secret 后重新发版，客户端（ERP、PDA）同步更新令牌。

## 日志与诊断导出

- 日志写在 `%LOCALAPPDATA%\com.td.printbridge\logs\TdPrint.log`：INFO 级别（本项目模块 DEBUG），时间为本机时间；单文件 1 MB，另保留 3 个归档，合计不超过约 4 MB。
- 任务页「下载日志」导出一个 txt：运行环境（版本、默认打印机 / 纸张、白名单、令牌是否启用，不含远程 Token）、最近 300 条任务（前 100 条带状态明细）、最新 2 MB 应用日志。仓库人员把文件发给开发即可。

## 签名

- **更新签名（已配置）**：客户端“检查更新”用 minisign 公钥校验安装包，公钥在 `tauri.conf.json` 的 `plugins.updater.pubkey`，私钥与密码在仓库 Secret `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。根目录的 `printbridge.key.pub` 是上游的公钥，与本发行版无关。私钥丢失后旧版本客户端无法再自动更新，需妥善备份。
- **Windows 代码签名（未配置，可选）**：不签名时安装会提示“未知发布者”/ SmartScreen 警告，功能不受影响。需要消除时购买 OV / EV 代码签名证书（或 Azure Trusted Signing），在 `tauri.windows.conf.json` 配置 `bundle.windows.signCommand` 或证书指纹即可；自签名证书只对手动导入了根证书的电脑有效。

## 发版

只构建 Windows NSIS 安装包，手动触发：

```bash
gh workflow run release-windows.yml -f bump=patch -f notes="更新说明"
```

流程会递增版本号、提交并打 `tdprint-v*` tag、创建 GitHub Release，并生成客户端“检查更新”读取的 `latest.json`。

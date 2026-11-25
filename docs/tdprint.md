# TdPrint 发行版说明

TdPrint 是基于 [PrintBridge](https://github.com/vergil-lai/print-bridge) 的二次开发版本。定制改动尽量集中在独立文件，上游文件只保留少量调用点，便于继续合并上游更新（`git fetch upstream && git merge upstream/main`）。

## 与上游的差异

| 项 | 位置 |
| --- | --- |
| 产品名 TdPrint、identifier、更新器公钥与端点 | `apps/desktop/src-tauri/tauri*.conf.json` |
| 默认 Origin 白名单 `https://oms.tongdacdsc.com`、`https://pt.kuasuda.com` | `crates/core/src/config.rs` |
| 固定访问令牌（见下） | `crates/runtime/src/tdprint_auth.rs`，调用点在 `crates/runtime/src/server.rs` |
| Windows 发版流程 | `.github/workflows/release-windows.yml`、`scripts/tdprint-*.mjs` |

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

## 发版

只构建 Windows NSIS 安装包，手动触发：

```bash
gh workflow run release-windows.yml -f bump=patch -f notes="更新说明"
```

流程会递增版本号、提交并打 `tdprint-v*` tag、创建 GitHub Release，并生成客户端“检查更新”读取的 `latest.json`。

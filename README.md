# TdPrint

Windows 本地打印代理，让受信任的网页把打印任务静默发送到本机打印机，用于面单、标签等场景。基于 [PrintBridge](https://github.com/vergil-lai/print-bridge) 二次开发。

## 安装

下载最新版 [TdPrint_x64-setup.exe](https://github.com/bcdws/print-bridge/releases/latest/download/TdPrint_x64-setup.exe)（Windows x64）。安装包未做代码签名，出现 SmartScreen 提示属正常。

首次运行后在设置界面：

1. 选择默认打印机和纸张。
2. 把业务系统的 Origin（协议 + 域名 + 端口）加入网站白名单。内置的 ERP 域名无需添加。
3. 局域网内其他设备要连接时，把其 IP 或网段加入 IP 白名单。

## 接入

网页通过 [`print-bridge-sdk`](https://github.com/vergil-lai/print-bridge-jssdk) 连接本机 WebSocket 服务（默认端口 `17890`）：

```ts
import { PrintBridgeClient } from "print-bridge-sdk";

const client = new PrintBridgeClient();
client.on("status", (event) => console.log(event.jobId, event.status, event.message));

await client.connect();
await client.print({ type: "pdf", fileUrl: "https://example.com/label.pdf" });
```

- 支持 PDF、PNG/JPEG 图片、HTML 和原始打印指令（ESC/POS、TSPL、ZPL 等）；不支持 Office 文档。
- HTML 打印需要本机装有 Edge、Chrome 或 Chromium，且页面资源必须是公网地址。
- 任务成功只表示已进入系统打印队列，不代表已经出纸。
- 局域网设备（如 PDA）可用内置令牌免配白名单，见[固定访问令牌](docs/tdprint.md#固定访问令牌)。

不要把服务端口暴露到不可信网络。

## 文档

- [定制说明、构建与发版](docs/tdprint.md)
- [协议与 API](docs/printbridge-technical.md#websocket-api)（上游文档，部分功能 TdPrint 未开放）
- [开发约定与命令](AGENTS.md)

打印失败时，在任务页点「下载日志」，把文件发给开发。

## License

[Apache License 2.0](./LICENSE)。随包的 SumatraPDF 适用其自身许可证，见 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。

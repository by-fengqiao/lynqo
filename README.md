# LYNQO

LYNQO 是一个局域网文件传输工具：在你明确授权的设备之间发送和接收文件，不依赖公共云端存储。

## 开源许可证

Copyright (C) 2026 LYNQO contributors.

本项目以 [GNU General Public License v3.0](LICENSE)（`GPL-3.0-only`）发布。你可以运行、研究、修改和再发布本项目；发布修改版本或二进制版本时，须遵守 GPL-3.0 的源代码提供和同许可证要求。第三方依赖的许可证在 [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) 中单独列出。

## 本地运行

前置条件：Node.js 20+、Rust stable，以及对应平台的 Tauri 构建依赖。

```bash
npm install
npm run tauri dev
```

构建安装包：

```bash
npm run tauri build
```

验证前端和后端：

```bash
npm run build
cd src-tauri
cargo test
```

正常的桌面端和扫码后的局域网页端均不需要 `.env`、固定 IP、Token、数据库地址或运营主体信息。仅当你将前端部署到自定义 API 网关时，才可在 `.env` 中可选配置 `VITE_LYNQO_API_BASE_URL`；通常保持为空即可使用当前访问地址。

## 安全与隐私

- 安装包会展示 GPL-3.0 许可证；桌面端首次启动会要求阅读并确认使用协议、隐私说明和免责声明。
- 文件会在本机与已授权的局域网设备之间流转；请只在可信网络使用。
- 请自行核验文件来源、使用安全软件检查接收文件，并备份重要数据。

详细说明可在应用的“设置 → 开源许可与协议”查看。

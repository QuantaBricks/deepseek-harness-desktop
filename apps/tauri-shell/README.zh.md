![Quanta 标志](assets/quanta-logo.png)

[English](README.md) | 中文

欢迎加入我们的 Discord 社区：[加入 Discord](https://discord.com/invite/xJ562EPafb)。

# Tauri 桌面壳

此目录是刻意与 Harness Web UI 分离的桌面壳。
它不会导入 DeepSeek Harness 包，也不会修改 Harness 运行时。
二者唯一的约定是 HTTP 地址，默认值为 `http://127.0.0.1:3080`。

## 开发

在仓库根目录安装依赖并构建 Web UI 后，运行：

```sh
pnpm install
pnpm run build
pnpm --dir apps/tauri-shell exec tauri dev
```

Tauri 开发命令通过 `beforeDevCommand` 启动根工作区的 `pnpm dsh web`，并打开生成的本地服务。

## 外部服务模式

独立启动 Harness Web，再启动已安装的桌面壳。
桌面壳默认使用 `http://127.0.0.1:3080`；如需其他地址，请在启动前设置 `DSH_WEB_URL`。
这使安装包不依赖 Harness 运行时及其 Node.js 依赖。

## 更新

Windows 桌面壳会在启动时检查最新 GitHub Release，并在有可用更新时安装经过签名验证的版本。

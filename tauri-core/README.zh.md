![Quanta 标志](assets/quanta-logo.png)

[English](README.md) | 中文

欢迎加入我们的 Discord 社区：[加入 Discord](https://discord.com/invite/xJ562EPafb)。

# Tauri 桌面壳

此目录是刻意与 Harness Web UI 分离的桌面壳。
Release 安装包会把官方 Harness CLI 和私有 Node 运行时完整嵌入安装包，不再在安装后下载内核。
开发模式下二者通过 HTTP 地址连接，默认值为 `http://127.0.0.1:3080`。

## 开发

在仓库根目录初始化官方 Harness 子模块、安装其依赖并构建 Web UI 后，运行：

```sh
git submodule update --init --recursive
pnpm --dir deepseek-harness install
pnpm --dir deepseek-harness run build
pnpm --dir tauri-core exec tauri dev
```

Tauri 开发命令通过 `beforeDevCommand` 启动 `deepseek-harness`，并打开生成的本地服务。

## 外部服务模式

独立启动 Harness Web，再启动已安装的桌面壳。
桌面壳默认使用 `http://127.0.0.1:3080`；如需连接其他地址，请在启动前设置 `DSH_WEB_URL`。
这使安装包不依赖 Harness 运行时及其 Node.js 依赖。

## 更新

Windows 桌面壳仍可通过 Tauri updater 更新整个安装包；Harness 内核本身不再单独下载或替换。

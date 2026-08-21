# Agent Note: 定时上游同步发布桌面构建

Status: implemented

[English](2026-08-21-automated-upstream-desktop-sync.md) | 中文

## 问题

QuantaBricks 桌面发行版依赖 DeepSeek Harness Web，因此手动同步上游核心变更会延迟 Windows Release，并可能让桌面壳与其打开的服务脱节。

## 决策

`.github/workflows/sync-upstream.yml` 每天及手动触发时检查 `deepseek-ai/deepseek-harness` 的 `master`。上游有新提交时，工作流在其 checkout 中合并这些提交、安装依赖、构建 Harness Web、构建经过签名的 Windows Tauri 安装包、将合并结果推送到 `main`，并发布带唯一版本号的 GitHub Release。手动触发也可以发布当前 `main`，用于启用更新器。

工作流会在上游品牌变更与 QuantaBricks 根 README 文件冲突时保留 QuantaBricks 文件；其余合并冲突仍会停止同步。工作流只会在两个构建均成功后推送。其余合并冲突、依赖失败或安装包构建失败都会保持 `main` 与最新 Release 不变。Tauri 更新器会在应用启动时检查最新 Release，并且只接受由 GitHub Actions 中私钥签名的文件。

## Alternatives considered

**手动同步**要求操作者发现每次上游更新并重新执行完整构建流程，桌面 Release 无法及时更新。

**验证前推送合并结果**可能发布没有可用 Windows 安装包的核心版本。

**创建同步 Pull Request**保留了评审，但无法满足此发行版所要求的自动核心更新与 Release。

## Consequences

可干净合并的 Harness 核心变更无需本地构建即可生成 Windows Release。已安装的桌面壳会在启动时下载下一版经过签名的 Release。上游冲突仍需维护者解决，定时构建会消耗 GitHub Actions 额度。

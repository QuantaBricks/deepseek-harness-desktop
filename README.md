<p align="center">
  <img src="tauri-core/assets/quanta-logo.png" alt="QuantaBricks" width="150">
</p>

# DeepSeek Harness Desktop

QuantaBricks provides a Windows desktop shell and future chemistry-simulation extensions around [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness). The Harness kernel remains official upstream code and is not modified by this repository.

Welcome to our Discord channel: [Join Discord](https://discord.com/invite/xJ562EPafb).

## Layout

- `deepseek-harness/` — the official DeepSeek Harness repository, pinned as a Git submodule.
- `tauri-core/` — the independently buildable Tauri desktop shell. It communicates with Harness only over HTTP.
- `extension/` — QuantaBricks extensions, including planned chemistry-simulation adaptations.

## Get started

Clone with the kernel included:

```sh
git clone --recurse-submodules https://github.com/QuantaBricks/deepseek-harness-desktop.git
```

For an existing clone:

```sh
git submodule update --init --recursive
```

Build the Harness Web UI and start the desktop shell:

```sh
pnpm --dir deepseek-harness install
pnpm --dir deepseek-harness run build
pnpm --dir tauri-core exec tauri dev
```

The shell starts Harness Web on `http://127.0.0.1:3080` during development. The packaged Web UI comes from `deepseek-harness/apps/web/dist`; in installed/external-service mode, start Harness Web independently and then open the desktop application.

## Upstream updates and releases

GitHub Actions checks the official Harness `master` branch every day. When it changes, the workflow advances only the `deepseek-harness` submodule pointer, builds the Web UI and signed Windows installer, then publishes a GitHub Release. No manual merge of the Harness source is required.

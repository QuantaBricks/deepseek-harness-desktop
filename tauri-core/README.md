![Quanta logo](assets/quanta-logo.png)

English | [中文](README.zh.md)

Welcome to our Discord channel: [Join Discord](https://discord.com/invite/xJ562EPafb).

# Tauri shell

This directory is an intentionally separate desktop shell for the Harness Web UI.
It does not import DeepSeek Harness packages or modify the Harness runtime. The
only contract is an HTTP URL, defaulting to `http://127.0.0.1:3080`.

## Development

From the repository root, initialize the official Harness submodule, install its dependencies, and build the Web UI:

```sh
git submodule update --init --recursive
pnpm --dir deepseek-harness install
pnpm --dir deepseek-harness run build
pnpm --dir tauri-core exec tauri dev
```

The Tauri dev command starts `deepseek-harness` through `beforeDevCommand` and opens the resulting local service.

## External service mode

Start Harness Web independently, then launch the installed shell. The shell
uses `http://127.0.0.1:3080` by default; set `DSH_WEB_URL` before launching it
to use another address. This keeps the installer independent of the Harness
runtime and its Node.js dependencies.

## Updates

The Windows shell checks the latest GitHub Release at startup and installs a signed update when one is available.

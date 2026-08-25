![Quanta logo](assets/quanta-logo.png)

English | [中文](README.zh.md)

Welcome to our Discord channel: [Join Discord](https://discord.com/invite/xJ562EPafb).

# Tauri shell

This directory is an intentionally separate desktop shell for the Harness Web UI.
Release installers embed the official Harness CLI and private Node runtime, so
the installed application does not download a separate core. In development,
the only contract is an HTTP URL, defaulting to `http://127.0.0.1:3080`.

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
to use another address.

## Updates

The Windows shell can update the complete installer through Tauri updater. The
Harness core is not downloaded or replaced separately.

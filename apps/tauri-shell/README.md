![Quanta logo](assets/quanta-logo.png)

# Tauri shell

This directory is an intentionally separate desktop shell for the Harness Web UI.
It does not import DeepSeek Harness packages or modify the Harness runtime. The
only contract is an HTTP URL, defaulting to `http://127.0.0.1:3080`.

## Development

From the repository root, after installing dependencies and building the Web UI:

```sh
pnpm install
pnpm run build
pnpm --dir apps/tauri-shell exec tauri dev
```

The Tauri dev command starts the root workspace's `pnpm dsh web` through `beforeDevCommand` and opens
the resulting local service.

## External service mode

Start Harness Web independently, then launch the installed shell. The shell
uses `http://127.0.0.1:3080` by default; set `DSH_WEB_URL` before launching it
to use another address. This keeps the installer independent of the Harness
runtime and its Node.js dependencies.

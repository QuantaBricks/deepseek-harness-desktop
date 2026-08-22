#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::Duration,
};
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

const DEFAULT_URL: &str = "http://127.0.0.1:3080";

struct HarnessProcess(Mutex<Option<Child>>);

fn web_url() -> String {
    env::var("DSH_WEB_URL").unwrap_or_else(|_| DEFAULT_URL.to_string())
}

fn wait_for_server(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let Some(port) = parsed.port_or_known_default() else {
        return false;
    };
    for _ in 0..120 {
        if TcpStream::connect((host, port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

fn start_embedded_harness(
    app: &tauri::AppHandle,
) -> Result<Option<Child>, Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) || env::var_os("DSH_WEB_URL").is_some() {
        return Ok(None);
    }

    let resource_dir = app.path().resource_dir()?;
    let harness_dir = resource_dir.join("h");
    let node = resource_dir.join("n.exe");
    // `pnpm deploy --filter @deepseek-ai/dsh` makes the CLI package itself
    // the deployment root, so its entry point is `h/lib/bin.js`.
    let cli = harness_dir.join("lib").join("bin.js");
    if !node.is_file() || !cli.is_file() {
        return Err(format!(
            "embedded Harness runtime is incomplete: node={} cli={}",
            node.display(),
            cli.display()
        )
        .into());
    }

    let child = Command::new(node)
        .arg(cli)
        .args(["web", "--host", "127.0.0.1", "--port", "3080", "--no-open"])
        .current_dir(harness_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(Some(child))
}

fn install_update(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = app.updater() else { return };
        let Ok(Some(update)) = updater.check().await else {
            return;
        };
        let _ = update.download_and_install(|_, _| {}, || {}).await;
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let url = web_url();
            let harness = start_embedded_harness(app.handle())?;
            app.manage(HarnessProcess(Mutex::new(harness)));
            let window = app
                .get_webview_window("main")
                .ok_or("main window was not created by Tauri configuration")?;
            if wait_for_server(&url) {
                let parsed = url
                    .parse()
                    .map_err(|error| format!("invalid DSH_WEB_URL: {error}"))?;
                window.navigate(parsed)?;
            }
            install_update(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<HarnessProcess>() {
                    if let Ok(mut child) = state.0.lock() {
                        if let Some(child) = child.as_mut() {
                            let _ = child.kill();
                        }
                    }
                }
            }
        });
}

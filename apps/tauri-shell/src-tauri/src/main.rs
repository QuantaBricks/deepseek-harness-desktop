#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, net::TcpStream, thread, time::Duration};
use tauri::Manager;

const DEFAULT_URL: &str = "http://127.0.0.1:3080";

fn web_url() -> String {
  env::var("DSH_WEB_URL").unwrap_or_else(|_| DEFAULT_URL.to_string())
}

fn wait_for_server(url: &str) -> bool {
  let Ok(parsed) = url::Url::parse(url) else { return false };
  let Some(host) = parsed.host_str() else { return false };
  let Some(port) = parsed.port_or_known_default() else { return false };
  for _ in 0..30 {
    if TcpStream::connect((host, port)).is_ok() { return true; }
    thread::sleep(Duration::from_millis(100));
  }
  false
}

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      let url = web_url();
      let window = app
        .get_webview_window("main")
        .ok_or("main window was not created by Tauri configuration")?;
      if wait_for_server(&url) {
        let parsed = url.parse().map_err(|error| format!("invalid DSH_WEB_URL: {error}"))?;
        window.navigate(parsed)?;
      }
      Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while building Tauri application")
    .run(|_, _| {});
}

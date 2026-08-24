#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
  env, fs,
  fs::File,
  io::{Read, Write},
  net::TcpStream,
  path::PathBuf,
  process::{Child, Command, Stdio},
  sync::Mutex,
  thread,
  time::Duration,
};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const URL: &str = "http://127.0.0.1:3080";
const MANIFEST: &str = "https://github.com/QuantaBricks/deepseek-harness-desktop/releases/latest/download/core-manifest.json";

struct Core(Mutex<Option<Child>>);

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
  version: String,
  url: String,
  sha256: String,
}

fn core(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
  Ok(app.path().app_local_data_dir()?.join("harness-core"))
}

fn update_core(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  let core = core(app)?;
  let parent = core.parent().ok_or("missing core parent")?;
  let stage = parent.join("harness-core.next");
  let old = parent.join("harness-core.previous");
  let manifest = env::var("DSH_CORE_MANIFEST_URL").unwrap_or_else(|_| MANIFEST.into());
  let remote: Manifest = reqwest::blocking::get(manifest)?.error_for_status()?.json()?;
  if core.join("core-manifest.json").is_file() {
    let local: Manifest = serde_json::from_str(&fs::read_to_string(core.join("core-manifest.json"))?)?;
    if local.version == remote.version { return Ok(()) }
  }
  if stage.exists() { fs::remove_dir_all(&stage)?; }
  fs::create_dir_all(&stage)?;
  let archive = stage.join("core.zip.download");
  let mut response = reqwest::blocking::get(&remote.url)?.error_for_status()?;
  let mut output = File::create(&archive)?;
  response.copy_to(&mut output)?;
  output.flush()?;
  let mut file = File::open(&archive)?;
  let mut hasher = Sha256::new();
  let mut buffer = [0u8; 1024 * 1024];
  loop { let n = file.read(&mut buffer)?; if n == 0 { break } hasher.update(&buffer[..n]); }
  let actual = format!("{:x}", hasher.finalize());
  if actual != remote.sha256.to_lowercase() { return Err("core checksum mismatch".into()) }
  let mut zip = zip::ZipArchive::new(File::open(&archive)?)?;
  for index in 0..zip.len() {
    let mut entry = zip.by_index(index)?;
    let Some(relative) = entry.enclosed_name().map(|p| p.to_owned()) else { continue };
    let destination = stage.join(relative);
    if entry.is_dir() { fs::create_dir_all(&destination)?; continue }
    if let Some(parent) = destination.parent() { fs::create_dir_all(parent)?; }
    let mut target = File::create(destination)?;
    std::io::copy(&mut entry, &mut target)?;
  }
  fs::remove_file(&archive)?;
  let links: Vec<Link> = serde_json::from_str(&fs::read_to_string(stage.join("links.json"))?)?;
  for link in links {
    let path = stage.join(&link.path);
    let target = path.parent().ok_or("invalid link parent")?.join(&link.target);
    if path.exists() { continue }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    junction::create(&target, &path)?;
  }
  fs::write(stage.join("core-manifest.json"), serde_json::to_vec_pretty(&remote)?)?;
  if !stage.join("node.exe").is_file() || !stage.join("harness/lib/bin.js").is_file() { return Err("incomplete core".into()) }
  if old.exists() { fs::remove_dir_all(&old)?; }
  if core.exists() { fs::rename(&core, &old)?; }
  fs::rename(stage, core)?;
  Ok(())
}

#[derive(Debug, Deserialize)]
struct Link { path: String, target: String }

fn launch_core(app: &AppHandle) -> Result<Child, Box<dyn std::error::Error>> {
  let core = core(app)?;
  let node = core.join("node.exe");
  let cli = core.join("harness/lib/bin.js");
  if !node.is_file() || !cli.is_file() { return Err("Harness core is not installed".into()) }
  Ok(Command::new(node)
    .arg(cli)
    .args(["web", "--host", "127.0.0.1", "--port", "3080", "--no-open"])
    .current_dir(core.join("harness"))
    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
    .spawn()?)
}

fn navigate_when_ready(app: &AppHandle) -> Result<(), String> {
  let window = app.get_webview_window("main").ok_or("missing main window")?;
  for _ in 0..120 {
    if TcpStream::connect("127.0.0.1:3080").is_ok() {
      window.navigate(URL.parse::<tauri::Url>().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
      return Ok(());
    }
    thread::sleep(Duration::from_millis(250));
  }
  Err("Harness started but did not become ready in 30 seconds".into())
}

#[tauri::command]
fn install_or_update_core(app: AppHandle, state: State<'_, Core>) -> Result<(), String> {
  {
    let mut child = state.0.lock().map_err(|_| "core process lock failed")?;
    if let Some(process) = child.as_mut() {
      let _ = process.kill();
      let _ = process.wait();
      *child = None;
    }
  }
  update_core(&app).map_err(|error| error.to_string())?;
  let mut child = state.0.lock().map_err(|_| "core process lock failed")?;
  if child.is_none() { *child = Some(launch_core(&app).map_err(|error| error.to_string())?); }
  navigate_when_ready(&app)
}

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      let handle = app.handle();
      if let Ok(web_url) = env::var("DSH_WEB_URL") {
        let window = app.get_webview_window("main").ok_or("missing main window")?;
        window.navigate(web_url.parse::<tauri::Url>().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
        app.manage(Core(Mutex::new(None)));
        return Ok(());
      }
      let mut child = None;
      let core_ready = core(&handle)
        .map(|path| path.join("node.exe").is_file() && path.join("harness").join("lib").join("bin.js").is_file())
        .unwrap_or(false);
      // Never block an installed core on a remote manifest request. The
      // existing core starts immediately; updates are downloaded separately.
      if !cfg!(debug_assertions) && core_ready {
        if let Ok(process) = launch_core(&handle) { child = Some(process); }
      }
      app.manage(Core(Mutex::new(child)));
      if app.state::<Core>().0.lock().ok().and_then(|process| process.as_ref().map(|_| ())).is_some() {
        navigate_when_ready(&handle).map_err(Into::into)
      } else {
        Ok(())
      }
    })
    .invoke_handler(tauri::generate_handler![install_or_update_core])
    .build(tauri::generate_context!())
    .expect("build")
    .run(|_, _| {});
}

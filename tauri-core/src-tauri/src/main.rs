#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
  env,
  net::TcpStream,
  path::PathBuf,
  process::{Child, Command, Stdio},
  sync::Mutex,
  thread,
  time::Duration,
};
use tauri::{AppHandle, Manager, State};

const URL: &str = "http://127.0.0.1:3080";
const MANIFEST: &str = "https://github.com/QuantaBricks/deepseek-harness-desktop/releases/latest/download/core-manifest.json";

struct Core(Mutex<Option<Child>>);

fn core(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
  Ok(app.path().app_local_data_dir()?.join("harness-core"))
}

fn update_core(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  let core = core(app)?;
  let parent = core.parent().ok_or("missing core parent")?;
  let stage = parent.join("harness-core.next");
  let old = parent.join("harness-core.previous");
  let manifest = env::var("DSH_CORE_MANIFEST_URL").unwrap_or_else(|_| MANIFEST.into());
  let quote = |value: String| value.replace('\'', "''");
  let script = format!(
    "$ErrorActionPreference='Stop';$u='{}';$c='{}';$s='{}';$o='{}';$m=Invoke-RestMethod $u;$l=Join-Path $c 'core-manifest.json';if((Test-Path $l)-and((Get-Content $l -Raw|ConvertFrom-Json).version -eq $m.version)){{exit 0}};Remove-Item -LiteralPath $s -Recurse -Force -ErrorAction SilentlyContinue;New-Item -ItemType Directory -Path $s|Out-Null;$z=Join-Path $s 'core.zip';Invoke-WebRequest $m.url -OutFile $z;if((Get-FileHash $z -Algorithm SHA256).Hash.ToLower() -ne $m.sha256){{throw 'checksum mismatch'}};Add-Type -AssemblyName System.IO.Compression.FileSystem;[IO.Compression.ZipFile]::ExtractToDirectory($z,$s,$true);Remove-Item -LiteralPath $z -Force;$links=Get-Content -LiteralPath (Join-Path $s 'links.json') -Raw|ConvertFrom-Json;foreach($x in $links){{$p=Join-Path $s $x.path;$t=Join-Path (Split-Path $p -Parent) $x.target;New-Item -ItemType Junction -Path $p -Target $t|Out-Null}};if(!(Test-Path -LiteralPath (Join-Path $s 'node.exe')) -or !(Test-Path -LiteralPath (Join-Path $s 'harness\\lib\\bin.js'))){{throw 'incomplete core'}};$m|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $s 'core-manifest.json') -Encoding utf8;Remove-Item -LiteralPath $o -Recurse -Force -ErrorAction SilentlyContinue;if(Test-Path -LiteralPath $c){{Move-Item -LiteralPath $c -Destination $o}};Move-Item -LiteralPath $s -Destination $c",
    quote(manifest), quote(core.display().to_string()), quote(stage.display().to_string()), quote(old.display().to_string())
  );
  if Command::new("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script]).status()?.success() {
    Ok(())
  } else {
    Err("core update failed".into())
  }
}

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

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use std::{
    env, fs,
    fs::File,
    net::TcpStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::Duration,
};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Manager};

const URL: &str = "http://127.0.0.1:3080";

struct Core {
    _process: Mutex<Option<Child>>,
}

#[derive(Debug, Deserialize)]
struct Link {
    path: String,
    target: String,
}

fn embedded_core(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parent = app.path().app_local_data_dir()?;
    let core = parent.join("harness-core");
    if core.join("node.exe").is_file() && core.join("harness/lib/bin.js").is_file() {
        return Ok(core);
    }

    let archive = app.path().resource_dir()?.join("r/harness-core.zip");
    let stage = parent.join("harness-core.next");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        // ZIPs produced by Windows tooling may store separators as `\\`.
        // Normalize them before applying the traversal-safe path check.
        let raw_name = entry.name().replace('\\', "/");
        let relative = PathBuf::from(raw_name);
        if relative.is_absolute() || relative.components().any(|component| {
            matches!(component, std::path::Component::ParentDir)
        }) {
            continue;
        }
        let destination = stage.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
    }

    if !stage.join("node.exe").is_file() || !stage.join("harness/lib/bin.js").is_file() {
        return Err("embedded Harness archive is incomplete".into());
    }
    if core.exists() {
        fs::remove_dir_all(&core)?;
    }
    // Junction targets are absolute on Windows. Rename the extracted tree
    // before creating them, otherwise they would point at harness-core.next.
    fs::rename(&stage, &core)?;
    let links_text = fs::read_to_string(core.join("links.json"))?;
    let links: Vec<Link> = serde_json::from_str(links_text.trim_start_matches('\u{feff}'))?;
    for link in links {
        let path = core.join(&link.path);
        let target = path
            .parent()
            .ok_or("invalid embedded link")?
            .join(&link.target);
        if path.exists() || fs::symlink_metadata(&path).is_ok() {
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    if let Err(error) = junction::create(&target, &path) {
      let _ = fs::write(
        parent.join("embedded-core-error.txt"),
        format!("path={}\\ntarget={}\\nerror={}\\n", path.display(), target.display(), error),
      );
      return Err(error.into());
    }
    }
    Ok(core)
}

fn launch_core(app: &AppHandle) -> Result<Child, Box<dyn std::error::Error>> {
    let core = embedded_core(app)?;
    let node = core.join("node.exe");
    let cli = core.join("harness/lib/bin.js");
    if !node.is_file() || !cli.is_file() {
        return Err("embedded Harness runtime is missing from the installer".into());
    }
    let mut command = Command::new(node);
    command
        .arg(cli)
        .args(["web", "--host", "127.0.0.1", "--port", "3080", "--no-open"])
        .current_dir(core.join("harness"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    Ok(command.spawn()?)
}

fn navigate_when_ready(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("missing main window")?;
    for _ in 0..120 {
        if TcpStream::connect("127.0.0.1:3080").is_ok() {
            window
                .navigate(
                    URL.parse::<tauri::Url>()
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err("embedded Harness started but did not become ready in 30 seconds".into())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            if let Ok(web_url) = env::var("DSH_WEB_URL") {
                let window = app
                    .get_webview_window("main")
                    .ok_or("missing main window")?;
                window
                    .navigate(
                        web_url
                            .parse::<tauri::Url>()
                            .map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
                app.manage(Core {
                    _process: Mutex::new(None),
                });
                return Ok(());
            }

            // Development runs can point at a separately managed Harness Web.
            // Release builds always launch the runtime embedded in the installer.
            if cfg!(debug_assertions) {
                app.manage(Core {
                    _process: Mutex::new(None),
                });
                return Ok(());
            }

            let child = match launch_core(&handle) {
                Ok(child) => child,
                Err(error) => {
                    if let Ok(path) = handle.path().app_local_data_dir() {
                        let _ = fs::write(path.join("embedded-core-error.txt"), error.to_string());
                    }
                    return Err(error.to_string().into());
                }
            };
            app.manage(Core {
                _process: Mutex::new(Some(child)),
            });
            navigate_when_ready(&handle).map_err(Into::into)
        })
        .build(tauri::generate_context!())
        .expect("build")
        .run(|_, _| {});
}

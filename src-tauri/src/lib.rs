mod browser;
mod config;
mod deployment;
mod files;
mod image_preview;
mod keys;
mod listing;
mod markdown;
mod orphan_ssh;
mod platform;
mod runtime;
use config::Config;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{Manager, State};
struct AppState {
    data: PathBuf,
    config: Mutex<Config>,
    runtime: tokio::sync::Mutex<runtime::Runtime>,
    pending_host: Mutex<Option<String>>,
    exiting: AtomicBool,
    exit_ready: AtomicBool,
}
type Result<T> = std::result::Result<T, String>;
#[tauri::command]
fn list_directory(s: State<AppState>, id: String, relative: String) -> Result<Vec<browser::Entry>> {
    browser::entries(&s.config.lock().unwrap(), &id, &relative)
}
#[tauri::command]
fn lan_addresses() -> Result<Vec<String>> {
    browser::lan_addresses()
}
#[tauri::command]
fn open_url(url: String) -> Result<()> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "地址不合法")?;
    if !["http", "https"].contains(&parsed.scheme())
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("只允许打开 HTTP(S) 地址".into());
    }
    platform::open_url(parsed.as_str())
}
#[tauri::command]
fn load_config(s: State<AppState>) -> Config {
    let mut c = s.config.lock().unwrap().clone();
    for m in &mut c.mappings {
        if !m.password_hash.is_empty() {
            m.password_hash = "protected".into()
        }
    }
    c
}
#[tauri::command]
async fn save_config(s: State<'_, AppState>, mut config: Config) -> Result<()> {
    persist_config(&s, &mut config).await
}

async fn persist_config(s: &AppState, config: &mut Config) -> Result<()> {
    let mut runtime = s.runtime.lock().await;
    ensure_not_exiting(s)?;
    let old = s.config.lock().unwrap().clone();
    for m in &mut config.mappings {
        if m.password_hash == "protected" {
            m.password_hash = old
                .mappings
                .iter()
                .find(|o| o.id == m.id)
                .map(|o| o.password_hash.clone())
                .unwrap_or_default()
        }
    }
    if *config == old {
        return Ok(());
    }
    config.validate()?;
    *s.pending_host.lock().unwrap() = None;
    runtime.stop().await?;
    config::save(&s.data.join("config.json"), config)?;
    *s.config.lock().unwrap() = config.clone();
    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn unchanged_config_keeps_local_service_running() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = Config {
        local_port: listener.local_addr().unwrap().port(),
        ..Config::default()
    };
    drop(listener);
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = runtime::Runtime::default();
    runtime.start(config.clone()).await.unwrap();
    let state = AppState {
        data: dir.path().to_owned(),
        config: Mutex::new(config.clone()),
        runtime: tokio::sync::Mutex::new(runtime),
        pending_host: Mutex::new(None),
        exiting: AtomicBool::new(false),
        exit_ready: AtomicBool::new(false),
    };
    persist_config(&state, &mut config).await.unwrap();
    assert!(state.runtime.lock().await.status.lock().unwrap().local);
    assert!(!dir.path().join("config.json").exists());
    config.base_path = "/changed".into();
    persist_config(&state, &mut config).await.unwrap();
    assert!(!state.runtime.lock().await.status.lock().unwrap().local);
    assert!(dir.path().join("config.json").exists());
}
#[tauri::command]
async fn set_password(
    s: State<'_, AppState>,
    id: String,
    username: String,
    password: String,
) -> Result<()> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let mut runtime = s.runtime.lock().await;
    ensure_not_exiting(&s)?;
    let mut c = s.config.lock().unwrap().clone();
    let m = c
        .mappings
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or("找不到目录，请先保存")?;
    if password.len() < 8 {
        return Err("密码至少8位".into());
    }
    m.username = username;
    m.password_hash = Argon2::default()
        .hash_password(
            password.as_bytes(),
            &SaltString::generate(&mut rand_core::OsRng),
        )
        .map_err(|e| e.to_string())?
        .to_string();
    c.validate()?;
    runtime.stop().await?;
    config::save(&s.data.join("config.json"), &c)?;
    *s.config.lock().unwrap() = c;
    Ok(())
}
#[tauri::command]
fn generate_key(s: State<AppState>, source: Option<String>) -> Result<keys::KeyInfo> {
    keys::create(&s.data.join("keys"), source)
}
#[tauri::command]
fn list_keys(s: State<AppState>) -> Vec<keys::KeyInfo> {
    std::fs::read_dir(s.data.join("keys"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            if id.contains('.') {
                None
            } else {
                keys::inspect(&s.data.join("keys"), &id).ok()
            }
        })
        .collect()
}
#[tauri::command]
fn deployment(s: State<AppState>, config: Config) -> Result<deployment::Deployment> {
    let k = keys::inspect(&s.data.join("keys"), &config.key_id)?;
    deployment::generate(&config, &k.public_key)
}
#[tauri::command]
async fn start_local(s: State<'_, AppState>) -> Result<()> {
    let c = s.config.lock().unwrap().clone();
    let mut runtime = s.runtime.lock().await;
    ensure_not_exiting(&s)?;
    runtime.start(c).await
}
#[tauri::command]
async fn stop_all(s: State<'_, AppState>) -> Result<()> {
    s.runtime.lock().await.stop().await
}
#[tauri::command]
async fn connect(s: State<'_, AppState>) -> Result<()> {
    let c = s.config.lock().unwrap().clone();
    let mut runtime = s.runtime.lock().await;
    ensure_not_exiting(&s)?;
    runtime.connect(c, s.data.clone()).await
}
#[tauri::command]
async fn disconnect(s: State<'_, AppState>) -> Result<()> {
    s.runtime.lock().await.stop_tunnel().await
}
#[tauri::command]
async fn status(s: State<'_, AppState>) -> Result<runtime::Status> {
    Ok(s.runtime.lock().await.status.lock().unwrap().clone())
}
#[tauri::command]
async fn check_public(s: State<'_, AppState>) -> Result<bool> {
    ensure_not_exiting(&s)?;
    let result = s.runtime.lock().await.request_public_check()?;
    result
        .await
        .map_err(|_| "验证已取消：隧道已停止或连接发生变化".into())
}
#[tauri::command]
async fn scan_host(s: State<'_, AppState>) -> Result<String> {
    *s.pending_host.lock().unwrap() = None;
    let c = s.config.lock().unwrap().clone();
    c.validate()?;
    if c.host.is_empty() {
        return Err("请先保存服务器地址".into());
    }
    let output = platform::background_async_command(platform::ssh_keyscan())
        .args([
            "-T",
            "5",
            "-p",
            &c.ssh_port.to_string(),
            "-t",
            "ed25519",
            &c.host,
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        return Err("未获得主机公钥".into());
    }
    *s.pending_host.lock().unwrap() = Some(text.clone());
    let mut child = platform::background_async_command(platform::ssh_keygen())
        .args(["-lf", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    use tokio::io::AsyncWriteExt;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
#[tauri::command]
fn trust_host(s: State<AppState>) -> Result<()> {
    let text = s
        .pending_host
        .lock()
        .unwrap()
        .take()
        .ok_or("请先读取指纹")?;
    std::fs::write(s.data.join("known_hosts"), text).map_err(|e| e.to_string())
}
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        // 必须先于其他插件和 setup，阻止第二实例初始化配置、托盘及服务。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            platform::set_private_dir(&data)?;
            let path = data.join("config.json");
            let c = if path.exists() {
                serde_json::from_slice(&std::fs::read(path)?)?
            } else {
                Config::default()
            };
            app.manage(AppState {
                data,
                config: Mutex::new(c),
                runtime: tokio::sync::Mutex::new(runtime::Runtime::default()),
                pending_host: Mutex::new(None),
                exiting: AtomicBool::new(false),
                exit_ready: AtomicBool::new(false),
            });
            let show = tauri::menu::MenuItem::with_id(app, "show", "打开随享", true, None::<&str>)?;
            let quit =
                tauri::menu::MenuItem::with_id(app, "quit", "停止服务并退出", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;
            tauri::tray::TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray-template.png"
                ))?)
                .icon_as_template(true)
                .tooltip("随享")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            list_directory,
            lan_addresses,
            open_url,
            save_config,
            set_password,
            generate_key,
            list_keys,
            deployment,
            start_local,
            stop_all,
            connect,
            disconnect,
            status,
            check_public,
            scan_host,
            trust_host
        ])
        .build(tauri::generate_context!())
        .expect("启动随享失败")
        .run(|app, event| {
            // macOS 点击 Dock 图标通常发送 Reopen，而不是启动第二个进程。
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = &event {
                show_main_window(app);
            }
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if app.state::<AppState>().exit_ready.load(Ordering::SeqCst) {
                    return;
                }
                api.prevent_exit();
                let state = app.state::<AppState>();
                if state.exiting.swap(true, Ordering::SeqCst) {
                    return;
                }
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = handle.state::<AppState>().runtime.lock().await.stop().await;
                    match result {
                        Ok(()) => {
                            handle
                                .state::<AppState>()
                                .exit_ready
                                .store(true, Ordering::SeqCst);
                            handle.exit(code.unwrap_or(0));
                        }
                        Err(error) => {
                            let state = handle.state::<AppState>();
                            state
                                .runtime
                                .lock()
                                .await
                                .status
                                .lock()
                                .unwrap()
                                .logs
                                .push(error);
                            state.exiting.store(false, Ordering::SeqCst);
                        }
                    }
                });
            }
        })
}

fn ensure_not_exiting(state: &AppState) -> Result<()> {
    if state.exiting.load(Ordering::SeqCst) {
        Err("客户端正在退出，请稍候".into())
    } else {
        Ok(())
    }
}

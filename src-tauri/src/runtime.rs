use crate::config::Config;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

const PUBLIC_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(600);

// OpenSSH parses -o values as configuration syntax, even with Command's argv API.
fn known_hosts_option(path: &Path) -> Result<String, String> {
    let path = path.to_str().ok_or("主机指纹文件路径不是有效 UTF-8")?;
    if path.contains(['\n', '\r', '\0']) {
        return Err("主机指纹文件路径含非法控制字符".into());
    }
    Ok(format!(
        "UserKnownHostsFile=\"{}\"",
        path.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub local: bool,
    pub tunnel: String,
    pub public: String,
    pub public_checked_at: Option<u64>,
    pub logs: Vec<String>,
}
pub struct Runtime {
    pub file: Option<JoinHandle<()>>,
    tunnel: Option<JoinHandle<Result<(), String>>>,
    tunnel_stop: Option<oneshot::Sender<()>>,
    public_check: Option<mpsc::Sender<oneshot::Sender<bool>>>,
    pub status: Arc<Mutex<Status>>,
    pub instance: String,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            file: None,
            tunnel: None,
            tunnel_stop: None,
            public_check: None,
            status: Arc::new(Mutex::new(Status::default())),
            instance: uuid::Uuid::new_v4().to_string(),
        }
    }
}
impl Runtime {
    pub async fn stop_tunnel(&mut self) -> Result<(), String> {
        self.public_check = None;
        if let Some(stop) = self.tunnel_stop.take() {
            let _ = stop.send(());
        }
        if let Some(t) = self.tunnel.take() {
            t.await
                .map_err(|e| format!("等待隧道任务结束失败：{e}"))??;
        }
        let mut s = self.status.lock().unwrap();
        s.tunnel = "停止".into();
        s.public = "未检查".into();
        s.public_checked_at = None;
        Ok(())
    }
    pub async fn stop(&mut self) -> Result<(), String> {
        let result = self.stop_tunnel().await;
        if let Some(f) = self.file.take() {
            f.abort();
            let _ = f.await;
        }
        self.status.lock().unwrap().local = false;
        result
    }
    pub async fn start(&mut self, c: Config) -> Result<(), String> {
        self.stop().await?;
        c.validate()?;
        let bind = if c.mappings.iter().any(|m| m.enabled && m.allow_lan) {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };
        let listener = tokio::net::TcpListener::bind((bind, c.local_port))
            .await
            .map_err(|e| format!("本地端口无法监听：{e}"))?;
        let app = crate::files::router(c, self.instance.clone());
        let status = self.status.clone();
        self.file = Some(tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await;
            status.lock().unwrap().local = false;
        }));
        self.status.lock().unwrap().local = true;
        Ok(())
    }
    pub async fn connect(&mut self, c: Config, data: PathBuf) -> Result<(), String> {
        self.connect_with_program(c, data, crate::platform::ssh())
            .await
    }
    pub fn request_public_check(&self) -> Result<oneshot::Receiver<bool>, String> {
        let status = self.status.lock().unwrap();
        if !status.local || status.tunnel != "SSH进程运行中" {
            return Err("请先启动本地服务并连接SSH隧道".into());
        }
        let (send, receive) = oneshot::channel();
        self.public_check
            .as_ref()
            .ok_or("隧道未连接")?
            .try_send(send)
            .map_err(|_| "验证请求正在处理或隧道已断开")?;
        Ok(receive)
    }
    async fn connect_with_program(
        &mut self,
        c: Config,
        data: PathBuf,
        program: PathBuf,
    ) -> Result<(), String> {
        c.validate()?;
        if !self.status.lock().unwrap().local {
            return Err("请先启动本地服务".into());
        }
        if c.host.is_empty() || c.key_id.is_empty() {
            return Err("请先填写服务器并选择密钥".into());
        }
        if !data.join("known_hosts").exists() {
            return Err("请先核对并确认服务器主机指纹".into());
        }
        let known_hosts = known_hosts_option(&data.join("known_hosts"))?;
        self.stop_tunnel().await?;
        let (stop, mut stopping) = oneshot::channel();
        self.tunnel_stop = Some(stop);
        let (check_send, mut checks) = mpsc::channel(1);
        self.public_check = Some(check_send);
        let status = self.status.clone();
        let instance = self.instance.clone();
        self.tunnel = Some(tokio::spawn(async move {
            let mut attempt = 0u32;
            let mut checked_old = false;
            loop {
                if !matches!(
                    stopping.try_recv(),
                    Err(oneshot::error::TryRecvError::Empty)
                ) {
                    return Ok(());
                }
                status.lock().unwrap().tunnel = if attempt == 0 {
                    "连接中"
                } else {
                    "重连中"
                }
                .into();
                let mut cmd = crate::platform::background_async_command(&program);
                cmd.kill_on_drop(true)
                    .args(["-F", crate::platform::ssh_config_null(), "-N", "-T", "-p"])
                    .arg(c.ssh_port.to_string())
                    .args(["-i"])
                    .arg(data.join("keys").join(&c.key_id))
                    .args([
                        "-o",
                        "BatchMode=yes",
                        "-o",
                        "IdentitiesOnly=yes",
                        "-o",
                        "StrictHostKeyChecking=yes",
                        "-o",
                        "ExitOnForwardFailure=yes",
                        "-o",
                        "ConnectTimeout=10",
                        "-o",
                        "ServerAliveInterval=30",
                        "-o",
                        "ServerAliveCountMax=3",
                        "-o",
                    ])
                    .arg(&known_hosts)
                    .arg("-R")
                    .arg(format!(
                        "127.0.0.1:{}:127.0.0.1:{}",
                        c.remote_port, c.local_port
                    ))
                    .arg(format!("{}@{}", c.user, c.host))
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null());
                // 孤立 SSH 检查依赖 macOS 的 sysctl/ps/pgrep，其他平台跳过。
                if !checked_old && cfg!(target_os = "macos") && program == crate::platform::ssh() {
                    let mut expected = vec![program.as_os_str().to_owned()];
                    expected.extend(cmd.as_std().get_args().map(|s| s.to_owned()));
                    let result = tokio::select! {
                        _ = &mut stopping => return Ok(()),
                        result = crate::orphan_ssh::cleanup(expected) => result,
                    };
                    match result {
                        Ok(count) => status.lock().unwrap().logs.push(format!(
                            "连接前旧隧道检查完成，已清理{count}个完全匹配的孤立进程"
                        )),
                        Err(error) => {
                            let mut state = status.lock().unwrap();
                            state.tunnel = "连接失败".into();
                            state.logs.push(error);
                            return Ok(());
                        }
                    }
                    checked_old = true;
                }
                let result = cmd.spawn();
                match result {
                    Ok(mut child) => {
                        let stderr = child.stderr.take();
                        let reader = tokio::spawn(async move {
                            let mut text = String::new();
                            if let Some(mut s) = stderr {
                                let _ = s.read_to_string(&mut text).await;
                            }
                            text
                        });
                        let mut next_check =
                            tokio::time::Instant::now() + std::time::Duration::from_secs(3);
                        let errors = loop {
                            tokio::select! {
                                biased;
                                _ = &mut stopping => {
                                    // Child remains owned here until it is actually reaped.
                                    let result = child.kill().await.map_err(|e| format!("回收SSH进程失败：{e}"));
                                    reader.abort();
                                    let _ = reader.await;
                                    return result;
                                }
                                exit = child.wait() => {
                                    let exit = exit.map_err(|e| format!("等待SSH进程失败：{e}"))?;
                                    let errors = reader.await.unwrap_or_default();
                                    break format!("SSH进程退出（{exit}）：{}", errors.trim());
                                }
                                (reachable, reply) = scheduled_probe(&c, &instance, &mut checks, next_check) => {
                                    let mut s = status.lock().unwrap();
                                    s.tunnel = "SSH进程运行中".into();
                                    s.public = if reachable { "可访问" } else { "未验证 / 不可达" }.into();
                                    s.public_checked_at = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH).ok().map(|time| time.as_secs());
                                    next_check = tokio::time::Instant::now() + PUBLIC_CHECK_INTERVAL;
                                    if let Some(reply) = reply { let _ = reply.send(reachable); }
                                }
                            }
                        };
                        let mut s = status.lock().unwrap();
                        if s.logs.len() > 150 {
                            s.logs.drain(..50);
                        }
                        s.logs.push(errors.chars().take(2000).collect());
                        if errors.contains("remote port forwarding failed") {
                            s.logs.push(format!(
                                "服务器转发端口 {} 绑定失败：可能已被另一台电脑或旧隧道占用。每台设备应使用独立转发端口，并匹配服务端代理配置。不会自动停止其他设备。",
                                c.remote_port
                            ));
                        }
                        if errors.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
                            || errors.contains("Host key verification failed")
                        {
                            s.tunnel = "主机身份验证失败，已停止".into();
                            s.public = "未检查".into();
                            s.public_checked_at = None;
                            return Ok(());
                        }
                    }
                    Err(e) => status.lock().unwrap().logs.push(e.to_string()),
                }
                attempt = attempt.saturating_add(1);
                {
                    let mut s = status.lock().unwrap();
                    s.tunnel = "重连等待".into();
                    s.public = "未检查".into();
                    s.public_checked_at = None;
                }
                tokio::select! {
                    _ = &mut stopping => return Ok(()),
                    _ = tokio::time::sleep(std::time::Duration::from_secs(
                    (1u64 << attempt.saturating_sub(1).min(5)).min(30),
                    )) => {}
                }
            }
        }));
        Ok(())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(stop) = self.tunnel_stop.take() {
            let _ = stop.send(());
        }
        if let Some(f) = self.file.take() {
            f.abort();
        }
    }
}

async fn probe(c: &Config, instance: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let url = format!(
        "{}{}/_health",
        c.public_origin.trim_end_matches('/'),
        c.base_path
    );
    match client.get(url).send().await {
        Ok(r) => r.status().is_success() && r.text().await.unwrap_or_default() == instance,
        Err(_) => false,
    }
}

async fn scheduled_probe(
    c: &Config,
    instance: &str,
    checks: &mut mpsc::Receiver<oneshot::Sender<bool>>,
    deadline: tokio::time::Instant,
) -> (bool, Option<oneshot::Sender<bool>>) {
    let reply = tokio::select! {
        reply = checks.recv() => reply,
        _ = tokio::time::sleep_until(deadline) => None,
    };
    (probe(c, instance).await, reply)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn manual_check_does_not_wait_for_ten_minute_deadline() {
        assert_eq!(PUBLIC_CHECK_INTERVAL.as_secs(), 600);
        let (sender, mut receiver) = mpsc::channel(1);
        let mut c = Config::default();
        // Invalid URL fails immediately without contacting any server.
        c.public_origin = String::new();
        let (reply, result) = oneshot::channel();
        sender.send(reply).await.unwrap();
        let (reachable, reply) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            scheduled_probe(
                &c,
                "instance",
                &mut receiver,
                tokio::time::Instant::now() + PUBLIC_CHECK_INTERVAL,
            ),
        )
        .await
        .unwrap();
        assert!(!reachable);
        reply.unwrap().send(reachable).unwrap();
        assert!(!result.await.unwrap());
    }

    #[tokio::test]
    async fn automatic_check_waits_for_deadline() {
        let (_sender, mut receiver) = mpsc::channel(1);
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            scheduled_probe(
                &Config::default(),
                "instance",
                &mut receiver,
                tokio::time::Instant::now() + PUBLIC_CHECK_INTERVAL
            )
        )
        .await
        .is_err());
        assert!(Runtime::default().request_public_check().is_err());
    }

    async fn fake_connection(script: &str) -> (tempfile::TempDir, Config, Runtime, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("fake-ssh");
        let pid_file = dir.path().join("pid");
        std::fs::write(
            &program,
            format!("#!/bin/sh\necho $$ > '{}'\n{script}\n", pid_file.display()),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(dir.path().join("known_hosts"), "mock").unwrap();
        let mut c = Config::default();
        c.host = "example.com".into();
        c.key_id = "test".into();
        let mut runtime = Runtime::default();
        runtime.status.lock().unwrap().local = true;
        runtime
            .connect_with_program(c.clone(), dir.path().to_owned(), program.clone())
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while !pid_file.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        (dir, c, runtime, program)
    }

    fn assert_reaped(pid: &str) {
        assert!(
            !std::process::Command::new("/bin/kill")
                .args(["-0", pid.trim()])
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success(),
            "child {pid} still exists"
        );
    }

    #[tokio::test]
    async fn disconnect_reaps_child_and_keeps_local_service() {
        let (dir, _, mut runtime, _) = fake_connection("exec /bin/sleep 300").await;
        let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
        runtime.stop_tunnel().await.unwrap();
        assert_reaped(&pid);
        assert!(runtime.status.lock().unwrap().local);
        assert_eq!(runtime.status.lock().unwrap().tunnel, "停止");
        runtime.stop_tunnel().await.unwrap();
    }

    #[tokio::test]
    async fn reconnect_reaps_previous_child_before_replacement() {
        let (dir, c, mut runtime, program) = fake_connection("exec /bin/sleep 300").await;
        let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
        runtime
            .connect_with_program(c, dir.path().to_owned(), program)
            .await
            .unwrap();
        assert_reaped(&pid);
        runtime.stop().await.unwrap();
        assert!(!runtime.status.lock().unwrap().local);
    }

    #[tokio::test]
    async fn stop_during_backoff_cancels_retries() {
        let (dir, _, mut runtime, _) = fake_connection("exit 1").await;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while runtime.status.lock().unwrap().tunnel != "重连等待" {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        runtime.stop_tunnel().await.unwrap();
        std::fs::remove_file(dir.path().join("pid")).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        assert!(!dir.path().join("pid").exists());
        assert_eq!(runtime.status.lock().unwrap().tunnel, "停止");
    }

    #[tokio::test]
    async fn ssh_exit_code_and_forwarding_error_are_logged() {
        let (_dir, _, mut runtime, _) = fake_connection(
            "echo 'remote port forwarding failed for listen port 18790' >&2; exit 255",
        )
        .await;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if runtime
                    .status
                    .lock()
                    .unwrap()
                    .logs
                    .iter()
                    .any(|line| line.contains("绑定失败"))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let logs = runtime.status.lock().unwrap().logs.join("\n");
        assert!(logs.contains("255"));
        assert!(logs.contains("remote port forwarding failed"));
        assert!(logs.contains("另一台电脑"));
        runtime.stop().await.unwrap();
    }

    #[tokio::test]
    async fn stop_interrupts_pending_health_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (dir, mut c, mut runtime, program) = fake_connection("exec /bin/sleep 300").await;
        c.public_origin = format!("http://{}", listener.local_addr().unwrap());
        runtime
            .connect_with_program(c, dir.path().to_owned(), program)
            .await
            .unwrap();
        let (_connection, _) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), runtime.stop_tunnel())
            .await
            .unwrap()
            .unwrap();
        assert_reaped(&pid);
    }

    #[tokio::test]
    async fn dropping_runtime_notifies_child_owner() {
        let (dir, _, runtime, _) = fake_connection("exec /bin/sleep 300").await;
        let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
        drop(runtime);
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let alive = std::process::Command::new("/bin/kill")
                    .args(["-0", pid.trim()])
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap()
                    .success();
                if !alive {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn stop_releases_local_listener_before_returning() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut c = Config::default();
        c.local_port = port;
        let mut runtime = Runtime::default();
        runtime.start(c).await.unwrap();
        runtime.stop().await.unwrap();
        let _rebound = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .unwrap();
    }
    #[test]
    fn known_hosts_path_is_quoted_for_openssh() {
        assert_eq!(
            known_hosts_option(Path::new(
                "/Users/test/Library/Application Support/app/known_hosts"
            ))
            .unwrap(),
            "UserKnownHostsFile=\"/Users/test/Library/Application Support/app/known_hosts\""
        );
        assert_eq!(
            known_hosts_option(Path::new("/test/a\"b\\c")).unwrap(),
            "UserKnownHostsFile=\"/test/a\\\"b\\\\c\""
        );
        assert!(known_hosts_option(Path::new("/test/\ninjected")).is_err());
    }
    #[tokio::test]
    async fn port_conflict_never_reports_running() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut c = Config::default();
        c.local_port = listener.local_addr().unwrap().port();
        let mut runtime = Runtime::default();
        assert!(runtime.start(c).await.is_err());
        assert!(!runtime.status.lock().unwrap().local);
        assert!(runtime
            .connect(Config::default(), PathBuf::new())
            .await
            .is_err());
    }
}

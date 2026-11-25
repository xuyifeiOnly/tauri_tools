//! 平台相关命令与权限的集中封装。
//!
//! 业务代码只通过这里获取 SSH 工具路径、打开浏览器、设置私密权限和枚举局域网地址，
//! 避免在 macOS 之外硬编码 `/usr/bin/*` 或 `chmod` 语义。

use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// 系统 OpenSSH 客户端。macOS/Linux 使用绝对路径，Windows 优先使用系统自带 OpenSSH。
#[cfg(unix)]
pub fn ssh() -> PathBuf {
    PathBuf::from("/usr/bin/ssh")
}

#[cfg(windows)]
pub fn ssh() -> PathBuf {
    bundled_openssh("ssh.exe")
}

#[cfg(unix)]
pub fn ssh_keygen() -> PathBuf {
    PathBuf::from("/usr/bin/ssh-keygen")
}

#[cfg(windows)]
pub fn ssh_keygen() -> PathBuf {
    bundled_openssh("ssh-keygen.exe")
}

#[cfg(unix)]
pub fn ssh_keyscan() -> PathBuf {
    PathBuf::from("/usr/bin/ssh-keyscan")
}

#[cfg(windows)]
pub fn ssh_keyscan() -> PathBuf {
    bundled_openssh("ssh-keyscan.exe")
}

/// OpenSSH 的 `-F` 空配置文件参数值；Windows 没有 `/dev/null`。
#[cfg(unix)]
pub fn ssh_config_null() -> &'static str {
    "/dev/null"
}

#[cfg(windows)]
pub fn ssh_config_null() -> &'static str {
    "NUL"
}

#[cfg(windows)]
fn bundled_openssh(name: &str) -> PathBuf {
    // Windows 10/11 自带 OpenSSH 客户端通常位于 System32\OpenSSH。
    if let Ok(root) = std::env::var("SystemRoot") {
        let candidate = PathBuf::from(root)
            .join("System32")
            .join("OpenSSH")
            .join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    // 回退到 PATH 查找，避免系统未安装 OpenSSH 时静默失败。
    PathBuf::from(name)
}

/// 用系统默认浏览器打开 HTTP(S) 地址。调用方需先完成协议校验。
#[cfg(target_os = "macos")]
pub fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("/usr/bin/open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
pub fn open_url(url: &str) -> Result<(), String> {
    // 空标题参数避免 start 把带引号的 URL 当作窗口标题。
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 将文件权限收紧为仅属主可读写（Windows 无 POSIX 权限位，忽略）。
#[cfg(unix)]
pub fn set_private_file(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
pub fn set_private_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// 将目录权限收紧为仅属主可访问（Windows 无 POSIX 权限位，忽略）。
#[cfg(unix)]
pub fn set_private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
pub fn set_private_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// 枚举本机私网 IPv4 地址（排除回环）。仅查询接口，不发送网络流量、不修改配置。
#[cfg(target_os = "macos")]
pub fn lan_addresses() -> Result<Vec<String>, String> {
    let output = std::process::Command::new("/sbin/ifconfig")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("无法读取本机网络接口".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut ips = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("inet") {
            continue;
        }
        push_private_ip(words.next(), &mut ips);
    }
    Ok(ips)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn lan_addresses() -> Result<Vec<String>, String> {
    let output = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("无法读取本机网络接口".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut ips = Vec::new();
    for line in text.lines() {
        // 形如：2: en0    inet 192.168.1.20/24 brd ...
        let mut words = line.split_whitespace();
        while let Some(word) = words.next() {
            if word == "inet" {
                push_private_ip(
                    words.next().map(|s| s.split('/').next().unwrap_or(s)),
                    &mut ips,
                );
            }
        }
    }
    Ok(ips)
}

#[cfg(windows)]
pub fn lan_addresses() -> Result<Vec<String>, String> {
    let output = std::process::Command::new("ipconfig")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("无法读取本机网络接口".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut ips = Vec::new();
    for line in text.lines() {
        // 中英文 Windows 均形如 “IPv4 地址 . . . : 192.168.1.20”。
        if let Some((label, value)) = line.split_once(':') {
            if label.to_ascii_lowercase().contains("ipv4") {
                push_private_ip(value.split_whitespace().next(), &mut ips);
            }
        }
    }
    Ok(ips)
}

fn push_private_ip(candidate: Option<&str>, ips: &mut Vec<String>) {
    if let Some(ip) = candidate.and_then(|ip| ip.parse::<std::net::Ipv4Addr>().ok()) {
        if ip.is_private() && !ip.is_loopback() && !ips.contains(&ip.to_string()) {
            ips.push(ip.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_tool_paths_are_absolute_on_unix() {
        #[cfg(unix)]
        {
            assert!(ssh().is_absolute());
            assert!(ssh_keygen().is_absolute());
            assert!(ssh_keyscan().is_absolute());
            assert_eq!(ssh_config_null(), "/dev/null");
        }
    }

    #[test]
    fn private_ip_filter_rejects_loopback_and_public() {
        let mut ips = Vec::new();
        push_private_ip(Some("192.168.1.20"), &mut ips);
        push_private_ip(Some("127.0.0.1"), &mut ips);
        push_private_ip(Some("8.8.8.8"), &mut ips);
        push_private_ip(Some("192.168.1.20"), &mut ips);
        assert_eq!(ips, vec!["192.168.1.20".to_string()]);
    }
}

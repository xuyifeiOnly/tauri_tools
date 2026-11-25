//! Conservative legacy-process recovery. Never parse a flattened command line.
use std::ffi::OsString;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn matches_argv(actual: &[OsString], expected: &[OsString]) -> bool {
    actual == expected
        && expected
            .first()
            .is_some_and(|s| s.as_os_str() == crate::platform::ssh().as_os_str())
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn is_orphan(identity: &str, uid: u32) -> bool {
    let parts: Vec<_> = identity.split_whitespace().collect();
    parts.first().and_then(|s| s.parse::<u32>().ok()) == Some(uid) && parts.get(1) == Some(&"1")
}

#[cfg(target_os = "macos")]
fn argv(pid: i32) -> Result<Vec<OsString>, String> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut length = 0;
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    let mut bytes = vec![0u8; length];
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            3,
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    bytes.truncate(length);
    if bytes.len() < 4 {
        return Err("进程参数数据不完整".into());
    }
    let count = i32::from_ne_bytes(bytes[..4].try_into().unwrap());
    let mut pos = 4;
    while pos < bytes.len() && bytes[pos] != 0 {
        pos += 1;
    }
    while pos < bytes.len() && bytes[pos] == 0 {
        pos += 1;
    }
    let mut result = Vec::new();
    for _ in 0..count {
        let end = bytes[pos..]
            .iter()
            .position(|b| *b == 0)
            .ok_or("进程参数缺少结尾")?
            + pos;
        result.push(OsString::from_vec(bytes[pos..end].to_vec()));
        pos = end + 1;
    }
    Ok(result)
}

#[cfg(unix)]
async fn identity(pid: i32) -> Result<Option<String>, String> {
    let result = tokio::process::Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "uid=,ppid=,lstart="])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&result.stdout).trim().to_owned();
    if text.is_empty() {
        return Ok(None);
    }
    if !result.status.success() {
        return Err("读取SSH进程身份失败".into());
    }
    Ok(Some(text))
}

#[cfg(target_os = "macos")]
pub async fn cleanup(expected: Vec<OsString>) -> Result<usize, String> {
    let uid = unsafe { libc::geteuid() };
    let output = tokio::process::Command::new("/usr/bin/pgrep")
        .args(["-u", &uid.to_string(), "-x", "ssh"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.code() == Some(1) {
        return Ok(0);
    }
    if !output.status.success() {
        return Err("无法检查已有SSH进程，已取消连接".into());
    }
    let mut count = 0;
    for value in String::from_utf8_lossy(&output.stdout).split_whitespace() {
        let pid: i32 = value.parse().map_err(|_| "进程ID不合法")?;
        let Some(before) = identity(pid).await? else {
            continue;
        };
        let actual = match argv(pid) {
            Ok(value) => value,
            Err(error) => {
                if identity(pid).await?.is_none() {
                    continue;
                }
                return Err(format!("无法核验SSH进程{pid}：{error}，已取消连接"));
            }
        };
        if !matches_argv(&actual, &expected) {
            continue;
        }
        if !is_orphan(&before, uid) {
            return Err(format!(
                "同配置SSH进程{pid}仍由其他客户端管理，请先退出该客户端"
            ));
        }
        // Revalidate immediately before signalling, not just the initial scan.
        if identity(pid).await?.as_ref() != Some(&before) || argv(pid)? != actual {
            return Err("SSH进程身份已变化，已取消连接，请重试".into());
        }
        if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
            return Err(format!(
                "无法停止旧SSH进程{pid}：{}",
                std::io::Error::last_os_error()
            ));
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let current = identity(pid).await?;
            if current.as_ref() != Some(&before) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("旧SSH进程{pid}未退出，已取消新连接"));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        count += 1;
    }
    Ok(count)
}

#[cfg(not(target_os = "macos"))]
pub async fn cleanup(_: Vec<OsString>) -> Result<usize, String> {
    Err("当前平台不支持安全检查旧SSH进程".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_parent_and_other_user_are_protected() {
        assert!(is_orphan("501 1 Wed Nov 25 10:00:00 2026 S", 501));
        assert!(!is_orphan("501 123 Wed Nov 25 10:00:00 2026 S", 501));
        assert!(!is_orphan("502 1 Wed Nov 25 10:00:00 2026 S", 501));
        assert!(!is_orphan("", 501));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sysctl_reads_current_process_arguments() {
        let values = argv(std::process::id() as i32).unwrap();
        assert!(!values.is_empty());
        assert_eq!(values, std::env::args_os().collect::<Vec<_>>());
    }
    #[test]
    fn exact_arguments_only() {
        let mut expected: Vec<OsString> = vec![crate::platform::ssh().into_os_string()];
        expected.extend(
            [
                "-i",
                "/Application Support/app/key",
                "-R",
                "127.0.0.1:18790:127.0.0.1:20005",
                "user@host",
            ]
            .map(Into::into)
            .to_vec(),
        );
        assert!(matches_argv(&expected, &expected));
        for index in 0..expected.len() {
            let mut changed = expected.clone();
            changed[index].push("different");
            assert!(!matches_argv(&changed, &expected));
        }
        let flattened = vec![OsString::from(
            expected
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
        )];
        assert!(!matches_argv(&flattened, &expected));
    }
}

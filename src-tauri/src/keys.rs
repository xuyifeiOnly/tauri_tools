use serde::Serialize;
use std::{path::Path, process::Command};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyInfo {
    pub id: String,
    pub public_key: String,
    pub fingerprint: String,
    pub private_key_path: String,
    pub public_key_path: String,
}
pub fn inspect(dir: &Path, id: &str) -> Result<KeyInfo, String> {
    if id.is_empty() {
        return Err("请先在连接配置中选择电脑私钥，再生成部署脚本或连接隧道".into());
    }
    if !crate::config::segment(id) {
        return Err("密钥ID不合法".into());
    }
    let p = dir.join(id);
    if !p.is_file() {
        return Err("所选密钥不存在，请在连接配置中重新选择电脑私钥".into());
    }
    let o = Command::new(crate::platform::ssh_keygen())
        .args(["-y", "-P", ""])
        .arg("-f")
        .arg(&p)
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err("无法读取密钥：首版仅支持无口令私钥".into());
    }
    let public_key = String::from_utf8_lossy(&o.stdout).trim().to_string();
    std::fs::write(p.with_extension("pub"), &public_key).map_err(|e| e.to_string())?;
    let out = Command::new(crate::platform::ssh_keygen())
        .arg("-lf")
        .arg(p.with_extension("pub"))
        .output()
        .map_err(|e| e.to_string())?;
    Ok(KeyInfo {
        id: id.into(),
        public_key,
        fingerprint: String::from_utf8_lossy(&out.stdout).trim().into(),
        private_key_path: p.to_string_lossy().into_owned(),
        public_key_path: p.with_extension("pub").to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_selection_is_not_reported_as_invalid_key() {
        let dir = tempfile::tempdir().unwrap();
        assert!(inspect(dir.path(), "")
            .err()
            .unwrap()
            .contains("选择电脑私钥"));
        assert_eq!(inspect(dir.path(), "../bad").err().unwrap(), "密钥ID不合法");
        assert!(inspect(dir.path(), "missing")
            .err()
            .unwrap()
            .contains("不存在"));
    }

    #[test]
    fn generated_key_can_be_used_for_deployment() {
        let dir = tempfile::tempdir().unwrap();
        let key = create(dir.path(), None).unwrap();
        assert!(crate::config::segment(&key.id));
        let loaded = inspect(dir.path(), &key.id).unwrap();
        assert_eq!(loaded.public_key, key.public_key);
        assert_eq!(
            std::path::Path::new(&loaded.private_key_path),
            dir.path().join(&key.id)
        );
        assert_eq!(
            std::path::Path::new(&loaded.public_key_path),
            dir.path().join(&key.id).with_extension("pub")
        );
        assert!(std::path::Path::new(&loaded.public_key_path).is_file());
        let mut config = crate::config::Config::default();
        config.key_id = key.id;
        assert!(crate::deployment::generate(&config, &loaded.public_key).is_ok());
    }
}
pub fn create(dir: &Path, source: Option<String>) -> Result<KeyInfo, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::platform::set_private_dir(dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let p = dir.join(&id);
    if let Some(source) = source {
        std::fs::copy(source, &p).map_err(|e| e.to_string())?;
    } else {
        let o = Command::new(crate::platform::ssh_keygen())
            .args(["-t", "ed25519", "-N", "", "-C", "mac-tunnel", "-f"])
            .arg(&p)
            .output()
            .map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err("密钥生成失败".into());
        }
    }
    crate::platform::set_private_file(&p).map_err(|e| e.to_string())?;
    inspect(dir, &id)
}

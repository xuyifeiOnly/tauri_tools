use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub route: String,
    pub enabled: bool,
    #[serde(default)]
    pub allow_lan: bool,
    pub username: String,
    #[serde(default)]
    pub password_hash: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub host: String,
    pub ssh_port: u16,
    pub user: String,
    pub local_port: u16,
    pub remote_port: u16,
    pub public_origin: String,
    pub base_path: String,
    pub key_id: String,
    pub mappings: Vec<Mapping>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::new(),
            ssh_port: 22,
            user: "image-tunnel".into(),
            local_port: 20005,
            remote_port: 18790,
            public_origin: String::new(),
            base_path: "/my_mac".into(),
            key_id: String::new(),
            mappings: vec![],
        }
    }
}
pub fn segment(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.local_port < 1024 || self.remote_port < 1024 || self.ssh_port == 0 {
            return Err("端口必须有效，本地和转发端口须 ≥1024".into());
        }
        if !self.host.is_empty()
            && (!self
                .host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-:".contains(&b))
                || self.host.starts_with('-'))
        {
            return Err("服务器地址不合法".into());
        }
        if !segment(&self.user)
            || !segment(self.base_path.trim_start_matches('/'))
            || !self.base_path.starts_with('/')
        {
            return Err("账号或访问前缀不合法（仅允许字母数字、横线、下划线）".into());
        }
        if !self.key_id.is_empty() && !segment(&self.key_id) {
            return Err("密钥标识不合法".into());
        }
        if !self.public_origin.is_empty() {
            let u = reqwest::Url::parse(&self.public_origin).map_err(|_| "公网源站不合法")?;
            if !["https", "http"].contains(&u.scheme())
                || u.host_str().is_none()
                || u.path() != "/"
                || !u.username().is_empty()
                || u.password().is_some()
                || u.query().is_some()
                || u.fragment().is_some()
            {
                return Err("公网地址只能填写 http(s) 源站，不含路径或凭据".into());
            }
        }
        let mut routes = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        for m in &self.mappings {
            if !segment(&m.id)
                || !ids.insert(&m.id)
                || !segment(&m.route)
                || !routes.insert(&m.route)
                || m.route == "_health"
            {
                return Err("目录路径和ID必须唯一且合法".into());
            }
            if m.enabled && !Path::new(&m.local_path).is_dir() {
                return Err(format!("目录不存在：{}", m.name));
            }
            if !m.password_hash.is_empty() && (m.username.is_empty() || m.username.contains(':')) {
                return Err("受保护目录需要有效用户名".into());
            }
        }
        Ok(())
    }
}
pub fn save(path: &Path, c: &Config) -> Result<(), String> {
    c.validate()?;
    let tmp = path.with_extension("tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    crate::platform::set_private_file(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn validate_defaults_and_injection() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        c.host = "x;touch /tmp/test".into();
        assert!(c.validate().is_err());
        c.host = "example.com".into();
        c.local_port = 80;
        assert!(c.validate().is_err())
    }
    #[test]
    fn save_private() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        save(&p, &Config::default()).unwrap();
        #[cfg(unix)]
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
        #[cfg(not(unix))]
        assert!(p.exists());
    }
}

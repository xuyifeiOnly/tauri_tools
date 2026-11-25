use crate::{config::Config, files::safe_path};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    path: String,
    directory: bool,
}

pub fn entries(c: &Config, id: &str, relative: &str) -> Result<Vec<Entry>, String> {
    let m = c
        .mappings
        .iter()
        .find(|m| m.id == id)
        .ok_or("目录尚未保存")?;
    let root = Path::new(&m.local_path);
    let dir = safe_path(root, relative).map_err(|_| "目录不可访问")?;
    let mut result = Vec::new();
    for item in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let Ok(name) = item.file_name().into_string() else {
            continue;
        };
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        let Ok(safe) = safe_path(root, &path) else {
            continue;
        };
        let kind = std::fs::symlink_metadata(safe)
            .map_err(|e| e.to_string())?
            .file_type();
        if !kind.is_dir() && !kind.is_file() {
            continue;
        }
        result.push(Entry {
            name,
            path,
            directory: kind.is_dir(),
        });
        if result.len() > 5000 {
            return Err("此目录超过5000项，请拆分目录后浏览".into());
        }
    }
    result.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(result)
}

pub fn lan_addresses() -> Result<Vec<String>, String> {
    crate::platform::lan_addresses()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn listing_filters_and_defaults() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("中文 #.html"), "ok").unwrap();
        std::fs::write(d.path().join(".secret"), "no").unwrap();
        std::fs::write(d.path().join("private.key"), "no").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", d.path().join("escape")).unwrap();
        let m: crate::config::Mapping = serde_json::from_value(serde_json::json!({"id":"docs", "name":"docs", "localPath":d.path(), "route":"docs", "enabled":true, "username":""})).unwrap();
        assert!(!m.allow_lan);
        let mut c = Config::default();
        c.mappings.push(m);
        assert_eq!(entries(&c, "docs", "").unwrap().len(), 1);
        assert!(entries(&c, "docs", "../").is_err());
        assert!(entries(&c, "missing", "").is_err());
    }
}

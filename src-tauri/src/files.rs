use crate::config::{Config, Mapping};
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use base64::Engine;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::services::ServeFile;

#[cfg(test)]
#[path = "markdown_http_tests.rs"]
mod markdown_http_tests;

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, ()> {
    let root = root.canonicalize().map_err(|_| ())?;
    let mut p = root.clone();
    for part in relative.split('/').filter(|s| !s.is_empty()) {
        if part.starts_with('.')
            || part.contains(['\\', '\0'])
            || part.ends_with(".pem")
            || part.ends_with(".key")
            || part.starts_with("id_rsa")
            || part.starts_with("id_ed25519")
        {
            return Err(());
        }
        p.push(part);
        if std::fs::symlink_metadata(&p)
            .map_err(|_| ())?
            .file_type()
            .is_symlink()
        {
            return Err(());
        }
    }
    if !p.canonicalize().map_err(|_| ())?.starts_with(root) {
        return Err(());
    }
    Ok(p)
}
fn authorized(m: &Mapping, r: &Request) -> bool {
    if m.password_hash.is_empty() {
        return true;
    }
    let Some(value) = r
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Basic "))
    else {
        return false;
    };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(value) else {
        return false;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    let Some((u, p)) = text.split_once(':') else {
        return false;
    };
    u == m.username
        && PasswordHash::new(&m.password_hash)
            .is_ok_and(|h| Argon2::default().verify_password(p.as_bytes(), &h).is_ok())
}

#[cfg(test)]
mod lan_tests {
    use super::*;
    #[tokio::test]
    async fn non_loopback_cannot_access_private_mapping_or_health() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("中文 #.txt"), "hello").unwrap();
        let mut c = Config::default();
        c.mappings.push(Mapping {
            id: "docs".into(),
            name: "docs".into(),
            local_path: d.path().to_string_lossy().into(),
            route: "docs".into(),
            enabled: true,
            allow_lan: false,
            username: "".into(),
            password_hash: "".into(),
        });
        async fn request(c: Config, url: &str, ip: &str) -> Response {
            let mut r = Request::builder().uri(url).body(Body::empty()).unwrap();
            r.extensions_mut().insert(axum::extract::ConnectInfo(
                ip.parse::<std::net::SocketAddr>().unwrap(),
            ));
            deliver(State(Arc::new((c, "instance".into()))), r).await
        }
        let url = "/my_mac/docs/%E4%B8%AD%E6%96%87%20%23.txt";
        assert_eq!(
            request(c.clone(), url, "192.168.1.2:1234").await.status(),
            404
        );
        assert_eq!(
            request(c.clone(), url, "127.0.0.1:1234").await.status(),
            200
        );
        c.mappings[0].allow_lan = true;
        assert_eq!(
            request(c.clone(), url, "192.168.1.2:1234").await.status(),
            200
        );
        assert_eq!(
            request(c.clone(), "/_health", "192.168.1.2:1234")
                .await
                .status(),
            404
        );
        c.mappings[0].password_hash = "invalid".into();
        assert_eq!(request(c, url, "192.168.1.2:1234").await.status(), 401);
    }
}
fn html(body: String, head: bool) -> Response {
    let mut r = Response::new(Body::from(if head { String::new() } else { body }));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().unwrap(),
    );
    r
}
pub fn router(c: Config, instance: String) -> Router {
    Router::new()
        .fallback(deliver)
        .with_state(Arc::new((c, instance)))
}
async fn deliver(State(state): State<Arc<(Config, String)>>, r: Request) -> Response {
    let c = &state.0;
    let lan = r
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .is_none_or(|peer| !peer.0.ip().is_loopback());
    let head = r.method() == "HEAD";
    if r.method() != "GET" && !head {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let raw = r.uri().path();
    let local = if raw == c.base_path {
        "/"
    } else {
        raw.strip_prefix(&(c.base_path.clone() + "/"))
            .map(|s| &raw[raw.len() - s.len() - 1..])
            .unwrap_or(raw)
    };
    if local == "/_health" {
        if lan {
            return StatusCode::NOT_FOUND.into_response();
        }
        return (
            [
                (header::CONTENT_TYPE, "text/plain"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            state.1.clone(),
        )
            .into_response();
    }
    if local == "/" {
        let links = c
            .mappings
            .iter()
            .filter(|m| m.enabled && (!lan || m.allow_lan))
            .map(|m| {
                format!(
                    "<li><a href=\"./{}/\">{}</a>{}</li>",
                    m.route,
                    escape(&m.name),
                    if m.password_hash.is_empty() {
                        ""
                    } else {
                        " · 密码保护"
                    }
                )
            })
            .collect::<String>();
        return html(format!("<!doctype html><meta charset=utf-8><title>随享</title><h1>共享目录</h1><ul>{links}</ul>"),head);
    }
    let decoded = match percent_encoding::percent_decode_str(local).decode_utf8() {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mut parts = decoded.trim_start_matches('/').splitn(2, '/');
    let route = parts.next().unwrap_or("");
    let relative = parts.next().unwrap_or("");
    let Some(m) = c
        .mappings
        .iter()
        .find(|m| m.enabled && m.route == route && (!lan || m.allow_lan))
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !authorized(m, &r) {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                format!("Basic realm=\"Suixiang {}\", charset=\"UTF-8\"", m.route),
            )],
            "需要目录用户名和密码",
        )
            .into_response();
    }
    let Ok(mut path) = safe_path(Path::new(&m.local_path), relative) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if path.is_dir() {
        if !raw.ends_with('/') {
            return (
                StatusCode::TEMPORARY_REDIRECT,
                [(header::LOCATION, format!("{raw}/"))],
            )
                .into_response();
        }
        if let Ok(index) = safe_path(&path, "index.html") {
            if index.is_file() {
                path = index
            }
        }
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(&path) else {
                return StatusCode::FORBIDDEN.into_response();
            };
            let mut links = vec![];
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if safe_path(&path, &name).is_err() {
                    continue;
                }
                let suffix = if e.path().is_dir() { "/" } else { "" };
                links.push(format!(
                    "<li><a href=\"./{}{}\">{}{}</a></li>",
                    percent_encoding::utf8_percent_encode(
                        &name,
                        percent_encoding::NON_ALPHANUMERIC
                    ),
                    suffix,
                    escape(&name),
                    suffix
                ))
            }
            links.sort();
            return html(
                format!(
                    "<!doctype html><meta charset=utf-8><title>{}</title><h1>{}</h1><ul>{}</ul>",
                    escape(&m.name),
                    escape(&m.name),
                    links.join("")
                ),
                head,
            );
        }
    }
    let markdown = path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("md") || s.eq_ignore_ascii_case("markdown"));
    let query = r.uri().query().unwrap_or("");
    let download = query.split('&').any(|s| s == "download=1");
    let raw_source = query.split('&').any(|s| s == "raw=1");
    let tiff = path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("tif") || s.eq_ignore_ascii_case("tiff"));
    if tiff && !download && !raw_source {
        return crate::image_preview::respond(&path, head).await;
    }
    if markdown && !download && !raw_source {
        return crate::markdown::respond(&path, head).await;
    }
    let disposition = if (markdown || tiff) && download {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        Some(format!(
            "attachment; filename=\"download\"; filename*=UTF-8''{}",
            percent_encoding::utf8_percent_encode(&filename, percent_encoding::NON_ALPHANUMERIC)
        ))
    } else {
        None
    };
    let mut service = ServeFile::new(path);
    match service.try_call(r).await {
        Ok(res) => {
            let mut res = res.map(Body::new);
            if markdown {
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    "text/plain; charset=utf-8".parse().unwrap(),
                );
            }
            if let Some(value) = disposition {
                res.headers_mut()
                    .insert(header::CONTENT_DISPOSITION, value.parse().unwrap());
            }
            res.headers_mut()
                .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
            res.headers_mut()
                .insert("x-content-type-options", "nosniff".parse().unwrap());
            res
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_boundary() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("ok.txt"), "ok").unwrap();
        assert!(safe_path(d.path(), "ok.txt").is_ok());
        assert!(safe_path(d.path(), "../secret").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", d.path().join("escape")).unwrap();
            assert!(safe_path(d.path(), "escape").is_err());
        }
        assert!(safe_path(d.path(), ".ssh").is_err())
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use argon2::{password_hash::SaltString, PasswordHasher};
    #[tokio::test]
    async fn serves_local_public_prefix_and_protects_every_resource() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "hello tunnel").unwrap();
        std::fs::write(dir.path().join("image.txt"), "asset").unwrap();
        let mut c = Config::default();
        c.mappings.push(Mapping {
            allow_lan: false,
            id: "test".into(),
            name: "Test".into(),
            local_path: dir.path().to_string_lossy().into(),
            route: "docs".into(),
            enabled: true,
            username: "alice".into(),
            password_hash: Argon2::default()
                .hash_password(b"password123", &SaltString::generate(&mut rand_core::OsRng))
                .unwrap()
                .to_string(),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                router(c, "instance".into())
                    .into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = reqwest::Client::new();
        let base = format!("http://127.0.0.1:{port}");
        for path in ["/docs/", "/docs/image.txt", "/my_mac/docs/image.txt"] {
            assert_eq!(
                client
                    .get(format!("{base}{path}"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                401
            );
        }
        let response = client
            .get(format!("{base}/docs/"))
            .basic_auth("alice", Some("password123"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await.unwrap(), "hello tunnel");
        let response = client
            .get(format!("{base}/my_mac/docs/image.txt"))
            .basic_auth("alice", Some("password123"))
            .header("Range", "bytes=0-1")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.text().await.unwrap(), "as");
        assert_eq!(
            client
                .post(format!("{base}/docs/"))
                .send()
                .await
                .unwrap()
                .status(),
            405
        );
        assert_eq!(
            client
                .get(format!("{base}/_health"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "instance"
        );
        task.abort();
    }
}

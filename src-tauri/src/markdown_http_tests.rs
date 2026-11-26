use super::*;

#[tokio::test]
async fn styled_listing_navigation_encoding_empty_and_permissions() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("子 & 目录")).unwrap();
    std::fs::write(d.path().join("a #<x>.md"), "# hello").unwrap();
    std::fs::write(d.path().join(".hidden"), "secret").unwrap();
    let mut c = config(d.path());
    let mut locked = c.mappings[0].clone();
    locked.id = "locked".into();
    locked.route = "locked".into();
    locked.name = "保护目录".into();
    locked.password_hash = "invalid".into();
    c.mappings.push(locked);
    for prefix in ["", "/my_mac"] {
        let root = text(request(c.clone(), &format!("{prefix}/"), "GET", false, false).await).await;
        assert!(root.contains("密码保护"));
        assert!(root.contains("class=\"browser\""));
        assert!(root.contains(&format!("href=\"{prefix}/docs/\"")));
        let listing =
            text(request(c.clone(), &format!("{prefix}/docs/"), "GET", false, false).await).await;
        assert!(listing.find("子 &amp; 目录").unwrap() < listing.find("a #&lt;x&gt;.md").unwrap());
        assert!(listing.contains("./a%20%23%3Cx%3E%2Emd"));
        assert!(!listing.contains(".hidden"));
        assert!(!listing.contains(&d.path().to_string_lossy().to_string()));
        let sub = text(
            request(
                c.clone(),
                &format!("{prefix}/docs/%E5%AD%90%20%26%20%E7%9B%AE%E5%BD%95/"),
                "GET",
                false,
                false,
            )
            .await,
        )
        .await;
        assert!(sub.contains("此目录为空"));
        assert!(sub.contains(&format!("href=\"{prefix}/docs/\"")));
        let r = request(c.clone(), &format!("{prefix}/docs/"), "HEAD", false, false).await;
        assert!(r.headers().contains_key(header::CONTENT_SECURITY_POLICY));
        assert_eq!(text(r).await, "");
        assert_eq!(
            request(c.clone(), &format!("{prefix}/locked/"), "GET", false, false)
                .await
                .status(),
            401
        );
    }
    let lan = text(request(c.clone(), "/", "GET", false, true).await).await;
    assert!(lan.contains("暂无共享目录"));
    assert!(!lan.contains("保护目录"));
    std::fs::write(d.path().join("index.html"), "user index").unwrap();
    let r = request(c, "/docs/", "GET", false, false).await;
    assert!(!r.headers().contains_key(header::CONTENT_SECURITY_POLICY));
    assert_eq!(text(r).await, "user index");
}

#[tokio::test]
async fn tiff_preview_preserves_original_permissions_and_range() {
    let d = tempfile::tempdir().unwrap();
    let mut original = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(4, 3)
        .write_to(&mut original, image::ImageFormat::Tiff)
        .unwrap();
    let original = original.into_inner();
    std::fs::write(d.path().join("图.TIFF"), &original).unwrap();
    std::fs::write(d.path().join("bad.tif"), b"broken").unwrap();
    let c = config(d.path());
    for prefix in ["", "/my_mac"] {
        let url = format!("{prefix}/docs/%E5%9B%BE%2ETIFF");
        let r = request(c.clone(), &url, "GET", true, false).await;
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/png");
        let bytes = axum::body::to_bytes(r.into_body(), 10000).await.unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (4, 3));
        let head = request(c.clone(), &url, "HEAD", false, false).await;
        assert_eq!(head.status(), 200);
        assert_eq!(text(head).await, "");
        let raw = request(c.clone(), &format!("{url}?raw=1"), "GET", false, false).await;
        assert_eq!(
            &axum::body::to_bytes(raw.into_body(), 10000).await.unwrap()[..],
            &original
        );
        let download = request(c.clone(), &format!("{url}?download=1"), "GET", true, false).await;
        assert_eq!(download.status(), 206);
        assert!(download.headers().contains_key(header::CONTENT_DISPOSITION));
        assert_eq!(
            &axum::body::to_bytes(download.into_body(), 10000)
                .await
                .unwrap()[..],
            &original[..3]
        );
        assert_eq!(
            request(c.clone(), &url, "GET", false, true).await.status(),
            404
        );
        let mut protected = c.clone();
        protected.mappings[0].password_hash = "invalid".into();
        assert_eq!(
            request(protected, &url, "GET", false, false).await.status(),
            401
        );
    }
    assert_eq!(
        request(c.clone(), "/docs/bad.tif", "GET", false, false)
            .await
            .status(),
        422
    );
    let file = std::fs::File::create(d.path().join("large.tiff")).unwrap();
    file.set_len(crate::image_preview::MAX_INPUT + 1).unwrap();
    assert_eq!(
        request(c.clone(), "/docs/large.tiff", "GET", false, false)
            .await
            .status(),
        413
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(d.path().join("图.TIFF"), d.path().join("link.tiff")).unwrap();
        assert_eq!(
            request(c, "/docs/link.tiff", "GET", false, false)
                .await
                .status(),
            404
        );
    }
}

fn config(root: &Path) -> Config {
    let mut c = Config::default();
    c.mappings.push(Mapping {
        id: "test".into(),
        name: "测试".into(),
        route: "docs".into(),
        local_path: root.to_string_lossy().into(),
        enabled: true,
        allow_lan: false,
        username: "".into(),
        password_hash: "".into(),
    });
    c
}

async fn request(c: Config, path: &str, method: &str, range: bool, lan: bool) -> Response {
    let mut builder = Request::builder().uri(path).method(method);
    if range {
        builder = builder.header("Range", "bytes=0-2");
    }
    let mut r = builder.body(Body::empty()).unwrap();
    r.extensions_mut().insert(axum::extract::ConnectInfo(
        if lan {
            "192.168.1.2:1234"
        } else {
            "127.0.0.1:1234"
        }
        .parse::<std::net::SocketAddr>()
        .unwrap(),
    ));
    deliver(State(Arc::new((c, "test".into()))), r).await
}

async fn text(r: Response) -> String {
    String::from_utf8(
        axum::body::to_bytes(r.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn preview_raw_download_head_range_and_prefix() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("sub")).unwrap();
    std::fs::write(d.path().join("sub/文档.MD"), "# 标题\n\n![图](a.png)").unwrap();
    std::fs::write(d.path().join("sub/a.png"), "image").unwrap();
    let c = config(d.path());
    for prefix in ["", "/my_mac"] {
        let url = format!("{prefix}/docs/sub/%E6%96%87%E6%A1%A3%2EMD");
        let r = request(c.clone(), &url, "GET", true, false).await;
        assert_eq!(r.status(), 200);
        assert_eq!(
            r.headers()[header::CONTENT_TYPE],
            "text/html; charset=utf-8"
        );
        assert!(r.headers().contains_key("content-security-policy"));
        let body = text(r).await;
        assert!(body.contains("<h1>标题</h1>"));
        assert!(body.contains("src=\"a.png\""));
        assert_eq!(
            request(
                c.clone(),
                &format!("{prefix}/docs/sub/a.png"),
                "GET",
                false,
                false
            )
            .await
            .status(),
            200
        );
        let raw = request(c.clone(), &format!("{url}?raw=1"), "GET", false, false).await;
        assert_eq!(
            raw.headers()[header::CONTENT_TYPE],
            "text/plain; charset=utf-8"
        );
        assert!(text(raw).await.starts_with("# 标题"));
        let download = request(c.clone(), &format!("{url}?download=1"), "GET", true, false).await;
        assert_eq!(download.status(), 206);
        assert!(download.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment;"));
        let bytes = axum::body::to_bytes(download.into_body(), 32)
            .await
            .unwrap();
        assert_eq!(&bytes[..], &"# 标题".as_bytes()[..3]);
    }
}

#[tokio::test]
async fn permissions_limits_encoding_and_head() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.md"), "# Hello").unwrap();
    std::fs::write(d.path().join("bad.md"), [0xff]).unwrap();
    std::fs::write(
        d.path().join("large.md"),
        vec![b'x'; crate::markdown::MAX_BYTES as usize + 1],
    )
    .unwrap();
    let c = config(d.path());
    let r = request(c.clone(), "/docs/a.md", "HEAD", false, false).await;
    assert_eq!(r.status(), 200);
    assert_eq!(text(r).await, "");
    assert_eq!(
        request(c.clone(), "/docs/a.md", "GET", false, true)
            .await
            .status(),
        404
    );
    for (path, status) in [("/docs/large.md", 413), ("/docs/bad.md", 422)] {
        let r = request(c.clone(), path, "GET", false, false).await;
        assert_eq!(r.status().as_u16(), status);
        assert!(text(r).await.contains("?download=1"));
    }
    for path in ["/docs/a.md?raw=1", "/docs/a.md?download=1"] {
        assert_eq!(
            request(c.clone(), path, "GET", false, true).await.status(),
            404
        );
    }
    assert_eq!(
        request(c.clone(), "/docs/%2e%2e/a.md?raw=1", "GET", false, false)
            .await
            .status(),
        404
    );
    let mut protected = c;
    protected.mappings[0].password_hash = "invalid".into();
    for path in ["/docs/a.md", "/docs/a.md?raw=1", "/docs/a.md?download=1"] {
        assert_eq!(
            request(protected.clone(), path, "GET", false, false)
                .await
                .status(),
            401
        );
    }
}

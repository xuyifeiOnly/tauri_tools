//! Isolated, loopback-only UI fixture: cargo run --example markdown_preview
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;
#[path = "../src/files.rs"]
mod files;
#[path = "../src/image_preview.rs"]
mod image_preview;
#[path = "../src/markdown.rs"]
mod markdown;
#[path = "../src/platform.rs"]
#[allow(dead_code)]
mod platform;

#[tokio::main]
async fn main() {
    let dir = tempfile::tempdir().unwrap();
    let image = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"720\" height=\"180\" viewBox=\"0 0 720 180\"><rect width=\"720\" height=\"180\" rx=\"12\" fill=\"#dbeafe\"/><path d=\"M42 125V55h70l18 20h130v50Z\" fill=\"#2563eb\"/><text x=\"292\" y=\"101\" font-family=\"sans-serif\" font-size=\"25\" fill=\"#1e3a8a\">Suixiang · Markdown</text></svg>";
    std::fs::write(dir.path().join("image test.svg"), image).unwrap();
    let source = format!("# 共享文档阅读预览\n\n这是 **随享** 的 Markdown 阅读页面。图片、表格和代码都在原地址直接展示。\n\n![目录示意图](image%20test.svg)\n\n## 代码高亮\n\n```rust\nfn main() {{\n    let message = \"你好，随享\";\n    println!(\"{{}}\", message);\n}}\n```\n\n```javascript\nconst image = {{ name: \"photo.png\", enabled: true }};\nconsole.log(image);\n```\n\n## 长内容与表格\n\n```unknown\n{}\n```\n\n| 功能 | 状态 | 说明 |\n| --- | --- | --- |\n| 图片 | 已支持 | 相对路径不改变 |\n| 高亮 | 已支持 | 按语言识别 |\n\n- [x] 图片渲染\n- [x] 代码高亮\n\n> 原文件保持不变，可查看源码或下载。\n", "long_code_without_wrap_".repeat(16));
    std::fs::write(dir.path().join("测试.md"), source).unwrap();
    let mut c = config::Config::default();
    c.mappings.push(config::Mapping {
        id: "fixture".into(),
        name: "预览验证".into(),
        local_path: std::env::args()
            .nth(1)
            .unwrap_or_else(|| dir.path().to_string_lossy().into()),
        route: "docs".into(),
        enabled: true,
        allow_lan: false,
        username: "".into(),
        password_hash: "".into(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!(
        "PREVIEW_URL=http://{}/my_mac/docs/%E6%B5%8B%E8%AF%95.md",
        listener.local_addr().unwrap()
    );
    axum::serve(
        listener,
        files::router(c, "fixture".into())
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await
    .unwrap();
}

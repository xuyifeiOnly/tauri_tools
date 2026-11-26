use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::sync::LazyLock;
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' http: https:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub async fn respond(path: &std::path::Path, head: bool) -> axum::response::Response {
    use axum::{
        body::Body,
        http::{header, StatusCode},
        response::Response,
    };
    use tokio::io::AsyncReadExt;
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let result = async {
        let file = tokio::fs::File::open(path).await?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes).await?;
        Ok::<_, std::io::Error>(bytes)
    }
    .await;
    let (status, body) = match result {
        Ok(bytes) if bytes.len() as u64 > MAX_BYTES => (StatusCode::PAYLOAD_TOO_LARGE, page("<h1>文件较大，无法预览</h1><p>Markdown 预览上限为 2 MiB，请查看源码或下载原文件。</p>", &filename)),
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(source) => {
                let output = tokio::task::spawn_blocking(move || render(&source, &filename)).await;
                match output { Ok(body) => (StatusCode::OK, body), Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "文档预览失败".into()) }
            }
            Err(_) => (StatusCode::UNPROCESSABLE_ENTITY, page("<h1>无法预览此文件编码</h1><p>预览仅支持 UTF-8，请下载原文件。</p>", &filename)),
        },
        Err(_) => (StatusCode::NOT_FOUND, "文件不可读取".into()),
    };
    let mut r = Response::new(if head {
        Body::empty()
    } else {
        Body::from(body)
    });
    *r.status_mut() = status;
    for (key, value) in [
        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
        (header::CACHE_CONTROL, "no-store"),
        (header::CONTENT_SECURITY_POLICY, CSP),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ] {
        r.headers_mut().insert(key, value.parse().unwrap());
    }
    r
}

pub fn page(body: &str, filename: &str) -> String {
    let title = crate::files::escape(filename);
    format!("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><title>{title} · 随享</title><style>{}</style></head><body><header><div class=\"toolbar\"><div class=\"identity\"><span class=\"brand\">随享 / 文档</span><strong>{title}</strong></div><nav aria-label=\"文档操作\"><a href=\"?raw=1\">查看源码</a><a href=\"?download=1\" download>下载原文件</a></nav></div></header><main><article>{body}</article><footer>随享 · Markdown 阅读预览</footer></main></body></html>", include_str!("markdown.css"))
}

pub fn render(source: &str, filename: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut events = Vec::new();
    let mut code: Option<(String, String)> = None;
    for event in Parser::new_ext(source, options) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(s) => {
                        s.split_whitespace().next().unwrap_or("").to_owned()
                    }
                    _ => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::Text(text) if code.is_some() => code.as_mut().unwrap().1.push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                let (lang, text) = code.take().unwrap();
                // Bound syntax work as well as file reads; very large blocks stay readable.
                let syntax = (text.len() <= 64 * 1024)
                    .then(|| SYNTAXES.find_syntax_by_token(&lang))
                    .flatten();
                // Unknown languages remain escaped plain code, rather than guessed syntax.
                let block = syntax
                    .and_then(|s| {
                        highlighted_html_for_string(
                            &text,
                            &SYNTAXES,
                            s,
                            &THEMES.themes["base16-ocean.dark"],
                        )
                        .ok()
                    })
                    .unwrap_or_else(|| {
                        format!("<pre><code>{}</code></pre>", crate::files::escape(&text))
                    });
                events.push(Event::Html(block.into()));
            }
            // Raw document HTML cannot introduce styles, scripts or embeds.
            Event::Html(_) | Event::InlineHtml(_) => {}
            other => events.push(other),
        }
    }
    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    let body = ammonia::Builder::default()
        .add_tags(&["input"])
        .add_tag_attributes("input", &["type", "checked", "disabled"])
        .add_tag_attributes("span", &["style"])
        .add_tag_attributes("pre", &["style"])
        .url_schemes(["http", "https", "mailto"].into_iter().collect())
        .clean(&body)
        .to_string();
    page(&body, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_document_images_and_highlighted_code() {
        let page = render("# 文档\n\n![本地](images/a%20b.png)\n\n![远程](https://example.com/a.png)\n\n|列|值|\n|-|-|\n|a|b|\n\n- [x] 完成\n\n```rust\nfn main() { println!(\"hi\"); }\n```", "测试.md");
        assert!(page.contains("<h1>文档</h1>"));
        assert!(page.contains("src=\"images/a%20b.png\""));
        assert!(page.contains("src=\"https://example.com/a.png\""));
        assert!(page.contains("<table>"));
        assert!(page.contains("checked"));
        assert!(page.contains("<span style="));
        assert!(page.contains("?raw=1"));
        assert!(page.contains("?download=1"));
    }

    #[test]
    fn escapes_code_and_removes_active_content() {
        let page = render("<script>alert('x')</script>\n\n<img src=x onerror=alert(1)>\n\n[x](javascript:alert%281%29)\n\n```unknown-language\n<script>evil()</script>\n```", "<img onerror=x>.md");
        assert!(!page.contains("<script>"));
        assert!(!page.contains("<img"));
        assert!(page.contains("&lt;img onerror=x&gt;.md"));
        assert!(!page.contains("href=\"javascript:"));
        assert!(page.contains("&lt;script&gt;evil()&lt;/script&gt;"));
    }

    #[test]
    fn large_code_blocks_fall_back_and_image_protocols_are_filtered() {
        let code = "let value = 1;\n".repeat(6000);
        let page = render(&format!("```rust\n{code}```\n\n![bad](javascript:evil)\n\n![bad](data:image/svg+xml,bad)\n\n[文档](../readme.md)"), "large.md");
        assert!(!page.contains("<span style="));
        assert!(!page.contains("src=\"javascript:"));
        assert!(!page.contains("src=\"data:"));
        assert!(page.contains("href=\"../readme.md\""));
        assert!(page.contains("let value = 1;"));
    }
}

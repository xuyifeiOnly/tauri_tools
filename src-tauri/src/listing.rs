use crate::files::escape;

pub struct Entry {
    pub name: String,
    pub href: String,
    pub directory: bool,
    pub size: Option<u64>,
    pub protected: bool,
}

pub fn encoded(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn kind(name: &str, directory: bool) -> (&'static str, &'static str) {
    if directory {
        return ("folder", "文件夹");
    }
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => ("pdf", "PDF 文档"),
        "md" | "markdown" => ("document", "Markdown"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "tif" | "tiff" | "avif" | "bmp" => {
            ("image", "图片")
        }
        "html" | "htm" | "js" | "ts" | "json" | "css" | "rs" | "py" => ("code", "代码文件"),
        "zip" | "gz" | "tar" | "rar" | "7z" => ("archive", "压缩文件"),
        "mp4" | "mov" | "webm" => ("media", "视频"),
        "mp3" | "wav" | "flac" => ("media", "音频"),
        _ => ("file", "文件"),
    }
}

fn icon(kind: &str) -> String {
    let shape = match kind {
        "folder" => "<path d='M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3Z'/>",
        "image" => "<rect x='3' y='3' width='18' height='18' rx='2'/><circle cx='8' cy='8' r='1.5'/><path d='m3 17 5-5 4 4 4-6 5 7'/>",
        "code" => "<path d='m8 7-5 5 5 5m8-10 5 5-5 5m-3-13-2 16'/>",
        "archive" => "<rect x='4' y='3' width='16' height='18' rx='2'/><path d='M12 3v12m-2-8h4m-4 4h4m-4 5h4v3h-4Z'/>",
        "media" => "<rect x='3' y='3' width='18' height='18' rx='2'/><path d='m10 8 6 4-6 4Z'/>",
        _ => "<path d='M14 2H6a2 2 0 0 0-2 2v16h16V8Zm0 0v6h6M8 13h8m-8 4h6'/>",
    };
    format!("<span class=\"icon {kind}\"><svg aria-hidden=\"true\" width=\"22\" height=\"22\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{shape}</svg></span>")
}

fn size(bytes: Option<u64>) -> String {
    match bytes {
        None => "—".into(),
        Some(n) if n < 1024 => format!("{n} B"),
        Some(n) if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.),
        Some(n) if n < 1024 * 1024 * 1024 => format!("{:.1} MB", n as f64 / (1024. * 1024.)),
        Some(n) => format!("{:.1} GB", n as f64 / (1024. * 1024. * 1024.)),
    }
}

pub fn render(
    title: &str,
    crumbs: &[(String, String)],
    parent: Option<&str>,
    mut entries: Vec<Entry>,
    home: bool,
) -> String {
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    let count = entries.len();
    let navigation = crumbs
        .iter()
        .map(|(label, href)| format!("<a href=\"{}\">{}</a>", escape(href), escape(label)))
        .collect::<Vec<_>>()
        .join("<span aria-hidden=\"true\">/</span>");
    let back = parent
        .map(|p| {
            format!(
                "<a class=\"back\" href=\"{}\"><span aria-hidden=\"true\">←</span> 返回上级</a>",
                escape(p)
            )
        })
        .unwrap_or_default();
    let rows = entries.iter().map(|e| {
        let (class, label) = kind(&e.name,e.directory);
        let tag = if home { if e.protected { "密码保护" } else { "无需密码" } } else { label };
        let badge = if home && e.protected { "badge protected" } else { "badge" };
        format!("<li><a class=\"row\" href=\"{}\">{}<span class=\"name\">{}</span><span class=\"{badge}\">{tag}</span><span class=\"size\">{}</span><span class=\"chevron\" aria-hidden=\"true\">›</span></a></li>",escape(&e.href),icon(class),escape(&e.name),if home { "进入目录".into() } else { size(e.size) })
    }).collect::<String>();
    let empty = if count == 0 {
        format!(
            "<div class=\"empty\">{}<h2>{}</h2><p>{}</p></div>",
            icon("folder"),
            if home {
                "暂无共享目录"
            } else {
                "此目录为空"
            },
            if home {
                "当前没有可访问的共享目录。"
            } else {
                "这里还没有可展示的文件或文件夹。"
            }
        )
    } else {
        String::new()
    };
    let title = escape(title);
    let subtitle = if home {
        "选择目录，浏览共享的文件与文档"
    } else {
        "浏览文件，点击即可打开"
    };
    format!("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"color-scheme\" content=\"light dark\"><title>{title} · 随享</title><style>{}</style></head><body><header><div class=\"topbar\"><a class=\"brand\" href=\"{}\">{}<span>随享<span class=\"brand-note\">文件共享</span></span></a><span class=\"readonly\">只读浏览</span></div></header><main><nav aria-label=\"当前位置\">{navigation}</nav><section class=\"heading\"><div><h1>{title}</h1><p>{subtitle}</p></div><span class=\"count\">{count} 项</span></section><section class=\"browser\" aria-label=\"目录内容\"><div class=\"list-heading\"><span>名称</span><span>类型 / 访问</span><span>大小</span></div><ul>{rows}</ul>{empty}</section>{back}<footer>随享 · 让文件分享更简单</footer></main></body></html>", include_str!("listing.css"), escape(crumbs.first().map(|c|c.1.as_str()).unwrap_or("./")), icon("folder"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_sorted_compact_listing() {
        let page = render(
            "<目录>",
            &[("共享目录".into(), "/my_mac/".into())],
            Some("/my_mac/"),
            vec![
                Entry {
                    name: "a.pdf".into(),
                    href: "./a.pdf".into(),
                    directory: false,
                    size: Some(2048),
                    protected: false,
                },
                Entry {
                    name: "Z & folder".into(),
                    href: "./Z%20%26%20folder/".into(),
                    directory: true,
                    size: None,
                    protected: false,
                },
            ],
            false,
        );
        assert!(page.find("Z &amp; folder").unwrap() < page.find("a.pdf").unwrap());
        assert!(page.contains("2.0 KB"));
        assert!(page.contains("PDF 文档"));
        assert!(page.contains("&lt;目录&gt;"));
        assert!(page.contains("返回上级"));
        assert!(page.contains("href=\"/my_mac/\""));
    }
}

//! 线路（数据源）：固定线路 + 地址发布页上的镜像线路。
//!
//! 上游只是域名在变，请求链路与数据模型完全一致，所以「切线路」等价于换 `net::base()`。
//! 镜像地址来自 <https://huang-guo.pages.dev/>：那个页面本身是空壳，真正的清单在
//! `publish.js` 里（`var urls` 域名数组 + `var subdomains` 子域数组，页面自己随机拼）。
//!
//! 2026-11-17：旧发布页 `huangguoai.pages.dev` 已下线，官方迁到 `huang-guo.pages.dev`；
//! 镜像根域名也轮换为 `fvlelaqt.cc` / `bmikbxhi.cc`（从新 publish.js 自动读取，无需改代码）。
//! 固定线路的 `huangguoai.com` / `.ai` 仍是官方声明的不变地址，但大陆网络通常需代理。

use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// 固定线路：写死在客户端里，不依赖任何外部页面。
/// 2026-11-27：`huangguoai.ai` 已改为静态导航页，移除出业务线路列表。
pub const FIXED: [(&str, &str); 1] = [("固定线路（官方主站）", "https://huangguoai.com")];

const PUBLISH_PAGE: &str = "https://huang-guo.pages.dev/";
const FALLBACK_NAV_PAGE: &str = "https://huangguoai.ai/";
const DEFAULT_SUBDOMAINS: [&str; 19] = [
    "thu", "pku", "fdu", "sjtu", "zju", "nju", "xmu", "sysu", "whu", "jlu", "scu", "ruc", "bnu",
    "nku", "tju", "sdu", "hust", "xjtu", "hit",
];
/// 镜像列表上限（子域 × 域名是笛卡尔积，界面只展示这么多）
const MIRROR_MAX: usize = 60;
/// 单条线路探测超时
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub fn fixed() -> Value {
    json!(FIXED
        .iter()
        .map(|(name, url)| json!({ "name": name, "url": url }))
        .collect::<Vec<_>>())
}

fn build_mirror_items(domains: Vec<String>, subs: Vec<String>) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    'outer: for domain in domains {
        let domain = domain.trim_end_matches('/').to_string();
        if domain.is_empty() {
            continue;
        }
        let hosts: Vec<String> = if subs.is_empty() {
            vec![domain.clone()]
        } else {
            subs.iter().map(|s| format!("{s}.{domain}")).collect()
        };
        for host in hosts {
            if !seen.insert(host.clone()) {
                continue;
            }
            items.push(json!({ "url": format!("https://{host}"), "host": host }));
            if items.len() >= MIRROR_MAX {
                break 'outer;
            }
        }
    }
    items
}

async fn mirrors_from_publish_page() -> Result<Value, String> {
    let html = get_text(PUBLISH_PAGE).await?;
    let js_rel = script_src(&html).unwrap_or_else(|| "publish.js".to_string());
    let js_url = absolute(PUBLISH_PAGE, &js_rel);
    let js = get_text(&js_url).await?;

    let domains = js_array(&js, "var urls");
    let subs = js_array(&js, "var subdomains");
    if domains.is_empty() {
        return Err("地址发布页格式有变化：没找到镜像域名".to_string());
    }

    let items = build_mirror_items(domains, subs);
    if items.is_empty() {
        return Err("地址发布页里没有可用的镜像地址".to_string());
    }

    Ok(json!({ "source": PUBLISH_PAGE, "script": js_url, "items": items }))
}

fn extract_nav_tails(html: &str) -> Vec<String> {
    let mut tails = Vec::new();
    let mut rest = html;
    let needle = "data-route-prefix-tail=\"";
    while let Some(i) = rest.find(needle) {
        let after = &rest[i + needle.len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let candidate = after[..end].trim();
        if !candidate.is_empty() && !tails.iter().any(|x| x == candidate) {
            tails.push(candidate.to_string());
        }
        rest = &after[end..];
    }
    tails
}

async fn mirrors_from_nav_page() -> Result<Value, String> {
    let html = get_text(FALLBACK_NAV_PAGE).await?;
    let tails = extract_nav_tails(&html);
    if tails.is_empty() {
        return Err("备用导航页未包含镜像根域名".to_string());
    }
    let subs = DEFAULT_SUBDOMAINS.iter().map(|s| s.to_string()).collect();
    let items = build_mirror_items(tails, subs);
    if items.is_empty() {
        return Err("备用导航页没有可用的镜像地址".to_string());
    }
    Ok(json!({ "source": FALLBACK_NAV_PAGE, "items": items }))
}

/// 拉地址发布页 → 解析 publish.js → 拼出镜像地址（失败时自动回退到官方导航页 `huangguoai.ai`）。
pub async fn mirrors() -> Result<Value, String> {
    match mirrors_from_publish_page().await {
        Ok(v) => Ok(v),
        Err(primary_err) => mirrors_from_nav_page().await.map_err(|_| primary_err),
    }
}

/// 实测线路 API 握手可用性。
pub async fn probe_api(origin: &str) -> Result<u64, (Option<u16>, String)> {
    crate::edge_crypto::probe_hello(origin, PROBE_TIMEOUT).await
}

/// 可达性探测（并发 6 条一组）。只报结果，不做自动切换。
pub async fn probe(urls: Vec<String>) -> Value {
    let mut out: Vec<Value> = Vec::new();
    for chunk in urls.chunks(6) {
        let mut handles = Vec::new();
        for u in chunk {
            let u = u.clone();
            handles.push(tokio::spawn(async move { probe_one(u).await }));
        }
        for h in handles {
            if let Ok(v) = h.await {
                out.push(v);
            }
        }
    }
    json!(out)
}

async fn probe_one(url: String) -> Value {
    let base = url.trim_end_matches('/');
    let started = Instant::now();
    match probe_api(base).await {
        Ok(ms) => json!({ "url": url, "ok": true, "status": 200, "ms": ms }),
        Err((status, e)) => {
            let ms = started.elapsed().as_millis() as u64;
            json!({ "url": url, "ok": false, "status": status, "ms": ms, "error": e })
        }
    }
}

async fn get_text(url: &str) -> Result<String, String> {
    let resp = crate::net::get(url, Duration::from_secs(12), None, None).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| crate::net::err_text(&e))
}

/// 从 HTML 里挑出脚本地址（发布页只有一个 `publish.js?<版本>`）。
fn script_src(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(i) = rest.find("src=\"") {
        let after = &rest[i + 5..];
        let end = after.find('"')?;
        let candidate = &after[..end];
        if candidate.contains(".js") {
            return Some(candidate.to_string());
        }
        rest = &after[end..];
    }
    None
}

fn absolute(base: &str, rel: &str) -> String {
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    let origin = url::Url::parse(base)
        .ok()
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or_default()));
    match origin {
        Some(o) if rel.starts_with('/') => format!("{o}{rel}"),
        Some(o) => format!("{o}/{rel}"),
        None => rel.to_string(),
    }
}

/// 取 JS 里 `var <name> = [ ... ]` 的字符串数组（去掉行尾 `//` 注释，但保留字符串里的 `//`）。
fn js_array(src: &str, decl: &str) -> Vec<String> {
    let Some(at) = src.find(decl) else {
        return Vec::new();
    };
    let rest = &src[at..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find(']') else {
        return Vec::new();
    };
    strip_line_comments(&rest[open + 1..open + close])
        .split(',')
        .map(|raw| {
            raw.trim()
                .trim_matches(|c: char| c == '\'' || c == '"')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// 去掉 `//` 行尾注释（引号内的 `//` 是 URL，不能动）。
fn strip_line_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut quote: Option<char> = None;
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                out.push(c);
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                    out.push(c);
                } else if c == '/' && chars.peek() == Some(&'/') {
                    for n in chars.by_ref() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
var subdomains = [
    'thu',  // 清华
    'pku',  // 北大
];
var urls=[
	'xxbgwypx.cc/',
];
var otherUrls = [
	'https://huangguoai.pages.dev',
];
var foreverUrls = [
	  'https://huangguoai.com',
	JumpPage,
	'https://huangguoai.ai',
];
"#;

    #[test]
    fn parses_publish_js_arrays() {
        assert_eq!(js_array(SAMPLE, "var urls"), vec!["xxbgwypx.cc/"]);
        assert_eq!(js_array(SAMPLE, "var subdomains"), vec!["thu", "pku"]);
        assert_eq!(
            js_array(SAMPLE, "var foreverUrls"),
            vec![
                "https://huangguoai.com",
                "JumpPage",
                "https://huangguoai.ai"
            ]
        );
    }

    #[test]
    fn resolves_relative_script() {
        assert_eq!(
            absolute("https://huangguoai.pages.dev/", "publish.js?20261103-2"),
            "https://huangguoai.pages.dev/publish.js?20261103-2"
        );
        assert_eq!(
            absolute("https://huangguoai.pages.dev/", "/a/b.js"),
            "https://huangguoai.pages.dev/a/b.js"
        );
    }

    #[test]
    fn picks_script_src() {
        let html = r#"<link rel="stylesheet" href="publish.css?1" /><script type="text/javascript" src="publish.js?20261103-2"></script>"#;
        assert_eq!(script_src(html).as_deref(), Some("publish.js?20261103-2"));
    }

    #[test]
    fn extracts_nav_tails_uniquely() {
        let html = r#"
            <article data-go-home-route data-route-prefix-tail="qxeylvrkf.cc"></article>
            <article data-go-home-route data-route-prefix-tail="qxeylvrkf.cc"></article>
            <article data-go-home-route data-route-prefix-tail="hvgbwwrxd.cc"></article>
        "#;
        assert_eq!(
            extract_nav_tails(html),
            vec!["qxeylvrkf.cc", "hvgbwwrxd.cc"]
        );
    }

    /// 手动跑：`cargo test --lib -- --ignored lines::tests::mirrors_live -- --nocapture`
    #[tokio::test]
    #[ignore = "联网测试：请求上游镜像列表，手动用 --ignored 运行"]
    async fn mirrors_live() {
        match mirrors().await {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v).expect("json")),
            Err(e) => panic!("mirrors failed: {e}"),
        }
    }

    /// 手动跑：`cargo test --lib -- --ignored lines::tests::probe_live -- --nocapture`
    #[tokio::test]
    #[ignore = "联网测试：探测上游线路连通性，手动用 --ignored 运行"]
    async fn probe_live() {
        let urls = vec![
            "https://huangguoai.com".to_string(),
            "https://huangguoai.ai".to_string(),
            "https://thu.xxbgwypx.cc".to_string(),
        ];
        println!(
            "{}",
            serde_json::to_string_pretty(&probe(urls).await).expect("json")
        );
    }
}

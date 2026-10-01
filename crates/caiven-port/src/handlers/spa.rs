//! Serves the built SPA's `index.html` for any GET path that isn't an API
//! route or a static asset — the client-side router handles the rest.

use std::path::PathBuf;

use rocket::{
    State,
    fs::NamedFile,
    get,
    response::content::{RawHtml, RawText, RawXml},
};

use crate::{PortState, db, handlers::valid_id, mailer::escape_html};

#[get("/<path..>", rank = 20)]
pub async fn fallback(path: PathBuf, state: &State<PortState>) -> Option<NamedFile> {
    if path.starts_with("api") {
        return None;
    }
    NamedFile::open(state.web_dir.join("index.html")).await.ok()
}

// `/api/` stays crawlable: the rendered SPA fetches its content from there.
#[get("/robots.txt")]
pub fn robots(state: &State<PortState>) -> RawText<String> {
    let mut body = String::from("User-agent: *\n");
    for path in [
        "/admin",
        "/dashboard",
        "/settings",
        "/profile",
        "/library",
        "/activity",
        "/upload",
        "/login",
        "/register",
        "/verify-email",
        "/forgot-password",
        "/reset-password",
        "/link-studio",
        "/report",
        "/remix/",
    ] {
        body.push_str(&format!("Disallow: {path}\n"));
    }
    body.push_str(&format!("\nSitemap: {}/sitemap.xml\n", origin(state)));
    RawText(body)
}

#[get("/sitemap.xml")]
pub async fn sitemap(state: &State<PortState>) -> RawXml<String> {
    let origin = origin(state);
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for path in [
        "",
        "/browse",
        "/tags",
        "/collections",
        "/jams",
        "/privacy",
        "/terms",
    ] {
        xml.push_str(&format!("<url><loc>{origin}{path}</loc></url>"));
    }
    // A DB hiccup still serves the static pages rather than a 500.
    for (id, uploaded_at) in db::sitemap_carts(&state.db).await.unwrap_or_default() {
        xml.push_str(&format!(
            "<url><loc>{origin}/cart/{}</loc><lastmod>{}</lastmod></url>",
            escape_html(&id),
            escape_html(&uploaded_at)
        ));
    }
    xml.push_str("</urlset>");
    RawXml(xml)
}

/// Crawlers need absolute URLs; a wrong origin only breaks previews and crawling.
fn origin(state: &PortState) -> &str {
    state.base_url.as_deref().unwrap_or(&state.local_origin)
}

#[get("/play/<id>", rank = 19)]
pub async fn play_page(id: &str, state: &State<PortState>) -> Option<RawHtml<String>> {
    render_cart_page(id, "play", state).await
}

#[get("/cart/<id>", rank = 19)]
pub async fn cart_detail_page(id: &str, state: &State<PortState>) -> Option<RawHtml<String>> {
    render_cart_page(id, "cart", state).await
}

/// `index.html` with link-preview tags, so a shared cart link unfurls as the
/// game (title, cover, lineage) instead of a generic "Caiven Port".
async fn render_cart_page(
    id: &str,
    kind: &str,
    state: &State<PortState>,
) -> Option<RawHtml<String>> {
    let html = tokio::fs::read_to_string(state.web_dir.join("index.html"))
        .await
        .ok()?;
    let cart = if valid_id(id) {
        db::get(&state.db, id).await.ok().flatten()
    } else {
        None
    };
    let Some(cart) = cart else {
        return Some(RawHtml(html));
    };
    let parent = match &cart.parent_cart_id {
        Some(parent_id) => db::get(&state.db, parent_id).await.ok().flatten(),
        None => None,
    };

    let creator = cart.owner.as_deref().unwrap_or(&cart.author);
    let mut description = match &parent {
        Some(parent) => format!(
            "@{creator} remixed {} by @{}.",
            parent.title,
            parent.owner.as_deref().unwrap_or(&parent.author)
        ),
        None => format!("A tiny game by @{creator}."),
    };
    if !cart.description.trim().is_empty() {
        description = format!("{description} {}", cart.description.trim());
    }
    description.push_str(if cart.remixable {
        " Play it in your browser, then remix it."
    } else {
        " Play it in your browser."
    });

    let origin = origin(state);
    let mut tags = vec![
        meta("description", &description),
        format!(
            "<link rel=\"canonical\" href=\"{}\" />",
            escape_html(&format!("{origin}/cart/{}", cart.id))
        ),
        meta("og:type", "website"),
        meta("og:site_name", "Caiven"),
        meta("og:title", &cart.title),
        meta("og:description", &description),
        meta("og:url", &format!("{origin}/{kind}/{}", cart.id)),
        meta("twitter:title", &cart.title),
        meta("twitter:description", &description),
    ];
    if cart.has_screenshot {
        let image = format!("{origin}/api/v1/carts/{}/screenshot", cart.id);
        tags.push(meta("og:image", &image));
        tags.push(meta("twitter:image", &image));
        tags.push(meta("twitter:card", "summary_large_image"));
    } else {
        tags.push(meta("twitter:card", "summary"));
    }
    Some(RawHtml(with_head(
        &html,
        &format!("{} · Caiven", cart.title),
        &tags.concat(),
    )))
}

fn meta(property: &str, content: &str) -> String {
    let attr = if property.starts_with("og:") {
        "property"
    } else {
        "name"
    };
    format!(
        "<meta {attr}=\"{property}\" content=\"{}\" />",
        escape_html(content)
    )
}

fn with_head(html: &str, title: &str, tags: &str) -> String {
    let title = format!("<title>{}</title>", escape_html(title));
    let html = match (html.find("<title>"), html.find("</title>")) {
        (Some(start), Some(end)) if start < end => {
            format!(
                "{}{title}{}",
                &html[..start],
                &html[end + "</title>".len()..]
            )
        }
        _ => html.to_string(),
    };
    // Cart tags replace the shell's site-wide defaults between the markers.
    const START: &str = "<!-- meta -->";
    const END: &str = "<!-- /meta -->";
    match (html.find(START), html.find(END), html.find("</head>")) {
        (Some(start), Some(end), _) if start < end => {
            format!("{}{tags}{}", &html[..start], &html[end + END.len()..])
        }
        (_, _, Some(at)) => format!("{}{tags}{}", &html[..at], &html[at..]),
        _ => html,
    }
}

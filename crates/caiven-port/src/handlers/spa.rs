//! Serves the built SPA's `index.html` for any GET path that isn't an API
//! route or a static asset — the client-side router handles the rest.

use std::path::PathBuf;

use rocket::{State, fs::NamedFile, get, response::content::RawHtml};

use crate::{PortState, db, handlers::valid_id};

#[get("/<path..>", rank = 20)]
pub async fn fallback(path: PathBuf, state: &State<PortState>) -> Option<NamedFile> {
    if path.starts_with("api") {
        return None;
    }
    NamedFile::open(state.web_dir.join("index.html")).await.ok()
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

    // Preview crawlers need absolute URLs; a wrong origin only breaks a preview.
    let origin = state.base_url.as_deref().unwrap_or(&state.local_origin);
    let mut tags = vec![
        meta("og:type", "website"),
        meta("og:site_name", "Caiven"),
        meta("og:title", &cart.title),
        meta("og:description", &description),
        meta("og:url", &format!("{origin}/{kind}/{}", cart.id)),
        meta("twitter:title", &cart.title),
        meta("twitter:description", &description),
    ];
    if cart.has_screenshot {
        let image = format!("{origin}/api/v2/carts/{}/screenshot", cart.id);
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
    match html.find("</head>") {
        Some(at) => format!("{}{tags}{}", &html[..at], &html[at..]),
        None => html,
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

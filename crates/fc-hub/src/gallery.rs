use crate::models::Cart;

pub fn render(carts: &[Cart], total: u64, page: u32, per_page: u32, query: Option<&str>) -> String {
    let cards = if carts.is_empty() {
        r#"<div class="empty">No carts found.</div>"#.to_string()
    } else {
        carts.iter().map(render_card).collect::<Vec<_>>().join("\n")
    };

    let total_pages = ((total as u32).saturating_add(per_page - 1)) / per_page;
    let pagination = render_pagination(page, total_pages, query);
    let query_val = html_escape(query.unwrap_or(""));
    let clear = if query.is_some() {
        r#"<a href="/" style="color:#ff9f43;padding:8px 12px;border:1px solid #ff9f43;text-decoration:none">Clear</a>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Fantasy Console Hub</title>
  <style>
    *{{box-sizing:border-box;margin:0;padding:0}}
    body{{background:#1a1a2e;color:#e0e0e0;font-family:monospace;padding:24px;min-height:100vh}}
    h1{{color:#ff9f43;margin-bottom:20px;font-size:1.4em;letter-spacing:3px}}
    .search{{display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap;align-items:center}}
    .search input{{background:#16213e;border:1px solid #ff9f43;color:#e0e0e0;padding:8px 12px;font-family:monospace;font-size:0.9em;flex:1;min-width:200px;outline:none}}
    .search input:focus{{border-color:#ffd32a}}
    .search button{{background:#ff9f43;border:none;color:#1a1a2e;padding:8px 16px;cursor:pointer;font-family:monospace;font-weight:bold}}
    .search button:hover{{background:#ffd32a}}
    .meta{{color:#666;font-size:0.78em;margin-bottom:18px}}
    .grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(176px,1fr));gap:12px}}
    .card{{background:#16213e;border:1px solid #0f3460;padding:12px;display:flex;flex-direction:column;transition:border-color 0.1s}}
    .card:hover{{border-color:#ff9f43}}
    .thumb{{width:128px;height:128px;display:block;margin:0 auto 10px;image-rendering:pixelated;object-fit:cover}}
    .no-thumb{{width:128px;height:128px;display:flex;align-items:center;justify-content:center;margin:0 auto 10px;background:#0f3460;color:#2a3a60;font-size:3em;font-weight:bold;user-select:none}}
    .card-title{{color:#ff9f43;font-size:0.85em;font-weight:bold;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin-bottom:3px}}
    .card-author{{color:#888;font-size:0.75em;margin-bottom:8px}}
    .card-desc{{font-size:0.72em;color:#bbb;flex:1;overflow:hidden;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;margin-bottom:8px;min-height:3em}}
    .tags{{display:flex;flex-wrap:wrap;gap:3px;margin-bottom:8px}}
    .tag{{background:#0f3460;color:#88aaff;padding:1px 5px;font-size:0.65em}}
    .card-footer{{display:flex;justify-content:space-between;align-items:center;margin-top:auto}}
    .dl-count{{color:#555;font-size:0.7em}}
    .dl-btn{{background:#ff9f43;color:#1a1a2e;text-decoration:none;padding:3px 10px;font-size:0.75em;font-family:monospace;font-weight:bold}}
    .dl-btn:hover{{background:#ffd32a}}
    .pagination{{margin-top:20px;display:flex;gap:6px;flex-wrap:wrap}}
    .pagination a{{color:#ff9f43;text-decoration:none;padding:4px 10px;border:1px solid #ff9f43;font-size:0.82em}}
    .pagination a:hover,.pagination a.cur{{background:#ff9f43;color:#1a1a2e}}
    .empty{{color:#666;text-align:center;padding:60px 20px;font-size:0.9em}}
  </style>
</head>
<body>
  <h1>FANTASY CONSOLE HUB</h1>
  <form class="search" method="get" action="/">
    <input name="q" placeholder="Search carts, authors..." value="{qv}">
    <button type="submit">Search</button>
    {clear}
  </form>
  <div class="meta">{total} cart(s) &nbsp;·&nbsp; upload via POST /api/carts</div>
  <div class="grid">{cards}</div>
  {pagination}
</body>
</html>"#,
        qv = query_val,
        clear = clear,
        total = total,
        cards = cards,
        pagination = pagination,
    )
}

fn render_card(cart: &Cart) -> String {
    let thumb = if cart.has_screenshot {
        format!(
            r#"<img class="thumb" src="/api/carts/{}/screenshot" alt="screenshot" loading="lazy">"#,
            cart.id
        )
    } else {
        r#"<div class="no-thumb">FC</div>"#.to_string()
    };

    let tags = if cart.tags.is_empty() {
        String::new()
    } else {
        let inner: String = cart
            .tags
            .iter()
            .take(4)
            .map(|t| format!(r#"<span class="tag">{}</span>"#, html_escape(t)))
            .collect();
        format!(r#"<div class="tags">{}</div>"#, inner)
    };

    let size_kb = (cart.rom_size.max(0) as u64 + 511) / 1024;

    format!(
        r#"<div class="card">
  {thumb}
  <div class="card-title" title="{title}">{title}</div>
  <div class="card-author">by {author}</div>
  {tags}
  <div class="card-desc">{desc}</div>
  <div class="card-footer">
    <span class="dl-count">{dl} dl · {size}KB</span>
    <a class="dl-btn" href="/api/carts/{id}/rom">Get</a>
  </div>
</div>"#,
        thumb = thumb,
        title = html_escape(&cart.title),
        author = html_escape(&cart.author),
        tags = tags,
        desc = html_escape(&cart.description),
        dl = cart.downloads,
        size = size_kb,
        id = cart.id,
    )
}

fn render_pagination(page: u32, total_pages: u32, query: Option<&str>) -> String {
    if total_pages <= 1 {
        return String::new();
    }
    let qp = query
        .map(|q| format!("&q={}", url_encode(q)))
        .unwrap_or_default();

    let start = page.saturating_sub(3);
    let end = (page + 4).min(total_pages);
    let mut out = String::from(r#"<div class="pagination">"#);

    if page > 0 {
        out.push_str(&format!(
            r#"<a href="/?page={p}{q}">Prev</a>"#,
            p = page - 1,
            q = qp
        ));
    }
    for p in start..end {
        let cls = if p == page { r#" class="cur""# } else { "" };
        out.push_str(&format!(
            r#"<a href="/?page={p}{q}"{cls}>{n}</a>"#,
            p = p,
            q = qp,
            cls = cls,
            n = p + 1
        ));
    }
    if page + 1 < total_pages {
        out.push_str(&format!(
            r#"<a href="/?page={p}{q}">Next</a>"#,
            p = page + 1,
            q = qp
        ));
    }
    out.push_str("</div>");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

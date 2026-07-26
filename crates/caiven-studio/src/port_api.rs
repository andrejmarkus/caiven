use crate::port_client::{build_multipart, capture_screenshot};
use caiven_vm::VmConfig;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

const SESSION_COOKIE: &str = "caiven_session";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortCart {
    pub id: String,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub rating_avg: f64,
    #[serde(default)]
    pub rating_count: i64,
    #[serde(default)]
    pub latest_version: i32,
    #[serde(default)]
    pub cart_size: i64,
    #[serde(default)]
    pub has_screenshot: bool,
    #[serde(skip_deserializing, default)]
    pub screenshot_url: String,
}

#[derive(Deserialize)]
struct PortCartListWire {
    carts: Vec<PortCart>,
    total: u64,
    #[serde(default)]
    page: u32,
    #[serde(default)]
    per_page: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortCartList {
    carts: Vec<PortCart>,
    total: u64,
    page: u32,
    per_page: u32,
    port_url: String,
}

#[derive(Deserialize)]
struct TokenCreated {
    token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortSession {
    authenticated: bool,
    username: String,
    port_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalCart {
    path: String,
    name: String,
    title: String,
    author: String,
    modified: u64,
    project: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishProgress {
    pub step: String,
    pub pct: u8,
    pub note: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishResult {
    pub cart_id: String,
    pub version: Option<i32>,
}

pub(crate) struct PublishMeta {
    pub title: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub changelog: String,
    pub target_cart_id: Option<String>,
    pub frames: u32,
}

fn port_url() -> String {
    std::env::var("CAIVEN_PORT_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn token_file_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(
            PathBuf::from(appdata)
                .join("caiven-studio")
                .join("port_token"),
        );
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/caiven-studio/port_token"))
}

fn load_token() -> Option<(String, String)> {
    if let Ok(token) = std::env::var("CAIVEN_PORT_API_KEY")
        && !token.is_empty()
    {
        return Some(("API key".to_string(), token));
    }
    let text = std::fs::read_to_string(token_file_path()?).ok()?;
    let mut lines = text.lines();
    Some((lines.next()?.to_string(), lines.next()?.to_string()))
}

fn save_token(username: &str, token: &str) -> Result<(), String> {
    let path = token_file_path().ok_or_else(|| "No config directory available".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, format!("{username}\n{token}"))
        .map_err(|error| format!("Could not save port token: {error}"))
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn error_message(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, response) => {
            let body: serde_json::Value =
                serde_json::from_reader(response.into_reader()).unwrap_or_default();
            body.get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("Port returned HTTP {code}"))
        }
        ureq::Error::Transport(error) => format!("Port unavailable: {error}"),
    }
}

fn parse_session_cookie(response: &ureq::Response) -> Option<String> {
    response
        .header("Set-Cookie")?
        .split(';')
        .next()?
        .split_once('=')
        .filter(|(name, _)| name.trim() == SESSION_COOKIE)
        .map(|(_, value)| value.trim().to_string())
}

#[tauri::command]
pub(crate) fn port_session() -> PortSession {
    let saved = load_token();
    PortSession {
        authenticated: saved.is_some(),
        username: saved.map(|value| value.0).unwrap_or_default(),
        port_url: port_url(),
    }
}

#[tauri::command]
pub(crate) fn port_login(username: String, password: String) -> Result<PortSession, String> {
    let base = port_url();
    let response = ureq::post(&format!("{base}/api/v2/auth/login"))
        .set("Content-Type", "application/json")
        .send_string(&serde_json::json!({ "username": username, "password": password }).to_string())
        .map_err(error_message)?;
    let session = parse_session_cookie(&response)
        .ok_or_else(|| "Login returned no session cookie".to_string())?;
    let token_response = ureq::post(&format!("{base}/api/v2/auth/tokens"))
        .set("Cookie", &format!("{SESSION_COOKIE}={session}"))
        .set("Content-Type", "application/json")
        .send_string(&serde_json::json!({ "name": "Studio" }).to_string())
        .map_err(error_message)?;
    let token: TokenCreated = serde_json::from_reader(token_response.into_reader())
        .map_err(|error| format!("Invalid token response: {error}"))?;
    save_token(&username, &token.token)?;
    Ok(PortSession {
        authenticated: true,
        username,
        port_url: base,
    })
}

#[tauri::command]
pub(crate) fn port_logout() -> Result<PortSession, String> {
    if let Some(path) = token_file_path() {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(port_session())
}

#[tauri::command]
pub(crate) fn port_list_carts(
    query: String,
    sort: String,
    page: u32,
) -> Result<PortCartList, String> {
    let base = port_url();
    let sort = match sort.as_str() {
        "popular" | "trending" | "top" => sort,
        _ => "new".to_string(),
    };
    let mut url = format!("{base}/api/v2/carts?page={page}&per_page=24&sort={sort}");
    if !query.trim().is_empty() {
        url.push_str("&q=");
        url.push_str(&url_encode(query.trim()));
    }
    let response = ureq::get(&url).call().map_err(error_message)?;
    let mut list: PortCartListWire = serde_json::from_reader(response.into_reader())
        .map_err(|error| format!("Invalid cart list: {error}"))?;
    for cart in &mut list.carts {
        if cart.has_screenshot {
            cart.screenshot_url = format!("{base}/api/v2/carts/{}/screenshot", cart.id);
        }
    }
    Ok(PortCartList {
        carts: list.carts,
        total: list.total,
        page: list.page,
        per_page: list.per_page,
        port_url: base,
    })
}

#[tauri::command]
pub(crate) fn port_download(id: String, title: String) -> Result<String, String> {
    let url = format!("{}/api/v2/carts/{id}/cart", port_url());
    let mut bytes = Vec::new();
    ureq::get(&url)
        .call()
        .map_err(error_message)?
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let safe: String = title
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || char == '-' {
                char
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    let dir = std::env::temp_dir().join("caiven-port");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(format!("{safe}.cav"));
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub(crate) fn studio_scan_library(path: PathBuf) -> Result<Vec<LocalCart>, String> {
    let mut carts = Vec::new();
    let entries = std::fs::read_dir(&path)
        .map_err(|error| format!("Could not scan {}: {error}", path.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let project = caiven_cart::is_project(&path);
        if !project && path.extension().and_then(|value| value.to_str()) != Some("cav") {
            continue;
        }
        let cart = if project {
            caiven_cart::load_project(&path).ok()
        } else {
            caiven_cart::load(&path).ok()
        };
        let Some(cart) = cart else { continue };
        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        carts.push(LocalCart {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: path.display().to_string(),
            title: cart.header.title,
            author: cart.header.author,
            modified,
            project,
        });
    }
    carts.sort_by_key(|cart| std::cmp::Reverse(cart.modified));
    Ok(carts)
}

pub(crate) fn publish(
    packed: &Path,
    meta: PublishMeta,
    mut progress: impl FnMut(PublishProgress),
) -> Result<PublishResult, String> {
    let (_, token) = load_token().ok_or_else(|| "Log in to port before publishing".to_string())?;
    let base = port_url();
    let cart =
        caiven_cart::load(packed).map_err(|error| format!("Packed cart invalid: {error}"))?;
    let cart_bytes = std::fs::read(packed).map_err(|error| error.to_string())?;
    progress(PublishProgress {
        step: "cover".into(),
        pct: 25,
        note: "Capturing cover".into(),
    });
    let screenshot = capture_screenshot(&cart, VmConfig::default(), meta.frames)
        .map_err(|error| format!("Cover capture failed: {error:#}"))?;
    progress(PublishProgress {
        step: "upload".into(),
        pct: 50,
        note: "Uploading cartridge".into(),
    });

    let boundary = "----CaivenStudioBoundary7x3k9p";
    let filename = packed
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cart.cav");
    let (url, metadata) = match &meta.target_cart_id {
        Some(id) => (
            format!("{base}/api/v2/carts/{id}/versions"),
            serde_json::json!({ "changelog": meta.changelog }),
        ),
        None => (
            format!("{base}/api/v2/carts"),
            serde_json::json!({ "title": meta.title, "author": meta.author, "description": meta.description, "tags": meta.tags }),
        ),
    };
    let metadata = metadata.to_string();
    let body = build_multipart(
        boundary,
        &[
            ("meta", None, "application/json", metadata.as_bytes()),
            (
                "cart",
                Some(filename),
                "application/octet-stream",
                &cart_bytes,
            ),
        ],
    );
    let response = ureq::post(&url)
        .set("X-Api-Key", &token)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&body)
        .map_err(error_message)?;
    let payload: serde_json::Value =
        serde_json::from_reader(response.into_reader()).unwrap_or_default();
    let cart_id = meta
        .target_cart_id
        .or_else(|| {
            payload
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| "Upload response missing cart id".to_string())?;

    progress(PublishProgress {
        step: "cover".into(),
        pct: 75,
        note: "Uploading cover".into(),
    });
    let shot_boundary = "----CaivenStudioScreenshotBoundary";
    let shot_body = build_multipart(
        shot_boundary,
        &[(
            "screenshot",
            Some("screenshot.png"),
            "image/png",
            &screenshot,
        )],
    );
    ureq::post(&format!("{base}/api/v2/carts/{cart_id}/screenshot"))
        .set("X-Api-Key", &token)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={shot_boundary}"),
        )
        .send_bytes(&shot_body)
        .map_err(error_message)?;
    progress(PublishProgress {
        step: "notify".into(),
        pct: 100,
        note: "Published".into(),
    });
    Ok(PublishResult {
        cart_id,
        version: payload
            .get("latest_version")
            .and_then(|value| value.as_i64())
            .map(|value| value as i32),
    })
}

#[cfg(test)]
mod tests {
    use super::url_encode;

    #[test]
    fn encodes_port_query() {
        assert_eq!(url_encode("cave cart/α"), "cave%20cart%2F%CE%B1");
        assert_eq!(url_encode("tag-safe_1"), "tag-safe_1");
    }
}

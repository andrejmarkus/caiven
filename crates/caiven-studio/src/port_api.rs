use crate::port_client::{build_multipart, capture_screenshot};
use caiven_vm::VmConfig;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri_plugin_opener::OpenerExt;

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
#[serde(rename_all = "snake_case")]
struct StudioLinkStart {
    request_id: String,
    poll_secret: String,
    browser_url: String,
    expires_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct StudioLinkPoll {
    status: String,
    username: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortLinkPending {
    pub request_id: String,
    pub poll_secret: String,
    pub expires_at: String,
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
    pub description: String,
    pub tags: Vec<String>,
    pub changelog: String,
    pub target_cart_id: Option<String>,
    pub frames: u32,
}

fn config_dir() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(PathBuf::from(appdata).join("caiven-studio"));
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/caiven-studio"))
}

fn token_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("port_token"))
}

fn url_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("port_url"))
}

/// Trims trailing slashes and rejects anything that isn't a well-formed
/// `http(s)://` URL. This is a Tauri IPC input boundary — validate before
/// persisting or using it to build outgoing requests.
fn validate_port_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Server URL cannot be empty".to_string());
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("Server URL must start with http:// or https://".to_string());
    }
    if trimmed.len() <= "https://".len() {
        return Err("Server URL is missing a host".to_string());
    }
    Ok(trimmed.to_string())
}

fn load_saved_url() -> Option<String> {
    let text = std::fs::read_to_string(url_file_path()?).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn port_url() -> String {
    if let Some(saved) = load_saved_url() {
        return saved;
    }
    std::env::var("CAIVEN_PORT_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
        .trim_end_matches('/')
        .to_string()
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
pub(crate) fn port_link_start(app: tauri::AppHandle) -> Result<PortLinkPending, String> {
    let base = port_url();
    let response = ureq::post(&format!("{base}/api/v2/auth/studio-link"))
        .send_string("")
        .map_err(error_message)?;
    let link: StudioLinkStart = serde_json::from_reader(response.into_reader())
        .map_err(|error| format!("Invalid Studio link response: {error}"))?;
    app.opener()
        .open_url(&link.browser_url, None::<&str>)
        .map_err(|error| error.to_string())?;
    Ok(PortLinkPending {
        request_id: link.request_id,
        poll_secret: link.poll_secret,
        expires_at: link.expires_at,
    })
}

#[tauri::command]
pub(crate) fn port_link_poll(
    request_id: String,
    poll_secret: String,
) -> Result<Option<PortSession>, String> {
    let base = port_url();
    let response = ureq::post(&format!("{base}/api/v2/auth/studio-link/poll"))
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::json!({ "request_id": request_id, "poll_secret": poll_secret })
                .to_string(),
        )
        .map_err(error_message)?;
    let link: StudioLinkPoll = serde_json::from_reader(response.into_reader())
        .map_err(|error| format!("Invalid Studio link poll response: {error}"))?;
    if link.status == "pending" {
        return Ok(None);
    }
    let username = link
        .username
        .ok_or_else(|| "Studio link returned no username".to_string())?;
    let token = link
        .token
        .ok_or_else(|| "Studio link returned no token".to_string())?;
    save_token(&username, &token)?;
    Ok(Some(PortSession {
        authenticated: true,
        username,
        port_url: base,
    }))
}

#[tauri::command]
pub(crate) fn port_link_cancel(request_id: String, poll_secret: String) -> Result<(), String> {
    let base = port_url();
    ureq::post(&format!(
        "{base}/api/v2/auth/studio-link/{request_id}/cancel"
    ))
    .set("Content-Type", "application/json")
    .send_string(
        &serde_json::json!({ "request_id": request_id, "poll_secret": poll_secret }).to_string(),
    )
    .map_err(error_message)?;
    Ok(())
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

/// Persists a custom Port server URL for creators self-hosting or using a
/// non-default community instance. An empty `url` clears the override and
/// falls back to `CAIVEN_PORT_URL` / the localhost default.
#[tauri::command]
pub(crate) fn port_set_url(url: String) -> Result<PortSession, String> {
    let path = url_file_path().ok_or_else(|| "No config directory available".to_string())?;
    if url.trim().is_empty() {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        return Ok(port_session());
    }
    let validated = validate_port_url(&url)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, &validated)
        .map_err(|error| format!("Could not save port URL: {error}"))?;
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
            serde_json::json!({ "title": meta.title, "description": meta.description, "tags": meta.tags }),
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
    use super::{load_saved_url, port_set_url, url_encode, validate_port_url};
    use std::sync::Mutex;

    #[test]
    fn encodes_port_query() {
        assert_eq!(url_encode("cave cart/α"), "cave%20cart%2F%CE%B1");
        assert_eq!(url_encode("tag-safe_1"), "tag-safe_1");
    }

    #[test]
    fn validates_port_url_scheme_and_host() {
        assert_eq!(
            validate_port_url("http://example.com/"),
            Ok("http://example.com".to_string())
        );
        assert_eq!(
            validate_port_url("  https://cave.example/  "),
            Ok("https://cave.example".to_string())
        );
        assert!(validate_port_url("").is_err());
        assert!(validate_port_url("   ").is_err());
        assert!(validate_port_url("ftp://example.com").is_err());
        assert!(validate_port_url("https://").is_err());
        assert!(validate_port_url("example.com").is_err());
    }

    /// Guards tests that mutate the process-wide `HOME` env var: `set_var`
    /// is `unsafe` under edition 2024 because concurrent getenv/setenv from
    /// other threads is a real race, so tests touching it run serialized.
    static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn persists_and_clears_custom_port_url() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "caiven-port-url-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let previous_home = std::env::var("HOME").ok();
        let previous_appdata = std::env::var("APPDATA").ok();
        // SAFETY: serialized by HOME_ENV_LOCK; no other test in this binary
        // reads/writes HOME or APPDATA concurrently.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::remove_var("APPDATA");
        }

        assert_eq!(load_saved_url(), None);

        let session = port_set_url("http://cave.example:9090/".to_string()).unwrap();
        assert_eq!(session.port_url, "http://cave.example:9090");
        assert_eq!(
            load_saved_url(),
            Some("http://cave.example:9090".to_string())
        );

        let cleared = port_set_url(String::new()).unwrap();
        assert_eq!(cleared.port_url, "http://localhost:8080");
        assert_eq!(load_saved_url(), None);

        // SAFETY: same serialization guard as above, restoring prior state.
        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_appdata {
                Some(value) => std::env::set_var("APPDATA", value),
                None => std::env::remove_var("APPDATA"),
            }
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_invalid_url_without_persisting() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "caiven-port-url-reject-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let previous_home = std::env::var("HOME").ok();
        let previous_appdata = std::env::var("APPDATA").ok();
        // SAFETY: serialized by HOME_ENV_LOCK.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::remove_var("APPDATA");
        }

        assert!(port_set_url("not-a-url".to_string()).is_err());
        assert_eq!(load_saved_url(), None);

        // SAFETY: same serialization guard as above, restoring prior state.
        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_appdata {
                Some(value) => std::env::set_var("APPDATA", value),
                None => std::env::remove_var("APPDATA"),
            }
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

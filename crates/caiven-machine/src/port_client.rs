//! Talks to a Caiven Port server: browse the cart listing and download one.
//!
//! Blocking (`ureq`) rather than async — the machine's frame loop is a
//! synchronous SDL event pump (`app.rs`), and this mirrors the existing
//! `Effect::LoadCart`/`Effect::DeleteCart` handling, which also blocks on
//! filesystem I/O inside `handle_effect`. `caiven-studio`'s `port_api.rs`
//! uses the same blocking-`ureq` convention for the same reason (no tokio in
//! either crate).
//!
//! Response fields (`id`/`title`/`author`/`cart_size`) match
//! `caiven-port/src/models.rs::Cart` verbatim — that struct has no
//! `rename_all`, so the wire format is plain snake_case.

use std::io::Read;

use crate::shell::state::PortSort;

/// One row of the Port listing, trimmed to what the shell draws and what a
/// download needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortEntry {
    pub id: String,
    pub title: String,
    pub author: String,
    pub bytes: u64,
}

const PER_PAGE: u32 = 24;

/// The configured Port server, in priority order: a value set on the
/// Settings screen (not implemented yet — T48 only reads the env override),
/// then `CAIVEN_PORT_URL`, then the same localhost default `caiven-studio`
/// falls back to.
pub fn port_url() -> String {
    std::env::var("CAIVEN_PORT_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(serde::Deserialize)]
struct WireCart {
    id: String,
    title: String,
    author: String,
    #[serde(default)]
    cart_size: i64,
}

#[derive(serde::Deserialize)]
struct WireList {
    carts: Vec<WireCart>,
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

/// Fetches the first page of the Port listing, sorted as requested.
pub fn list(sort: PortSort) -> Result<Vec<PortEntry>, String> {
    let url = format!(
        "{}/api/v2/carts?page=1&per_page={PER_PAGE}&sort={}",
        port_url(),
        sort.query_value()
    );
    let response = ureq::get(&url).call().map_err(error_message)?;
    let wire: WireList = serde_json::from_reader(response.into_reader())
        .map_err(|error| format!("Invalid cart list: {error}"))?;
    Ok(wire
        .carts
        .into_iter()
        .map(|cart| PortEntry {
            id: cart.id,
            title: cart.title,
            author: cart.author,
            bytes: cart.cart_size.max(0) as u64,
        })
        .collect())
}

/// Downloads one cart's bytes by id.
pub fn download(id: &str) -> Result<Vec<u8>, String> {
    let url = format!("{}/api/v2/carts/{id}/cart", port_url());
    let mut bytes = Vec::new();
    ureq::get(&url)
        .call()
        .map_err(error_message)?
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// Turns a Port cart id into a safe single-path-component filename stem —
/// same discipline `Cargo.toml`'s cart-id-as-save-key invariant (SPEC V56)
/// applies here: the id came off the network, so it must not carry `/`,
/// `\`, `..`, or anything else that could escape the library directory.
pub fn safe_filename(id: &str) -> String {
    let safe: String = id
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
    if safe.is_empty() {
        "port-cart".to_string()
    } else {
        safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Both env-mutating cases live in one test: `CAIVEN_PORT_URL` is process
    // global state, and cargo runs tests in this file concurrently by
    // default, so two separate tests toggling the same var would race.
    #[test]
    fn port_url_reads_env_with_localhost_fallback() {
        // SAFETY: test-only env mutation; kept to this single test so no
        // other test in this file touches CAIVEN_PORT_URL concurrently.
        unsafe {
            std::env::remove_var("CAIVEN_PORT_URL");
        }
        assert_eq!(port_url(), "http://localhost:8080");

        unsafe {
            std::env::set_var("CAIVEN_PORT_URL", "https://cave.example/");
        }
        assert_eq!(port_url(), "https://cave.example");

        unsafe {
            std::env::remove_var("CAIVEN_PORT_URL");
        }
    }

    #[test]
    fn safe_filename_replaces_unsafe_characters() {
        assert_eq!(safe_filename("abc-123"), "abc-123");
        assert_eq!(safe_filename("../../etc/passwd"), "______etc_passwd");
        assert_eq!(safe_filename(""), "port-cart");
    }

    #[test]
    fn safe_filename_never_reproduces_a_path_separator() {
        let safe = safe_filename("a/b\\c:d");
        assert!(!safe.contains(['/', '\\', ':']));
    }
}

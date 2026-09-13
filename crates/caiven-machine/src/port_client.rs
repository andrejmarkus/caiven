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
use std::sync::OnceLock;
use std::time::Duration;

use crate::shell::state::PortSort;

/// Shared agent with explicit timeouts — ureq's default agent has none, so
/// an unreachable or stalled Port server would otherwise hang the frame
/// loop indefinitely (`handle_effect` calls this synchronously, see the
/// module doc above).
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
    })
}

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
    let response = agent().get(&url).call().map_err(error_message)?;
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

/// Downloads one cart's bytes by id. Bounded to one byte past
/// `MAX_CART_BYTES` — a malicious or misbehaving server sending an
/// unbounded body must not be allowed to grow this indefinitely; a cart
/// that size is invalid anyway and `caiven_cart::parse` would reject it.
pub fn download(id: &str) -> Result<Vec<u8>, String> {
    let url = format!("{}/api/v2/carts/{id}/cart", port_url());
    let mut bytes = Vec::new();
    agent()
        .get(&url)
        .call()
        .map_err(error_message)?
        .into_reader()
        .take(caiven_cart::MAX_CART_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > caiven_cart::MAX_CART_BYTES {
        return Err(format!(
            "cart exceeds {} KiB limit",
            caiven_cart::MAX_CART_BYTES / 1024
        ));
    }
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

// Any test anywhere in this crate that touches `CAIVEN_PORT_URL`
// (process-global state) must hold this for its whole env-mutate-then-call
// span — cargo runs every test in this binary concurrently by default, and
// that includes `port_worker`'s tests, not just this file's.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn port_url_reads_env_with_localhost_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: test-only env mutation, serialized by ENV_LOCK above.
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

    /// A server sending more than `MAX_CART_BYTES` must not be allowed to
    /// grow `download`'s buffer without bound — regression test for the
    /// `.take()` cap added alongside this test.
    #[test]
    fn download_rejects_body_over_max_cart_bytes() {
        let _guard = ENV_LOCK.lock().unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let oversized = vec![0u8; caiven_cart::MAX_CART_BYTES + 1024];

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Drain the request line/headers so the client isn't left
            // waiting on us before we write the response.
            let mut buf = [0u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                oversized.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&oversized).unwrap();
        });

        unsafe {
            std::env::set_var("CAIVEN_PORT_URL", format!("http://{addr}"));
        }
        let result = download("some-id");
        unsafe {
            std::env::remove_var("CAIVEN_PORT_URL");
        }
        server.join().unwrap();

        let err = result.expect_err("oversized body must be rejected");
        assert!(err.contains("128 KiB"), "unexpected error: {err}");
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

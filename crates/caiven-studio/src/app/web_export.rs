//! Self-contained "Export to Web" builder: packs a cart into a single
//! offline-playable `.html` file (SPEC §I `export-web`, §V19-21). No
//! emscripten rebuild happens here — this only inlines the WASM runtime
//! already vendored for Caiven Port's browser player
//! (`crates/caiven-port/web/public/wasm/caiven_web.{js,wasm}`) plus the
//! audio worklet and a framework-free player script, all base64-encoded
//! into one HTML shell so the result has zero runtime network dependency.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

const TEMPLATE: &str = include_str!("../../assets/web-export-template.html");
const PLAYER_JS: &str = include_str!("../../assets/web-export-player.js");
const WASM_JS: &str = include_str!("../../../caiven-port/web/public/wasm/caiven_web.js");
const WASM_BIN: &[u8] = include_bytes!("../../../caiven-port/web/public/wasm/caiven_web.wasm");
const WORKLET_JS: &str = include_str!("../../../caiven-port/web/public/caiven-audio-worklet.js");

/// Minimal HTML-attribute/text escaping — `title` only ever lands inside
/// `<title>...</title>`, a text context, so this covers it.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Substitutes `{{TOKEN}}` placeholders in a single left-to-right scan of
/// `template` — each token's replacement text is copied straight to the
/// output and never re-scanned for further tokens. Unlike chaining
/// `str::replace` calls (which re-scans the whole accumulated string on
/// every call), this can't let one substitution's content be mistaken for a
/// later placeholder: e.g. a cart title of literally `{{PLAYER_JS}}` would,
/// under chained `.replace()`, get re-matched by the later `{{PLAYER_JS}}`
/// substitution and splice the player script into the title. Here that
/// string only ever appears in the *output*, which is never re-read.
fn substitute(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find("}}") else {
            break;
        };
        let token = &rest[2..end];
        match values.iter().find(|(key, _)| *key == token) {
            Some((_, value)) => out.push_str(value),
            None => out.push_str(&rest[..end + 2]),
        }
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    out
}

/// Builds a single self-contained `.html` string that plays `packed_cav`
/// fully offline: no `<script src>`, no `fetch`, no CDN — everything is
/// inlined as base64 or literal script bodies (V19). `title` is used for
/// the page `<title>` only; cosmetic.
pub fn build_web_html(packed_cav: &[u8], title: &str) -> String {
    let cart_b64 = BASE64.encode(packed_cav);
    let wasm_b64 = BASE64.encode(WASM_BIN);
    let worklet_b64 = BASE64.encode(WORKLET_JS.as_bytes());
    let title = escape_html(title);

    substitute(
        TEMPLATE,
        &[
            ("TITLE", &title),
            ("WASM_JS", WASM_JS),
            ("WASM_B64", &wasm_b64),
            ("CART_B64", &cart_b64),
            ("WORKLET_B64", &worklet_b64),
            ("PLAYER_JS", PLAYER_JS),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_external_network_references() {
        let html = build_web_html(b"fake cart bytes", "Test Cart");
        // V19: the exported page must never load anything over the network.
        // Every <script> is inlined (no `src=` at all — not even
        // same-origin) and the wasm module is instantiated from bytes we
        // already have in hand via `Module.instantiateWasm` — the hook this
        // emscripten build actually honors (its glue keeps `wasmBinary` as
        // an internal var it populates from its own fetch; overriding a
        // `Module["wasmBinary"]` property, the older/other builds'
        // documented mechanism, is silently ignored here). That makes
        // emscripten's normal fetch-then-compile path dead code (the
        // vendored glue still *contains* the string "fetch(" as that now
        // unreachable path — not a call this export ever makes, so it isn't
        // asserted away).
        assert!(
            !html.contains("<script src"),
            "found a <script src=...> (should be inlined)"
        );
        assert!(
            html.contains("instantiateWasm"),
            "wasm must be instantiated from in-memory bytes via Module.instantiateWasm, not fetched"
        );
        // The vendored glue must genuinely read the hook, not just have our
        // own player.js mention the same word — otherwise this assertion
        // would still pass even if a `caiven-web` rebuild dropped support
        // for it (the string would still come from our own player.js).
        assert!(
            WASM_JS.contains(r#"Module["instantiateWasm"]"#),
            "vendored caiven_web.js no longer appears to read Module.instantiateWasm; \
             the offline export path (V19) may be broken for real browsers even though \
             this test's own string check would still pass"
        );
    }

    #[test]
    fn title_matching_a_later_placeholder_does_not_corrupt_output() {
        // Regression: chained str::replace() re-scans the whole
        // accumulated string on every call, so a title equal to a
        // not-yet-substituted placeholder token used to get re-matched by
        // that later substitution, splicing the player script (or base64
        // payload) into <title>. `substitute` fixes this by never
        // re-scanning inserted content.
        let html = build_web_html(b"cart bytes", "{{PLAYER_JS}}");
        assert!(
            html.contains("<title>{{PLAYER_JS}}</title>"),
            "title placeholder collision corrupted the <title> tag"
        );
    }

    #[test]
    fn embeds_cart_bytes_verbatim() {
        let cart_bytes = b"CAIVEN-cart-bytes-round-trip-check";
        let html = build_web_html(cart_bytes, "Round Trip");
        let expected_b64 = BASE64.encode(cart_bytes);
        assert!(
            html.contains(&expected_b64),
            "packed cart bytes must appear verbatim (base64) in the export (V21)"
        );
    }

    #[test]
    fn escapes_title_for_html_context() {
        let html = build_web_html(b"x", "<script>alert(1)</script>");
        assert!(!html.contains("<title><script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}

//! Drift guard: Port's remix editor has no VM to ask for the API list, so the
//! shared editor ships a JSON snapshot of `api_registry`. Regenerate it with
//! `CAIVEN_UPDATE_LUA_API=1 cargo test -p caiven-vm --test lua_api_json_sync`.

use caiven_vm::prelude_module_catalog;
use caiven_vm::vm::api_registry::{ApiEntry, BUILTINS, PRELUDE, STDLIB};
use serde_json::{Value, json};

const JSON_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../caiven-ui/src/components/ui/lua-editor/lua-api.json"
);

// Same shape and categories as Studio's `api_payload`/`prelude_modules_payload`.
fn snapshot() -> String {
    let entries = |entries: &[ApiEntry], category: &str| -> Vec<Value> {
        entries
            .iter()
            .map(|entry| {
                json!({
                    "name": entry.name,
                    "params": entry.params.iter().map(|param| json!({ "name": param.name, "ty": param.ty })).collect::<Vec<_>>(),
                    "returns": entry.returns,
                    "doc": entry.doc,
                    "category": category,
                })
            })
            .collect()
    };
    let api: Vec<Value> = [
        entries(BUILTINS, "Console builtins"),
        entries(PRELUDE, "Gameplay stdlib"),
        entries(STDLIB, "Lua standard library"),
    ]
    .concat();
    let modules: Vec<Value> = prelude_module_catalog()
        .into_iter()
        .map(|(name, export)| json!({ "name": name, "export": export }))
        .collect();
    let text = serde_json::to_string_pretty(&json!({ "api": api, "preludeModules": modules }))
        .expect("json values always serialize");
    format!("{text}\n")
}

#[test]
fn lua_api_json_matches_registry() {
    let want = snapshot();
    if std::env::var_os("CAIVEN_UPDATE_LUA_API").is_some() {
        std::fs::write(JSON_PATH, &want).expect("write lua-api.json");
        return;
    }
    let got = std::fs::read_to_string(JSON_PATH)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        got == want,
        "lua-api.json is stale; rerun with CAIVEN_UPDATE_LUA_API=1 (see test header)"
    );
}

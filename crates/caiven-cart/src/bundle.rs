//! Multi-module Lua bundling: turns a project's entry file plus any sibling
//! `.lua` modules into a single self-contained chunk, so `require("foo")`
//! works both in the dev loop (project dir, filesystem present) and in the
//! distributed `.cav` (no filesystem — the VM only ever loads one
//! `LuaSource` string, see `Vm::load_lua_source`).

use std::path::{Path, PathBuf};

/// Recursively collects every `.lua` file under `dir` except `entry`
/// (a path relative to `dir`), sorted for deterministic bundling.
pub fn list_lua_files(dir: &Path, entry: &Path) -> Vec<PathBuf> {
    let entry_abs = dir.join(entry);
    let mut out = Vec::new();
    collect(dir, &entry_abs, &mut out);
    out.sort();
    out
}

fn collect(current: &Path, entry_abs: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, entry_abs, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("lua") && path != *entry_abs {
            out.push(path);
        }
    }
}

/// Module key for a project-relative `.lua` path, Lua `require` convention:
/// `ui/panel.lua` -> `ui.panel`. `path` may be absolute (as returned by
/// `list_lua_files`) or already relative to `dir`.
pub fn module_key(dir: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(dir).unwrap_or(path);
    let rel = rel.with_extension("");
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(".")
}

/// Bundles `entry_src` plus `modules` (dotted key -> source) into one
/// self-contained Lua chunk. Each module is registered into
/// `package.preload` under both its dotted key (`ui.panel`) and slashed
/// alias (`ui/panel`), compiled as its own named chunk so error messages and
/// stack traces keep correct per-file line numbers. `package.path`/`cpath`
/// are cleared first so `require` can only resolve through `preload` —
/// never the host filesystem, which the distributed `.cav` doesn't have.
///
/// With no modules, returns `entry_src` completely unchanged (not even a
/// trivial prefix line) — line numbers in compile errors and breakpoints
/// must keep matching the entry buffer exactly for the (overwhelmingly
/// common) single-file case. With modules, the preload block that's
/// prepended shifts every line of `entry_src` down in the compiled chunk;
/// Studio's breakpoint/error-jump line mapping does not currently correct
/// for that shift, so gutter breakpoints and error-jump in the entry file
/// can land on the wrong line whenever a project uses modules. Module files
/// themselves compile as their own named chunk (`@module.lua`), so a syntax
/// error there is at least reported against the module's own real line
/// number — same as it would be for any other Lua `load()` caller.
pub fn bundle_lua(entry_src: &str, modules: &[(String, String)]) -> String {
    if modules.is_empty() {
        return entry_src.to_string();
    }

    let mut out = String::new();
    out.push_str("package.path = \"\" package.cpath = \"\"\n");
    out.push_str("do\n  local __pre = package.preload\n");
    for (key, src) in modules {
        let level = bracket_level(src);
        let eq = "=".repeat(level);
        let slash_key = key.replace('.', "/");
        out.push_str(&format!(
            "  __pre[\"{key}\"] = assert(load([{eq}[\n{src}]{eq}], \"@{slash_key}.lua\"))\n"
        ));
        if slash_key != *key {
            out.push_str(&format!("  __pre[\"{slash_key}\"] = __pre[\"{key}\"]\n"));
        }
    }
    out.push_str("end\n");
    out.push_str(entry_src);
    out
}

/// The lowest long-bracket level (count of `=` between the brackets) whose
/// closing sequence `]=...=]` does not already occur in `src`, so wrapping
/// `src` in `[level[ ... ]level]` can't be closed early by its own content.
fn bracket_level(src: &str) -> usize {
    let bytes = src.as_bytes();
    let mut max_eq: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b']' {
            let mut j = i + 1;
            let mut eq = 0usize;
            while j < bytes.len() && bytes[j] == b'=' {
                eq += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b']' {
                max_eq = Some(max_eq.map_or(eq, |m| m.max(eq)));
            }
        }
        i += 1;
    }
    max_eq.map_or(0, |m| m + 1)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn module_key_converts_slashes_to_dots() {
        let dir = Path::new("/proj");
        let path = Path::new("/proj/ui/panel.lua");
        assert_eq!(module_key(dir, path), "ui.panel");
    }

    #[test]
    fn module_key_with_empty_dir_keys_an_already_relative_path() {
        // Used when only a project-relative path is available (no absolute
        // project dir on hand) — an empty dir must strip nothing.
        let dir = Path::new("");
        let path = Path::new("ui/panel.lua");
        assert_eq!(module_key(dir, path), "ui.panel");
    }

    #[test]
    fn bracket_level_is_zero_when_no_closing_sequence_present() {
        assert_eq!(bracket_level("local x = 1"), 0);
    }

    #[test]
    fn bracket_level_escalates_past_existing_closers() {
        assert_eq!(bracket_level("contains ]] literally"), 1);
        assert_eq!(bracket_level("contains ]=] and ]==] "), 3);
    }

    #[test]
    fn bundle_registers_dotted_and_slashed_preload_keys() {
        let out = bundle_lua(
            "return 1",
            &[("ui.panel".to_string(), "return 2".to_string())],
        );
        assert!(out.contains("__pre[\"ui.panel\"]"));
        assert!(out.contains("__pre[\"ui/panel\"]"));
        assert!(out.contains("package.path = \"\""));
        assert!(out.ends_with("return 1"));
    }

    #[test]
    fn bundle_with_no_modules_is_byte_identical_to_entry() {
        // No prefix line at all — must not shift breakpoint/error line
        // numbers for the common single-file case.
        let out = bundle_lua("return 1", &[]);
        assert_eq!(out, "return 1");
    }

    #[test]
    fn bundle_survives_module_source_containing_long_brackets() {
        let tricky = "local s = ]]  -- not real lua but exercises the scanner";
        let out = bundle_lua("return 1", &[("tricky".to_string(), tricky.to_string())]);
        assert!(out.contains("[=[\n"));
    }
}

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
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // Never follow directory cycles or bundle files outside the project.
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect(&path, entry_abs, out);
        } else if file_type.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("lua")
            && path != *entry_abs
        {
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
/// With no modules, returns `entry_src` completely unchanged. With modules,
/// the generated preload setup executes first, then the entry source is
/// compiled and executed as a separate chunk named `cart`. This keeps entry
/// breakpoints and compile/runtime error lines identical to the editor even
/// though setup code exists in the outer bundle.
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
        let quoted_key = lua_string(key);
        let quoted_slash_key = lua_string(&slash_key);
        let chunk_name = lua_string(&format!("@{slash_key}.lua"));
        out.push_str(&format!(
            "  __pre[{quoted_key}] = assert(load([{eq}[\n{src}]{eq}], {chunk_name}))\n"
        ));
        if slash_key != *key {
            out.push_str(&format!(
                "  __pre[{quoted_slash_key}] = __pre[{quoted_key}]\n"
            ));
        }
    }
    out.push_str("end\n");

    // Lua long strings discard the first newline after the opening bracket,
    // so entry line 1 remains line 1 in this separately named chunk.
    let level = bracket_level(entry_src);
    let eq = "=".repeat(level);
    out.push_str(&format!(
        "return assert(load([{eq}[\n{entry_src}]{eq}], \"=cart\"))()\n"
    ));
    out
}

/// Quote names as Lua data. Decimal escapes use three digits so a following
/// digit cannot become part of the escape; JSON escaping is not Lua escaping.
fn lua_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ch if ch.is_ascii_control() => out.push_str(&format!("\\{:03}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
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
    fn bundle_registers_modules_and_loads_entry_as_cart_chunk() {
        let out = bundle_lua(
            "return 1",
            &[("ui.panel".to_string(), "return 2".to_string())],
        );
        assert!(out.contains("__pre[\"ui.panel\"]"));
        assert!(out.contains("__pre[\"ui/panel\"]"));
        assert!(out.contains("package.path = \"\""));
        assert!(out.contains("\"=cart\"))()"));
    }

    #[test]
    fn bundle_with_no_modules_is_byte_identical_to_entry() {
        let out = bundle_lua("return 1", &[]);
        assert_eq!(out, "return 1");
    }

    #[cfg(unix)]
    #[test]
    fn discovery_skips_external_symlinks() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.lua"), "return 1").unwrap();
        std::fs::write(project.path().join("safe.lua"), "return 2").unwrap();
        std::fs::write(outside.path().join("secret.lua"), "return 3").unwrap();
        std::os::unix::fs::symlink(outside.path(), project.path().join("external")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.lua"),
            project.path().join("linked.lua"),
        )
        .unwrap();
        assert_eq!(
            list_lua_files(project.path(), Path::new("main.lua")),
            vec![project.path().join("safe.lua")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn discovery_skips_directory_cycles() {
        let project = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(project.path(), project.path().join("cycle")).unwrap();
        assert!(list_lua_files(project.path(), Path::new("main.lua")).is_empty());
    }

    #[test]
    fn bundle_survives_module_and_entry_sources_containing_long_brackets() {
        let tricky = "local s = ]]  -- not real lua but exercises the scanner";
        let out = bundle_lua(tricky, &[("tricky".to_string(), tricky.to_string())]);
        assert!(out.matches("[=[\n").count() >= 2);
    }
}

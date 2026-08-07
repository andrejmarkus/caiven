use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};

fn fresh_vm() -> (Vm, Input, Font) {
    (Vm::new(VmConfig::default()), Input::new(), Font::empty())
}

#[test]
fn dangerous_globals_are_absent() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source("", &input, &font)
        .expect("empty cart loads");

    // `package`/`require`/`load` stay on the globals table: they back the
    // multi-module bundling format (`caiven_cart::bundle_lua`), which
    // registers sibling modules into `package.preload` and pulls them in via
    // `require`. `require` is restricted to `package.preload` only (all
    // other `package.searchers` entries are removed, see
    // `require_cannot_reach_the_filesystem_even_after_reassigning_package_path`
    // below) so that mechanism can't reach the host filesystem even if a
    // cart later reassigns `package.path`/`cpath`, and `load` is wrapped to
    // force text-only mode (see `load_rejects_binary_bytecode_chunks`).
    for name in ["io", "os", "dofile", "loadfile"] {
        let src = format!("assert({name} == nil, \"{name} should be nil\")");
        vm.load_lua_source(&src, &input, &font)
            .unwrap_or_else(|e| panic!("expected {name} to be nil, got error instead: {e}"));
    }
}

#[test]
fn calling_a_removed_global_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dofile('whatever')", &input, &font);
    assert!(result.is_err(), "dofile must not be callable from a cart");
}

#[test]
fn require_cannot_reach_the_filesystem() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "assert(package.path == ''); assert(package.cpath == '')",
        &input,
        &font,
    )
    .expect("package.path/cpath must be forced empty");

    let result = vm.load_lua_source("require('some_real_module')", &input, &font);
    assert!(
        result.is_err(),
        "require must not resolve anything outside package.preload"
    );
}

/// `package.path`/`cpath` are only cleared once at construction — a cart
/// could reassign them at runtime and try to point `require` at the
/// filesystem. The searchers behind `package.path`/`cpath` are removed
/// entirely (not just the path strings cleared), so reassigning the path
/// string must not resurrect filesystem access.
#[test]
fn require_cannot_reach_the_filesystem_even_after_reassigning_package_path() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        r#"
            package.path = "/etc/?.lua"
            package.cpath = "/usr/lib/?.so"
            local ok, _ = pcall(require, "passwd")
            assert(not ok, "require must not resolve via a reassigned package.path")
        "#,
        &input,
        &font,
    )
    .expect("reassigning package.path must not enable require to hit the filesystem");
}

#[test]
fn package_loadlib_is_disabled() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("package.loadlib('lib', 'sym')", &input, &font);
    assert!(
        result.is_err(),
        "package.loadlib must stay disabled (C module loading)"
    );
}

/// `load` is exposed by Lua's base library unconditionally (outside the
/// `StdLib` mask), and its default "bt" mode accepts precompiled bytecode
/// strings — a memory-safety hazard independent of filesystem access. The
/// sandbox wraps `load` to force text-only ("t") mode.
#[test]
fn load_rejects_binary_bytecode_chunks() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        r#"
            local bytecode = string.char(27) .. "Lua" .. string.rep(string.char(0), 20)
            local chunk, err = load(bytecode, "chunk")
            assert(chunk == nil, "a binary/bytecode chunk must be rejected")
            assert(err ~= nil)
        "#,
        &input,
        &font,
    )
    .expect("rejecting a binary chunk must not itself be a Lua error");
}

/// The text-only `load` wrapper must still support the ordinary source-text
/// use `caiven_cart::bundle_lua` depends on (see `bundled_module_*` tests in
/// `tests/lua_script.rs`, which exercise it through the real bundler).
#[test]
fn load_still_compiles_ordinary_source_text() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        r#"
            local f = assert(load("return 1 + 1", "=inline"))
            assert(f() == 2)
        "#,
        &input,
        &font,
    )
    .expect("load must still compile plain source text");
}

#[test]
fn sanctioned_stdlib_still_works() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "assert(math.floor(1.5) == 1); assert(string.upper('a') == 'A'); local t = {}; table.insert(t, 1); assert(#t == 1)",
        &input,
        &font,
    )
    .expect("math/string/table must remain available");
}

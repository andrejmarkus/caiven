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
    // `require`. `package.path`/`cpath` are forced empty (see
    // `require_cannot_reach_the_filesystem` below) so that mechanism can
    // never touch the host filesystem.
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

#[test]
fn package_loadlib_is_disabled() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("package.loadlib('lib', 'sym')", &input, &font);
    assert!(
        result.is_err(),
        "package.loadlib must stay disabled (C module loading)"
    );
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

use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};

fn make_vm() -> Vm {
    Vm::new(VmConfig::default())
}

#[test]
fn core_only_default_exposes_rng_and_easing() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local r = random_range(1, 10)
          local e = ease_linear(0.5)
          local c = clamp(5, 0, 10)
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("core-only cart should load: {e}"));
}

#[test]
fn omitting_stdlib_leaves_gameplay_globals_nil() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        assert(Vec2 == nil, "Vec2 should be nil without [stdlib] modules")
        assert(Sprite == nil, "Sprite should be nil without [stdlib] modules")
        assert(Camera == nil, "Camera should be nil without [stdlib] modules")
        assert(Scenes == nil, "Scenes should be nil without [stdlib] modules")
        assert(Entities == nil, "Entities should be nil without [stdlib] modules")
        assert(Particles == nil, "Particles should be nil without [stdlib] modules")
        assert(aabb_overlap == nil, "aabb_overlap should be nil without [stdlib] modules")
        assert(new_tween == nil, "new_tween should be nil without [stdlib] modules")
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn declaring_a_module_exposes_exactly_its_globals() {
    let mut vm = make_vm();
    vm.set_prelude_modules(&["vec2"])
        .unwrap_or_else(|e| panic!("set_prelude_modules failed: {e}"));
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        assert(Vec2 ~= nil, "Vec2 should be defined by the vec2 module")
        assert(Sprite ~= nil, "Sprite should be defined by the vec2 module")
        assert(Camera == nil, "Camera should stay nil, only vec2 was declared")
        assert(Scenes == nil, "Scenes should stay nil, only vec2 was declared")
        assert(aabb_overlap == nil, "aabb_overlap should stay nil, only vec2 was declared")
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn unknown_module_name_errors_with_the_name() {
    let mut vm = make_vm();
    let err = vm
        .set_prelude_modules(&["physics"])
        .expect_err("unknown module name should be rejected");
    assert!(
        err.contains("physics"),
        "error should name the unknown module, got: {err}"
    );
}

#[test]
fn lua_globals_only_excludes_currently_selected_module_names() {
    let mut vm = make_vm(); // core only — "camera" module not selected
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        Camera = "cart-defined, not the prelude module"
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let globals = vm.lua_globals();
    assert!(
        globals.iter().any(|(name, _)| name == "Camera"),
        "cart-defined Camera should surface in the debugger snapshot when the camera module isn't selected, got: {globals:?}"
    );
}

#[test]
fn active_prelude_modules_reflects_set_prelude_modules() {
    let mut vm = make_vm();
    assert!(vm.active_prelude_modules().is_empty());
    vm.set_prelude_modules(&["vec2", "camera"])
        .unwrap_or_else(|e| panic!("set_prelude_modules failed: {e}"));
    assert_eq!(vm.active_prelude_modules(), &["vec2", "camera"]);
}

#[test]
fn hot_reload_preserves_prelude_module_selection() {
    let mut vm = make_vm();
    vm.set_prelude_modules(&["vec2"])
        .unwrap_or_else(|e| panic!("set_prelude_modules failed: {e}"));
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source("function _update() end", &input, &font)
        .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.hot_reload_lua_source(
        r#"
        assert(Vec2 ~= nil, "Vec2 should still be available after reload without re-supplying set_prelude_modules")
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("hot_reload_lua_source failed: {e}"));
}

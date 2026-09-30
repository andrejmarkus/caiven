use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};

fn load(src: &str) -> Result<Vm, mlua::Error> {
    let mut vm = Vm::new(VmConfig::default());
    vm.load_lua_source(src, &Input::new(), &Font::empty())
        .map(|()| vm)
}

#[test]
fn core_only_default_exposes_rng_and_easing() {
    load(
        r#"
        function _update()
          local r = random_range(1, 10)
          local e = ease_linear(0.5)
          local c = clamp(5, 0, 10)
        end
        "#,
    )
    .unwrap_or_else(|e| panic!("core-only cart should load: {e}"));
}

#[test]
fn modules_define_no_globals() {
    load(
        r#"
        for _, name in ipairs({ "vec2", "actor", "collision", "movement", "tween", "anim",
                                "particles", "scenes", "entities", "camera" }) do
          assert(type(require(name)) == "table", name .. " should return its table")
        end
        for _, global in ipairs({ "Vec2", "Actor", "Sprite", "Camera", "Scenes", "Entities",
                                  "Particles", "collision", "tween", "anim", "movement",
                                  "aabb_overlap", "new_tween", "move_and_collide" }) do
          assert(_G[global] == nil, global .. " should not be a global")
        end
        "#,
    )
    .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn module_tables_expose_their_functions() {
    load(
        r#"
        local Vec2 = require "vec2"
        local tween = require "tween"
        local anim = require "anim"
        local collision = require "collision"
        local movement = require "movement"
        assert((Vec2.new(1, 2) + Vec2.new(1, 1)).x == 2)
        local t = tween.new(0, 10, 2)
        tween.update(t)
        assert(tween.update(t) == 10 and t.done)
        local a = anim.new({ 4, 5 }, 1)
        anim.update(a)
        assert(anim.sprite(a) == 5)
        assert(collision.aabb_overlap(0, 0, 4, 4, 2, 2, 4, 4))
        assert(type(movement.move_and_collide) == "function")
        "#,
    )
    .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn require_returns_the_same_table_every_time() {
    load(r#"assert(require "camera" == require "camera")"#)
        .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn entities_loads_the_collision_it_needs() {
    load(
        r#"
        local Entities = require "entities"
        local Vec2 = require "vec2"
        Entities.add({ pos = Vec2.new(0, 0), w = 4, h = 4 })
        assert(#Entities.overlapping(2, 2, 4, 4) == 1)
        "#,
    )
    .unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn unknown_module_name_errors_with_the_name() {
    let err = load(r#"require "physics""#)
        .err()
        .expect("unknown module name should be rejected");
    assert!(
        err.to_string().contains("physics"),
        "error should name the unknown module, got: {err}"
    );
}

#[test]
fn a_cart_module_with_a_builtin_name_wins() {
    let bundled = caiven_cart::bundle_lua(
        r#"assert(require "camera" == "mine", "cart module should shadow the builtin")"#,
        &[("camera".to_string(), r#"return "mine""#.to_string())],
    );
    load(&bundled).unwrap_or_else(|e| panic!("assertions failed: {e}"));
}

#[test]
fn a_missing_module_local_error_names_the_line_to_add() {
    let mut vm = load("function _update() Camera.update() end")
        .unwrap_or_else(|e| panic!("load failed: {e}"));
    vm.run_frame(&Input::new(), &Font::empty());
    let message = vm
        .fault_message()
        .expect("indexing a nil Camera should fault");
    assert!(
        message.contains(r#"add local Camera = require "camera""#),
        "expected a require hint, got: {message}"
    );
}

#[test]
fn a_cart_global_named_like_a_module_stays_visible_to_the_debugger() {
    let mut vm = load(
        r#"
        Camera = "cart-defined"
        function _update() end
        "#,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    assert!(
        vm.lua_globals().iter().any(|(name, _)| name == "Camera"),
        "cart-defined Camera should surface in the debugger snapshot"
    );
}

#[test]
fn hot_reload_keeps_module_state() {
    let input = Input::new();
    let font = Font::empty();
    let src = r#"local Particles = require "particles"
        function _update() end
        function count() return Particles.count() end"#;
    let mut vm = load(&format!(
        "{src}\nrequire('particles').spawn(1, 1, 0, 0, 1, 99)"
    ))
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.hot_reload_lua_source(
        &format!("{src}\nassert(count() == 1, 'particle state should survive reload')"),
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("hot_reload_lua_source failed: {e}"));
}

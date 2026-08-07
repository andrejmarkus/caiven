use caiven_core::memory::{RTC_RAM_BASE, SFX_RAM_BASE, SPRITE_SHEET_RAM_BASE};
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{
    LuaBreakpoint, LuaRunOutcome, Vm, VmConfig, VmFault, describe_lua_error,
    describe_lua_error_location,
};

fn make_vm() -> Vm {
    Vm::new(VmConfig::default())
}

fn read_rgba(vm: &Vm, x: u32, y: u32) -> [u8; 4] {
    let width = VmConfig::default().width;
    let i = ((y * width + x) * 4) as usize;
    let px = vm.world_pixels();
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn lua_pset_draws_palette_color() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          clear_screen()
          set_pixel(10, 20, 8)
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // Palette index 8 = red, (200, 60, 70) per DEFAULT_COLORS.
    assert_eq!(read_rgba(&vm, 10, 20), [200, 60, 70, 255]);
}

#[test]
fn lua_btn_reads_input_state() {
    let mut vm = make_vm();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        result = 0
        function _update()
          if button_down(4) then
            result = 1
          else
            result = 2
          end
          set_pixel(0, 0, result)
        end
        "#,
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let mut input = Input::new();
    input.set_button(caiven_vm::input::Button::A, true);
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // color index 1 = dark blue (32, 51, 123) confirms the true branch ran.
    assert_eq!(read_rgba(&vm, 0, 0), [32, 51, 123, 255]);
}

#[test]
fn lua_reads_select_at_index_six_and_nothing_beyond_it() {
    let mut vm = make_vm();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          -- Index 7 is where START would sit if carts could see it. They
          -- cannot, so it must stay false however the console is wired.
          if button_down(6) and not button_down(7) then
            set_pixel(0, 0, 1)
          else
            set_pixel(0, 0, 2)
          end
        end
        "#,
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let mut input = Input::new();
    input.set_button(caiven_vm::input::Button::Select, true);
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // color index 1 = dark blue (32, 51, 123) confirms the true branch ran.
    assert_eq!(read_rgba(&vm, 0, 0), [32, 51, 123, 255]);
}

#[test]
fn lua_runtime_error_faults_cleanly() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          error("boom")
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), Some(VmFault::LuaError));
}

#[test]
fn loading_fixed_source_clears_previous_lua_fault() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source("function _update() error(\"boom\") end", &input, &font)
        .expect("load failing-at-runtime cart");
    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), Some(VmFault::LuaError));

    vm.load_lua_source("function _update() end", &input, &font)
        .expect("load fixed cart");
    assert_eq!(vm.get_fault(), None);
}

#[test]
fn lua_run_frame_bp_stops_at_breakpointed_line() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          x = 1
          x = 2
          x = 3
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    // Line 4 is `x = 2`.
    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 4)]) {
        LuaRunOutcome::Breakpoint(breakpoint) => assert_eq!(breakpoint.line, 4),
        other => panic!("expected a breakpoint stop, got {other:?}"),
    }
    assert_eq!(vm.get_fault(), None, "a breakpoint stop isn't a fault");
}

#[test]
fn lua_run_frame_bp_exposes_locals_at_breakpoint() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local answer = 42
          local label = "hi"
          answer = answer + 1
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    assert!(vm.lua_debug_locals().is_empty());
    // Line 5 is `answer = answer + 1`, after both locals are declared.
    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 5)]) {
        LuaRunOutcome::Breakpoint(breakpoint) => assert_eq!(breakpoint.line, 5),
        other => panic!("expected a breakpoint stop, got {other:?}"),
    }
    let locals = vm.lua_debug_locals();
    assert!(
        locals.contains(&("answer".to_string(), "42".to_string())),
        "expected local `answer` = 42, got {locals:?}"
    );
    assert!(
        locals.contains(&("label".to_string(), "\"hi\"".to_string())),
        "expected local `label` = \"hi\", got {locals:?}"
    );

    // Resuming past the breakpoint clears the snapshot.
    match vm.run_frame_lua_bp(&input, &font, &[]) {
        LuaRunOutcome::Completed => {}
        other => panic!("expected completion, got {other:?}"),
    }
    assert!(vm.lua_debug_locals().is_empty());
}

#[test]
fn lua_run_frame_bp_locals_reflect_shadowing_and_loop_scope() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local shadow = 1
          do
            local shadow = 2
            for i = 1, 3 do
              local loopvar = i * 10
              shadow = shadow + loopvar
            end
          end
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    // Line 8 is `shadow = shadow + loopvar`, first loop iteration
    // (i = 1, loopvar = 10), inner `shadow` (2) still shadowing the outer one.
    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 8)]) {
        LuaRunOutcome::Breakpoint(breakpoint) => assert_eq!(breakpoint.line, 8),
        other => panic!("expected a breakpoint stop, got {other:?}"),
    }
    let locals = vm.lua_debug_locals();
    assert!(
        locals.contains(&("shadow".to_string(), "2".to_string())),
        "expected shadowed inner `shadow` = 2 (not outer's 1), got {locals:?}"
    );
    assert!(
        locals.contains(&("i".to_string(), "1".to_string())),
        "expected loop control var `i` = 1, got {locals:?}"
    );
    assert!(
        locals.contains(&("loopvar".to_string(), "10".to_string())),
        "expected loop-body local `loopvar` = 10, got {locals:?}"
    );
    // Only one `shadow` entry: the innermost visible binding wins, the
    // shadowed outer one isn't reported alongside it.
    assert_eq!(
        locals.iter().filter(|(name, _)| name == "shadow").count(),
        1,
        "expected exactly one `shadow` entry (innermost wins), got {locals:?}"
    );
}

#[test]
fn lua_run_frame_bp_locals_exclude_captured_upvalues() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local outer = 100
          local function inner()
            local innerlocal = 5
            innerlocal = innerlocal + 1
          end
          inner()
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    // Line 6 is `innerlocal = innerlocal + 1`, inside `inner()` — `outer` is
    // only reachable there as a captured upvalue, not a local of this frame.
    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 6)]) {
        LuaRunOutcome::Breakpoint(breakpoint) => assert_eq!(breakpoint.line, 6),
        other => panic!("expected a breakpoint stop, got {other:?}"),
    }
    let locals = vm.lua_debug_locals();
    assert!(
        locals.contains(&("innerlocal".to_string(), "5".to_string())),
        "expected innermost frame's own local `innerlocal` = 5, got {locals:?}"
    );
    assert!(
        !locals.iter().any(|(name, _)| name == "outer"),
        "upvalue `outer` isn't a local of this frame — lua_getlocal shouldn't \
         report it (V23 documents this as read-only-current-frame, not \
         full-scope-chain), got {locals:?}"
    );
}

#[test]
fn lua_debug_locals_stay_empty_outside_the_breakpoint_hook_path() {
    // read_active_locals is a plain Rust fn only ever invoked from inside
    // run_frame_lua_bp's EVERY_LINE hook — cart Lua has no registered
    // builtin that reaches it, and plain run_frame() never wires the hook
    // at all, so locals must never populate off that path (V8, V23).
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local secret = 42
          x = secret
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), None);
    assert!(
        vm.lua_debug_locals().is_empty(),
        "plain run_frame must never populate debugger locals"
    );

    // Even run_frame_lua_bp with breakpoints that don't match anything must
    // leave locals empty — the hook only reads/reports on an actual hit.
    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 999)]) {
        LuaRunOutcome::Completed => {}
        other => panic!("expected Completed, got {other:?}"),
    }
    assert!(vm.lua_debug_locals().is_empty());
}

#[test]
fn lua_run_frame_bp_completes_when_no_breakpoint_hit() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          x = 1
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("*", 999)]) {
        LuaRunOutcome::Completed => {}
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn lua_run_frame_bp_ticks_audio_players() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    // SFX slot 0, step 0: note=49, vol=12, wave=0 (square), fx=0.
    vm.load_section_to_ram(SFX_RAM_BASE, &[49, 12, 0, 0]);
    vm.load_lua_source(
        r#"
        function _update()
          play_sfx(0)
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    // Studio's breakpoint-aware path used to skip tick_audio_players
    // entirely, so play_sfx() would mark a player active without ever
    // advancing it into the shared Sound state the CPAL callback reads.
    // Two frames: frame 1's _update() calls play_sfx (marks the player
    // active); frame 2's tick (which runs before _update) is what actually
    // reads RAM into Sound — same one-frame latency plain run_frame has.
    for _ in 0..2 {
        match vm.run_frame_lua_bp(&input, &font, &[]) {
            LuaRunOutcome::Completed => {}
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    let sound = vm.get_sound_shared();
    let s = sound.lock().unwrap_or_else(|e| e.into_inner());
    assert!(s.square.enabled, "square channel should be enabled");
    assert!(s.square.volume > 0.0, "volume should be nonzero");
}

#[test]
fn stop_audio_silences_players_and_shared_channels() {
    let mut vm = make_vm();
    vm.load_section_to_ram(SFX_RAM_BASE, &[49, 12, 0, 0]);
    vm.start_sfx(0);
    vm.start_music(0);
    vm.tick_audio_players();
    assert!(vm.sfx_player().active);
    assert!(vm.music_player().active);

    vm.stop_audio();
    assert!(!vm.sfx_player().active);
    assert!(!vm.music_player().active);
    let sound = vm.get_sound_shared();
    let sound = sound.lock().unwrap_or_else(|error| error.into_inner());
    assert!(!sound.square.enabled);
    assert!(!sound.noise.enabled);
}

#[test]
fn describe_lua_error_extracts_line_and_message() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    let err = vm
        .load_lua_source(
            r#"
        function _update()
        end
        this is not valid lua
        "#,
            &input,
            &font,
        )
        .expect_err("malformed source should fail to load");

    let (line, message) = describe_lua_error(&err);
    assert!(line.is_some(), "expected a source line, got none");
    assert!(!message.is_empty());
}

#[test]
fn lua_globals_excludes_builtins_and_stdlib() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        score = 42
        player_name = "hero"
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let globals = vm.lua_globals();
    let names: Vec<&str> = globals.iter().map(|(k, _)| k.as_str()).collect();
    assert!(names.contains(&"score"));
    assert!(names.contains(&"player_name"));
    assert!(!names.contains(&"draw_text"), "builtins shouldn't appear");
    assert!(!names.contains(&"print"), "stdlib shouldn't appear");
    assert!(!names.contains(&"_update"), "entry points shouldn't appear");
    assert!(
        !names.contains(&"lerp") && !names.contains(&"Particles"),
        "gameplay prelude shouldn't appear"
    );
}

/// `caiven_cart::bundle_lua` is how the project-dir authoring format turns
/// an entry file plus sibling `.lua` modules into the single `LuaSource`
/// string the VM ever sees. This drives the actual bundle output through a
/// real Lua interpreter to confirm `require()` resolves the preloaded
/// module — not just that the bundled string looks right.
#[test]
fn bundled_module_resolves_via_require() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();

    let entry = r#"
        local util = require("util")
        result = util.double(21)
        function _update() end
    "#;
    let modules = [(
        "util".to_string(),
        "return { double = function(n) return n * 2 end }".to_string(),
    )];
    let bundled = caiven_cart::bundle_lua(entry, &modules);

    vm.load_lua_source(&bundled, &input, &font)
        .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let globals = vm.lua_globals();
    let result = globals
        .iter()
        .find(|(k, _)| k == "result")
        .map(|(_, v)| v.as_str());
    assert_eq!(result, Some("42"));
}

#[test]
fn bundled_module_breakpoint_keeps_source_and_line() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    let entry =
        "local util = require(\"util\")\nfunction _update()\n  result = util.double(21)\nend\n";
    let module = "local M = {}\nfunction M.double(n)\n  return n * 2\nend\nreturn M\n";
    let bundled = caiven_cart::bundle_lua(entry, &[("util".to_string(), module.to_string())]);
    vm.load_lua_source(&bundled, &input, &font)
        .unwrap_or_else(|error| panic!("load_lua_source failed: {error}"));

    match vm.run_frame_lua_bp(&input, &font, &[LuaBreakpoint::new("util.lua", 3)]) {
        LuaRunOutcome::Breakpoint(location) => {
            assert_eq!(location, LuaBreakpoint::new("util.lua", 3));
        }
        other => panic!("expected module breakpoint, got {other:?}"),
    }
    assert!(
        vm.lua_call_stack()
            .iter()
            .any(|(_, location)| location.ends_with("util.lua:3")),
        "call stack should retain module frame"
    );
}

#[test]
fn bundled_module_syntax_error_reports_module_location() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    let bundled = caiven_cart::bundle_lua(
        "function _update() end\n",
        &[("ui.panel".to_string(), "local x =\nreturn {}\n".to_string())],
    );
    let error = vm
        .load_lua_source(&bundled, &input, &font)
        .expect_err("malformed module should fail bundle load");
    let (location, _) = describe_lua_error_location(&error);
    let location = location.expect("module source location");
    assert_eq!(location.source, "ui/panel.lua");
    assert_eq!(location.line, 2);
}

#[test]
fn rtc_peripheral_ticks_and_is_readable_from_lua() {
    let mut vm = make_vm();
    // RealTimeClock::init runs in Vm::new(), before any cart loads.
    let hour = vm.peek_memory(RTC_RAM_BASE);
    let minute = vm.peek_memory(RTC_RAM_BASE + 1);
    let second = vm.peek_memory(RTC_RAM_BASE + 2);
    assert!(hour < 24);
    assert!(minute < 60);
    assert!(second < 60);

    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        rtc_hour, rtc_minute, rtc_second = real_time()
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    // Nothing ticks the peripheral between Vm::new() and load_lua_source,
    // so the RAM-mapped registers real_time() reads are unchanged.
    assert_eq!(get("rtc_hour"), hour.to_string());
    assert_eq!(get("rtc_minute"), minute.to_string());
    assert_eq!(get("rtc_second"), second.to_string());
}

#[test]
fn lua_draw_runs_after_update_each_frame() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        update_count = 0
        draw_count = 0
        function _update()
          update_count = update_count + 1
        end
        function _draw()
          draw_count = draw_count + 1
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    assert_eq!(get("update_count"), "2");
    assert_eq!(get("draw_count"), "2");
}

#[test]
fn lua_cart_without_draw_still_runs() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          set_pixel(0, 0, 1)
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
}

#[test]
fn lua_frame_count_and_time_advance() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        fc = 0
        t = 0
        function _update()
          fc = frame_count()
          t = time()
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    for _ in 0..60 {
        vm.run_frame(&input, &font);
    }

    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    // run_frame() increments frame_count before calling _update(), so after
    // 60 calls the Lua-visible count is 60 and time() is exactly 1 second.
    assert_eq!(get("fc"), "60");
    assert_eq!(get("t"), "1");
}

fn run_and_get(src_update_body: &str, snapshot_vars: &[&str]) -> Vec<String> {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    let src = format!("function _update()\n{src_update_body}\nend\n");
    vm.load_lua_source(&src, &input, &font)
        .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    snapshot_vars
        .iter()
        .map(|name| {
            globals
                .iter()
                .find(|(k, _)| k == name)
                .unwrap_or_else(|| panic!("missing global {name}"))
                .1
                .clone()
        })
        .collect()
}

#[test]
fn prelude_lerp_and_clamp() {
    let got = run_and_get(
        "a = lerp(0, 10, 0.5)\nb = clamp(15, 0, 10)\nc = clamp(-5, 0, 10)",
        &["a", "b", "c"],
    );
    assert_eq!(got, vec!["5", "10", "0"]);
}

#[test]
fn prelude_easing_bounds() {
    let got = run_and_get(
        "a = ease_in_quad(0)\nb = ease_in_quad(1)\nc = ease_out_quad(1)\nd = ease_in_out_quad(1)",
        &["a", "b", "c", "d"],
    );
    assert_eq!(got, vec!["0", "1", "1", "1"]);
}

#[test]
fn prelude_aabb_overlap() {
    let got = run_and_get(
        "a = aabb_overlap(0,0,10,10, 5,5,10,10)\nb = aabb_overlap(0,0,5,5, 10,10,5,5)",
        &["a", "b"],
    );
    assert_eq!(got, vec!["true", "false"]);
}

#[test]
fn prelude_tile_solid_and_box_touches_solid() {
    let got = run_and_get(
        r#"
        set_collision(0, 0, 1)
        a = tile_solid(0, 0)
        b = tile_solid(1, 0)
        c = box_touches_solid(0, 0, SPRITE_SIZE, SPRITE_SIZE)
        d = box_touches_solid(SPRITE_SIZE * 3, SPRITE_SIZE * 3, SPRITE_SIZE, SPRITE_SIZE)
        "#,
        &["a", "b", "c", "d"],
    );
    assert_eq!(got, vec!["true", "false", "true", "false"]);
}

#[test]
fn custom_solid_collision_type_is_respected_by_tile_solid() {
    let mut vm = make_vm();
    let mut types = caiven_core::builtin_collision_types();
    types.push(caiven_core::CollisionType {
        id: 3,
        name: "water".to_string(),
        color: [0, 128, 255],
        flags: caiven_core::CollisionTypeFlags::from_bits(caiven_core::CollisionTypeFlags::SOLID),
    });
    vm.set_collision_types(types);

    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          set_collision(0, 0, 3)
          solid = tile_solid(0, 0)
          is_solid = collision_is_solid(3)
          name = collision_type_name(3)
          id = collision_type_id("water")
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), None);

    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    assert_eq!(get("solid"), "true");
    assert_eq!(get("is_solid"), "true");
    assert_eq!(get("name"), "\"water\"");
    assert_eq!(get("id"), "3");
}

#[test]
fn prelude_tween_reaches_target_and_marks_done() {
    let got = run_and_get(
        r#"
        tw = new_tween(0, 10, 5)
        for i = 1, 5 do
          v = tween_update(tw)
        end
        done = tw.done
        "#,
        &["v", "done"],
    );
    assert_eq!(got, vec!["10", "true"]);
}

#[test]
fn prelude_anim_cycles_frames() {
    let got = run_and_get(
        r#"
        a = new_anim({7, 8, 9}, 2)
        for i = 1, 2 do anim_update(a) end
        first = anim_sprite(a)
        for i = 1, 2 do anim_update(a) end
        second = anim_sprite(a)
        "#,
        &["first", "second"],
    );
    assert_eq!(got, vec!["8", "9"]);
}

#[test]
fn prelude_particles_spawn_update_expire() {
    let got = run_and_get(
        r#"
        Particles.spawn(1, 1, 1, 0, 8, 2)
        n0 = Particles.count()
        Particles.draw()
        Particles.update()
        n1 = Particles.count()
        Particles.update()
        n2 = Particles.count()
        "#,
        &["n0", "n1", "n2"],
    );
    assert_eq!(got, vec!["1", "1", "0"]);
}

/// Pokes an 8x8 "L" sprite (id 0, palette color 8) into sprite RAM:
/// a full left column plus a full bottom row. Asymmetric under every
/// flip/rotate combination, so each transform produces a distinct,
/// checkable pixel set.
fn poke_l_sprite(vm: &mut Vm) {
    let base = SPRITE_SHEET_RAM_BASE;
    for sy in 0..8usize {
        for sx in 0..8usize {
            let lit = sx == 0 || sy == 7;
            vm.poke_memory(base + sy * 8 + sx, if lit { 8 } else { 0 });
        }
    }
}

/// Returns the set of (x, y) offsets within an 8x8 region at (ox, oy)
/// that are lit (non-background) after drawing.
fn lit_offsets(vm: &Vm, ox: u32, oy: u32) -> std::collections::BTreeSet<(u32, u32)> {
    let mut set = std::collections::BTreeSet::new();
    for dy in 0..8u32 {
        for dx in 0..8u32 {
            if read_rgba(vm, ox + dx, oy + dy) != [0, 0, 0, 0] {
                set.insert((dx, dy));
            }
        }
    }
    set
}

#[test]
fn lua_sprite_no_optional_args_matches_current_output() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 7 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_flip_x_mirrors_horizontally() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, true, false) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // Left column (sx==0) mirrors to the right column (sx==7); bottom row unchanged.
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 7 || sy == 7 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_flip_y_mirrors_vertically() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, true) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // Bottom row (sy==7) mirrors to the top row (sy==0); left column unchanged.
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 0 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_rotate_90_clockwise() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, false, 90) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // 90 deg CW: source (sx, sy) -> (7 - sy, sx). Left column (sx==0) -> top row
    // (dy==0); bottom row (sy==7) -> right column (dx==7).
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 7 {
                let (dx, dy) = (7 - sy, sx);
                expected.insert((dx, dy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_invalid_rotate_errors() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, false, 45) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert!(vm.get_fault().is_some(), "expected a fault for rotate=45");
}

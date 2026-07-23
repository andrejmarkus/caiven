use caiven_core::memory::{RTC_RAM_BASE, SFX_RAM_BASE};
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{LuaRunOutcome, Vm, VmConfig, VmFault, describe_lua_error};

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
    match vm.run_frame_lua_bp(&input, &font, &[4]) {
        LuaRunOutcome::Breakpoint(line) => assert_eq!(line, 4),
        other => panic!("expected a breakpoint stop, got {other:?}"),
    }
    assert_eq!(vm.get_fault(), None, "a breakpoint stop isn't a fault");
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

    match vm.run_frame_lua_bp(&input, &font, &[999]) {
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
        set_tile(0, 0, 5)
        set_sprite_flags(5, 1)
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

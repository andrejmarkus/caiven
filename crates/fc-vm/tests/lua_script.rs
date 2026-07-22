use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig, VmFault, default_instruction_set};
use std::sync::Arc;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()), VmConfig::default())
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
    vm.load_lua_source(
        r#"
        function _update()
          clear_screen()
          set_pixel(10, 20, 8)
        end
        "#,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let input = Input::new();
    let font = Font::empty();
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // Palette index 8 = red, (200, 60, 70) per DEFAULT_COLORS.
    assert_eq!(read_rgba(&vm, 10, 20), [200, 60, 70, 255]);
}

#[test]
fn lua_btn_reads_input_state() {
    let mut vm = make_vm();
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
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let mut input = Input::new();
    input.set_button(fc_vm::input::Button::A, true);
    let font = Font::empty();
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // color index 1 = dark blue (32, 51, 123) confirms the true branch ran.
    assert_eq!(read_rgba(&vm, 0, 0), [32, 51, 123, 255]);
}

#[test]
fn lua_runtime_error_faults_cleanly() {
    let mut vm = make_vm();
    vm.load_lua_source(
        r#"
        function _update()
          error("boom")
        end
        "#,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    let input = Input::new();
    let font = Font::empty();
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), Some(VmFault::LuaError));
}

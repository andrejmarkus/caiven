use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::vm::Vm;
use caiven_vm::vm::config::VmConfig;

fn fresh_vm() -> (Vm, Input, Font) {
    (Vm::new(VmConfig::default()), Input::new(), Font::empty())
}

#[test]
fn dset_dget_round_trip_and_default_zero() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "dset(0, 42); assert(dget(0) == 42); assert(dget(1) == 0)",
        &input,
        &font,
    )
    .expect("dset/dget round trip");
}

#[test]
fn dset_out_of_range_slot_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dset(64, 1)", &input, &font);
    assert!(result.is_err(), "slot 64 is out of the 0-63 range");
}

#[test]
fn dget_out_of_range_slot_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dget(64)", &input, &font);
    assert!(result.is_err(), "slot 64 is out of the 0-63 range");
}

#[test]
fn load_data_with_no_prior_save_returns_empty_table() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "local t = load_data(); local count = 0; for _ in pairs(t) do count = count + 1 end; assert(count == 0)",
        &input,
        &font,
    )
    .expect("load_data with nothing saved yet returns {}");
}

#[test]
fn save_data_load_data_round_trip() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "save_data({ level = 3, name = 'ok' }); local t = load_data(); assert(t.level == 3); assert(t.name == 'ok')",
        &input,
        &font,
    )
    .expect("save_data/load_data round trip");
}

#[test]
fn save_data_over_size_cap_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let src = "save_data({ s = string.rep('x', 5000) })";
    let result = vm.load_lua_source(src, &input, &font);
    assert!(result.is_err(), "5000+ bytes must exceed the 4096-byte cap");
}

#[test]
fn dset_marks_vm_save_data_dirty() {
    let (mut vm, input, font) = fresh_vm();
    assert!(!vm.save_data().is_dirty());
    vm.load_lua_source("dset(0, 1)", &input, &font)
        .expect("dset succeeds");
    assert!(vm.save_data().is_dirty());
}

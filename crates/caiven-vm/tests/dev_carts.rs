use caiven_vm::input::{Button, Input};
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};
use std::path::Path;

/// Every checked-in dev cart boots and runs a few seconds of input without a
/// Lua fault — an API rename that misses a demo fails here, not in a player.
#[test]
fn every_dev_cart_runs_without_faulting() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../carts/dev");
    let mut carts: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "cav"))
        .collect();
    carts.sort();
    assert!(!carts.is_empty(), "no dev carts found in {}", dir.display());

    let font = Font::empty();
    for path in carts {
        let cart = caiven_cart::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut vm = Vm::new(VmConfig::default());
        let source = vm
            .load_cart_sections(&cart.sections)
            .unwrap_or_else(|| panic!("{}: no Lua source", path.display()));
        let mut input = Input::new();
        vm.load_lua_source(&source, &input, &font)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for frame in 0..180 {
            input.set_button(Button::Right, frame % 60 < 40);
            input.set_button(Button::A, frame % 20 == 0);
            vm.run_frame(&input, &font);
            assert_eq!(
                vm.get_fault(),
                None,
                "{} faulted on frame {frame}: {:?}",
                path.display(),
                vm.fault_message()
            );
        }
    }
}

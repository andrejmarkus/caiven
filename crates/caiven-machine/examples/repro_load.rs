use caiven_vm::{Vm, VmConfig};

fn main() {
    let path = std::env::args().nth(1).expect("cart path");
    let bytes = std::fs::read(&path).expect("read cart");
    let cart = caiven_cart::parse(&bytes).expect("parse cart");

    let config = VmConfig::default();
    let mut vm = Vm::new(config);

    for section in &cart.sections {
        if section.kind == caiven_cart::SectionKind::ModManifest {
            let manifest = String::from_utf8_lossy(&section.data);
            let registered = vm.registered_peripheral_names();
            for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !registered.contains(&required) {
                    panic!("cart requires mod '{}' but it is not loaded", required);
                }
            }
        }
    }

    let input = caiven_vm::input::Input::new();
    let font = caiven_vm::rendering::font::Font::from_bytes(
        include_bytes!("../../../assets/font.png"),
        " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>",
        3,
        5,
    )
    .expect("font");

    let lua_source = vm
        .load_cart_sections(&cart.sections)
        .expect("cart has no Lua source section");
    vm.load_lua_source(&lua_source, &input, &font)
        .expect("load_lua_source");

    println!("loaded ok, now ticking...");
    for i in 0..120 {
        let outcome = vm.run_frame_lua_bp(&input, &font, &[]);
        if let caiven_vm::LuaRunOutcome::Error(location, message) = outcome {
            let text = match location {
                Some(location) => format!("{}:{}: {message}", location.source, location.line),
                None => message,
            };
            panic!("frame {i} fault: {text}");
        }
    }
    println!("ran 120 frames ok");
}

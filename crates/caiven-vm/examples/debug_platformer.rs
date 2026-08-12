//! Scratch debug tool: loads a .cav, drives it with scripted input, dumps
//! PNG screenshots at chosen frames. Not part of any shipped build — used to
//! visually verify the platformer showcase cart headlessly (no GUI access
//! in this environment). Delete once no longer needed.
use caiven_cart::SectionKind;
use caiven_core::memory::{COLLISION_RAM_BASE, MAP_RAM_BASE, MAP_W};
use caiven_vm::input::Input;
use caiven_vm::input::button::Button;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};
use std::path::Path;

struct StderrLogger;
impl log::Log for StderrLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        eprintln!("[{}] {}", record.level(), record.args());
    }
    fn flush(&self) {}
}

fn dump_tiles(vm: &Vm, ox: usize, oy: usize, w: usize, h: usize) {
    println!("--- tile ids (ox={ox} oy={oy}) ---");
    for ty in 0..h {
        let mut row = String::new();
        for tx in 0..w {
            let addr = MAP_RAM_BASE + (oy + ty) * MAP_W + (ox + tx);
            row.push_str(&format!("{:2} ", vm.peek_memory(addr)));
        }
        println!("{row}");
    }
    println!("--- collision ids (ox={ox} oy={oy}) ---");
    for ty in 0..h {
        let mut row = String::new();
        for tx in 0..w {
            let addr = COLLISION_RAM_BASE + (oy + ty) * MAP_W + (ox + tx);
            row.push_str(&format!("{} ", vm.peek_memory(addr)));
        }
        println!("{row}");
    }
}

fn dump_player(vm: &mut Vm) {
    let globals = vm.lua_globals();
    let Some((_, player)) = globals.iter().find(|(name, _)| name == "player") else {
        println!("player: <not found in globals>");
        return;
    };
    let Some(node_id) = player.node_id.clone() else {
        println!("player: {}", player.text);
        return;
    };
    let fields = vm.expand_debug_node(&node_id).unwrap_or_default();
    let mut out = String::from("player {");
    for (k, v) in &fields {
        if k == "pos" {
            if let Some(pos_id) = &v.node_id {
                let pos_fields = vm.expand_debug_node(pos_id).unwrap_or_default();
                out.push_str("pos={");
                for (pk, pv) in &pos_fields {
                    out.push_str(&format!("{pk}={} ", pv.text));
                }
                out.push('}');
                continue;
            }
        }
        out.push_str(&format!("{k}={} ", v.text));
    }
    out.push('}');
    println!("{out}");
}

fn dump_table(vm: &mut Vm, name: &str) -> String {
    let globals = vm.lua_globals();
    let Some((_, val)) = globals.iter().find(|(n, _)| n == name) else {
        return format!("{name}: <not found>");
    };
    let Some(node_id) = val.node_id.clone() else {
        return format!("{name}: {}", val.text);
    };
    let fields = vm.expand_debug_node(&node_id).unwrap_or_default();
    let mut out = format!("{name} {{");
    for (k, v) in &fields {
        out.push_str(&format!("{k}={} ", v.text));
    }
    out.push('}');
    out
}

fn dump(vm: &Vm, path: &str) {
    let w = VmConfig::default().width;
    let h = VmConfig::default().height;
    let px = vm.world_pixels();
    let img = image::RgbaImage::from_raw(w, h, px.to_vec()).expect("pixel buffer size mismatch");
    img.save(path).expect("failed to write png");
    println!("wrote {path}");
}

fn main() {
    static LOGGER: StderrLogger = StderrLogger;
    log::set_logger(&LOGGER).expect("logger init");
    log::set_max_level(log::LevelFilter::Error);

    let cav_path = std::env::args()
        .nth(1)
        .expect("usage: debug_platformer <cav path> <out dir>");
    let out_dir = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/tmp".to_string());

    let cart = caiven_cart::open(Path::new(&cav_path)).expect("failed to open cart");
    let mut vm = Vm::new(VmConfig::default());

    if let Some(section) = cart
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::PreludeModules)
    {
        let manifest = String::from_utf8_lossy(&section.data);
        let modules: Vec<&str> = manifest
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        vm.set_prelude_modules(&modules)
            .expect("bad stdlib modules");
    }

    let lua_source = vm
        .load_cart_sections(&cart.sections)
        .expect("no lua source section");

    let mut input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(&lua_source, &input, &font)
        .expect("failed to run _init");

    let mut frame = 0usize;
    let step = |input: &Input, vm: &mut Vm, n: usize, frame: &mut usize| {
        for _ in 0..n {
            vm.run_frame(input, &font);
            *frame += 1;
            if let Some(fault) = vm.get_fault() {
                panic!("VM fault at frame {frame}: {fault:?}");
            }
        }
    };

    dump(&vm, &format!("{out_dir}/00_title.png"));

    // Press A to start play.
    input.set_button(Button::A, true);
    step(&input, &mut vm, 1, &mut frame);
    input.set_button(Button::A, false);
    step(&input, &mut vm, 1, &mut frame);
    dump(&vm, &format!("{out_dir}/01_room1_start.png"));
    dump_tiles(&vm, 0, 0, 16, 16);

    // "Dumb bot": hold Right throughout, mash jump and dash periodically so
    // gaps that need a jump/dash don't just stall a straight-line walker.
    // Not meant to actually win — just to prove multi-room progression,
    // camera transitions, and death/respawn all work when input isn't
    // trivial.
    input.set_button(Button::Right, true);
    for i in 0..120 {
        input.set_button(Button::A, i % 6 == 0);
        input.set_button(Button::B, i % 15 == 0);
        step(&input, &mut vm, 10, &mut frame);
        if i % 5 == 0 {
            print!("frame {frame}: ");
            dump_player(&mut vm);
            println!("  {}", dump_table(&mut vm, "GAME"));
        }
        if i % 20 == 0 {
            dump(&vm, &format!("{out_dir}/room_walk_{frame:04}.png"));
        }
    }

    println!("done at frame {frame}");
}

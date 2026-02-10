use crate::assembler::instructions::InstructionSet;
use crate::assembler::instructions::operations;
use crate::assembler::item::{ArgType, Instruction};

pub fn default_instruction_set() -> InstructionSet {
    let mut set = InstructionSet::new();

    set.register(Instruction {
        name: "CLS",
        size: 1,
        opcode: 0x00,
        args: vec![],
        execute: operations::clear_screen,
        debug_info: |_| "CLS".to_string(),
    });

    set.register(Instruction {
        name: "MOV",
        size: 4,
        opcode: 0x01,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::move_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("MOV R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "ADD",
        size: 4,
        opcode: 0x02,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::add_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("ADD R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "DEC",
        size: 2,
        opcode: 0x03,
        args: vec![ArgType::Register],
        execute: operations::decrement_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            format!("DEC R{}", reg)
        },
    });

    set.register(Instruction {
        name: "DPX",
        size: 6,
        opcode: 0x04,
        args: vec![
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
        ],
        execute: operations::draw_pixel,
        debug_info: |bytes| {
            let x = bytes[1];
            let y = bytes[2];
            let r = bytes[3];
            let g = bytes[4];
            let b = bytes[5];
            format!("DPX {}, {}, {}, {}, {}", x, y, r, g, b)
        },
    });

    set.register(Instruction {
        name: "DPXR",
        size: 6,
        opcode: 0x05,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
        ],
        execute: operations::draw_pixel_from_register,
        debug_info: |bytes| {
            let rx = bytes[1];
            let ry = bytes[2];
            let r = bytes[3];
            let g = bytes[4];
            let b = bytes[5];
            format!("DPXR R{}, R{}, {}, {}, {}", rx, ry, r, g, b)
        },
    });

    set.register(Instruction {
        name: "SND",
        size: 3,
        opcode: 0x80,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::play_sound,
        debug_info: |bytes| {
            let rf = bytes[1];
            let rv = bytes[2];
            format!("SND R{}, R{}", rf, rv)
        },
    });

    set.register(Instruction {
        name: "SNDV",
        size: 4,
        opcode: 0x81,
        args: vec![ArgType::Address, ArgType::Value],
        execute: operations::play_sound_value,
        debug_info: |bytes| {
            let freq = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let vol = bytes[3];
            format!("SNDV {}, {}", freq, vol)
        },
    });

    set.register(Instruction {
        name: "NOSND",
        size: 1,
        opcode: 0x82,
        args: vec![],
        execute: operations::stop_sound,
        debug_info: |_| "NOSND".to_string(),
    });

    set.register(Instruction {
        name: "SPT",
        size: 4,
        opcode: 0x06,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::sprite,
        debug_info: |bytes| {
            let rx = bytes[1];
            let ry = bytes[2];
            let raddr = bytes[3];
            format!("SPT R{}, R{}, R{}", rx, ry, raddr)
        },
    });

    set.register(Instruction {
        name: "PAL",
        size: 5,
        opcode: 0x07,
        args: vec![
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
            ArgType::Value,
        ],
        execute: operations::palette,
        debug_info: |bytes| {
            let index = bytes[1];
            let r = bytes[2];
            let g = bytes[3];
            let b = bytes[4];
            format!("PAL {}, {}, {}, {}", index, r, g, b)
        },
    });

    set.register(Instruction {
        name: "TIL",
        size: 7,
        opcode: 0x08,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Value,
            ArgType::Value,
        ],
        execute: operations::tilemap,
        debug_info: |bytes| {
            let rx = bytes[1];
            let ry = bytes[2];
            let raddr = bytes[3];
            let rpal = bytes[4];
            let width = bytes[5];
            let height = bytes[6];
            format!(
                "TIL R{}, R{}, R{}, R{}, {}, {}",
                rx, ry, raddr, rpal, width, height
            )
        },
    });

    set.register(Instruction {
        name: "PRN",
        size: 5,
        opcode: 0x09,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
        ],
        execute: operations::print,
        debug_info: |bytes| {
            let raddr = bytes[1];
            let rpal = bytes[2];
            let width = bytes[3];
            let height = bytes[4];
            format!("PRN R{}, R{}, {}, {}", raddr, rpal, width, height)
        },
    });

    set.register(Instruction {
        name: "JMP",
        size: 3,
        opcode: 0x10,
        args: vec![ArgType::Address],
        execute: operations::jump,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("JMP 0x{:04X}", addr)
        },
    });

    set.register(Instruction {
        name: "JNZ",
        size: 4,
        opcode: 0x11,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::jump_if_not_zero,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("JNZ R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "JZ",
        size: 4,
        opcode: 0x12,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::jump_if_zero,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("JZ R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "IN",
        size: 3,
        opcode: 0x20,
        args: vec![ArgType::Register, ArgType::Value],
        execute: operations::input,
        debug_info: |bytes| {
            let reg = bytes[1];
            let input_id = bytes[2];
            format!("IN R{}, {}", reg, input_id)
        },
    });

    set.register(Instruction {
        name: "LDM",
        size: 4,
        opcode: 0x30,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::load_from_memory,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("LDM R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "STM",
        size: 4,
        opcode: 0x31,
        args: vec![ArgType::Address, ArgType::Register],
        execute: operations::store_to_memory,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let reg = bytes[3];
            format!("STM 0x{:04X}, R{}", addr, reg)
        },
    });

    set.register(Instruction {
        name: "LDMI",
        size: 3,
        opcode: 0x32,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::load_from_memory_indirect,
        debug_info: |bytes| {
            let reg_dest = bytes[1];
            let reg_addr = bytes[2];
            format!("LDMI R{}, R{}", reg_dest, reg_addr)
        },
    });

    set.register(Instruction {
        name: "STMI",
        size: 3,
        opcode: 0x33,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::store_to_memory_indirect,
        debug_info: |bytes| {
            let reg_addr = bytes[1];
            let reg_src = bytes[2];
            format!("STMI R{}, R{}", reg_addr, reg_src)
        },
    });

    set.register(Instruction {
        name: "CPY",
        size: 7,
        opcode: 0x34,
        args: vec![ArgType::Address, ArgType::Address, ArgType::Address],
        execute: operations::copy,
        debug_info: |bytes| {
            let dest = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let src = (bytes[3] as u16) | ((bytes[4] as u16) << 8);
            let length = (bytes[5] as u16) | ((bytes[6] as u16) << 8);
            format!("CPY 0x{:04X}, 0x{:04X}, {}", dest, src, length)
        },
    });

    set.register(Instruction {
        name: "TAT",
        size: 6,
        opcode: 0x40,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Value,
        ],
        execute: operations::tile_at,
        debug_info: |bytes| {
            let rdest = bytes[1];
            let rx = bytes[2];
            let ry = bytes[3];
            let rmap = bytes[4];
            let w = bytes[5];
            format!("TAT R{}, R{}, R{}, R{}, {}", rdest, rx, ry, rmap, w)
        },
    });

    set.register(Instruction {
        name: "TSD",
        size: 4,
        opcode: 0x41,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::tile_solid,
        debug_info: |bytes| {
            let rdest = bytes[1];
            let rtile = bytes[2];
            let rflags = bytes[3];
            format!("TSD R{}, R{}, R{}", rdest, rtile, rflags)
        },
    });

    set.register(Instruction {
        name: "TXT",
        size: 6,
        opcode: 0x42,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Value,
        ],
        execute: operations::text,
        debug_info: |bytes| {
            let rx = bytes[1];
            let ry = bytes[2];
            let rcolor = bytes[3];
            let rstr = bytes[4];
            let len = bytes[5];
            format!("TXT R{}, R{}, R{}, R{}, {}", rx, ry, rcolor, rstr, len)
        },
    });

    set.register(Instruction {
        name: "POSC",
        size: 3,
        opcode: 0x60,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::set_camera_position,
        debug_info: |bytes| {
            let reg_x = bytes[1];
            let reg_y = bytes[2];
            format!("POSC R{}, R{}", reg_x, reg_y)
        },
    });

    set.register(Instruction {
        name: "MOVC",
        size: 3,
        opcode: 0x61,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::move_camera,
        debug_info: |bytes| {
            let reg_dx = bytes[1];
            let reg_dy = bytes[2];
            format!("MOVC R{}, R{}", reg_dx, reg_dy)
        },
    });

    set.register(Instruction {
        name: "LOGR",
        size: 2,
        opcode: 0x70,
        args: vec![ArgType::Register],
        execute: operations::log_register,
        debug_info: |bytes| format!("LOGR R{}", bytes[1]),
    });

    set.register(Instruction {
        name: "LOGV",
        size: 3,
        opcode: 0x71,
        args: vec![ArgType::Value],
        execute: operations::log_value,
        debug_info: |bytes| {
            let value = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("LOGV {}", value)
        },
    });

    set.register(Instruction {
        name: "WAIT",
        size: 1,
        opcode: 0xFF,
        args: vec![],
        execute: operations::wait,
        debug_info: |_| "WAIT".to_string(),
    });

    set
}

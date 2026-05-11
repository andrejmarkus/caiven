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
        name: "FILL",
        size: 2,
        opcode: 0x0E,
        args: vec![ArgType::Value],
        execute: operations::fill_screen,
        debug_info: |bytes| format!("FILL {}", bytes[1]),
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
        name: "SUB",
        size: 4,
        opcode: 0x0A,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::subtract_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("SUB R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "RND",
        size: 4,
        opcode: 0x0B,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::random_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let max = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("RND R{}, {}", reg, max)
        },
    });

    set.register(Instruction {
        name: "MOVR",
        size: 3,
        opcode: 0x0C,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::move_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("MOVR R{}, R{}", dest, src)
        },
    });

    set.register(Instruction {
        name: "SLT",
        size: 4,
        opcode: 0x0D,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::set_less_than,
        debug_info: |bytes| {
            let dest = bytes[1];
            let s1 = bytes[2];
            let s2 = bytes[3];
            format!("SLT R{}, R{}, R{}", dest, s1, s2)
        },
    });

    set.register(Instruction {
        name: "ADDR",
        size: 3,
        opcode: 0x15,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::add_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("ADDR R{}, R{}", dest, src)
        },
    });

    set.register(Instruction {
        name: "SUBR",
        size: 3,
        opcode: 0x16,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::subtract_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("SUBR R{}, R{}", dest, src)
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
        size: 4,
        opcode: 0x80,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::play_sound,
        debug_info: |bytes| {
            let rf = bytes[1];
            let rv = bytes[2];
            let rd = bytes[3];
            format!("SND R{}, R{}, R{}", rf, rv, rd)
        },
    });

    set.register(Instruction {
        name: "SNDV",
        size: 5,
        opcode: 0x81,
        args: vec![ArgType::Address, ArgType::Value, ArgType::Value],
        execute: operations::play_sound_value,
        debug_info: |bytes| {
            let freq = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let vol = bytes[3];
            let dur = bytes[4];
            format!("SNDV {}, {}, {}", freq, vol, dur)
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
        name: "NSND",
        size: 4,
        opcode: 0x83,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::play_noise,
        debug_info: |bytes| {
            let rr = bytes[1];
            let rv = bytes[2];
            let rd = bytes[3];
            format!("NSND R{}, R{}, R{}", rr, rv, rd)
        },
    });

    set.register(Instruction {
        name: "NSNDV",
        size: 5,
        opcode: 0x84,
        args: vec![ArgType::Address, ArgType::Value, ArgType::Value],
        execute: operations::play_noise_value,
        debug_info: |bytes| {
            let rate = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let vol = bytes[3];
            let dur = bytes[4];
            format!("NSNDV {}, {}, {}", rate, vol, dur)
        },
    });

    set.register(Instruction {
        name: "SSTOP",
        size: 1,
        opcode: 0x85,
        args: vec![],
        execute: operations::stop_square,
        debug_info: |_| "SSTOP".to_string(),
    });

    set.register(Instruction {
        name: "NSTOP",
        size: 1,
        opcode: 0x86,
        args: vec![],
        execute: operations::stop_noise,
        debug_info: |_| "NSTOP".to_string(),
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
        name: "JSR",
        size: 3,
        opcode: 0x13,
        args: vec![ArgType::Address],
        execute: operations::jump_subroutine,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("JSR 0x{:04X}", addr)
        },
    });

    set.register(Instruction {
        name: "RET",
        size: 1,
        opcode: 0x14,
        args: vec![],
        execute: operations::return_subroutine,
        debug_info: |_| "RET".to_string(),
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
        name: "LDMW",
        size: 4,
        opcode: 0x35,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::load_word_from_memory,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("LDMW R{}, 0x{:04X}", reg, addr)
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
        name: "STMW",
        size: 4,
        opcode: 0x36,
        args: vec![ArgType::Address, ArgType::Register],
        execute: operations::store_word_to_memory,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let reg = bytes[3];
            format!("STMW 0x{:04X}, R{}", addr, reg)
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
        name: "NUM",
        size: 5,
        opcode: 0x43,
        args: vec![
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
            ArgType::Register,
        ],
        execute: operations::draw_number,
        debug_info: |bytes| {
            let rx = bytes[1];
            let ry = bytes[2];
            let rcol = bytes[3];
            let rval = bytes[4];
            format!("NUM R{}, R{}, R{}, R{}", rx, ry, rcol, rval)
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

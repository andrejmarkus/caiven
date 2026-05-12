use crate::isa::operations;
use crate::isa::{Instruction, InstructionSet};

pub fn default_instruction_set() -> InstructionSet {
    let mut set = InstructionSet::new();

    set.register(Instruction {
        name: "CLS",
        opcode: 0x00,
        execute: operations::clear_screen,
        debug_info: |_| "CLS".to_string(),
    });

    set.register(Instruction {
        name: "FILL",
        opcode: 0x0E,
        execute: operations::fill_screen,
        debug_info: |bytes| format!("FILL {}", bytes[1]),
    });

    set.register(Instruction {
        name: "MOV",
        opcode: 0x01,
        execute: operations::move_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("MOV R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "ADD",
        opcode: 0x02,
        execute: operations::add_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("ADD R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "SUB",
        opcode: 0x0A,
        execute: operations::subtract_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let value = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("SUB R{}, {}", reg, value)
        },
    });

    set.register(Instruction {
        name: "RND",
        opcode: 0x0B,
        execute: operations::random_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            let max = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("RND R{}, {}", reg, max)
        },
    });

    set.register(Instruction {
        name: "MOVR",
        opcode: 0x0C,
        execute: operations::move_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("MOVR R{}, R{}", dest, src)
        },
    });

    set.register(Instruction {
        name: "SLT",
        opcode: 0x0D,
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
        opcode: 0x15,
        execute: operations::add_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("ADDR R{}, R{}", dest, src)
        },
    });

    set.register(Instruction {
        name: "SUBR",
        opcode: 0x16,
        execute: operations::subtract_register,
        debug_info: |bytes| {
            let dest = bytes[1];
            let src = bytes[2];
            format!("SUBR R{}, R{}", dest, src)
        },
    });

    set.register(Instruction {
        name: "DEC",
        opcode: 0x03,
        execute: operations::decrement_value,
        debug_info: |bytes| {
            let reg = bytes[1];
            format!("DEC R{}", reg)
        },
    });

    set.register(Instruction {
        name: "DPX",
        opcode: 0x04,
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
        opcode: 0x05,
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
        opcode: 0x80,
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
        opcode: 0x81,
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
        opcode: 0x82,
        execute: operations::stop_sound,
        debug_info: |_| "NOSND".to_string(),
    });

    set.register(Instruction {
        name: "NSND",
        opcode: 0x83,
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
        opcode: 0x84,
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
        opcode: 0x85,
        execute: operations::stop_square,
        debug_info: |_| "SSTOP".to_string(),
    });

    set.register(Instruction {
        name: "NSTOP",
        opcode: 0x86,
        execute: operations::stop_noise,
        debug_info: |_| "NSTOP".to_string(),
    });

    set.register(Instruction {
        name: "SPT",
        opcode: 0x06,
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
        opcode: 0x07,
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
        opcode: 0x08,
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
        opcode: 0x09,
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
        opcode: 0x10,
        execute: operations::jump,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("JMP 0x{:04X}", addr)
        },
    });

    set.register(Instruction {
        name: "JNZ",
        opcode: 0x11,
        execute: operations::jump_if_not_zero,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("JNZ R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "JZ",
        opcode: 0x12,
        execute: operations::jump_if_zero,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("JZ R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "JSR",
        opcode: 0x13,
        execute: operations::jump_subroutine,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("JSR 0x{:04X}", addr)
        },
    });

    set.register(Instruction {
        name: "RET",
        opcode: 0x14,
        execute: operations::return_subroutine,
        debug_info: |_| "RET".to_string(),
    });

    set.register(Instruction {
        name: "IN",
        opcode: 0x20,
        execute: operations::input,
        debug_info: |bytes| {
            let reg = bytes[1];
            let input_id = bytes[2];
            format!("IN R{}, {}", reg, input_id)
        },
    });

    set.register(Instruction {
        name: "LDM",
        opcode: 0x30,
        execute: operations::load_from_memory,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("LDM R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "LDMW",
        opcode: 0x35,
        execute: operations::load_word_from_memory,
        debug_info: |bytes| {
            let reg = bytes[1];
            let addr = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            format!("LDMW R{}, 0x{:04X}", reg, addr)
        },
    });

    set.register(Instruction {
        name: "STM",
        opcode: 0x31,
        execute: operations::store_to_memory,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let reg = bytes[3];
            format!("STM 0x{:04X}, R{}", addr, reg)
        },
    });

    set.register(Instruction {
        name: "STMW",
        opcode: 0x36,
        execute: operations::store_word_to_memory,
        debug_info: |bytes| {
            let addr = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            let reg = bytes[3];
            format!("STMW 0x{:04X}, R{}", addr, reg)
        },
    });

    set.register(Instruction {
        name: "LDMI",
        opcode: 0x32,
        execute: operations::load_from_memory_indirect,
        debug_info: |bytes| {
            let reg_dest = bytes[1];
            let reg_addr = bytes[2];
            format!("LDMI R{}, R{}", reg_dest, reg_addr)
        },
    });

    set.register(Instruction {
        name: "STMI",
        opcode: 0x33,
        execute: operations::store_to_memory_indirect,
        debug_info: |bytes| {
            let reg_addr = bytes[1];
            let reg_src = bytes[2];
            format!("STMI R{}, R{}", reg_addr, reg_src)
        },
    });

    set.register(Instruction {
        name: "CPY",
        opcode: 0x34,
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
        opcode: 0x40,
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
        opcode: 0x41,
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
        opcode: 0x42,
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
        opcode: 0x43,
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
        opcode: 0x60,
        execute: operations::set_camera_position,
        debug_info: |bytes| {
            let reg_x = bytes[1];
            let reg_y = bytes[2];
            format!("POSC R{}, R{}", reg_x, reg_y)
        },
    });

    set.register(Instruction {
        name: "MOVC",
        opcode: 0x61,
        execute: operations::move_camera,
        debug_info: |bytes| {
            let reg_dx = bytes[1];
            let reg_dy = bytes[2];
            format!("MOVC R{}, R{}", reg_dx, reg_dy)
        },
    });

    set.register(Instruction {
        name: "LOGR",
        opcode: 0x70,
        execute: operations::log_register,
        debug_info: |bytes| format!("LOGR R{}", bytes[1]),
    });

    set.register(Instruction {
        name: "LOGV",
        opcode: 0x71,
        execute: operations::log_value,
        debug_info: |bytes| {
            let value = (bytes[1] as u16) | ((bytes[2] as u16) << 8);
            format!("LOGV {}", value)
        },
    });

    set.register(Instruction {
        name: "WAIT",
        opcode: 0xFF,
        execute: operations::wait,
        debug_info: |_| "WAIT".to_string(),
    });

    set
}

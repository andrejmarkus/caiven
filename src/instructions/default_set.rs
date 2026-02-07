use crate::instructions::{ArgType, Instruction, InstructionSet, operations};

pub fn default_instruction_set() -> InstructionSet {
    let mut set = InstructionSet::new();

    set.register(Instruction {
        name: "CLS",
        size: 1,
        opcode: 0x00,
        args: vec![],
        execute: operations::clear_screen,
    });

    set.register(Instruction {
        name: "MOV",
        size: 3,
        opcode: 0x01,
        args: vec![ArgType::Register, ArgType::Value],
        execute: operations::move_value,
    });

    set.register(Instruction {
        name: "ADD",
        size: 3,
        opcode: 0x02,
        args: vec![ArgType::Register, ArgType::Value],
        execute: operations::add_value,
    });

    set.register(Instruction {
        name: "DEC",
        size: 2,
        opcode: 0x03,
        args: vec![ArgType::Register],
        execute: operations::decrement_value,
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
    });

    set.register(Instruction {
        name: "SPT",
        size: 4,
        opcode: 0x06,
        args: vec![ArgType::Register, ArgType::Register, ArgType::Register],
        execute: operations::sprite,
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
    });

    set.register(Instruction {
        name: "JMP",
        size: 3,
        opcode: 0x10,
        args: vec![ArgType::Address],
        execute: operations::jump,
    });

    set.register(Instruction {
        name: "JNZ",
        size: 4,
        opcode: 0x11,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::jump_if_not_zero,
    });

    set.register(Instruction {
        name: "JZ",
        size: 4,
        opcode: 0x12,
        args: vec![ArgType::Register, ArgType::Address],
        execute: operations::jump_if_zero,
    });

    set.register(Instruction {
        name: "IN",
        size: 3,
        opcode: 0x20,
        args: vec![ArgType::Register, ArgType::Value],
        execute: operations::input,
    });

    set.register(Instruction {
        name: "LDM",
        size: 3,
        opcode: 0x30,
        args: vec![ArgType::Register, ArgType::Value],
        execute: operations::load_from_memory,
    });

    set.register(Instruction {
        name: "STM",
        size: 3,
        opcode: 0x31,
        args: vec![ArgType::Value, ArgType::Register],
        execute: operations::store_to_memory,
    });

    set.register(Instruction {
        name: "LDMI",
        size: 3,
        opcode: 0x32,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::load_from_memory_indirect,
    });

    set.register(Instruction {
        name: "STMI",
        size: 3,
        opcode: 0x33,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::store_to_memory_indirect,
    });

    set.register(Instruction {
        name: "CPY",
        size: 7,
        opcode: 0x34,
        args: vec![ArgType::Address, ArgType::Address, ArgType::Address],
        execute: operations::copy,
    });

    set.register(Instruction {
        name: "POSC",
        size: 3,
        opcode: 0x60,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::set_camera_position,
    });

    set.register(Instruction {
        name: "MOVC",
        size: 3,
        opcode: 0x61,
        args: vec![ArgType::Register, ArgType::Register],
        execute: operations::move_camera,
    });

    set.register(Instruction {
        name: "WAIT",
        size: 1,
        opcode: 0xFF,
        args: vec![],
        execute: operations::wait,
    });

    set
}

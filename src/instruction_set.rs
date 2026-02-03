use log::info;

use crate::{
    buttons::Button,
    instruction::{ArgType, Instruction},
};

pub struct InstructionSet {
    pub instructions: Vec<Instruction>,
}

impl InstructionSet {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    pub fn default() -> Self {
        let mut set = Self::new();

        set.register(Instruction {
            name: "CLS",
            size: 1,
            opcode: 0x00,
            args: vec![],
            execute: |_vm, _input, screen| {
                screen.clear();
            },
        });

        set.register(Instruction {
            name: "MOV",
            size: 3,
            opcode: 0x01,
            args: vec![ArgType::Register, ArgType::Value],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let value = _vm.get_program()[_vm.get_pc() + 1];

                if reg_index < _vm.get_registers_len() {
                    info!("Moved value {} into register {}", value, reg_index);
                    _vm.set_register(reg_index, value);
                } else {
                    panic!("Invalid register index: {}", reg_index);
                }
                _vm.shift_pc(2);
            },
        });

        set.register(Instruction {
            name: "ADD",
            size: 3,
            opcode: 0x02,
            args: vec![ArgType::Register, ArgType::Value],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let value = _vm.get_program()[_vm.get_pc() + 1];

                if reg_index < _vm.get_registers_len() {
                    info!("Added value {} to register {}", value, reg_index);
                    _vm.increment_register_value(reg_index, value);
                } else {
                    panic!("Invalid register index: {}", reg_index);
                }
                _vm.shift_pc(2);
            },
        });

        set.register(Instruction {
            name: "DEC",
            size: 2,
            opcode: 0x03,
            args: vec![ArgType::Register],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;

                if reg_index < _vm.get_registers_len() {
                    info!("Decremented register {}", reg_index);
                    _vm.decrement_register_value(reg_index, 1);
                } else {
                    panic!("Invalid register index: {}", reg_index);
                }
                _vm.shift_pc(1);
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
            execute: |_vm, _input, screen| {
                let x = _vm.get_program()[_vm.get_pc()] as u32;
                let y = _vm.get_program()[_vm.get_pc() + 1] as u32;
                let r = _vm.get_program()[_vm.get_pc() + 2];
                let g = _vm.get_program()[_vm.get_pc() + 3];
                let b = _vm.get_program()[_vm.get_pc() + 4];

                info!(
                    "Drawing pixel at ({}, {}) with color ({}, {}, {})",
                    x, y, r, g, b
                );
                screen.set_pixel(x, y, r, g, b);
                _vm.shift_pc(5);
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
            execute: |_vm, _input, screen| {
                let rx = _vm.get_program()[_vm.get_pc()] as usize;
                let ry = _vm.get_program()[_vm.get_pc() + 1] as usize;
                let r = _vm.get_program()[_vm.get_pc() + 2];
                let g = _vm.get_program()[_vm.get_pc() + 3];
                let b = _vm.get_program()[_vm.get_pc() + 4];

                let x = _vm.get_register_value(rx) as u32;
                let y = _vm.get_register_value(ry) as u32;

                info!(
                    "Drawing pixel at ({}, {}) with color ({}, {}, {}) from registers r{} and r{}",
                    x, y, r, g, b, rx, ry
                );
                screen.set_pixel(x, y, r, g, b);
                _vm.shift_pc(5);
            },
        });

        set.register(Instruction {
            name: "JMP",
            size: 3,
            opcode: 0x10,
            args: vec![ArgType::Address],
            execute: |_vm, _input, _screen| {
                let low = _vm.get_program()[_vm.get_pc()] as u16;
                let high = _vm.get_program()[_vm.get_pc() + 1] as u16;
                let address = Self::read_address(low, high);

                info!("Jumping to address {}", address);
                _vm.set_pc(address as usize);
            },
        });

        set.register(Instruction {
            name: "JNZ",
            size: 4,
            opcode: 0x11,
            args: vec![ArgType::Register, ArgType::Address],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let low = _vm.get_program()[_vm.get_pc() + 1] as u16;
                let high = _vm.get_program()[_vm.get_pc() + 2] as u16;
                let address = Self::read_address(low, high);

                let reg_value = _vm.get_register_value(reg_index);
                info!(
                    "JNZ check: register {} value is {}, jumping to {} if not zero",
                    reg_index, reg_value, address
                );
                if reg_value != 0 {
                    _vm.set_pc(address as usize);
                } else {
                    _vm.shift_pc(3);
                }
            },
        });

        set.register(Instruction {
            name: "JZ",
            size: 4,
            opcode: 0x12,
            args: vec![ArgType::Register, ArgType::Address],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let low = _vm.get_program()[_vm.get_pc() + 1] as u16;
                let high = _vm.get_program()[_vm.get_pc() + 2] as u16;
                let address = Self::read_address(low, high);

                let reg_value = _vm.get_register_value(reg_index);
                info!(
                    "JZ check: register {} value is {}, jumping to {} if zero",
                    reg_index, reg_value, address
                );
                if reg_value == 0 {
                    _vm.set_pc(address as usize);
                } else {
                    _vm.shift_pc(3);
                }
            },
        });

        set.register(Instruction {
            name: "IN",
            size: 3,
            opcode: 0x20,
            args: vec![ArgType::Register, ArgType::Value],
            execute: |_vm, input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let button_code = _vm.get_program()[_vm.get_pc() + 1];

                let pressed = Button::from_u8(button_code)
                    .map(|btn| input.is_pressed(btn))
                    .unwrap_or(false);

                info!(
                    "Reading input for button code {} into register {}: {}",
                    button_code, reg_index, pressed
                );
                _vm.set_register(reg_index, if pressed { 1 } else { 0 });
                _vm.shift_pc(2);
            },
        });

        set.register(Instruction {
            name: "LDM",
            size: 3,
            opcode: 0x30,
            args: vec![ArgType::Register, ArgType::Value],
            execute: |_vm, _input, _screen| {
                let reg_index = _vm.get_program()[_vm.get_pc()] as usize;
                let address = _vm.get_program()[_vm.get_pc() + 1] as usize;

                let value = _vm.read_memory(address);

                info!(
                    "Loaded value {} from memory address {} into register {}",
                    value, address, reg_index
                );
                _vm.set_register(reg_index, value);
                _vm.shift_pc(2);
            },
        });

        set.register(Instruction {
            name: "STM",
            size: 3,
            opcode: 0x31,
            args: vec![ArgType::Value, ArgType::Register],
            execute: |_vm, _input, _screen| {
                let address = _vm.get_program()[_vm.get_pc()] as usize;
                let reg_index = _vm.get_program()[_vm.get_pc() + 1] as usize;

                let value = _vm.get_register_value(reg_index);

                info!(
                    "Stored value {} from register {} into memory address {}",
                    value, reg_index, address
                );
                _vm.write_memory(address, value);
                _vm.shift_pc(2);
            },
        });

        set.register(Instruction {
            name: "WAIT",
            size: 1,
            opcode: 0xFF,
            args: vec![],
            execute: |_vm, _input, _screen| {
                info!("Waiting for next frame");
                _vm.pause();
            },
        });

        set
    }

    pub fn register(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn get_by_opcode(&self, opcode: u8) -> Option<&Instruction> {
        self.instructions
            .iter()
            .find(|instr| instr.opcode == opcode)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Instruction> {
        self.instructions.iter().find(|instr| instr.name == name)
    }

    fn read_address(low: u16, high: u16) -> u16 {
        low | (high << 8)
    }
}

use crate::buttons::Button;
use crate::input::Input;
use crate::screen::Screen;
use log::info;

pub struct Vm {
    pc: usize,
    program: Vec<u8>,
    registers: [u8; 4],
    ram: [u8; 256],
    waiting: bool,
}

impl Vm {
    pub fn new(program: Vec<u8>) -> Self {
        Self {
            pc: 0,
            program,
            registers: [0; 4],
            ram: [0; 256],
            waiting: false,
        }
    }

    pub fn run_frame(&mut self, input: &Input, screen: &mut Screen) {
        self.waiting = false;

        while !self.waiting {
            self.step(input, screen);
        }
    }

    pub fn step(&mut self, input: &Input, screen: &mut Screen) {
        if self.pc < self.program.len() {
            let opcode = self.program[self.pc];
            self.pc += 1;

            info!("Executing opcode: 0x{:02X}", opcode);
            match opcode {
                0x00 => {
                    // CLS
                    info!("Cleared the screen");
                    screen.clear();
                }
                0x01 => {
                    // MOV r value
                    let reg_index = self.program[self.pc] as usize;
                    let value = self.program[self.pc + 1];

                    if reg_index < self.registers.len() {
                        self.registers[reg_index] = value;
                        info!("Moved value {} into register {}", value, reg_index);
                    } else {
                        info!("Invalid register index: {}", reg_index);
                    }
                    self.pc += 2;
                }
                0x02 => {
                    // ADD r value
                    let reg_index = self.program[self.pc] as usize;
                    let value = self.program[self.pc + 1];

                    if reg_index < self.registers.len() {
                        let (result, _) = self.registers[reg_index].overflowing_add(value);
                        self.registers[reg_index] = result;
                        info!(
                            "Added value {} to register {}, result {}",
                            value, reg_index, result
                        );
                    } else {
                        info!("Invalid register index: {}", reg_index);
                    }
                    self.pc += 2;
                }
                0x03 => {
                    // DPX x y r g b
                    let x = self.program[self.pc] as u32;
                    let y = self.program[self.pc + 1] as u32;
                    let r = self.program[self.pc + 2];
                    let g = self.program[self.pc + 3];
                    let b = self.program[self.pc + 4];

                    info!("Draw pixel at x, y with color r, g, b");
                    screen.set_pixel(x, y, r, g, b);
                    self.pc += 5;
                }
                0x04 => {
                    // DPXR rx ry r g b
                    let rx = self.program[self.pc] as usize;
                    let ry = self.program[self.pc + 1] as usize;
                    let r = self.program[self.pc + 2];
                    let g = self.program[self.pc + 3];
                    let b = self.program[self.pc + 4];

                    info!("Draw pixel at (R{}, R{}) with color r, g, b", rx, ry);
                    if rx < self.registers.len() && ry < self.registers.len() {
                        let x = self.registers[rx] as u32;
                        let y = self.registers[ry] as u32;
                        screen.set_pixel(x, y, r, g, b);
                    } else {
                        info!("Invalid register index: R{} or R{}", rx, ry);
                    }
                    self.pc += 5;
                }
                0x05 => {
                    // DEC r
                    let reg_index = self.program[self.pc] as usize;
                    if reg_index < self.registers.len() {
                        let (result, _) = self.registers[reg_index].overflowing_sub(1);
                        self.registers[reg_index] = result;
                        info!("Decremented register {}, result {}", reg_index, result);
                    } else {
                        info!("Invalid register index: {}", reg_index);
                    }
                    self.pc += 1;
                }
                0x10 => {
                    // JMP addr_low addr_high
                    let addr = self.read_u16(self.pc) as usize;

                    info!("Jumping to address {}", addr);
                    self.pc = addr;
                }
                0x11 => {
                    // JNZ r addr_low addr_high
                    let reg_index = self.program[self.pc] as usize;
                    let addr = self.read_u16(self.pc + 1) as usize;

                    if self.registers[reg_index] != 0 {
                        info!(
                            "Register R{} is not zero ({}), jumping to address {}",
                            reg_index, self.registers[reg_index], addr
                        );
                        self.pc = addr;
                    } else {
                        info!("Register R{} is zero, not jumping", reg_index);
                        self.pc += 3;
                    }
                }
                0x12 => {
                    // JZ r addr_low addr_high
                    let reg_index = self.program[self.pc] as usize;
                    let addr = self.read_u16(self.pc + 1) as usize;

                    if self.registers[reg_index] == 0 {
                        info!(
                            "Register R{} is zero, jumping to address {}",
                            reg_index, addr
                        );
                        self.pc = addr;
                    } else {
                        info!(
                            "Register R{} is not zero ({}), not jumping",
                            reg_index, self.registers[reg_index]
                        );
                        self.pc += 3;
                    }
                }
                0x20 => {
                    // IN r button
                    let reg_index = self.program[self.pc] as usize;
                    let button_code = self.program[self.pc + 1];

                    let pressed = Button::from_u8(button_code)
                        .map(|btn| input.is_pressed(btn))
                        .unwrap_or(false);

                    info!(
                        "Input button {} into register {}, pressed: {}",
                        button_code, reg_index, pressed
                    );
                    self.registers[reg_index] = if pressed { 1 } else { 0 };
                    self.pc += 2;
                }
                0x30 => {
                    // LDM r addr
                    let reg_index = self.program[self.pc] as usize;
                    let addr = self.program[self.pc + 1] as usize;

                    if reg_index < self.registers.len() && addr < self.ram.len() {
                        self.registers[reg_index] = self.ram[addr];
                        info!(
                            "Loaded value {} from RAM address {} into register {}",
                            self.ram[addr], addr, reg_index
                        );
                    } else {
                        info!(
                            "Invalid register index: {} or RAM address: {}",
                            reg_index, addr
                        );
                    }
                    self.pc += 2;
                }
                0x31 => {
                    // STM addr r
                    let addr = self.program[self.pc] as usize;
                    let reg_index = self.program[self.pc + 1] as usize;

                    if reg_index < self.registers.len() && addr < self.ram.len() {
                        self.ram[addr] = self.registers[reg_index];
                        info!(
                            "Stored value {} from register {} into RAM address {}",
                            self.registers[reg_index], reg_index, addr
                        );
                    } else {
                        info!(
                            "Invalid register index: {} or RAM address: {}",
                            reg_index, addr
                        );
                    }
                    self.pc += 2;
                }
                0xFF => {
                    // WAIT
                    info!("Waiting for next frame");
                    self.waiting = true;
                }
                _ => {
                    info!("Unknown opcode: {}", opcode);
                    self.waiting = true;
                }
            }
        }
    }

    fn read_u16(&self, pc: usize) -> u16 {
        let low = self.program[pc];
        let high = self.program[pc + 1];
        low as u16 | ((high as u16) << 8)
    }
}

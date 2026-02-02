use crate::screen::Screen;
use log::info;

pub struct Vm {
    pc: usize,
    program: Vec<u8>,
}

impl Vm {
    pub fn new(program: Vec<u8>) -> Self {
        Self { pc: 0, program }
    }

    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn program(&self) -> &Vec<u8> {
        &self.program
    }

    pub fn step(&mut self, screen: &mut Screen) {
        if self.pc < self.program.len() {
            let opcode = self.program[self.pc];
            self.pc += 1;

            info!("Executing opcode: {}", opcode);
            match opcode {
                0x00 => {
                    // CLS
                    info!("Cleared the screen");
                    screen.clear();
                }
                0x01 => {
                    // DPX x y r g b
                    info!("Draw pixel at x, y with color r, g, b");
                    let x = self.program[self.pc] as u32;
                    let y = self.program[self.pc + 1] as u32;
                    let r = self.program[self.pc + 2];
                    let g = self.program[self.pc + 3];
                    let b = self.program[self.pc + 4];
                    screen.set_pixel(x, y, r, g, b);
                    self.pc += 5;
                }
                _ => {
                    info!("Unknown opcode: {}", opcode);
                    self.pc = self.program.len(); // Stop execution on unknown opcode
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0;
    }
}

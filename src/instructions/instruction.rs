use crate::{input::input::Input, screen::Screen, vm::Vm};

pub enum ArgType {
    Register,
    Value,
    Address, // Represents 2 bytes (u16)
}

pub struct Instruction {
    pub name: &'static str,
    pub size: usize,
    pub opcode: u8,
    pub args: Vec<ArgType>,

    pub execute: fn(vm: &mut Vm, input: &Input, screen: &mut Screen),
}

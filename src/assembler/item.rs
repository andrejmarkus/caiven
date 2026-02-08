use crate::input::input::Input;
use crate::rendering::screen::ScreenLayer;
use crate::vm::Vm;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Register,
    Value,
    Address,
}

pub type InstructionHandler = fn(vm: &mut Vm, input: &Input, layer: &mut ScreenLayer);
pub type InstructionDebugHandler = fn(bytes: &[u8]) -> String;

pub struct Instruction {
    pub name: &'static str,
    pub size: usize,
    pub opcode: u8,
    pub args: Vec<ArgType>,
    pub execute: InstructionHandler,
    pub debug_info: InstructionDebugHandler,
}

pub type DirectiveHandler =
    fn(args: &[&str], labels: &HashMap<String, u16>, pc: u16) -> Result<Vec<u8>, String>;
pub type DirectiveSizeHandler = fn(args: &[&str], pc: u16) -> usize;

pub struct Directive {
    pub name: &'static str,
    pub size: DirectiveSizeHandler,
    pub emit: DirectiveHandler,
}

#[derive(Debug, Clone)]
pub enum AssemblyItem {
    Instruction {
        name: String,
        opcode: u8,
        size: usize,
    },
    Directive {
        name: String,
        size: usize,
    },
}

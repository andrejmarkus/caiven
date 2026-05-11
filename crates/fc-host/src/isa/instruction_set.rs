use crate::vm::ExecutionContext;
use fc_asm::ArgType;

pub type InstructionHandler = fn(ctx: &mut ExecutionContext);
pub type InstructionDebugHandler = fn(bytes: &[u8]) -> String;

pub struct Instruction {
    pub name: &'static str,
    pub size: usize,
    pub opcode: u8,
    pub args: Vec<ArgType>,
    pub execute: InstructionHandler,
    pub debug_info: InstructionDebugHandler,
}

pub struct InstructionSet {
    instructions: Vec<Instruction>,
}

impl InstructionSet {
    pub fn new() -> Self {
        Self { instructions: Vec::new() }
    }

    pub fn register(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn get_by_opcode(&self, opcode: u8) -> Option<&Instruction> {
        self.instructions.iter().find(|i| i.opcode == opcode)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Instruction> {
        self.instructions.iter().find(|i| i.name == name)
    }
}

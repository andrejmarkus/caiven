use crate::vm::{ExecutionContext, VmFault};
use fc_asm::ArgType;

pub type InstructionHandler = fn(ctx: &mut ExecutionContext) -> Result<(), VmFault>;
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
    by_opcode: [Option<usize>; 256],
}

impl InstructionSet {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            by_opcode: [None; 256],
        }
    }

    pub fn register(&mut self, instruction: Instruction) {
        let opcode = instruction.opcode as usize;
        let idx = self.instructions.len();
        self.instructions.push(instruction);
        self.by_opcode[opcode] = Some(idx);
    }

    pub fn get_by_opcode(&self, opcode: u8) -> Option<&Instruction> {
        self.by_opcode[opcode as usize].map(|idx| &self.instructions[idx])
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Instruction> {
        self.instructions.iter().find(|i| i.name == name)
    }
}

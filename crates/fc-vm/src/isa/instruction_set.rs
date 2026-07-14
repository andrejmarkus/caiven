use crate::vm::{ExecutionContext, VmFault};
use fc_asm::OpcodeSpec;

pub type InstructionHandler = fn(ctx: &mut ExecutionContext) -> Result<(), VmFault>;

pub struct Instruction {
    /// Shape (mnemonic, opcode, operand types) from the shared fc-asm ISA table.
    pub spec: OpcodeSpec,
    pub execute: InstructionHandler,
}

pub struct InstructionSet {
    instructions: Vec<Instruction>,
    by_opcode: [Option<usize>; 256],
}

impl Default for InstructionSet {
    fn default() -> Self {
        Self::new()
    }
}

impl InstructionSet {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            by_opcode: [None; 256],
        }
    }

    pub fn register(&mut self, instruction: Instruction) {
        let opcode = instruction.spec.opcode as usize;
        let idx = self.instructions.len();
        self.instructions.push(instruction);
        self.by_opcode[opcode] = Some(idx);
    }

    pub fn get_by_opcode(&self, opcode: u8) -> Option<&Instruction> {
        self.by_opcode[opcode as usize].map(|idx| &self.instructions[idx])
    }
}

use crate::instructions::Instruction;

pub struct InstructionSet {
    pub instructions: Vec<Instruction>,
}

impl InstructionSet {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
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
}

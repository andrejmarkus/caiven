mod default_set;
mod instruction_set;
pub mod operations;

pub use default_set::default_instruction_set;
pub use instruction_set::{Instruction, InstructionSet};

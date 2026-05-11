mod assembler;
mod directive;
mod error;
mod isa;
mod source_map;

pub use assembler::Assembler;
pub use error::AsmError;
pub use isa::{ArgType, IsaTable, OpcodeSpec};
pub use source_map::{AddressInfo, ItemInfo, SourceMap};

pub fn assemble(source: &str) -> Result<Vec<u8>, AsmError> {
    Assembler::new().assemble(source)
}

pub fn assemble_with_source_map(source: &str) -> Result<(Vec<u8>, SourceMap), AsmError> {
    Assembler::new().assemble_with_source_map(source)
}

pub fn generate_source_map(bytecode: &[u8]) -> SourceMap {
    Assembler::new().generate_source_map(bytecode)
}

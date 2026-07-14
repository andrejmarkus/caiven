mod assembler;
mod directive;
mod error;
pub mod expr;
mod isa;
pub mod opcodes;
pub mod preprocess;
mod source_map;

pub use assembler::{Assembler, AssemblerOutput};
pub use directive::DirectiveError;
pub use error::AsmError;
pub use expr::EvalError;
pub use isa::{ArgType, IsaTable, OpcodeSpec, default_isa, default_specs};
pub use preprocess::{MacroDef, Preprocessor, SourceLine};
pub use source_map::{AddressInfo, ItemInfo, SourceMap};

pub fn assemble(source: &str) -> Result<Vec<u8>, AsmError> {
    Assembler::new().assemble(source)
}

pub fn assemble_with_source_map(source: &str) -> Result<(Vec<u8>, SourceMap), AsmError> {
    Assembler::new().assemble_with_source_map(source)
}

pub fn assemble_with_sections(source: &str) -> Result<AssemblerOutput, AsmError> {
    Assembler::new().assemble_with_sections(source)
}

pub fn assemble_file_with_sections(path: &std::path::Path) -> Result<AssemblerOutput, AsmError> {
    Assembler::new().assemble_file_with_sections(path)
}

pub fn generate_source_map(bytecode: &[u8]) -> SourceMap {
    Assembler::new().generate_source_map(bytecode)
}

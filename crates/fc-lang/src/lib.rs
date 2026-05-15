pub mod ast;
pub mod error;
pub mod lexer;
pub mod lower;
pub mod parser;

use fc_asm::SourceMap;

pub struct CompilerOutput {
    pub program: Vec<u8>,
    pub source_map: SourceMap,
}

pub fn compile(src: &str) -> error::Result<CompilerOutput> {
    let mut lex = lexer::Lexer::new(src);
    let tokens = lex.tokenize()?;
    let mut parser = parser::Parser::new(tokens);
    let file = parser.parse_file()?;
    let mut compiler = lower::Compiler::new();
    compiler.compile(&file)?;
    let (program, source_map) = compiler.finish();
    Ok(CompilerOutput { program, source_map })
}

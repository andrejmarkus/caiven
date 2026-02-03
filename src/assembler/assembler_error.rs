use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct AssemblerError {
    pub line: usize,
    pub message: String,
    pub source: String,
}

impl Display for AssemblerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Assembler Error on line {}: {}\n>> {}",
            self.line, self.message, self.source
        )
    }
}

impl std::error::Error for AssemblerError {}

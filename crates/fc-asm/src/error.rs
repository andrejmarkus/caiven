use thiserror::Error;

#[derive(Debug, Error)]
pub enum AsmError {
    #[error("line {line}: {message}\n>> {source_line}")]
    Syntax {
        line: usize,
        source_line: String,
        message: String,
    },
}

impl AsmError {
    pub fn syntax(line: usize, source_line: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Syntax {
            line,
            source_line: source_line.into(),
            message: message.into(),
        }
    }
}

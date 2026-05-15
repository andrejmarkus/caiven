use thiserror::Error;

#[derive(Debug, Error)]
pub enum LangError {
    #[error("line {line}: unexpected character '{ch}'")]
    UnexpectedChar { line: usize, ch: char },

    #[error("line {line}: unexpected token '{got}', expected {expected}")]
    UnexpectedToken { line: usize, got: String, expected: String },

    #[error("line {line}: unterminated string")]
    UnterminatedString { line: usize },

    #[error("line {line}: undefined variable '{name}'")]
    UndefinedVariable { line: usize, name: String },

    #[error("line {line}: undefined function '{name}'")]
    UndefinedFunction { line: usize, name: String },

    #[error("line {line}: wrong arg count for '{name}': expected {expected}, got {got}")]
    ArgCount { line: usize, name: String, expected: usize, got: usize },

    #[error("line {line}: '{name}' requires a literal argument")]
    RequiresLiteral { line: usize, name: String },

    #[error("line {line}: return outside function")]
    ReturnOutsideFunction { line: usize },

    #[error("line {line}: too many local variables (max 2044)")]
    TooManyLocals { line: usize },

    #[error("line {line}: break outside loop")]
    BreakOutsideLoop { line: usize },

    #[error("line {line}: '{feature}' not yet implemented")]
    NotImplemented { line: usize, feature: String },
}

pub type Result<T> = std::result::Result<T, LangError>;

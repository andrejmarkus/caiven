use thiserror::Error;

#[derive(Debug, Error)]
pub enum CartError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid cart magic bytes")]
    BadMagic,

    #[error("cart data is truncated")]
    Truncated,

    #[error("packed cart is {size} bytes; maximum is {max} bytes")]
    TooLarge { size: usize, max: usize },

    #[error("CRC32 mismatch: expected {expected:#010x}, got {actual:#010x}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    #[error("invalid caiven.toml: {0}")]
    BadToml(#[from] toml::de::Error),

    #[error("project has no entry Lua file: {0}")]
    MissingEntry(String),

    #[error("bad hex data in {file}: {message}")]
    BadHex { file: String, message: String },

    #[error("bad PNG data in {file}: {message}")]
    BadPng { file: String, message: String },

    #[error("bad JSON data in {file}: {message}")]
    BadJson { file: String, message: String },
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RomError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid ROM magic bytes")]
    BadMagic,

    #[error("ROM data is truncated")]
    Truncated,

    #[error("CRC32 mismatch: expected {expected:#010x}, got {actual:#010x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}

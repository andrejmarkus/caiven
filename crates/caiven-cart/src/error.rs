use thiserror::Error;

#[derive(Debug, Error)]
pub enum CartError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid cart magic bytes")]
    BadMagic,

    #[error("cart data is truncated")]
    Truncated,

    #[error("CRC32 mismatch: expected {expected:#010x}, got {actual:#010x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
}

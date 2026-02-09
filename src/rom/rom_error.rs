#[derive(Debug)]
pub enum RomError {
    IoError(std::io::Error),
    InvalidFormat,
}

impl std::fmt::Display for RomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RomError::IoError(e) => write!(f, "I/O Error: {}", e),
            RomError::InvalidFormat => write!(f, "Invalid ROM format"),
        }
    }
}

impl std::error::Error for RomError {}

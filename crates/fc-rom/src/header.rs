/// Fixed-length string field for ROM header.
fn str_to_field(s: &str, buf: &mut [u8; 32]) {
    buf.fill(0);
    let bytes = s.as_bytes();
    let len = bytes.len().min(32);
    buf[..len].copy_from_slice(&bytes[..len]);
}

fn field_to_str(buf: &[u8; 32]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(32);
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// ROM header metadata. Does not include program bytes.
pub struct RomHeader {
    pub title: String,
    pub author: String,
    pub entry_point: u32,
    pub flags: u32,
}

impl RomHeader {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: author.into(),
            entry_point: 0,
            flags: 0,
        }
    }

    pub fn default_for(name: &str) -> Self {
        Self::new(name, "")
    }

    /// Serializes to bytes (80 bytes: title[32] + author[32] + entry[4] + flags[4] + size[4] + crc[4]).
    /// `program_size` and `crc32` are provided separately so the header can be written atomically.
    pub(crate) fn to_bytes(&self, program_size: u32, crc32: u32) -> [u8; 80] {
        let mut buf = [0u8; 80];
        let mut title_field = [0u8; 32];
        let mut author_field = [0u8; 32];
        str_to_field(&self.title, &mut title_field);
        str_to_field(&self.author, &mut author_field);
        buf[0..32].copy_from_slice(&title_field);
        buf[32..64].copy_from_slice(&author_field);
        buf[64..68].copy_from_slice(&self.entry_point.to_le_bytes());
        buf[68..72].copy_from_slice(&self.flags.to_le_bytes());
        buf[72..76].copy_from_slice(&program_size.to_le_bytes());
        buf[76..80].copy_from_slice(&crc32.to_le_bytes());
        buf
    }

    pub(crate) fn from_bytes(buf: &[u8; 80]) -> (Self, u32, u32) {
        let mut title_field = [0u8; 32];
        let mut author_field = [0u8; 32];
        title_field.copy_from_slice(&buf[0..32]);
        author_field.copy_from_slice(&buf[32..64]);
        let title = field_to_str(&title_field);
        let author = field_to_str(&author_field);
        let entry_point = u32::from_le_bytes(buf[64..68].try_into().unwrap());
        let flags = u32::from_le_bytes(buf[68..72].try_into().unwrap());
        let program_size = u32::from_le_bytes(buf[72..76].try_into().unwrap());
        let crc32 = u32::from_le_bytes(buf[76..80].try_into().unwrap());
        (Self { title, author, entry_point, flags }, program_size, crc32)
    }
}

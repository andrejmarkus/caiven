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

pub struct CartHeader {
    pub title: String,
    pub author: String,
    pub entry_point: u32,
    pub flags: u32,
}

impl CartHeader {
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

    // 72 bytes: title[32] author[32] entry[4] flags[4]
    pub(crate) fn to_bytes(&self) -> [u8; 72] {
        let mut buf = [0u8; 72];
        let mut title_field = [0u8; 32];
        let mut author_field = [0u8; 32];
        str_to_field(&self.title, &mut title_field);
        str_to_field(&self.author, &mut author_field);
        buf[0..32].copy_from_slice(&title_field);
        buf[32..64].copy_from_slice(&author_field);
        buf[64..68].copy_from_slice(&self.entry_point.to_le_bytes());
        buf[68..72].copy_from_slice(&self.flags.to_le_bytes());
        buf
    }

    pub(crate) fn from_bytes(buf: &[u8; 72]) -> Self {
        let mut title_field = [0u8; 32];
        let mut author_field = [0u8; 32];
        title_field.copy_from_slice(&buf[0..32]);
        author_field.copy_from_slice(&buf[32..64]);
        let entry_point = u32::from_le_bytes([buf[64], buf[65], buf[66], buf[67]]);
        let flags = u32::from_le_bytes([buf[68], buf[69], buf[70], buf[71]]);
        Self {
            title: field_to_str(&title_field),
            author: field_to_str(&author_field),
            entry_point,
            flags,
        }
    }
}

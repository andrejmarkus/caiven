/// Copies `s` into a 32-byte zero-padded field, truncating on a UTF-8
/// character boundary so the stored bytes always decode cleanly.
fn str_to_field(s: &str, buf: &mut [u8; 32]) {
    buf.fill(0);
    let mut len = s.len().min(32);
    while !s.is_char_boundary(len) {
        len -= 1;
    }
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
}

fn field_to_str(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

pub struct CartHeader {
    pub title: String,
    pub author: String,
}

impl CartHeader {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: author.into(),
        }
    }

    pub fn default_for(name: &str) -> Self {
        Self::new(name, "")
    }

    // 64 bytes: title[32] author[32]
    pub(crate) fn to_bytes(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        let mut title_field = [0u8; 32];
        let mut author_field = [0u8; 32];
        str_to_field(&self.title, &mut title_field);
        str_to_field(&self.author, &mut author_field);
        buf[0..32].copy_from_slice(&title_field);
        buf[32..64].copy_from_slice(&author_field);
        buf
    }

    pub(crate) fn from_bytes(buf: &[u8; 64]) -> Self {
        Self {
            title: field_to_str(&buf[0..32]),
            author: field_to_str(&buf[32..64]),
        }
    }
}

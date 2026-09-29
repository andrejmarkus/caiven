//! Line-wrapped hex text for the project-dir `.hex` asset files.

/// Encodes bytes as line-wrapped hex, 64 bytes (128 hex chars) per line,
/// each line newline-terminated.
pub fn encode_hex_block(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2 + data.len() / 64 + 1);
    for chunk in data.chunks(64) {
        for b in chunk {
            out.push_str(&format!("{b:02x}"));
        }
        out.push('\n');
    }
    out
}

/// Decodes a block of line-wrapped hex (whitespace between lines ignored)
/// into raw bytes.
pub fn decode_hex_block(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let hex = line.trim();
        if !hex.is_empty() {
            decode_hex_line(hex, &mut out)?;
        }
    }
    Ok(out)
}

fn decode_hex_line(hex: &str, out: &mut Vec<u8>) -> Result<(), String> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(format!("odd number of hex digits in line '{hex}'"));
    }
    for pair in bytes.as_chunks::<2>().0 {
        let hi =
            hex_val(pair[0]).ok_or_else(|| format!("invalid hex digit '{}'", pair[0] as char))?;
        let lo =
            hex_val(pair[1]).ok_or_else(|| format!("invalid hex digit '{}'", pair[1] as char))?;
        out.push((hi << 4) | lo);
    }
    Ok(())
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn trim_trailing_zeros(data: &[u8]) -> &[u8] {
    let end = data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    &data[..end]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn hex_block_roundtrips_and_wraps_at_64_bytes() {
        let data: Vec<u8> = (0..130).map(|i| i as u8).collect();
        let text = encode_hex_block(&data);
        assert_eq!(text.lines().count(), 3);
        assert_eq!(decode_hex_block(&text).unwrap(), data);
    }

    #[test]
    fn bad_hex_is_rejected() {
        assert!(
            decode_hex_block("zz\n")
                .unwrap_err()
                .contains("invalid hex digit")
        );
        assert!(
            decode_hex_block("abc\n")
                .unwrap_err()
                .contains("odd number")
        );
    }

    #[test]
    fn trailing_zeros_are_trimmed() {
        assert_eq!(trim_trailing_zeros(&[1, 2, 0, 0]), &[1, 2]);
        assert!(trim_trailing_zeros(&[0, 0]).is_empty());
    }
}

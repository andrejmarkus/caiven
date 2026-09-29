//! Persistent save data: a single JSON blob, dirty-tracked so a host
//! (`caiven-machine`, `caiven-studio`) knows when to flush `encode()`'s
//! bytes to disk. This module never touches the filesystem itself —
//! `caiven-vm` must stay usable from `caiven-web`, which has no
//! filesystem. Encoding mirrors `caiven-machine/src/shell/save_state.rs`:
//! magic + version + length-prefixed blob, `decode` rejecting anything
//! that doesn't fit rather than trusting lengths it read, since a save
//! file is untrusted the same way a `.cav` is.

use std::fmt;

pub const SAVE_DATA_BLOB_MAX_BYTES: usize = 4096;

const MAGIC: &[u8; 4] = b"CVSD";
/// The only save-data version this build reads and writes; any other is
/// rejected (the cart starts with an empty blob).
const FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveDataError {
    BlobTooLarge { size: usize, max: usize },
}

impl fmt::Display for SaveDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveDataError::BlobTooLarge { size, max } => {
                write!(f, "save data is {size} bytes, over the {max}-byte limit")
            }
        }
    }
}

pub struct SaveData {
    blob: serde_json::Value,
    dirty: bool,
}

impl Default for SaveData {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveData {
    pub fn new() -> Self {
        Self {
            blob: serde_json::Value::Object(Default::default()),
            dirty: false,
        }
    }

    pub fn blob(&self) -> &serde_json::Value {
        &self.blob
    }

    pub fn set_blob(&mut self, value: serde_json::Value) -> Result<(), SaveDataError> {
        let packed = serde_json::to_vec(&value).unwrap_or_default();
        if packed.len() > SAVE_DATA_BLOB_MAX_BYTES {
            return Err(SaveDataError::BlobTooLarge {
                size: packed.len(),
                max: SAVE_DATA_BLOB_MAX_BYTES,
            });
        }
        self.blob = value;
        self.dirty = true;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn encode(&self) -> Vec<u8> {
        let blob_bytes = serde_json::to_vec(&self.blob).unwrap_or_else(|_| b"{}".to_vec());
        let mut out = Vec::with_capacity(4 + 2 + 4 + blob_bytes.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&(blob_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&blob_bytes);
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut cursor = 0usize;

        let magic = bytes.get(cursor..cursor + 4)?;
        if magic != MAGIC {
            return None;
        }
        cursor += 4;

        let version = u16::from_le_bytes(bytes.get(cursor..cursor + 2)?.try_into().ok()?);
        if version != FORMAT_VERSION {
            return None;
        }
        cursor += 2;

        let blob_len = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
        cursor += 4;
        let remaining = bytes.len().checked_sub(cursor)?;
        if blob_len > remaining {
            return None;
        }
        let blob_bytes = &bytes[cursor..cursor + blob_len];
        let blob: serde_json::Value = serde_json::from_slice(blob_bytes).ok()?;

        Some(Self { blob, dirty: false })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn default_blob_is_empty() {
        let data = SaveData::new();
        assert_eq!(data.blob(), &serde_json::Value::Object(Default::default()));
    }

    #[test]
    fn set_blob_marks_dirty() {
        let mut data = SaveData::new();
        assert!(!data.is_dirty());
        data.set_blob(serde_json::json!({ "level": 1 })).unwrap();
        assert!(data.is_dirty());
        data.clear_dirty();
        assert!(!data.is_dirty());
    }

    #[test]
    fn oversized_blob_is_rejected_without_mutating_state() {
        let mut data = SaveData::new();
        let huge = serde_json::json!({ "s": "x".repeat(SAVE_DATA_BLOB_MAX_BYTES) });
        let err = data.set_blob(huge).unwrap_err();
        assert!(matches!(err, SaveDataError::BlobTooLarge { .. }));
        assert_eq!(data.blob(), &serde_json::Value::Object(Default::default()));
        assert!(!data.is_dirty());
    }

    #[test]
    fn round_trips_blob() {
        let mut data = SaveData::new();
        data.set_blob(serde_json::json!({ "level": 3, "name": "ok" }))
            .unwrap();

        let bytes = data.encode();
        let decoded = SaveData::decode(&bytes).expect("valid save data");

        assert_eq!(
            decoded.blob(),
            &serde_json::json!({ "level": 3, "name": "ok" })
        );
        assert!(!decoded.is_dirty());
    }

    #[test]
    fn rejects_truncated_bytes() {
        let data = SaveData::new();
        let bytes = data.encode();
        assert!(SaveData::decode(&bytes[..bytes.len() - 2]).is_none());
        assert!(SaveData::decode(&[]).is_none());
    }

    #[test]
    fn rejects_bad_magic() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        bytes[0] = b'X';
        assert!(SaveData::decode(&bytes).is_none());
    }

    #[test]
    fn encodes_version_1_layout() {
        let mut data = SaveData::new();
        data.set_blob(serde_json::json!({ "a": 1 })).unwrap();
        let bytes = data.encode();
        assert_eq!(&bytes[..4], b"CVSD");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 7);
        assert_eq!(&bytes[10..], br#"{"a":1}"#);
    }

    #[test]
    fn rejects_unknown_version() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert!(SaveData::decode(&bytes).is_none());
    }

    #[test]
    fn rejects_absurd_blob_len_without_overflow() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        // cursor is at 6 after magic (4) + version (2) reads, right before blob_len.
        // Overwrite it with u32::MAX, which would overflow cursor+blob_len on 32-bit.
        let blob_len_offset = 4 + 2;
        bytes[blob_len_offset..blob_len_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        // This must return None, not panic with "attempt to add with overflow".
        assert!(SaveData::decode(&bytes).is_none());
    }
}

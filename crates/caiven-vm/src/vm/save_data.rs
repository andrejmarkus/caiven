//! Persistent save data: 64 numeric slots plus a JSON blob, both
//! dirty-tracked so a host (`caiven-machine`, `caiven-studio`) knows when
//! to flush `encode()`'s bytes to disk. This module never touches the
//! filesystem itself — `caiven-vm` must stay usable from `caiven-web`,
//! which has no filesystem. Encoding mirrors
//! `caiven-machine/src/shell/save_state.rs`: magic + version +
//! length-prefixed sections, `decode` rejecting anything that doesn't fit
//! rather than trusting lengths it read, since a save file is untrusted
//! the same way a `.cav` is.

use std::fmt;

pub const SAVE_DATA_SLOT_COUNT: usize = 64;
pub const SAVE_DATA_BLOB_MAX_BYTES: usize = 4096;

const MAGIC: &[u8; 4] = b"CVSD";
const FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveDataError {
    SlotOutOfRange(u8),
    BlobTooLarge { size: usize, max: usize },
}

impl fmt::Display for SaveDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveDataError::SlotOutOfRange(slot) => write!(
                f,
                "slot {slot} out of range (0-{})",
                SAVE_DATA_SLOT_COUNT - 1
            ),
            SaveDataError::BlobTooLarge { size, max } => {
                write!(f, "save data is {size} bytes, over the {max}-byte limit")
            }
        }
    }
}

pub struct SaveData {
    slots: [f64; SAVE_DATA_SLOT_COUNT],
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
            slots: [0.0; SAVE_DATA_SLOT_COUNT],
            blob: serde_json::Value::Object(Default::default()),
            dirty: false,
        }
    }

    pub fn get_slot(&self, slot: u8) -> f64 {
        self.slots.get(slot as usize).copied().unwrap_or(0.0)
    }

    pub fn set_slot(&mut self, slot: u8, value: f64) -> Result<(), SaveDataError> {
        let cell = self
            .slots
            .get_mut(slot as usize)
            .ok_or(SaveDataError::SlotOutOfRange(slot))?;
        *cell = value;
        self.dirty = true;
        Ok(())
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
        let mut out = Vec::with_capacity(4 + 2 + SAVE_DATA_SLOT_COUNT * 8 + 4 + blob_bytes.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        for slot in &self.slots {
            out.extend_from_slice(&slot.to_le_bytes());
        }
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

        let mut slots = [0.0f64; SAVE_DATA_SLOT_COUNT];
        for slot in &mut slots {
            let raw: [u8; 8] = bytes.get(cursor..cursor + 8)?.try_into().ok()?;
            *slot = f64::from_le_bytes(raw);
            cursor += 8;
        }

        let blob_len = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
        cursor += 4;
        let blob_bytes = bytes.get(cursor..cursor + blob_len)?;
        let blob: serde_json::Value = serde_json::from_slice(blob_bytes).ok()?;

        Some(Self {
            slots,
            blob,
            dirty: false,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn default_slot_is_zero() {
        let data = SaveData::new();
        assert_eq!(data.get_slot(0), 0.0);
        assert_eq!(data.get_slot(63), 0.0);
    }

    #[test]
    fn set_slot_out_of_range_errors() {
        let mut data = SaveData::new();
        assert_eq!(
            data.set_slot(64, 1.0),
            Err(SaveDataError::SlotOutOfRange(64))
        );
    }

    #[test]
    fn set_slot_marks_dirty() {
        let mut data = SaveData::new();
        assert!(!data.is_dirty());
        data.set_slot(0, 42.0).unwrap();
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
    fn round_trips_slots_and_blob() {
        let mut data = SaveData::new();
        data.set_slot(0, 42.0).unwrap();
        data.set_slot(63, -1.5).unwrap();
        data.set_blob(serde_json::json!({ "level": 3, "name": "ok" }))
            .unwrap();

        let bytes = data.encode();
        let decoded = SaveData::decode(&bytes).expect("valid save data");

        assert_eq!(decoded.get_slot(0), 42.0);
        assert_eq!(decoded.get_slot(63), -1.5);
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
    fn rejects_unknown_version() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert!(SaveData::decode(&bytes).is_none());
    }
}

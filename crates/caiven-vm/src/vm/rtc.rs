//! Real-time clock: UTC hour/minute/second mapped into RAM each tick,
//! readable from Lua via the `real_time()` builtin.

use crate::vm::memory::Memory;
use caiven_core::memory::RTC_RAM_BASE;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn write_time(mem: &mut Memory) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() % 86_400);
    let _ = mem.write(RTC_RAM_BASE, (secs / 3600) as u8);
    let _ = mem.write(RTC_RAM_BASE + 1, (secs % 3600 / 60) as u8);
    let _ = mem.write(RTC_RAM_BASE + 2, (secs % 60) as u8);
}

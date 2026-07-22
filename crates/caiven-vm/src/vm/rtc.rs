//! Real-time clock peripheral — proves out the [`Peripheral`] trait as a
//! Rust-native modding mechanism: mapped registers in RAM, written each
//! tick, readable from Lua via the `real_time()` builtin.

use crate::peripheral::Peripheral;
use crate::vm::memory::Memory;
use caiven_core::memory::RTC_RAM_BASE;
use chrono::{Local, Timelike};

pub struct RealTimeClock;

impl Peripheral for RealTimeClock {
    fn name(&self) -> &'static str {
        "rtc"
    }

    fn init(&mut self, mem: &mut Memory) {
        write_time(mem);
    }

    fn tick(&mut self, mem: &mut Memory, _frame: u32) {
        write_time(mem);
    }
}

fn write_time(mem: &mut Memory) {
    let now = Local::now();
    let _ = mem.write(RTC_RAM_BASE, now.hour() as u8);
    let _ = mem.write(RTC_RAM_BASE + 1, now.minute() as u8);
    let _ = mem.write(RTC_RAM_BASE + 2, now.second() as u8);
}

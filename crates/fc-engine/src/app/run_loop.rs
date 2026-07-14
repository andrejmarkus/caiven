//! Per-iteration update for [`App`]: background polling, VM stepping and
//! debugger-mode handling. Called from `ApplicationHandler::about_to_wait`.

use super::{App, AppMode};
use crate::debugger::DebugMode;
use std::time::Instant;

impl App {
    pub(super) fn update(&mut self) {
        self.browser_editor.poll_hub();
        self.poll_browser_load();
        self.poll_hot_reload();

        if self.mode == AppMode::Run {
            let now = Instant::now();
            let dt = now.duration_since(self.last_tick);
            self.last_tick = now;

            match self.debugger.get_mode() {
                DebugMode::Running => {
                    let steps = self.timing.tick(dt);
                    for _ in 0..steps {
                        self.vm.run_frame(&self.input, &self.font);
                        self.debugger.push_state(self.vm.snapshot());
                    }
                }
                DebugMode::Step => {
                    self.vm.step(&self.input, &self.font);
                    self.debugger.check_breakpoint(self.vm.get_pc());
                    self.debugger.dump_state(&self.vm);
                    self.debugger.pause(self.vm.get_pc());
                }
                DebugMode::Paused => {}
            }
        } else {
            self.last_tick = Instant::now();
        }
    }
}

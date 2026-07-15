//! Per-iteration update for [`App`]: hot-reload polling, VM stepping and
//! debugger-mode handling. Called from `ApplicationHandler::about_to_wait`.

use super::App;
use crate::debugger::DebugMode;

impl App {
    pub(super) fn update(&mut self) {
        self.poll_hot_reload();

        let steps = self.core.frame_steps();
        match self.debugger.get_mode() {
            DebugMode::Running => {
                for _ in 0..steps {
                    self.core.run_frame();
                    self.debugger.push_state(self.core.vm.snapshot());
                }
            }
            DebugMode::Step => {
                self.core.step();
                self.debugger.check_breakpoint(self.core.vm.get_pc());
                self.debugger.dump_state(&self.core.vm);
                self.debugger.pause(self.core.vm.get_pc());
            }
            DebugMode::Paused => {}
        }
    }
}

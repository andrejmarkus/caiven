//! Per-iteration update for [`App`]: VM stepping and debugger-mode
//! handling. Called from `ApplicationHandler::about_to_wait`.

use super::App;
use crate::debugger::DebugMode;

impl App {
    pub(super) fn update(&mut self) {
        let steps = self.core.frame_steps();
        match self.debugger.get_mode() {
            DebugMode::Running => {
                for _ in 0..steps {
                    self.core.run_frame();
                }
            }
            DebugMode::Step => {
                self.core.run_frame();
                self.debugger.pause();
            }
            DebugMode::Paused => {}
        }
    }
}

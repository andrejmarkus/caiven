use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
struct DbgFile {
    #[serde(default)]
    breakpoints: Vec<usize>,
}

/// Breakpoint model shared with Caiven Studio's debugger panel (`studio::debug_panel`,
/// `studio::code_panel`'s gutter); state lives here, egui rendering lives there.
pub struct Debugger {
    breakpoints: Vec<usize>,
    cursor_addr: usize,
    dbg_path: Option<PathBuf>,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            breakpoints: Vec::new(),
            cursor_addr: 0,
            dbg_path: None,
        }
    }

    pub fn set_dbg_path(&mut self, path: PathBuf) {
        self.dbg_path = Some(path);
        self.load_dbg();
    }

    fn load_dbg(&mut self) {
        let Some(path) = &self.dbg_path else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(file) = toml::from_str::<DbgFile>(&text) else {
            return;
        };
        self.breakpoints = file.breakpoints;
    }

    fn save_dbg(&self) {
        let Some(path) = &self.dbg_path else { return };
        let file = DbgFile {
            breakpoints: self.breakpoints.clone(),
        };
        if let Ok(text) = toml::to_string(&file) {
            let _ = std::fs::write(path, text);
        }
    }

    /// Toggles a breakpoint on a Lua source line, set from the code editor's
    /// gutter.
    pub fn toggle_line_breakpoint(&mut self, line: usize) {
        if let Some(pos) = self.breakpoints.iter().position(|&a| a == line) {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(line);
        }
        self.save_dbg();
    }

    pub fn breakpoints(&self) -> &[usize] {
        &self.breakpoints
    }

    pub fn cursor_addr(&self) -> usize {
        self.cursor_addr
    }

    pub fn set_cursor_addr(&mut self, addr: usize) {
        self.cursor_addr = addr;
    }
}

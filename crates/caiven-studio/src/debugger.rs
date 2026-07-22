use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
struct FcDbgFile {
    #[serde(default)]
    breakpoints: Vec<usize>,
}

/// Breakpoint model shared with Caiven Studio's debugger panel (`studio::debug_panel`,
/// `studio::code_panel`'s gutter); state lives here, egui rendering lives there.
pub struct Debugger {
    breakpoints: Vec<usize>,
    cursor_addr: usize,
    fcdbg_path: Option<PathBuf>,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            breakpoints: Vec::new(),
            cursor_addr: 0,
            fcdbg_path: None,
        }
    }

    pub fn set_fcdbg_path(&mut self, path: PathBuf) {
        self.fcdbg_path = Some(path);
        self.load_fcdbg();
    }

    fn load_fcdbg(&mut self) {
        let Some(path) = &self.fcdbg_path else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(file) = toml::from_str::<FcDbgFile>(&text) else {
            return;
        };
        self.breakpoints = file.breakpoints;
    }

    fn save_fcdbg(&self) {
        let Some(path) = &self.fcdbg_path else { return };
        let file = FcDbgFile {
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
        self.save_fcdbg();
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

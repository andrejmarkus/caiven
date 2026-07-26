use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakpoint {
    pub source: String,
    pub line: usize,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StoredBreakpoint {
    Legacy(usize),
    Source(Breakpoint),
}

#[derive(Debug, Default, Deserialize)]
struct LoadDbgFile {
    #[serde(default)]
    breakpoints: Vec<StoredBreakpoint>,
    #[serde(default)]
    watches: Vec<String>,
}

#[derive(Serialize)]
struct SaveDbgFile<'a> {
    breakpoints: &'a [Breakpoint],
    watches: &'a [String],
}

/// Breakpoint model shared with Caiven Studio's debugger panel (`studio::debug_panel`,
/// `studio::code_panel`'s gutter); state lives here, egui rendering lives there.
pub struct Debugger {
    breakpoints: Vec<Breakpoint>,
    watches: Vec<String>,
    dbg_path: Option<PathBuf>,
    entry_source: String,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            breakpoints: Vec::new(),
            watches: Vec::new(),
            dbg_path: None,
            entry_source: "main.lua".to_string(),
        }
    }

    pub fn set_dbg_path(&mut self, path: PathBuf, entry_source: String) {
        self.breakpoints.clear();
        self.watches.clear();
        self.dbg_path = Some(path);
        self.entry_source = entry_source;
        self.load_dbg();
    }

    fn load_dbg(&mut self) {
        let Some(path) = &self.dbg_path else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(file) = toml::from_str::<LoadDbgFile>(&text) else {
            return;
        };
        self.breakpoints = file
            .breakpoints
            .into_iter()
            .map(|stored| match stored {
                StoredBreakpoint::Legacy(line) => Breakpoint {
                    source: self.entry_source.clone(),
                    line,
                },
                StoredBreakpoint::Source(breakpoint) => breakpoint,
            })
            .filter(|breakpoint| breakpoint.line > 0 && !breakpoint.source.trim().is_empty())
            .collect();
        self.watches = file.watches;
    }

    fn save_dbg(&self) {
        let Some(path) = &self.dbg_path else { return };
        let file = SaveDbgFile {
            breakpoints: &self.breakpoints,
            watches: &self.watches,
        };
        if let Ok(text) = toml::to_string(&file) {
            let _ = std::fs::write(path, text);
        }
    }

    /// Toggles a breakpoint on a Lua source line, set from the code editor's
    /// gutter.
    pub fn toggle_line_breakpoint(&mut self, source: String, line: usize) {
        let source = source.trim().to_string();
        if let Some(pos) = self
            .breakpoints
            .iter()
            .position(|breakpoint| breakpoint.source == source && breakpoint.line == line)
        {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(Breakpoint { source, line });
            self.breakpoints.sort_by(|left, right| {
                left.source
                    .cmp(&right.source)
                    .then(left.line.cmp(&right.line))
            });
        }
        self.save_dbg();
    }

    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    pub fn add_watch(&mut self, expression: String) -> bool {
        let expression = expression.trim().to_string();
        if expression.is_empty() || self.watches.contains(&expression) {
            return false;
        }
        self.watches.push(expression);
        self.save_dbg();
        true
    }

    pub fn watches(&self) -> &[String] {
        &self.watches
    }

    pub fn remove_watch(&mut self, expression: &str) -> bool {
        let Some(position) = self.watches.iter().position(|watch| watch == expression) else {
            return false;
        };
        self.watches.remove(position);
        self.save_dbg();
        true
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
        self.watches.clear();
        self.dbg_path = None;
        self.entry_source = "main.lua".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::{Breakpoint, Debugger};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn toggles_breakpoints_per_source() {
        let mut debugger = Debugger::new();
        debugger.toggle_line_breakpoint("main.lua".into(), 4);
        debugger.toggle_line_breakpoint("ui/panel.lua".into(), 4);
        assert_eq!(
            debugger.breakpoints(),
            &[
                Breakpoint {
                    source: "main.lua".into(),
                    line: 4,
                },
                Breakpoint {
                    source: "ui/panel.lua".into(),
                    line: 4,
                },
            ]
        );
        debugger.toggle_line_breakpoint("main.lua".into(), 4);
        assert_eq!(debugger.breakpoints().len(), 1);
    }

    #[test]
    fn removes_watches() {
        let mut debugger = Debugger::new();
        assert!(debugger.add_watch("player.x".into()));
        assert!(debugger.remove_watch("player.x"));
        assert!(!debugger.remove_watch("player.x"));
    }

    #[test]
    fn loads_legacy_lines_and_persists_source_breakpoints_and_watches() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "caiven-debugger-{}-{unique}.toml",
            std::process::id()
        ));
        std::fs::write(&path, "breakpoints = [4]\nwatches = [\"player.x\"]\n")
            .expect("write legacy debugger state");

        let mut debugger = Debugger::new();
        debugger.set_dbg_path(path.clone(), "main.lua".into());
        assert_eq!(
            debugger.breakpoints()[0],
            Breakpoint {
                source: "main.lua".into(),
                line: 4
            }
        );
        assert_eq!(debugger.watches(), &["player.x"]);
        debugger.toggle_line_breakpoint("ui/hud.lua".into(), 8);

        let mut reloaded = Debugger::new();
        reloaded.set_dbg_path(path.clone(), "main.lua".into());
        assert_eq!(reloaded.breakpoints().len(), 2);
        assert_eq!(reloaded.watches(), &["player.x"]);
        std::fs::remove_file(path).expect("remove debugger fixture");
    }
}

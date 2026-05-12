use std::path::PathBuf;
use std::time::SystemTime;

pub struct HotReload {
    path: Option<PathBuf>,
    mtime: Option<SystemTime>,
}

impl HotReload {
    pub fn new() -> Self {
        HotReload {
            path: None,
            mtime: None,
        }
    }

    pub fn watch(&mut self, path: PathBuf, initial_mtime: Option<SystemTime>) {
        self.path = Some(path);
        self.mtime = initial_mtime;
    }

    /// Returns the path if it has been modified since last check, updating mtime.
    pub fn poll(&mut self) -> Option<PathBuf> {
        let path = self.path.as_ref()?;
        let meta = path.metadata().ok()?;
        let mtime = meta.modified().ok()?;
        if Some(mtime) != self.mtime {
            self.mtime = Some(mtime);
            Some(path.clone())
        } else {
            None
        }
    }
}

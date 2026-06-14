use crate::browser_action::BrowserAction;
use crate::dirs::Directories;
use crate::store::atomic::AtomicWriter;
use crate::store::paths;
use std::fs;
use std::io::Result;

// ================================================================
//  BROWSER ACTIONS
// ================================================================
// File I/O for browser_actions.json. Resolution logic stays in
// browser_action.rs; this module only handles persistence.

pub fn save(dirs: Directories, actions: &[BrowserAction]) -> Result<()> {
    let path = paths::browser_actions(dirs);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::to_vec(actions)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    AtomicWriter::write(&path, &data)
}

pub fn load(dirs: Directories) -> Vec<BrowserAction> {
    let path = paths::browser_actions(dirs);
    if !path.exists() {
        return Vec::new();
    }
    let json = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_str(&json) {
        Ok(actions) => actions,
        Err(_) => Vec::new(),
    }
}

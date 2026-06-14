use std::io::Result;
use std::path::Path;

// ================================================================
//  ATOMIC WRITER
// ================================================================
// Shared utility for crash-safe file writes: write to a .tmp
// sibling, then rename into place. Used by all store sub-modules
// so the pattern is tested once, not duplicated.

pub struct AtomicWriter;

impl AtomicWriter {
    /// Write `data` to `path` atomically (write .tmp, then rename).
    pub fn write(path: &Path, data: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

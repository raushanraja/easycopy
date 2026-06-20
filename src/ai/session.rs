use crate::store::atomic::AtomicWriter;
use adk_rust::session::{SessionService, SqliteSessionService};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChatState {
    pub current_session_id: Option<String>,
}

impl ChatState {
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let st: Self = serde_json::from_str(&text).unwrap_or_default();
        Ok(st)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        AtomicWriter::write(path, text.as_bytes())
    }
}

/// Generate a unique session id (timestamp + counter, no extra dep).
pub fn new_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let c = CTR.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", nanos, c)
}

/// Build the SQLite-backed session service at `db_path`, running schema
/// migrations so the sessions table exists.
pub async fn build_session_service(
    db_path: &Path,
) -> std::result::Result<Arc<dyn SessionService>, Box<dyn std::error::Error>> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // sqlx's default connect does not create_if_missing, so touch an empty
    // file first (sqlite treats a 0-byte file as a fresh database).
    if !db_path.exists() {
        std::fs::File::create(db_path)?;
    }
    let url = format!("sqlite://{}", db_path.display());
    let svc = SqliteSessionService::new(&url).await?;
    svc.migrate().await?;
    Ok(Arc::new(svc))
}


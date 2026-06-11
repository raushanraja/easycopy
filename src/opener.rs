use crate::config::Config;
use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum OpenTarget {
    Text(String),
    Image(String),
}

pub fn open_item(target: &OpenTarget) -> std::io::Result<()> {
    match target {
        OpenTarget::Text(content) => {
            let temp_dir = std::env::temp_dir();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let temp_path = temp_dir.join(format!("clipit_{}.txt", timestamp));
            {
                let mut file = File::create(&temp_path)?;
                file.write_all(content.as_bytes())?;
            }
            Command::new("xdg-open").arg(temp_path).spawn()?;
            Ok(())
        }
        OpenTarget::Image(filename) => {
            let path = Config::images_dir().join(filename);
            Command::new("xdg-open").arg(path).spawn()?;
            Ok(())
        }
    }
}

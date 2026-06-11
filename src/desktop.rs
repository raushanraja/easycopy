use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ================================================================
//  DESKTOP APP
// ================================================================

#[derive(Debug, Clone)]
pub struct DesktopApp {
    pub name: String,
    pub comment: String,
    pub exec: String,
    pub icon_path: Option<String>,
}

// ================================================================
//  PUBLIC API
// ================================================================

pub fn load_desktop_apps() -> Vec<DesktopApp> {
    scan_desktop_files()
}

// ================================================================
//  SCAN .desktop FILES
// ================================================================

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let local = Path::new(&home).join(".local/share/applications");
        if local.exists() {
            dirs.push(local);
        }
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        let p = Path::new(&data_home).join("applications");
        if p.exists() {
            dirs.push(p);
        }
    }
    if let Ok(paths) = std::env::var("XDG_DATA_DIRS") {
        for p in paths.split(':') {
            let p = Path::new(p).join("applications");
            if p.exists() {
                dirs.push(p);
            }
        }
    }
    let fallback = Path::new("/usr/share/applications");
    if fallback.exists() && !dirs.contains(&fallback.to_path_buf()) {
        dirs.push(fallback.to_path_buf());
    }
    dirs
}

fn scan_desktop_files() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    for dir in desktop_dirs() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if !path.extension().map(|e| e == "desktop").unwrap_or(false) {
                continue;
            }
            if let Some(app) = parse_desktop_file(&path) {
                let key = app.name.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);
                apps.push(app);
            }
        }
    }
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

// ================================================================
//  PARSE .desktop FILE
// ================================================================

fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_section = false;
    let mut name = String::new();
    let mut comment = String::new();
    let mut icon = String::new();
    let mut exec = String::new();
    let mut app_type = String::new();
    let mut hidden = false;
    let mut no_display = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = &line[1..line.len() - 1] == "Desktop Entry";
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Type" => app_type = value.to_lowercase(),
                "Name" if name.is_empty() => name = value.to_string(),
                "Comment" if comment.is_empty() => comment = value.to_string(),
                "Icon" if icon.is_empty() => icon = value.to_string(),
                "Exec" if exec.is_empty() => exec = value.to_string(),
                "Hidden" if value.eq_ignore_ascii_case("true") => hidden = true,
                "NoDisplay" if value.eq_ignore_ascii_case("true") => no_display = true,
                _ => {}
            }
        }
    }

    if app_type != "application" || hidden || no_display || name.is_empty() || exec.is_empty() {
        return None;
    }

    Some(DesktopApp {
        name,
        comment,
        exec: strip_exec_codes(&exec),
        icon_path: resolve_icon(&icon),
    })
}

// ================================================================
//  EXEC FIELD CODE STRIPPING
// ================================================================

fn strip_exec_codes(exec: &str) -> String {
    let mut result = Vec::new();
    let mut parts = exec.split_whitespace();
    let mut skip_next = false;
    while let Some(part) = parts.next() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if part.starts_with('%') {
            if part == "%i" {
                skip_next = true;
                continue;
            }
            continue;
        }
        result.push(part);
    }
    result.join(" ")
}

// ================================================================
//  ICON RESOLUTION
// ================================================================

fn resolve_icon(icon_name: &str) -> Option<String> {
    if icon_name.starts_with('/') {
        return Some(icon_name.to_string());
    }

    let search_paths = icon_search_dirs();
    // Prefer concrete pixel sizes over "scalable" (which is often SVG-only),
    // since the image crate doesn't support SVG.
    let sizes = &["48x48", "64x64", "32x32", "24x24", "22x22", "16x16", "scalable"];
    let exts = &["svg", "png", "xpm"];
    let icon_lower = icon_name.to_lowercase();

    // Strategy 1: direct file probe (avoids read_dir issues)
    for dir in &search_paths {
        for size in sizes {
            for subdir in &["apps", "devices", "mimetypes", "actions", "places", "status"] {
                for ext in exts {
                    let candidate = dir.join(size).join(subdir).join(format!("{}.{}", icon_name, ext));
                    if candidate.exists() {
                        return Some(candidate.to_string_lossy().into_owned());
                    }
                }
            }
        }
        // Flat pixmaps fallback
        for ext in exts {
            let candidate = dir.join(format!("{}.{}", icon_name, ext));
            if candidate.exists() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }

    // Strategy 2: directory scan with CI matching (for themes we didn't probe directly)
    for dir in &search_paths {
        let Ok(dir_entries) = fs::read_dir(dir) else {
            continue;
        };
        let entries: Vec<_> = dir_entries.collect();
        let count = entries.len();
        if count == 0 {
            continue;
        }
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if path.is_dir() {
                for size in sizes {
                    for subdir in &["apps", "devices", "mimetypes", "actions", "places", "status"] {
                        let sub_path = path.join(size).join(subdir);
                        if !sub_path.is_dir() {
                            continue;
                        }
                        if let Some(found) = find_case_insensitive(&sub_path, &icon_lower, exts) {
                            return Some(found);
                        }
                    }
                }
            } else {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.eq_ignore_ascii_case(&icon_lower) {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if exts.contains(&ext) {
                                return Some(path.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_case_insensitive(dir: &Path, icon_lower: &str, exts: &[&str]) -> Option<String> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            if stem.eq_ignore_ascii_case(icon_lower) {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext) {
                        return Some(p.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    None
}

fn icon_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let p = Path::new(&home).join(".local/share/icons");
        if p.exists() {
            dirs.push(p);
        }
    }
    if let Ok(paths) = std::env::var("XDG_DATA_DIRS") {
        for p in paths.split(':') {
            let p = Path::new(p).join("icons");
            if p.exists() {
                dirs.push(p);
            }
        }
    }
    dirs.push(Path::new("/usr/share/icons").to_path_buf());
    dirs.push(Path::new("/usr/share/pixmaps").to_path_buf());
    dirs
}

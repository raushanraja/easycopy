use std::fs;
use std::path::{Path, PathBuf};

// ================================================================
//  ICON RESOLUTION
// ================================================================

/// Resolve an icon name to a file path using the default XDG search dirs.
pub fn resolve_icon(icon_name: &str) -> Option<String> {
    resolve_icon_in(icon_name, &icon_search_dirs())
}

/// Resolve an icon name using an explicit list of search directories.
/// This is the testable entry point — callers provide the dirs.
pub fn resolve_icon_in(icon_name: &str, search_paths: &[PathBuf]) -> Option<String> {
    if icon_name.starts_with('/') {
        return Some(icon_name.to_string());
    }

    let sizes = &[
        "48x48", "64x64", "32x32", "24x24", "22x22", "16x16", "scalable",
    ];
    let exts = &["svg", "png", "xpm"];
    let icon_lower = icon_name.to_lowercase();

    // Strategy 1: direct file probe (avoids read_dir issues)
    for dir in search_paths {
        for size in sizes {
            for subdir in &[
                "apps",
                "devices",
                "mimetypes",
                "actions",
                "places",
                "status",
            ] {
                for ext in exts {
                    let candidate = dir
                        .join(size)
                        .join(subdir)
                        .join(format!("{}.{}", icon_name, ext));
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

    // Strategy 2: directory scan with CI matching
    for dir in search_paths {
        let Ok(dir_entries) = fs::read_dir(dir) else {
            continue;
        };
        let entries: Vec<_> = dir_entries.collect();
        if entries.is_empty() {
            continue;
        }
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if path.is_dir() {
                for size in sizes {
                    for subdir in &[
                        "apps",
                        "devices",
                        "mimetypes",
                        "actions",
                        "places",
                        "status",
                    ] {
                        let sub_path = path.join(size).join(subdir);
                        if !sub_path.is_dir() {
                            continue;
                        }
                        if let Some(found) = find_case_insensitive(&sub_path, &icon_lower, exts) {
                            return Some(found);
                        }
                    }
                }
            } else if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
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

    None
}

/// Case-insensitive file lookup within a directory.
pub fn find_case_insensitive(dir: &Path, icon_lower: &str, exts: &[&str]) -> Option<String> {
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

/// XDG icon search directories, in priority order.
pub fn icon_search_dirs() -> Vec<PathBuf> {
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

// ================================================================
//  TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn setup_icon_tree(root: &Path) -> PathBuf {
        // Create: root/48x48/apps/firefox.png
        let app_dir = root.join("48x48").join("apps");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("firefox.png"), b"png").unwrap();

        // Create: root/scalable/apps/Thunderbird.svg
        let svg_dir = root.join("scalable").join("apps");
        fs::create_dir_all(&svg_dir).unwrap();
        fs::write(svg_dir.join("Thunderbird.svg"), b"svg").unwrap();

        // Create flat pixmap
        fs::write(root.join("gimp.png"), b"png").unwrap();

        // Create case-different file for CI matching (strategy 2)
        fs::create_dir_all(root.join("64x64").join("apps")).unwrap();
        fs::write(root.join("64x64").join("apps").join("LibreOffice.png"), b"png").unwrap();

        root.to_path_buf()
    }

    #[test]
    fn absolute_path_passthrough() {
        assert_eq!(
            resolve_icon("/usr/share/icons/firefox.png"),
            Some("/usr/share/icons/firefox.png".to_string())
        );
    }

    #[test]
    fn resolve_exact_match_in_standard_size() {
        let dir = tempdir().unwrap();
        let root = setup_icon_tree(dir.path());
        let result = resolve_icon_in("firefox", &[root]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("48x48"));
    }

    #[test]
    fn resolve_case_insensitive_match_strategy2() {
        let dir = tempdir().unwrap();
        let root = setup_icon_tree(dir.path());
        // File is "LibreOffice.png" but we query "libreoffice"
        // Place the icon at 48x48/apps/ so strategy2 finds it via sub_path = 64x48/48x48/apps
        // But strategy 2 joins: path(root/48x48) + size(48x48) + subdir(apps) = 48x48/48x48/apps
        // So put the icon in the same structure strategy 2 expects
        let ci_dir = root.join("64x64").join("64x64").join("apps");
        fs::create_dir_all(&ci_dir).unwrap();
        fs::write(ci_dir.join("LibreOffice.png"), b"png").unwrap();
        let result = resolve_icon_in("libreoffice", &[root]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("LibreOffice.png"));
    }

    #[test]
    fn resolve_scalable_svg() {
        let dir = tempdir().unwrap();
        let root = setup_icon_tree(dir.path());
        let result = resolve_icon_in("Thunderbird", &[root]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("scalable"));
    }

    #[test]
    fn resolve_flat_pixmap_fallback() {
        let dir = tempdir().unwrap();
        let root = setup_icon_tree(dir.path());
        let result = resolve_icon_in("gimp", &[root]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("gimp.png"));
    }

    #[test]
    fn resolve_missing_icon_returns_none() {
        let dir = tempdir().unwrap();
        let root = setup_icon_tree(dir.path());
        assert!(resolve_icon_in("nonexistent_app_xyz", &[root]).is_none());
    }

    #[test]
    fn find_case_insensitive_matches() {
        let dir = tempdir().unwrap();
        let apps_dir = dir.path().join("apps");
        fs::create_dir_all(&apps_dir).unwrap();
        fs::write(apps_dir.join("Firefox.png"), b"png").unwrap();

        let result = find_case_insensitive(&apps_dir, "firefox", &["png"]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Firefox.png"));
    }

    #[test]
    fn find_case_insensitive_no_match() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        assert!(find_case_insensitive(dir.path(), "missing", &["png"]).is_none());
    }

    #[test]
    fn find_case_insensitive_wrong_extension() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(dir.path().join("app.jpg"), b"jpg").unwrap();
        assert!(find_case_insensitive(dir.path(), "app", &["png"]).is_none());
    }

    #[test]
    fn icon_search_dirs_includes_standard_paths() {
        let dirs = icon_search_dirs();
        assert!(dirs.contains(&std::path::PathBuf::from("/usr/share/icons")));
        assert!(dirs.contains(&std::path::PathBuf::from("/usr/share/pixmaps")));
    }
}

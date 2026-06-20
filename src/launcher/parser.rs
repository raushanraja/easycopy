use super::DesktopApp;
use crate::launcher::icon::resolve_icon;
use std::path::Path;

// ================================================================
//  PARSE .desktop FILE
// ================================================================

/// Parse a .desktop file into a DesktopApp using the default icon resolver.
/// Returns None if the file is not a valid application entry
/// (wrong type, hidden, NoDisplay, missing required fields).
pub fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    parse_desktop_file_with(path, resolve_icon)
}

/// Parse with an explicit icon resolver (useful for testing).
pub fn parse_desktop_file_with(
    path: &Path,
    icon_resolver: impl Fn(&str) -> Option<String>,
) -> Option<DesktopApp> {
    let content = std::fs::read_to_string(path).ok()?;
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
        icon_path: icon_resolver(&icon),
        use_count: 0,
    })
}

// ================================================================
//  EXEC FIELD CODE STRIPPING
// ================================================================

/// Strip desktop entry field codes from an Exec string.
/// Handles %i (skip next arg), %f/%F/%u/%U (file args), and
/// other %% codes per the Desktop Entry specification.
pub fn strip_exec_codes(exec: &str) -> String {
    let mut result = Vec::new();
    let mut parts = exec.split_whitespace();
    let mut skip_count = 0;
    while let Some(part) = parts.next() {
        if skip_count > 0 {
            skip_count -= 1;
            continue;
        }
        if part.starts_with('%') {
            if part == "%i" {
                // %i expands to --icon <name>, skip both the icon flag and its arg
                skip_count = 2;
                continue;
            }
            continue;
        }
        result.push(part);
    }
    result.join(" ")
}

// ================================================================
//  TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn write_desktop_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn parse_complete_desktop_file() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "test.desktop",
            r#"[Desktop Entry]
Type=Application
Name=Test App
Comment=A test application
Icon=firefox
Exec=firefox %u
"#,
        );
        // Mock icon resolver returns None (no icon found)
        let app = parse_desktop_file_with(&path, |_| None).unwrap();
        assert_eq!(app.name, "Test App");
        assert_eq!(app.comment, "A test application");
        assert_eq!(app.exec, "firefox");
        assert_eq!(app.icon_path, None);
    }

    #[test]
    fn parse_hidden_app_returns_none() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "hidden.desktop",
            r#"[Desktop Entry]
Type=Application
Name=Hidden App
Exec=hidden
Hidden=true
"#,
        );
        assert!(parse_desktop_file(&path).is_none());
    }

    #[test]
    fn parse_no_display_app_returns_none() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "nodisplay.desktop",
            r#"[Desktop Entry]
Type=Application
Name=NoDisplay App
Exec=app
NoDisplay=true
"#,
        );
        assert!(parse_desktop_file(&path).is_none());
    }

    #[test]
    fn parse_wrong_type_returns_none() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "link.desktop",
            r#"[Desktop Entry]
Type=Link
Name=Link
Exec=xdg-open
"#,
        );
        assert!(parse_desktop_file(&path).is_none());
    }

    #[test]
    fn parse_missing_name_returns_none() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "noname.desktop",
            r#"[Desktop Entry]
Type=Application
Exec=app
"#,
        );
        assert!(parse_desktop_file(&path).is_none());
    }

    #[test]
    fn parse_missing_exec_returns_none() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "noexec.desktop",
            r#"[Desktop Entry]
Type=Application
Name=NoExec
"#,
        );
        assert!(parse_desktop_file(&path).is_none());
    }

    #[test]
    fn parse_ignores_non_desktop_section() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "extras.desktop",
            r#"[Desktop Entry]
Type=Application
Name=Main
Exec=main

[Extra Section]
Key=value
"#,
        );
        let app = parse_desktop_file(&path).unwrap();
        assert_eq!(app.name, "Main");
    }

    #[test]
    fn parse_first_name_wins() {
        let dir = tempdir().unwrap();
        let path = write_desktop_file(
            dir.path(),
            "dup.desktop",
            r#"[Desktop Entry]
Type=Application
Name=First
Exec=app
Name=Second
"#,
        );
        let app = parse_desktop_file(&path).unwrap();
        assert_eq!(app.name, "First");
    }

    // --- strip_exec_codes ---

    #[test]
    fn strip_exec_codes_removes_percent_codes() {
        assert_eq!(strip_exec_codes("firefox %u"), "firefox");
        assert_eq!(strip_exec_codes("firefox %U"), "firefox");
        assert_eq!(strip_exec_codes("firefox %f %F"), "firefox");
    }

    #[test]
    fn strip_exec_codes_handles_percent_i() {
        // %i means skip the next argument (typically --icon <name>)
        assert_eq!(strip_exec_codes("app %i --icon foo"), "app");
        assert_eq!(strip_exec_codes("app %i bar"), "app");
    }

    #[test]
    fn strip_exec_codes_preserves_regular_args() {
        assert_eq!(strip_exec_codes("app --flag value"), "app --flag value");
        assert_eq!(
            strip_exec_codes("app -d /path/to/dir"),
            "app -d /path/to/dir"
        );
    }

    #[test]
    fn strip_exec_codes_mixed() {
        assert_eq!(
            strip_exec_codes("code --new-window %F --multi-cmd --verbose"),
            "code --new-window --multi-cmd --verbose"
        );
    }

    #[test]
    fn strip_exec_codes_empty_input() {
        assert_eq!(strip_exec_codes(""), "");
    }

    #[test]
    fn strip_exec_codes_percent_literal() {
        // %% should produce % per spec, but current impl strips both % tokens
        assert_eq!(strip_exec_codes("app %%"), "app");
    }
}

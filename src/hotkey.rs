use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// Parse a human-readable hotkey string like `Ctrl+Alt+V` into a
/// `global_hotkey::HotKey`.
pub fn parse_hotkey(s: &str) -> Option<HotKey> {
    let parts: Vec<&str> = s
        .split('+')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }

    let mut mods = Modifiers::empty();
    let mut code = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "meta" | "win" | "cmd" | "command" => mods |= Modifiers::SUPER,
            "space" => code = Some(Code::Space),
            "tab" => code = Some(Code::Tab),
            "enter" | "return" => code = Some(Code::Enter),
            "escape" | "esc" => code = Some(Code::Escape),
            key if key.len() == 1 => {
                code = match key.chars().next().unwrap() {
                    'a' => Some(Code::KeyA),
                    'b' => Some(Code::KeyB),
                    'c' => Some(Code::KeyC),
                    'd' => Some(Code::KeyD),
                    'e' => Some(Code::KeyE),
                    'f' => Some(Code::KeyF),
                    'g' => Some(Code::KeyG),
                    'h' => Some(Code::KeyH),
                    'i' => Some(Code::KeyI),
                    'j' => Some(Code::KeyJ),
                    'k' => Some(Code::KeyK),
                    'l' => Some(Code::KeyL),
                    'm' => Some(Code::KeyM),
                    'n' => Some(Code::KeyN),
                    'o' => Some(Code::KeyO),
                    'p' => Some(Code::KeyP),
                    'q' => Some(Code::KeyQ),
                    'r' => Some(Code::KeyR),
                    's' => Some(Code::KeyS),
                    't' => Some(Code::KeyT),
                    'u' => Some(Code::KeyU),
                    'v' => Some(Code::KeyV),
                    'w' => Some(Code::KeyW),
                    'x' => Some(Code::KeyX),
                    'y' => Some(Code::KeyY),
                    'z' => Some(Code::KeyZ),
                    '0' => Some(Code::Digit0),
                    '1' => Some(Code::Digit1),
                    '2' => Some(Code::Digit2),
                    '3' => Some(Code::Digit3),
                    '4' => Some(Code::Digit4),
                    '5' => Some(Code::Digit5),
                    '6' => Some(Code::Digit6),
                    '7' => Some(Code::Digit7),
                    '8' => Some(Code::Digit8),
                    '9' => Some(Code::Digit9),
                    _ => None,
                };
            }
            key if key.starts_with('f') => {
                code = match &key[1..] {
                    "1" => Some(Code::F1),
                    "2" => Some(Code::F2),
                    "3" => Some(Code::F3),
                    "4" => Some(Code::F4),
                    "5" => Some(Code::F5),
                    "6" => Some(Code::F6),
                    "7" => Some(Code::F7),
                    "8" => Some(Code::F8),
                    "9" => Some(Code::F9),
                    "10" => Some(Code::F10),
                    "11" => Some(Code::F11),
                    "12" => Some(Code::F12),
                    _ => None,
                };
            }
            _ => return None,
        }
    }

    code.map(|c| HotKey::new(Some(mods), c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_hotkeys() {
        assert!(parse_hotkey("Ctrl+Alt+V").is_some());
        assert!(parse_hotkey("ctrl + shift + space").is_some());
        assert!(parse_hotkey("Super+F12").is_some());
    }

    #[test]
    fn rejects_unknown_or_missing_key() {
        assert!(parse_hotkey("Ctrl+Alt").is_none());
        assert!(parse_hotkey("Ctrl+Banana").is_none());
        assert!(parse_hotkey("").is_none());
    }
}

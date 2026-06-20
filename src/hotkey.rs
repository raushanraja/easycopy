use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// Letter key codes indexed by (c - 'a'), so 'a'→0, 'b'→1, … 'z'→25.
const LETTER_CODES: [Code; 26] = [
    Code::KeyA,
    Code::KeyB,
    Code::KeyC,
    Code::KeyD,
    Code::KeyE,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyI,
    Code::KeyJ,
    Code::KeyK,
    Code::KeyL,
    Code::KeyM,
    Code::KeyN,
    Code::KeyO,
    Code::KeyP,
    Code::KeyQ,
    Code::KeyR,
    Code::KeyS,
    Code::KeyT,
    Code::KeyU,
    Code::KeyV,
    Code::KeyW,
    Code::KeyX,
    Code::KeyY,
    Code::KeyZ,
];

/// Digit key codes indexed by digit value, so '0'→0, '1'→1, … '9'→9.
const DIGIT_CODES: [Code; 10] = [
    Code::Digit0,
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];

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
                let c = key.chars().next().unwrap();
                code = match c {
                    'a'..='z' => Some(LETTER_CODES[(c as u8 - b'a') as usize]),
                    '0'..='9' => Some(DIGIT_CODES[(c as u8 - b'0') as usize]),
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

//! ABNT2 keyboard layout mapping for realistic key event simulation.
//!
//! Maps characters to the physical key sequences a human would produce
//! on a Brazilian ABNT2 keyboard, including dead key composition for
//! accented characters.

/// A sequence of physical key actions needed to produce a character.
#[derive(Debug, Clone)]
pub enum KeySequence {
    /// A single key press (optionally with Shift).
    /// Fields: (key, code, shift, windows_virtual_key_code)
    Simple {
        key: &'static str,
        code: &'static str,
        shift: bool,
        vk: i32,
    },
    /// A dead key followed by a base key (e.g. ´ + e = é).
    /// Fields: dead key info, then base key info.
    DeadKey {
        dead_key: &'static str,
        dead_code: &'static str,
        dead_shift: bool,
        dead_vk: i32,
        base_key: &'static str,
        base_code: &'static str,
        base_shift: bool,
        base_vk: i32,
    },
    /// Character not typeable on ABNT2 — simulate paste (Ctrl+V).
    Paste,
}

/// Looks up the key sequence for a character on ABNT2 layout.
pub fn abnt2_sequence(ch: char) -> KeySequence {
    match ch {
        // --- Lowercase letters ---
        'a' => simple("a", "KeyA", false, 65),
        'b' => simple("b", "KeyB", false, 66),
        'c' => simple("c", "KeyC", false, 67),
        'd' => simple("d", "KeyD", false, 68),
        'e' => simple("e", "KeyE", false, 69),
        'f' => simple("f", "KeyF", false, 70),
        'g' => simple("g", "KeyG", false, 71),
        'h' => simple("h", "KeyH", false, 72),
        'i' => simple("i", "KeyI", false, 73),
        'j' => simple("j", "KeyJ", false, 74),
        'k' => simple("k", "KeyK", false, 75),
        'l' => simple("l", "KeyL", false, 76),
        'm' => simple("m", "KeyM", false, 77),
        'n' => simple("n", "KeyN", false, 78),
        'o' => simple("o", "KeyO", false, 79),
        'p' => simple("p", "KeyP", false, 80),
        'q' => simple("q", "KeyQ", false, 81),
        'r' => simple("r", "KeyR", false, 82),
        's' => simple("s", "KeyS", false, 83),
        't' => simple("t", "KeyT", false, 84),
        'u' => simple("u", "KeyU", false, 85),
        'v' => simple("v", "KeyV", false, 86),
        'w' => simple("w", "KeyW", false, 87),
        'x' => simple("x", "KeyX", false, 88),
        'y' => simple("y", "KeyY", false, 89),
        'z' => simple("z", "KeyZ", false, 90),

        // --- Uppercase letters ---
        'A' => simple("A", "KeyA", true, 65),
        'B' => simple("B", "KeyB", true, 66),
        'C' => simple("C", "KeyC", true, 67),
        'D' => simple("D", "KeyD", true, 68),
        'E' => simple("E", "KeyE", true, 69),
        'F' => simple("F", "KeyF", true, 70),
        'G' => simple("G", "KeyG", true, 71),
        'H' => simple("H", "KeyH", true, 72),
        'I' => simple("I", "KeyI", true, 73),
        'J' => simple("J", "KeyJ", true, 74),
        'K' => simple("K", "KeyK", true, 75),
        'L' => simple("L", "KeyL", true, 76),
        'M' => simple("M", "KeyM", true, 77),
        'N' => simple("N", "KeyN", true, 78),
        'O' => simple("O", "KeyO", true, 79),
        'P' => simple("P", "KeyP", true, 80),
        'Q' => simple("Q", "KeyQ", true, 81),
        'R' => simple("R", "KeyR", true, 82),
        'S' => simple("S", "KeyS", true, 83),
        'T' => simple("T", "KeyT", true, 84),
        'U' => simple("U", "KeyU", true, 85),
        'V' => simple("V", "KeyV", true, 86),
        'W' => simple("W", "KeyW", true, 87),
        'X' => simple("X", "KeyX", true, 88),
        'Y' => simple("Y", "KeyY", true, 89),
        'Z' => simple("Z", "KeyZ", true, 90),

        // --- Digits ---
        '0' => simple("0", "Digit0", false, 48),
        '1' => simple("1", "Digit1", false, 49),
        '2' => simple("2", "Digit2", false, 50),
        '3' => simple("3", "Digit3", false, 51),
        '4' => simple("4", "Digit4", false, 52),
        '5' => simple("5", "Digit5", false, 53),
        '6' => simple("6", "Digit6", false, 54),
        '7' => simple("7", "Digit7", false, 55),
        '8' => simple("8", "Digit8", false, 56),
        '9' => simple("9", "Digit9", false, 57),

        // --- Shift+Digit symbols (ABNT2) ---
        '!' => simple("!", "Digit1", true, 49),
        '@' => simple("@", "Digit2", true, 50),
        '#' => simple("#", "Digit3", true, 51),
        '$' => simple("$", "Digit4", true, 52),
        '%' => simple("%", "Digit5", true, 53),
        // Shift+6 = ¨ (dead key diaeresis — handled below in accented vowels)
        '&' => simple("&", "Digit7", true, 55),
        '*' => simple("*", "Digit8", true, 56),
        '(' => simple("(", "Digit9", true, 57),
        ')' => simple(")", "Digit0", true, 48),

        // --- Punctuation & symbols (ABNT2 layout) ---
        ' ' => simple(" ", "Space", false, 32),
        '-' => simple("-", "Minus", false, 189),
        '_' => simple("_", "Minus", true, 189),
        '=' => simple("=", "Equal", false, 187),
        '+' => simple("+", "Equal", true, 187),
        '[' => simple("[", "BracketLeft", false, 219),
        '{' => simple("{", "BracketLeft", true, 219),
        ']' => simple("]", "BracketRight", false, 221),
        '}' => simple("}", "BracketRight", true, 221),
        '\\' => simple("\\", "Backslash", false, 220),
        '|' => simple("|", "Backslash", true, 220),
        ';' => simple(";", "Semicolon", false, 186),
        ':' => simple(":", "Semicolon", true, 186),
        '/' => simple("/", "Slash", false, 191),
        '?' => simple("?", "Slash", true, 191),
        ',' => simple(",", "Comma", false, 188),
        '<' => simple("<", "Comma", true, 188),
        '.' => simple(".", "Period", false, 190),
        '>' => simple(">", "Period", true, 190),
        '\'' => simple("'", "Quote", false, 222),
        '"' => simple("\"", "Quote", true, 222),
        '`' => simple("`", "Backquote", false, 192),

        // --- ç (direct key on ABNT2, where ; is on US layout) ---
        'ç' => simple("ç", "Semicolon", false, 186),
        'Ç' => simple("Ç", "Semicolon", true, 186),

        // --- Tab, Enter, Backspace ---
        '\t' => simple("Tab", "Tab", false, 9),
        '\n' => simple("Enter", "Enter", false, 13),

        // =====================================================
        // Dead key accented characters
        // =====================================================

        // --- Acute accent (´ = Quote key unshifted on ABNT2) ---
        'á' => dead_key("Dead", "Quote", false, 222, "a", "KeyA", false, 65),
        'é' => dead_key("Dead", "Quote", false, 222, "e", "KeyE", false, 69),
        'í' => dead_key("Dead", "Quote", false, 222, "i", "KeyI", false, 73),
        'ó' => dead_key("Dead", "Quote", false, 222, "o", "KeyO", false, 79),
        'ú' => dead_key("Dead", "Quote", false, 222, "u", "KeyU", false, 85),
        'Á' => dead_key("Dead", "Quote", false, 222, "A", "KeyA", true, 65),
        'É' => dead_key("Dead", "Quote", false, 222, "E", "KeyE", true, 69),
        'Í' => dead_key("Dead", "Quote", false, 222, "I", "KeyI", true, 73),
        'Ó' => dead_key("Dead", "Quote", false, 222, "O", "KeyO", true, 79),
        'Ú' => dead_key("Dead", "Quote", false, 222, "U", "KeyU", true, 85),

        // --- Grave accent (` = Backquote key on some ABNT2, or Shift+Quote for `) ---
        // On ABNT2: the grave/tilde dead key is typically at the key left of 1
        'à' => dead_key("Dead", "Backquote", true, 192, "a", "KeyA", false, 65),
        'è' => dead_key("Dead", "Backquote", true, 192, "e", "KeyE", false, 69),
        'ì' => dead_key("Dead", "Backquote", true, 192, "i", "KeyI", false, 73),
        'ò' => dead_key("Dead", "Backquote", true, 192, "o", "KeyO", false, 79),
        'ù' => dead_key("Dead", "Backquote", true, 192, "u", "KeyU", false, 85),
        'À' => dead_key("Dead", "Backquote", true, 192, "A", "KeyA", true, 65),
        'È' => dead_key("Dead", "Backquote", true, 192, "E", "KeyE", true, 69),
        'Ì' => dead_key("Dead", "Backquote", true, 192, "I", "KeyI", true, 73),
        'Ò' => dead_key("Dead", "Backquote", true, 192, "O", "KeyO", true, 79),
        'Ù' => dead_key("Dead", "Backquote", true, 192, "U", "KeyU", true, 85),

        // --- Tilde (~ = Backquote key unshifted on ABNT2) ---
        'ã' => dead_key("Dead", "Backquote", false, 192, "a", "KeyA", false, 65),
        'õ' => dead_key("Dead", "Backquote", false, 192, "o", "KeyO", false, 79),
        'ñ' => dead_key("Dead", "Backquote", false, 192, "n", "KeyN", false, 78),
        'Ã' => dead_key("Dead", "Backquote", false, 192, "A", "KeyA", true, 65),
        'Õ' => dead_key("Dead", "Backquote", false, 192, "O", "KeyO", true, 79),
        'Ñ' => dead_key("Dead", "Backquote", false, 192, "N", "KeyN", true, 78),

        // --- Circumflex (^ = Shift+Backquote on some ABNT2, or Digit6 unshifted) ---
        // On ABNT2: ^ is typically Shift+Tilde key
        'â' => dead_key("Dead", "Digit6", true, 54, "a", "KeyA", false, 65),
        'ê' => dead_key("Dead", "Digit6", true, 54, "e", "KeyE", false, 69),
        'î' => dead_key("Dead", "Digit6", true, 54, "i", "KeyI", false, 73),
        'ô' => dead_key("Dead", "Digit6", true, 54, "o", "KeyO", false, 79),
        'û' => dead_key("Dead", "Digit6", true, 54, "u", "KeyU", false, 85),
        'Â' => dead_key("Dead", "Digit6", true, 54, "A", "KeyA", true, 65),
        'Ê' => dead_key("Dead", "Digit6", true, 54, "E", "KeyE", true, 69),
        'Î' => dead_key("Dead", "Digit6", true, 54, "I", "KeyI", true, 73),
        'Ô' => dead_key("Dead", "Digit6", true, 54, "O", "KeyO", true, 79),
        'Û' => dead_key("Dead", "Digit6", true, 54, "U", "KeyU", true, 85),

        // --- Diaeresis/Umlaut (¨ = Shift+6 on ABNT2, dead key) ---
        'ä' => dead_key("Dead", "Digit6", true, 54, "a", "KeyA", false, 65),
        'ë' => dead_key("Dead", "Digit6", true, 54, "e", "KeyE", false, 69),
        'ï' => dead_key("Dead", "Digit6", true, 54, "i", "KeyI", false, 73),
        'ö' => dead_key("Dead", "Digit6", true, 54, "o", "KeyO", false, 79),
        'ü' => dead_key("Dead", "Digit6", true, 54, "u", "KeyU", false, 85),
        'Ä' => dead_key("Dead", "Digit6", true, 54, "A", "KeyA", true, 65),
        'Ë' => dead_key("Dead", "Digit6", true, 54, "E", "KeyE", true, 69),
        'Ï' => dead_key("Dead", "Digit6", true, 54, "I", "KeyI", true, 73),
        'Ö' => dead_key("Dead", "Digit6", true, 54, "O", "KeyO", true, 79),
        'Ü' => dead_key("Dead", "Digit6", true, 54, "U", "KeyU", true, 85),

        // --- Everything else: paste ---
        _ => KeySequence::Paste,
    }
}

fn simple(key: &'static str, code: &'static str, shift: bool, vk: i32) -> KeySequence {
    KeySequence::Simple {
        key,
        code,
        shift,
        vk,
    }
}

#[allow(clippy::too_many_arguments)]
fn dead_key(
    dead_key: &'static str,
    dead_code: &'static str,
    dead_shift: bool,
    dead_vk: i32,
    base_key: &'static str,
    base_code: &'static str,
    base_shift: bool,
    base_vk: i32,
) -> KeySequence {
    KeySequence::DeadKey {
        dead_key,
        dead_code,
        dead_shift,
        dead_vk,
        base_key,
        base_code,
        base_shift,
        base_vk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_chars() {
        assert!(matches!(abnt2_sequence('a'), KeySequence::Simple { shift: false, .. }));
        assert!(matches!(abnt2_sequence('A'), KeySequence::Simple { shift: true, .. }));
        assert!(matches!(abnt2_sequence('1'), KeySequence::Simple { shift: false, .. }));
        assert!(matches!(abnt2_sequence('!'), KeySequence::Simple { shift: true, .. }));
        assert!(matches!(abnt2_sequence(' '), KeySequence::Simple { .. }));
    }

    #[test]
    fn cedilla_direct() {
        assert!(matches!(abnt2_sequence('ç'), KeySequence::Simple { shift: false, .. }));
        assert!(matches!(abnt2_sequence('Ç'), KeySequence::Simple { shift: true, .. }));
    }

    #[test]
    fn accented_dead_key() {
        assert!(matches!(abnt2_sequence('á'), KeySequence::DeadKey { .. }));
        assert!(matches!(abnt2_sequence('ã'), KeySequence::DeadKey { .. }));
        assert!(matches!(abnt2_sequence('â'), KeySequence::DeadKey { .. }));
        assert!(matches!(abnt2_sequence('é'), KeySequence::DeadKey { .. }));
        assert!(matches!(abnt2_sequence('ü'), KeySequence::DeadKey { .. }));
    }

    #[test]
    fn unmapped_chars_paste() {
        assert!(matches!(abnt2_sequence('🎉'), KeySequence::Paste));
        assert!(matches!(abnt2_sequence('©'), KeySequence::Paste));
        assert!(matches!(abnt2_sequence('→'), KeySequence::Paste));
    }
}

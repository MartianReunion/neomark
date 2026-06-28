/// Normalize a raw Neomark document string according to the character-level rules:
///
/// | Range              | Replacement | Description               |
/// |--------------------|-------------|---------------------------|
/// | U+0000–U+0008      | U+FFFD      | Invalid C0 control chars  |
/// | U+000B–U+000C      | U+000A      | VT, FF → newline          |
/// | U+0009             | U+0020      | TAB → space               |
/// | U+000E–U+001F      | U+FFFD      | Invalid C0 control chars  |
/// | U+0080–U+0090      | U+FFFD      | Invalid C1 control chars  |
/// | U+0093–U+009F      | U+FFFD      | Invalid C1 control chars  |
///
/// U+0091 and U+0092 are intentionally preserved (treated like Unicode private-use).
/// U+000A (LF) and U+000D (CR) are preserved for line-splitting.
pub fn normalize(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            // U+000B (VT) and U+000C (FF) → U+000A (LF)
            '\u{000B}' | '\u{000C}' => '\u{000A}',
            // U+0009 (TAB) → U+0020 (SPACE)
            '\u{0009}' => '\u{0020}',
            // Invalid C0 controls: U+0000–U+0008, U+000E–U+001F
            '\u{0000}'..='\u{0008}' => '\u{FFFD}',
            '\u{000E}'..='\u{001F}' => '\u{FFFD}',
            // Invalid C1 controls: U+0080–U+0090, U+0093–U+009F
            // (U+0091 and U+0092 are intentionally preserved)
            '\u{0080}'..='\u{0090}' => '\u{FFFD}',
            '\u{0093}'..='\u{009F}' => '\u{FFFD}',
            // All other characters (including LF, CR, U+0091, U+0092) pass through
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_to_space() {
        let result = normalize("hello\tworld");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_vt_ff_to_lf() {
        let result = normalize("a\u{000B}b\u{000C}c");
        assert_eq!(result, "a\nb\nc");
    }

    #[test]
    fn test_c0_controls_to_replacement() {
        let result = normalize("\u{0000}\u{0005}\u{000E}\u{001F}");
        assert_eq!(result, "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn test_c1_controls_to_replacement() {
        let result = normalize("\u{0080}\u{0090}\u{0093}\u{009F}");
        assert_eq!(result, "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn test_preserve_u0091_u0092() {
        let result = normalize("\u{0091}hello\u{0092}");
        assert_eq!(result, "\u{0091}hello\u{0092}");
    }

    #[test]
    fn test_preserve_lf_cr() {
        let result = normalize("line1\nline2\r\nline3");
        assert_eq!(result, "line1\nline2\r\nline3");
    }

    #[test]
    fn test_preserve_normal_text() {
        let input = "Hello, 世界! 🌍\n";
        let result = normalize(input);
        assert_eq!(result, input);
    }
}

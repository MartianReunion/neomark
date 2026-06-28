use std::string::ToString;

/// A line read from a Neomark document, with its indentation level.
///
/// Indentation is measured as the number of leading space (U+0020) characters.
/// The raw text including leading whitespace is stored; `content()` and
/// `full_text()` provide zero-copy access to different views of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The full line text including leading whitespace.
    pub raw: String,
    /// Number of leading space characters (indentation depth).
    pub indentation: usize,
}

impl Line {
    /// Returns `true` if this line is blank (empty or contains only spaces).
    pub fn is_blank(&self) -> bool {
        self.indentation == self.raw.len()
    }

    /// Returns the line text with leading whitespace removed (zero-copy).
    pub fn content(&self) -> &str {
        &self.raw[self.indentation..]
    }

    /// Returns the full line text including leading whitespace (zero-copy).
    pub fn full_text(&self) -> &str {
        &self.raw
    }
}

/// Split normalized input into lines.
///
/// Supports LF and CRLF line endings. A bare CR (not followed by LF) is NOT
/// recognized as a line terminator, consistent with Rust's [`String::lines()`].
///
/// Each line records its indentation (count of leading spaces) and stores the
/// raw text for zero-copy access via `content()` and `full_text()`.
pub fn split_lines(input: &str) -> Vec<Line> {
    input
        .lines()
        .map(|raw| {
            let indentation = raw.chars().take_while(|&c| c == ' ').count();
            Line {
                raw: raw.to_string(),
                indentation,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_simple_lines() {
        let lines = split_lines("hello\nworld");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content(), "hello");
        assert_eq!(lines[0].indentation, 0);
        assert_eq!(lines[1].content(), "world");
        assert_eq!(lines[1].indentation, 0);
    }

    #[test]
    fn test_split_with_indentation() {
        let lines = split_lines("  indented\n    more");
        assert_eq!(lines[0].content(), "indented");
        assert_eq!(lines[0].indentation, 2);
        assert_eq!(lines[0].full_text(), "  indented");
        assert_eq!(lines[1].content(), "more");
        assert_eq!(lines[1].indentation, 4);
        assert_eq!(lines[1].full_text(), "    more");
    }

    #[test]
    fn test_blank_lines() {
        let lines = split_lines("hello\n\n  \nworld");
        assert_eq!(lines.len(), 4);
        assert!(!lines[0].is_blank());
        assert!(lines[1].is_blank()); // empty
        assert!(lines[2].is_blank()); // spaces only
        assert!(!lines[3].is_blank());
        // Blank line full_text is just the spaces (or empty)
        assert_eq!(lines[1].full_text(), "");
        assert_eq!(lines[2].full_text(), "  ");
    }

    #[test]
    fn test_crlf_line_endings() {
        let lines = split_lines("line1\r\nline2\r\nline3");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content(), "line1");
        assert_eq!(lines[1].content(), "line2");
        assert_eq!(lines[2].content(), "line3");
    }

    #[test]
    fn test_empty_input() {
        let lines = split_lines("");
        assert_eq!(lines.len(), 0);
    }
}

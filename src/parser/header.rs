//! Invocation-block header parsing.
//!
//! Handles the `::` line (调用块头部) that begins every invocation block.
//! See [`InvocationHeader`] for the full grammar.

use super::line::Line;

/// A parameter parsed from an invocation block header.
///
/// Parameters take the form `name=value` or a bare `name` (boolean shorthand,
/// equivalent to `name=true`). Values are always stored as strings — no type
/// checking or conversion is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub value: String,
}

/// The parsed header of an invocation block.
///
/// The header line (after stripping leading whitespace) has the form:
///
/// ```neomark
/// ::name parameter1=value1 "parameter 2"="value 2" bool_parameter
/// ```
///
/// # Grammar
///
/// - `::` is followed by the called object's **name**. Spaces between `::`
///   and the name are allowed (e.g. `::  name`). If the name itself contains
///   spaces it may be wrapped in double quotes (e.g. `::"my name"`), with
///   the same `\"` / `\\` escape support as quoted parameter parts.
/// - Parameters are separated by one or more spaces. Each parameter is
///   either a `name=value` pair or a bare name (shorthand for `name=true`).
/// - If a parameter name or value contains spaces it must be wrapped in
///   double quotes. Inside quoted portions `\"` and `\\` are recognised
///   escape sequences.
/// - All values are stored as strings; no type checking is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationHeader {
    /// The name of the called object.
    pub name: String,
    /// Parsed parameters (name–value pairs).
    pub parameters: Vec<Parameter>,
}

impl InvocationHeader {
    /// Parse an [`InvocationHeader`] from a header [`Line`].
    ///
    /// The line's [`content`](Line::content) must start with `::` (the caller,
    /// [`super::block::split_blocks`], guarantees this).
    pub fn parse(line: &Line) -> Self {
        let content = line.content();
        let after_colons = &content[2..]; // strip leading `::`
        let after_colons = skip_spaces(after_colons);
        let (name, rest) = read_name(after_colons);
        let parameters = parse_parameters(rest);

        InvocationHeader { name, parameters }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────

/// Skip leading space (U+0020) characters, returning the remaining slice.
fn skip_spaces(s: &str) -> &str {
    let n = s.chars().take_while(|&c| c == ' ').count();
    &s[n..]
}

/// Read the invocation name.
///
/// If the name starts with `"` it is read as a quoted string (supporting the
/// same `\"` / `\\` escapes as quoted parameter names/values). Otherwise the
/// name extends to the first space or end of string.
fn read_name(s: &str) -> (String, &str) {
    let bytes = s.as_bytes();
    if bytes.first() == Some(&b'"') {
        let (name, i) = read_quoted(bytes, 0, bytes.len());
        (name, &s[i..]) // rest includes any trailing spaces, consumed by parse_parameters
    } else {
        let n = s.chars().take_while(|&c| c != ' ').count();
        (s[..n].to_string(), &s[n..])
    }
}

/// Parse the parameter list from the remainder of the header line.
fn parse_parameters(s: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        i = skip_spaces_at(bytes, i, len);
        if i >= len {
            break;
        }

        // Read key — quoted or unquoted
        let key: String;
        if bytes[i] == b'"' {
            (key, i) = read_quoted(bytes, i, len);
        } else {
            (key, i) = read_until(bytes, i, len, &[b' ', b'=']);
        }

        i = skip_spaces_at(bytes, i, len);

        let value: String;
        if i < len && bytes[i] == b'=' {
            i += 1; // consume '='
            i = skip_spaces_at(bytes, i, len);

            if i < len && bytes[i] == b'"' {
                (value, i) = read_quoted(bytes, i, len);
            } else {
                (value, i) = read_until(bytes, i, len, &[b' ']);
            }
        } else {
            // Bare name → boolean shorthand
            value = "true".to_string();
        }

        params.push(Parameter { name: key, value });
    }

    params
}

/// Skip spaces starting at byte position `i`, returning the new position.
fn skip_spaces_at(bytes: &[u8], mut i: usize, len: usize) -> usize {
    while i < len && bytes[i] == b' ' {
        i += 1;
    }
    i
}

/// Read an unquoted token, stopping at any byte in `stops`.
fn read_until(bytes: &[u8], i: usize, len: usize, stops: &[u8]) -> (String, usize) {
    let mut j = i;
    while j < len && !stops.contains(&bytes[j]) {
        j += 1;
    }
    // SAFETY: we only split at ASCII byte boundaries — UTF‑8 validity is preserved
    let token = std::str::from_utf8(&bytes[i..j])
        .unwrap_or_default()
        .to_string();
    (token, j)
}

/// Read a double-quoted string, handling `\"` and `\\` escapes.
///
/// `bytes[i]` must be `"`.  Returns the unescaped content and the position
/// after the closing `"` (or end-of-string for malformed input).
fn read_quoted(bytes: &[u8], mut i: usize, len: usize) -> (String, usize) {
    debug_assert!(i < len && bytes[i] == b'"');
    i += 1; // opening quote
    let mut result = String::new();

    while i < len {
        match bytes[i] {
            b'\\' if i + 1 < len => match bytes[i + 1] {
                b'"' => {
                    result.push('"');
                    i += 2;
                }
                b'\\' => {
                    result.push('\\');
                    i += 2;
                }
                _ => {
                    // Unknown escape — keep the backslash
                    result.push('\\');
                    i += 1;
                }
            },
            b'"' => {
                i += 1; // closing quote
                break;
            }
            _ => {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
    }

    (result, i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::line;

    /// Helper: parse a header from a raw string (wrapping :: line).
    fn parse_header(raw: &str) -> InvocationHeader {
        let line = line::split_lines(raw)[0].clone();
        InvocationHeader::parse(&line)
    }

    #[test]
    fn test_simple_name() {
        let h = parse_header("::func");
        assert_eq!(h.name, "func");
        assert!(h.parameters.is_empty());
    }

    #[test]
    fn test_spaces_after_colons() {
        let h = parse_header("::  name");
        assert_eq!(h.name, "name");
    }

    #[test]
    fn test_single_param() {
        let h = parse_header("::func key=value");
        assert_eq!(h.name, "func");
        assert_eq!(h.parameters.len(), 1);
        assert_eq!(h.parameters[0].name, "key");
        assert_eq!(h.parameters[0].value, "value");
    }

    #[test]
    fn test_multiple_params() {
        let h = parse_header("::func a=1 b=2");
        assert_eq!(h.parameters.len(), 2);
        assert_eq!(h.parameters[0].name, "a");
        assert_eq!(h.parameters[0].value, "1");
        assert_eq!(h.parameters[1].name, "b");
        assert_eq!(h.parameters[1].value, "2");
    }

    #[test]
    fn test_bool_param() {
        let h = parse_header("::func verbose");
        assert_eq!(h.parameters.len(), 1);
        assert_eq!(h.parameters[0].name, "verbose");
        assert_eq!(h.parameters[0].value, "true");
    }

    #[test]
    fn test_mixed_params() {
        let h = parse_header("::cmd key=val flag");
        assert_eq!(h.parameters.len(), 2);
        assert_eq!(h.parameters[0].name, "key");
        assert_eq!(h.parameters[0].value, "val");
        assert_eq!(h.parameters[1].name, "flag");
        assert_eq!(h.parameters[1].value, "true");
    }

    #[test]
    fn test_quoted_key() {
        let h = parse_header("::func \"my key\"=val");
        assert_eq!(h.parameters[0].name, "my key");
        assert_eq!(h.parameters[0].value, "val");
    }

    #[test]
    fn test_quoted_value() {
        let h = parse_header("::func key=\"my value\"");
        assert_eq!(h.parameters[0].name, "key");
        assert_eq!(h.parameters[0].value, "my value");
    }

    #[test]
    fn test_both_quoted() {
        let h = parse_header("::func \"param 2\"=\"value 2\"");
        assert_eq!(h.parameters[0].name, "param 2");
        assert_eq!(h.parameters[0].value, "value 2");
    }

    #[test]
    fn test_escape_quote() {
        let h = parse_header("::func \"key\\\"name\"=val");
        assert_eq!(h.parameters[0].name, "key\"name");
    }

    #[test]
    fn test_escape_backslash() {
        let h = parse_header("::func key=\"val\\\\ue\"");
        assert_eq!(h.parameters[0].value, "val\\ue");
    }

    #[test]
    fn test_quoted_name() {
        let h = parse_header("::\"my func\" param=val");
        assert_eq!(h.name, "my func");
        assert_eq!(h.parameters.len(), 1);
        assert_eq!(h.parameters[0].name, "param");
        assert_eq!(h.parameters[0].value, "val");
    }

    #[test]
    fn test_quoted_name_with_escape() {
        let h = parse_header("::\"hello\\\"world\"");
        assert_eq!(h.name, "hello\"world");
        assert!(h.parameters.is_empty());
    }

    #[test]
    fn test_quoted_name_with_params() {
        let h = parse_header("::\"some name\" flag key=\"value\"");
        assert_eq!(h.name, "some name");
        assert_eq!(h.parameters.len(), 2);
        assert_eq!(h.parameters[0].name, "flag");
        assert_eq!(h.parameters[0].value, "true");
        assert_eq!(h.parameters[1].name, "key");
        assert_eq!(h.parameters[1].value, "value");
    }

    #[test]
    fn test_just_colons() {
        let h = parse_header("::");
        assert_eq!(h.name, "");
        assert!(h.parameters.is_empty());
    }

    #[test]
    fn test_complex() {
        let h = parse_header(
            "::render title=\"Hello World\" \"alt text\"=\"A quote: \\\"hi\\\"\" show",
        );
        assert_eq!(h.name, "render");
        assert_eq!(h.parameters.len(), 3);

        assert_eq!(h.parameters[0].name, "title");
        assert_eq!(h.parameters[0].value, "Hello World");

        assert_eq!(h.parameters[1].name, "alt text");
        assert_eq!(h.parameters[1].value, "A quote: \"hi\"");

        assert_eq!(h.parameters[2].name, "show");
        assert_eq!(h.parameters[2].value, "true");
    }
}

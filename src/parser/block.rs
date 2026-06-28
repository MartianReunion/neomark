use super::header::InvocationHeader;
use super::line::Line;

/// A natural block (自然块).
///
/// Natural blocks are formed by consecutive non-blank lines that are not part
/// of an invocation block. Leading whitespace on each line is ignored during
/// rendering, but the [`Line`] metadata (indentation, raw text) is preserved
/// for source-level introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Natural {
    /// The lines that make up this natural block.
    pub lines: Vec<Line>,
}

/// An invocation block (调用块).
///
/// Invocation blocks are used to call templates, functions, or render nested
/// content. They begin with a `::` header line and contain all subsequent
/// lines whose indentation is strictly greater than that of the header line.
///
/// Nested `::` lines appear as regular interior lines at this stage and are
/// handled during recursive parsing later.
///
/// An invocation block may have an empty interior (header line only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The parsed header from the `::` line.
    pub header: InvocationHeader,
    /// Interior lines, stored with their original indentation.
    ///
    /// Blank lines that appear between content lines are materialized as
    /// empty [`Line`]s with `indentation == base_indentation`. Trailing blank
    /// lines are not included.
    pub lines: Vec<Line>,
}

/// The kind of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// A natural block — plain text.
    Natural(Natural),
    /// An invocation block — template/function call with nested content.
    Invocation(Invocation),
}

/// A top-level block in a Neomark document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Indentation of the block's first line within the document.
    pub base_indentation: usize,
    /// Reserved unique identifier (for future incremental-parsing use).
    pub id: u128,
    /// The content of this block.
    pub content: BlockKind,
}

/// Split a sequence of [`Line`]s into [`Block`]s.
///
/// # Algorithm
///
/// 1. **Skip blank lines** between blocks — consecutive blank lines are
///    collapsed into a single separator.
///
/// 2. **Invocation block** — if a non-blank line's [`content`](Line::content)
///    starts with `::`:
///    - Record the line's indentation as `base_indentation`.
///    - Collect subsequent lines whose `indentation > base_indentation`.
///    - Blank lines among the interior are counted and lazily materialized
///      only when the next content line is encountered — this naturally
///      trims trailing blank lines without an extra pass.
///    - An invocation block may have an empty interior (just the header).
///
/// 3. **Natural block** — otherwise:
///    - Collect consecutive non-blank lines that don't start with `::`.
///    - Stop at the first blank line or `::` line.
pub fn split_blocks(lines: &[Line]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Skip blank lines between blocks
        if lines[i].is_blank() {
            i += 1;
            continue;
        }

        // Check if this is an invocation block
        if lines[i].content().starts_with("::") {
            let base_indentation = lines[i].indentation;
            let header = InvocationHeader::parse(&lines[i]);
            i += 1; // skip the :: header line

            // Collect interior lines.
            // Blank lines are counted (empty_line) and lazily materialized
            // only when a non-blank content line follows — trailing blanks
            // are therefore never flushed and automatically trimmed.
            let mut interior: Vec<Line> = Vec::new();
            let mut empty_line: usize = 0;

            while i < lines.len() {
                let line = lines[i].clone();

                if line.is_blank() {
                    // Defer materialization; only count for now
                    empty_line += 1;
                    i += 1;
                    continue;
                }

                if line.indentation > base_indentation {
                    // Flush any pending blank lines before this content line
                    for _ in 0..empty_line {
                        interior.push(Line {
                            raw: String::new(),
                            indentation: base_indentation,
                        });
                    }
                    empty_line = 0;
                    interior.push(line);
                    i += 1;
                } else {
                    // Indentation too shallow — belongs to the next block
                    break;
                }
            }
            // Any remaining empty_line count is trailing blanks → discarded

            let invocation = Invocation {
                header,
                lines: interior,
            };
            let block = Block {
                base_indentation,
                id: 0, // TODO: assign stable IDs
                content: BlockKind::Invocation(invocation),
            };
            blocks.push(block);
        } else {
            // Natural block: collect lines until blank or invocation start
            let mut content_lines: Vec<Line> = Vec::new();
            while i < lines.len() {
                if lines[i].is_blank() {
                    break;
                }
                if lines[i].content().starts_with("::") {
                    break;
                }
                // Leading whitespace is ignored in natural blocks during
                // rendering; Line metadata is kept for source mapping.
                content_lines.push(lines[i].clone());
                i += 1;
            }
            let natural = Natural {
                lines: content_lines,
            };
            let block = Block {
                base_indentation: 0,
                id: 0, // TODO: assign stable IDs
                content: BlockKind::Natural(natural),
            };
            blocks.push(block);
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::line;

    /// Run the full pipeline: normalize → split_lines → split_blocks
    fn parse(input: &str) -> Vec<Block> {
        let normalized = crate::parser::reader::normalize(input);
        let lines = line::split_lines(&normalized);
        split_blocks(&lines)
    }

    // ── Natural blocks ────────────────────────────────────────────────

    #[test]
    fn test_single_natural_block() {
        let blocks = parse("hello world");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 1);
                assert_eq!(n.lines[0].content(), "hello world");
            }
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_natural_block_multiple_lines() {
        let blocks = parse("line one\nline two\nline three");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 3);
                assert_eq!(n.lines[0].content(), "line one");
                assert_eq!(n.lines[1].content(), "line two");
                assert_eq!(n.lines[2].content(), "line three");
            }
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_multiple_natural_blocks() {
        let blocks = parse("first block\n\nsecond block");
        assert_eq!(blocks.len(), 2);
        match &blocks[0].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "first block"),
            _ => panic!("Expected Natural"),
        }
        match &blocks[1].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "second block"),
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_multiple_blank_lines_between_blocks_collapsed() {
        let blocks = parse("first\n\n\n\nsecond");
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_natural_block_preserves_indentation_metadata() {
        let blocks = parse("  indented\n    more");
        match &blocks[0].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines[0].content(), "indented");
                assert_eq!(n.lines[0].indentation, 2);
                assert_eq!(n.lines[1].content(), "more");
                assert_eq!(n.lines[1].indentation, 4);
            }
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_natural_block_stops_at_invocation() {
        let blocks = parse("text\n::func\n  inside");
        assert_eq!(blocks.len(), 2);
        match &blocks[0].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "text"),
            _ => panic!("Expected Natural"),
        }
        match &blocks[1].content {
            BlockKind::Invocation(_) => {}
            _ => panic!("Expected Invocation"),
        }
    }

    // ── Invocation blocks ─────────────────────────────────────────────

    #[test]
    fn test_simple_invocation_block() {
        let blocks = parse("::func\n  content line 1\n  content line 2");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].base_indentation, 0);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 2);
                assert_eq!(inv.lines[0].content(), "content line 1");
                assert_eq!(inv.lines[0].indentation, 2);
                assert_eq!(inv.lines[1].content(), "content line 2");
                assert_eq!(inv.lines[1].indentation, 2);
            }
            _ => panic!("Expected Invocation"),
        }
    }

    #[test]
    fn test_invocation_header_only() {
        // ::func with no interior lines
        let blocks = parse("::func\nsibling natural");
        assert_eq!(blocks.len(), 2);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 0);
            }
            _ => panic!("Expected Invocation"),
        }
        match &blocks[1].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines[0].content(), "sibling natural");
            }
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_invocation_header_then_blank_then_natural() {
        // ::func\n\nnatural — blank line is a block separator
        let blocks = parse("::func\n\nnatural after blank");
        assert_eq!(blocks.len(), 2);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => assert_eq!(inv.lines.len(), 0),
            _ => panic!("Expected Invocation"),
        }
        match &blocks[1].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "natural after blank"),
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_nested_invocation_preserved_as_raw_lines() {
        // Nested ::inner is just a regular interior line at this stage
        let blocks = parse("::outer\n  ::inner\n    deep content");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 2);
                assert_eq!(inv.lines[0].content(), "::inner");
                assert_eq!(inv.lines[0].indentation, 2);
                assert_eq!(inv.lines[1].content(), "deep content");
                assert_eq!(inv.lines[1].indentation, 4);
            }
            _ => panic!("Expected Invocation"),
        }
    }

    #[test]
    fn test_trailing_blank_lines_trimmed() {
        let blocks = parse("::func\n  content\n  \n\noutside");
        assert_eq!(blocks.len(), 2);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 1);
                assert_eq!(inv.lines[0].content(), "content");
            }
            _ => panic!("Expected Invocation"),
        }
        match &blocks[1].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "outside"),
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_blank_lines_inside_invocation_preserved() {
        let blocks = parse("::func\n  line one\n  \n  line two");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 3);
                assert_eq!(inv.lines[0].content(), "line one");
                assert!(inv.lines[1].is_blank());
                assert_eq!(inv.lines[1].indentation, 0); // base_indentation
                assert_eq!(inv.lines[2].content(), "line two");
            }
            _ => panic!("Expected Invocation"),
        }
    }

    #[test]
    fn test_multiple_consecutive_blank_lines_inside_invocation() {
        let blocks = parse("::func\n  a\n  \n  \n  b");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 4); // a, blank, blank, b
                assert_eq!(inv.lines[0].content(), "a");
                assert!(inv.lines[1].is_blank());
                assert!(inv.lines[2].is_blank());
                assert_eq!(inv.lines[3].content(), "b");
            }
            _ => panic!("Expected Invocation"),
        }
    }

    #[test]
    fn test_invocation_with_indented_header() {
        let blocks = parse("  ::func\n    inside\noutside");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].base_indentation, 2);
        match &blocks[0].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 1);
                assert_eq!(inv.lines[0].content(), "inside");
                assert_eq!(inv.lines[0].indentation, 4);
            }
            _ => panic!("Expected Invocation"),
        }
        match &blocks[1].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "outside"),
            _ => panic!("Expected Natural"),
        }
    }

    #[test]
    fn test_natural_before_and_after_invocation() {
        let blocks = parse("before\n\n::func\n  inner\n\nafter");
        assert_eq!(blocks.len(), 3);
        match &blocks[0].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "before"),
            _ => panic!("Expected Natural"),
        }
        match &blocks[1].content {
            BlockKind::Invocation(inv) => assert_eq!(inv.lines[0].content(), "inner"),
            _ => panic!("Expected Invocation"),
        }
        match &blocks[2].content {
            BlockKind::Natural(n) => assert_eq!(n.lines[0].content(), "after"),
            _ => panic!("Expected Natural"),
        }
    }

    // ── Comprehensive example from block.md ───────────────────────────

    #[test]
    fn test_comprehensive_example() {
        let input = concat!(
            "这里是第一个自然块。\n",
            "这里仍然是第一个自然块。\n",
            "\n",
            "但这里就是第二个自然块。\n",
            "\n",
            "\n",
            "这里是第三个自然块，中间有两个空行，但也被忽略了。\n",
            "::func\n",
            "  这里是一个调用块的内部，即使调用块前没有空行，第三个自然块的解析也随之停止。\n",
            "  ::func\n",
            "    这是一个调用块内嵌套的调用块。\n",
            "\n",
            "    这里仍然是嵌套的一部分，即使中间存在空行。\n",
            "这里缩进同时不满足两个嵌套调用块的缩进，那么这里就是第四个自然块。\n",
            "\n",
            "::func\n",
            "一个调用块可以没有内部的行，而只有头部。\n",
            "\n",
            "::func\n",
            "  尽管 Neomark 解析器被设计为可以解析，但最好的写法仍然是在调用块的上下也留出空格。\n",
            "\n",
            "\n",
            "\n",
            "此处为第五个自然块，注意到上一个调用块后有三个空行，\n",
            "但它们都不是该调用块的一部分，因为它们出现在了调用块的末尾。",
        );

        let blocks = parse(input);

        assert_eq!(blocks.len(), 9);

        // Block 0: first natural block (2 lines)
        match &blocks[0].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 2);
                assert_eq!(n.lines[0].content(), "这里是第一个自然块。");
                assert_eq!(n.lines[1].content(), "这里仍然是第一个自然块。");
            }
            _ => panic!("Block 0: expected Natural"),
        }

        // Block 1: second natural block
        match &blocks[1].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 1);
                assert_eq!(n.lines[0].content(), "但这里就是第二个自然块。");
            }
            _ => panic!("Block 1: expected Natural"),
        }

        // Block 2: third natural block
        match &blocks[2].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 1);
                assert_eq!(
                    n.lines[0].content(),
                    "这里是第三个自然块，中间有两个空行，但也被忽略了。"
                );
            }
            _ => panic!("Block 2: expected Natural"),
        }

        // Block 3: first ::func invocation (contains nested ::func)
        match &blocks[3].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 5);
                assert_eq!(
                    inv.lines[0].content(),
                    "这里是一个调用块的内部，即使调用块前没有空行，第三个自然块的解析也随之停止。"
                );
                assert_eq!(inv.lines[0].indentation, 2);
                assert_eq!(inv.lines[1].content(), "::func");
                assert_eq!(inv.lines[1].indentation, 2);
                assert_eq!(inv.lines[2].content(), "这是一个调用块内嵌套的调用块。");
                assert_eq!(inv.lines[2].indentation, 4);
                assert!(inv.lines[3].is_blank());
                assert_eq!(
                    inv.lines[4].content(),
                    "这里仍然是嵌套的一部分，即使中间存在空行。"
                );
                assert_eq!(inv.lines[4].indentation, 4);
            }
            _ => panic!("Block 3: expected Invocation"),
        }

        // Block 4: fourth natural block
        match &blocks[4].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 1);
                assert_eq!(
                    n.lines[0].content(),
                    "这里缩进同时不满足两个嵌套调用块的缩进，那么这里就是第四个自然块。"
                );
            }
            _ => panic!("Block 4: expected Natural"),
        }

        // Block 5: second ::func — header only, no interior
        match &blocks[5].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 0);
            }
            _ => panic!("Block 5: expected Invocation (header-only)"),
        }

        // Block 6: "一个调用块可以没有内部的行，而只有头部。"
        match &blocks[6].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 1);
                assert_eq!(
                    n.lines[0].content(),
                    "一个调用块可以没有内部的行，而只有头部。"
                );
            }
            _ => panic!("Block 6: expected Natural"),
        }

        // Block 7: third ::func — with interior content
        match &blocks[7].content {
            BlockKind::Invocation(inv) => {
                assert_eq!(inv.lines.len(), 1);
                assert_eq!(
                    inv.lines[0].content(),
                    "尽管 Neomark 解析器被设计为可以解析，但最好的写法仍然是在调用块的上下也留出空格。"
                );
                assert_eq!(inv.lines[0].indentation, 2);
            }
            _ => panic!("Block 7: expected Invocation"),
        }

        // Block 8: fifth natural block (trailing, 2 lines)
        match &blocks[8].content {
            BlockKind::Natural(n) => {
                assert_eq!(n.lines.len(), 2);
                assert_eq!(
                    n.lines[0].content(),
                    "此处为第五个自然块，注意到上一个调用块后有三个空行，"
                );
                assert_eq!(
                    n.lines[1].content(),
                    "但它们都不是该调用块的一部分，因为它们出现在了调用块的末尾。"
                );
            }
            _ => panic!("Block 8: expected Natural"),
        }
    }
}

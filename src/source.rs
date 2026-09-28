/// Convert a byte offset into `source` to a 1-based line number.
///
/// ruff's AST ranges are byte offsets; diagnostics need line numbers.
/// Linear in the offset; use `LineIndex` for repeated lookups.
pub fn line_of(source: &str, offset: u32) -> u32 {
    let end = (offset as usize).min(source.len());
    source.as_bytes()[..end].iter().filter(|&&b| b == b'\n').count() as u32 + 1
}

/// Byte offsets of line starts, for O(log n) offset -> line lookups.
#[derive(Debug, Clone, Default)]
pub struct LineIndex {
    /// Offset just after each `\n`.
    newlines: Vec<u32>,
    len: u32,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        LineIndex {
            newlines: source
                .bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i as u32 + 1)
                .collect(),
            len: source.len() as u32,
        }
    }

    /// 1-based line of byte `offset` (the same result as `line_of`).
    pub fn line(&self, offset: u32) -> u32 {
        let offset = offset.min(self.len);
        self.newlines.partition_point(|&start| start <= offset) as u32 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_of() {
        let src = "a\nbc\n\nd";
        assert_eq!(line_of(src, 0), 1);
        assert_eq!(line_of(src, 2), 2);
        assert_eq!(line_of(src, 5), 3);
        assert_eq!(line_of(src, 6), 4);
        assert_eq!(line_of(src, 999), 4);
        let idx = LineIndex::new(src);
        for off in 0..12 {
            assert_eq!(idx.line(off), line_of(src, off), "offset {off}");
        }
    }
}

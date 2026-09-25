/// Convert a byte offset into `source` to a 1-based line number.
///
/// ruff's AST ranges are byte offsets; diagnostics need line numbers.
pub fn line_of(source: &str, offset: u32) -> u32 {
    let end = (offset as usize).min(source.len());
    source.as_bytes()[..end].iter().filter(|&&b| b == b'\n').count() as u32 + 1
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
    }
}

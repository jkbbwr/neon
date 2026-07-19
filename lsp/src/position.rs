//! Byte offsets to LSP positions.
//!
//! The compiler counts bytes; LSP counts UTF-16 code units within a line, which is neither
//! bytes nor characters. Getting this wrong is invisible in ASCII and silently misplaces
//! every span after the first non-ASCII character in a file — an underline that drifts a
//! little further right with each emoji. Neon's strings are unvalidated bytes and its
//! corpus already contains multi-byte text, so this is not a hypothetical.
//!
//! The index is built once per document version and binary-searched, rather than scanning
//! from the start for each diagnostic: a file with many errors is exactly the file being
//! edited, and quadratic behaviour there would be felt.

use lsp_types::Position;

/// Line start offsets for one document, for turning byte offsets into positions.
pub struct LineIndex {
    /// Byte offset of the first character of each line. Always starts with 0, so a
    /// document with no newlines still has one entry.
    starts: Vec<usize>,
    text: String,
}

impl LineIndex {
    pub fn new(text: &str) -> LineIndex {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts, text: text.to_string() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The position of a byte offset. An offset past the end clamps to the end of the
    /// document: a span that runs off the end is a compiler bug, but dropping the
    /// diagnostic or panicking in an editor session is a worse answer than pointing at
    /// the last character.
    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.text.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let line_start = self.starts[line];
        // Only the part of the line before the offset counts, and it counts in UTF-16.
        let prefix = &self.text[line_start..offset];
        let character = prefix.chars().map(char::len_utf16).sum::<usize>();
        Position { line: line as u32, character: character as u32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_multibyte_line_counts_utf16_units_not_bytes() {
        // "é" is two bytes and one UTF-16 unit; the emoji is four bytes and *two* units,
        // being outside the basic plane. A byte count would report 6 here and a character
        // count 2, and both would be wrong.
        let idx = LineIndex::new("é🙂x");
        let offset = "é🙂".len();
        assert_eq!(idx.position(offset), Position { line: 0, character: 3 });
    }

    #[test]
    fn positions_are_relative_to_their_own_line() {
        let idx = LineIndex::new("ab\ncd\nef");
        assert_eq!(idx.position(4), Position { line: 1, character: 1 });
        assert_eq!(idx.position(6), Position { line: 2, character: 0 });
    }

    #[test]
    fn an_offset_past_the_end_clamps() {
        let idx = LineIndex::new("ab\n");
        assert_eq!(idx.position(999), idx.position("ab\n".len()));
    }
}

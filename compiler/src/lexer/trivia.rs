//! Everything the grammar ignores and the formatter must not lose.
//!
//! Comments and the file's line table live here as side tables keyed by byte
//! offset, deliberately outside the token stream. That split is what lets the
//! parser's alphabet be exactly the grammar's — no rule has to tolerate a
//! comment appearing between any two symbols — while `neon fmt` still sees
//! every one of them.
//!
//! Whitespace is *not* recorded. The formatter does not need to know where the
//! spaces were, only where the author put line breaks, and that is recoverable
//! from `line_starts` plus the spans it already has: `line_of` turns an offset
//! into a line, `blank_lines_between` recovers the separation the author chose
//! between two items, and the formatter's `is_broken` asks whether a construct
//! spanned lines. Storing whitespace runs would be more data saying the same
//! thing, and would have to be kept in sync with the spans.
//!
//! Attachment is decided by the formatter, not here: a `Trivia` knows its own
//! extent and nothing about what it belongs to. The formatter walks the AST in
//! source order and flushes every comment ending before the item it is about to
//! print, which is why comments come out in the right place without anything
//! ever computing an "owner" — and why nothing can be orphaned, since the final
//! flush is unbounded.

use super::token::Span;

/// A comment. Trivia is everything the grammar ignores but the formatter must
/// not lose: without it, `neon fmt` could only ever delete comments.
///
/// It is a side table rather than a token, so the parser's input is unchanged
/// and no combinator has to step over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    /// Covers the whole comment, delimiters included.
    pub span: Span,
    /// The comment's text, delimiters stripped.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriviaKind {
    /// `// ...`
    Line,
    /// `/// ...` — attaches to the following declaration.
    Doc,
    /// `/* ... */`, possibly nested.
    Block,
}

/// The lexer's full output. Tokens drive the parser; trivia and line starts
/// exist for the formatter.
#[derive(Debug, Clone)]
pub struct Lexed {
    pub tokens: Vec<super::Spanned>,
    pub trivia: Vec<Trivia>,
    /// Byte offset of the start of each line, so a span can be turned into a
    /// line number. Blank lines the author left between items are recovered
    /// from this — a formatter that reflows them all on first run is not one
    /// people will use.
    pub line_starts: Vec<usize>,
}

impl Lexed {
    /// 0-based line containing `offset`.
    ///
    /// The `i - 1` cannot underflow: `line_starts` always begins with 0, so an
    /// insertion point of 0 would mean `offset` sorts before 0.
    ///
    /// An offset past the end of the file answers with the last line rather
    /// than failing, which the formatter relies on — it asks about
    /// `span.end`, one past the last byte, all over the place.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    /// Blank lines between the end of `before` and the start of `after`.
    ///
    /// The formatter uses this to decide whether the author separated two
    /// items, without needing to record whitespace itself.
    pub fn blank_lines_between(&self, before: usize, after: usize) -> usize {
        let a = self.line_of(before);
        let b = self.line_of(after);
        b.saturating_sub(a).saturating_sub(1)
    }

    /// Trivia lying inside a span.
    pub fn trivia_within<'a>(&'a self, span: &Span) -> impl Iterator<Item = &'a Trivia> + 'a {
        let (start, end) = (span.start, span.end);
        self.trivia
            .iter()
            .filter(move |t| t.span.start >= start && t.span.end <= end)
    }
}

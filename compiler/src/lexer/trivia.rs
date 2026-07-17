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

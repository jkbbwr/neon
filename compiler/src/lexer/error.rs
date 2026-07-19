//! What the lexer refuses, and how it says so.
//!
//! The kinds are a closed set with one variant per distinguishable mistake,
//! rather than a single "bad token" carrying prose. Two consequences are worth
//! the extra variants: a diagnostics pass can match on a kind and render it
//! however it likes, and the tests assert on kinds instead of on message text,
//! so rewording an error does not break them.
//!
//! Every kind blames a *span in the source*, and for unterminated constructs
//! that span is the opener, not EOF — `report_unclosed` in the lexer works out
//! which opener is actually at fault when several are still on the mode stack.
//! "Unterminated string at end of file" is technically true and useless.
//!
//! Several kinds exist only because their absence was a bug. `UnknownEscape`
//! replaces silently keeping `\q` as a backslash and a `q`, which compiled.
//! `UnexpectedBom` replaces a BOM mid-file reading as a mystery character.
//! `MisplacedUnderscore` and `EmptyIntLiteral` replace numeric literals that
//! quietly parsed as something other than what was written.

use super::token::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    pub fn new(kind: LexErrorKind, span: Span) -> Self {
        LexError { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedChar(char),
    /// A BOM anywhere other than the very start of the file.
    UnexpectedBom,

    IntegerOverflow,
    /// A numeric literal with a digit position left empty: `0x`, `0b` or `0o`
    /// with nothing after the prefix, and also `1e` — a float's exponent reuses
    /// this kind despite the name.
    ///
    /// Not `1.`: a float requires a digit after the dot, so `1.` lexes as the
    /// integer `1` followed by `.` and never reaches here.
    EmptyIntLiteral,
    /// `_` leading or trailing a numeric literal, or doubled.
    MisplacedUnderscore,

    UnterminatedString,
    UnterminatedBlockComment,
    UnterminatedInterp,
    UnterminatedRune,
    /// `''`
    EmptyRune,
    /// `'ab'` — a rune holds exactly one character. Despite the name this is the
    /// too-*many* case; the empty case is `EmptyRune` and a missing closing
    /// quote is `UnterminatedRune`. Raised only when a closing `'` is eventually
    /// found on the same line, so the three stay distinguishable.
    OvershortRune,

    /// `\q` — the previous implementation silently kept unknown escapes as a
    /// literal backslash plus the character, so a typo compiled.
    UnknownEscape(char),
    /// `\xZZ` or `\x4` — needs exactly two hex digits.
    BadHexEscape,
    /// `\u{...}` with no closing brace, no digits, or a value that is not a
    /// scalar value.
    BadUnicodeEscape,

    /// A `}` in code position that closes nothing.
    ///
    /// Never constructed: the lexer does not track brace nesting outside an
    /// interpolation hole, so a stray `}` becomes a `Token::RBrace` and the
    /// parser is what complains. The variant and its message are kept for when
    /// that changes; until then nothing reaches it.
    UnmatchedCloseBrace,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LexErrorKind::*;
        match &self.kind {
            UnexpectedChar(c) => write!(f, "unexpected character `{c}`"),
            UnexpectedBom => write!(f, "byte order mark is only allowed at the start of a file"),

            IntegerOverflow => write!(f, "integer literal does not fit in 64 bits"),
            EmptyIntLiteral => write!(f, "integer literal has no digits"),
            MisplacedUnderscore => write!(f, "`_` must separate digits, not lead or trail them"),

            UnterminatedString => write!(f, "unterminated string literal"),
            UnterminatedBlockComment => write!(f, "unterminated block comment"),
            UnterminatedInterp => write!(f, "unterminated `#{{` interpolation"),
            UnterminatedRune => write!(f, "unterminated rune literal"),
            EmptyRune => write!(f, "empty rune literal"),
            OvershortRune => write!(f, "rune literal holds more than one character"),

            UnknownEscape(c) => write!(f, "unknown escape `\\{c}`"),
            BadHexEscape => write!(f, "`\\x` needs exactly two hex digits"),
            BadUnicodeEscape => write!(f, "`\\u{{...}}` needs 1-6 hex digits naming a character"),

            UnmatchedCloseBrace => write!(f, "unmatched `}}`"),
        }
    }
}

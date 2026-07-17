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
    /// `0x`, `0b`, `0o` with no digits after it.
    EmptyIntLiteral,
    /// `_` leading or trailing a numeric literal, or doubled.
    MisplacedUnderscore,

    UnterminatedString,
    UnterminatedBlockComment,
    UnterminatedInterp,
    UnterminatedRune,
    /// `''`
    EmptyRune,
    /// `'ab'` — a rune holds exactly one character.
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

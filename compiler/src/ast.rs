//! The syntax tree.
//!
//! Spans are carried on every node the parser can point a diagnostic at.

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    Fn(FnDecl),
    /// `test "name" { ... }` / `bench "name" { ... }`.
    TestBlock(TestBlock),
    /// A declaration that failed to parse. Recovery produces one of these so a
    /// later pass can still see the shape of the file, and so one bad
    /// declaration does not discard the rest.
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    /// `throws E` — written before `->`.
    pub throws: Option<TypeSpec>,
    pub ret: Option<TypeSpec>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeSpec,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestBlock {
    pub kind: TestKind,
    pub name: String,
    pub body: Block,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestKind {
    Test,
    Bench,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeSpec {
    pub kind: TypeSpecKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpecKind {
    /// `i64`, `str`, `User`, `std::io::Reader`
    Named { path: Vec<String>, args: Vec<TypeSpec> },
    /// `:ok` used as a type — the singleton inhabited by that atom.
    Atom(String),
    Null,
    /// The one legitimate erasure boundary.
    Any,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The block's value: a trailing expression with no semicolon.
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let { name: String, ty: Option<TypeSpec>, value: Expr },
    Expr(Expr),
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// The magnitude only; `-` is a unary operator applied to it. Keeping the
    /// sign out of the literal is what makes `-9223372036854775808` expressible.
    Int(u64),
    Float(String),
    Str(Vec<StrPart>),
    Rune(char),
    Atom(String),
    Bool(bool),
    Null,
    Path(Vec<String>),
    Unary { op: UnOp, rhs: Box<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Block(Block),
    Error,
}

/// A string is a sequence of literal text and interpolated expressions. The
/// lexer emits these as a flat token run; this is the reassembled tree.
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Text(String),
    Interp(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Bnot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Band,
    Bor,
    Bxor,
    Bsl,
    Bsr,
    Orelse,
    Pipe,
}

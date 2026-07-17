mod error;

#[cfg(test)]
mod tests;

pub use error::{Expected, ParseError, ParseErrorKind, Span};

use crate::ast::*;
use crate::lexer::{Spanned, Token};
use chumsky::input::{Input, ValueInput};
use chumsky::prelude::*;

type Extra = extra::Err<ParseError>;

/// Adapts the lexer's output to chumsky. `.boxed()` is used liberally below to
/// keep the type of each sub-parser opaque: without it the combinator types
/// nest into one another and compile time grows superlinearly.
pub fn parse(tokens: &[Spanned], eoi: usize) -> (Option<Module>, Vec<ParseError>) {
    let owned: Vec<(Token, Span)> = tokens
        .iter()
        .map(|s| (s.token.clone(), s.span.clone()))
        .collect();
    let input = owned.as_slice().map(eoi..eoi, |(t, s)| (t, s));
    // Bound rather than inlined: as a temporary the parser outlives `owned`,
    // which it borrows, and is dropped after it.
    let parser = module();
    parser.parse(input).into_output_errors()
}

fn module<'t, I>() -> impl Parser<'t, I, Module, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    decl()
        .repeated()
        .collect::<Vec<_>>()
        .map(|decls| Module { decls })
        .then_ignore(end())
        .boxed()
}

fn decl<'t, I>() -> impl Parser<'t, I, Decl, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    let fn_decl = just(Token::Fn)
        .ignore_then(ident())
        .then(
            param()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LParen), just(Token::RParen)),
        )
        // `throws E` comes before `->`, matching the signature order.
        .then(just(Token::Throws).ignore_then(type_spec()).or_not())
        .then(just(Token::Arrow).ignore_then(type_spec()).or_not())
        .then(block())
        .map(|((((name, params), throws), ret), body)| {
            DeclKind::Fn(FnDecl { name, params, throws, ret, body })
        })
        .boxed();

    let test_block = choice((
        just(Token::Test).to(TestKind::Test),
        just(Token::Bench).to(TestKind::Bench),
    ))
    .then(plain_str())
    .then(block())
    .map(|((kind, name), body)| DeclKind::TestBlock(TestBlock { kind, name, body }))
    .boxed();

    // `enum` lexes as an ordinary identifier, so catch it here and say what to
    // do instead. Without this the user gets a cascade about an unexpected
    // identifier, which explains nothing.
    let enum_decl = ident_named("enum")
        .then(ident())
        .then(nested_braces())
        .validate(|_, e, emitter| {
            emitter.emit(ParseError::new(e.span(), ParseErrorKind::EnumDeclaration));
            DeclKind::Error
        })
        .boxed();

    // On a bad declaration, swallow it and resume at the next one. One broken
    // decl must not discard the rest of the file: reporting every error in a
    // pass is the point.
    //
    // The leading `any()` is load-bearing. Recovery restarts from where the
    // declaration *began* — which is itself a decl-start token — so a strategy
    // that skips "until a decl start" would match immediately, retry at the same
    // token, fail identically, and get abandoned. Consuming one token first
    // guarantees progress.
    let recovery = any()
        .then(none_of(DECL_START).repeated())
        .map_with(|_, e| Decl { kind: DeclKind::Error, span: e.span() })
        .boxed();

    choice((fn_decl, test_block, enum_decl))
        .map_with(|kind, e| Decl { kind, span: e.span() })
        .recover_with(via_parser(recovery))
        .boxed()
}

/// Tokens that can begin a declaration; recovery resumes at one of these.
const DECL_START: [Token; 10] = [
    Token::Fn,
    Token::Test,
    Token::Bench,
    Token::Record,
    Token::Type,
    Token::Mu,
    Token::Newtype,
    Token::Protocol,
    Token::Impl,
    Token::Use,
];

fn param<'t, I>() -> impl Parser<'t, I, Param, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    ident()
        .then_ignore(just(Token::Colon))
        .then(type_spec())
        .map_with(|(name, ty), e| Param { name, ty, span: e.span() })
        .boxed()
}

fn type_spec<'t, I>() -> impl Parser<'t, I, TypeSpec, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    recursive(|ty| {
        let path = ident()
            .separated_by(just(Token::ColonColon))
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                ty.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .or_not()
                    .map(Option::unwrap_or_default),
            )
            .map(|(path, args)| TypeSpecKind::Named { path, args });

        let atom = select! { Token::Atom(a) => TypeSpecKind::Atom(a) };
        let null = just(Token::Null).to(TypeSpecKind::Null);
        let any = ident_named("any").to(TypeSpecKind::Any);

        choice((null, any, atom, path))
            .map_with(|kind, e| TypeSpec { kind, span: e.span() })
            .labelled("a type")
            .boxed()
    })
}

fn block<'t, I>() -> impl Parser<'t, I, Block, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    // Placeholder: an empty body, enough for the vertical slice. Statements and
    // expressions land next.
    just(Token::LBrace)
        .ignore_then(just(Token::RBrace))
        .map_with(|_, e| Block { stmts: Vec::new(), tail: None, span: e.span() })
        .boxed()
}

// ---- leaves ----

fn ident<'t, I>() -> impl Parser<'t, I, String, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    select! { Token::Ident(name) => name }.labelled("an identifier")
}

/// A specific identifier. `enum` and `any` are not keywords, so they arrive as
/// idents and are matched by text.
fn ident_named<'t, I>(want: &'static str) -> impl Parser<'t, I, (), Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    select! { Token::Ident(name) if name == want => () }
}

/// A string with no interpolation. Test names are the only place this is
/// currently required.
fn plain_str<'t, I>() -> impl Parser<'t, I, String, Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    just(Token::StrStart)
        .ignore_then(select! { Token::StrText(s) => s }.or_not())
        .then_ignore(just(Token::StrEnd))
        .map(Option::unwrap_or_default)
        .labelled("a string")
        .boxed()
}

/// A balanced `{ ... }` run, consumed and discarded. Used only to swallow the
/// body of a construct we are rejecting outright.
fn nested_braces<'t, I>() -> impl Parser<'t, I, (), Extra> + Clone
where
    I: ValueInput<'t, Token = Token, Span = Span>,
{
    recursive(|inner| {
        choice((
            inner.clone().delimited_by(just(Token::LBrace), just(Token::RBrace)),
            none_of([Token::LBrace, Token::RBrace]).ignored(),
        ))
        .repeated()
        .ignored()
    })
    .delimited_by(just(Token::LBrace), just(Token::RBrace))
    .boxed()
}

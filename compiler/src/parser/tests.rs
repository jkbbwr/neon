use super::*;
use crate::lexer;

fn parse_src(src: &str) -> (Option<Module>, Vec<ParseError>) {
    let tokens = lexer::lex(src).expect("lexes");
    parse(&tokens, src.len())
}

fn ok(src: &str) -> Module {
    let (m, errs) = parse_src(src);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    m.expect("parses")
}

fn errs(src: &str) -> Vec<ParseError> {
    let (_, errs) = parse_src(src);
    assert!(!errs.is_empty(), "expected errors, got none");
    errs
}

#[test]
fn the_vertical_slice() {
    let m = ok("fn main() {}");
    assert_eq!(m.decls.len(), 1);
    match &m.decls[0].kind {
        DeclKind::Fn(f) => {
            assert_eq!(f.name, "main");
            assert!(f.params.is_empty());
            assert!(f.ret.is_none());
            assert!(f.throws.is_none());
        }
        other => panic!("expected a fn, got {other:?}"),
    }
}

#[test]
fn throws_comes_before_the_return_type() {
    let m = ok("fn get(i: i64) throws IndexError -> str {}");
    match &m.decls[0].kind {
        DeclKind::Fn(f) => {
            assert_eq!(f.params.len(), 1);
            assert_eq!(f.params[0].name, "i");
            assert!(f.throws.is_some(), "throws should parse");
            assert!(f.ret.is_some(), "return type should parse");
        }
        other => panic!("expected a fn, got {other:?}"),
    }
}

#[test]
fn generic_and_qualified_types() {
    let m = ok("fn f(a: List[i64], b: std::io::Reader, c: Map[str, List[i64]]) {}");
    match &m.decls[0].kind {
        DeclKind::Fn(f) => {
            assert_eq!(f.params.len(), 3);
            match &f.params[1].ty.kind {
                TypeSpecKind::Named { path, .. } => assert_eq!(path, &["std", "io", "Reader"]),
                other => panic!("expected a path, got {other:?}"),
            }
        }
        other => panic!("expected a fn, got {other:?}"),
    }
}

#[test]
fn atom_and_null_and_any_are_types() {
    let m = ok("fn f(a: :ok, b: null, c: any) {}");
    match &m.decls[0].kind {
        DeclKind::Fn(f) => {
            assert_eq!(f.params[0].ty.kind, TypeSpecKind::Atom("ok".into()));
            assert_eq!(f.params[1].ty.kind, TypeSpecKind::Null);
            assert_eq!(f.params[2].ty.kind, TypeSpecKind::Any);
        }
        other => panic!("expected a fn, got {other:?}"),
    }
}

#[test]
fn test_and_bench_blocks() {
    let m = ok(r#"test "adds two" {} bench "push 1k" {}"#);
    assert_eq!(m.decls.len(), 2);
    match &m.decls[0].kind {
        DeclKind::TestBlock(t) => {
            assert_eq!(t.kind, TestKind::Test);
            assert_eq!(t.name, "adds two");
        }
        other => panic!("expected a test, got {other:?}"),
    }
    match &m.decls[1].kind {
        DeclKind::TestBlock(t) => assert_eq!(t.kind, TestKind::Bench),
        other => panic!("expected a bench, got {other:?}"),
    }
}

#[test]
fn enum_gets_a_real_diagnostic() {
    // `enum` lexes as an ordinary identifier. Without a dedicated rule the user
    // gets a cascade about an unexpected identifier, which explains nothing.
    let e = errs("enum Color { Red, Green }");
    assert!(
        e.iter().any(|e| e.kind == ParseErrorKind::EnumDeclaration),
        "expected the enum diagnostic, got {e:?}"
    );
    let msg = e
        .iter()
        .find(|e| e.kind == ParseErrorKind::EnumDeclaration)
        .expect("the enum error")
        .to_string();
    assert!(msg.contains("record Red"), "should say what to do instead: {msg}");
}

#[test]
fn recovery_keeps_going_after_a_bad_decl() {
    // One broken declaration must not discard the rest of the file: a pass
    // should report every error it can find, not just the first.
    let (m, errs) = parse_src("fn broken( {} fn good() {} fn also_good() {}");
    assert!(!errs.is_empty(), "the broken decl should error");
    let m = m.expect("recovery still yields a module");
    let names: Vec<_> = m
        .decls
        .iter()
        .filter_map(|d| match &d.kind {
            DeclKind::Fn(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        names.contains(&"good") && names.contains(&"also_good"),
        "later decls should still parse, got {names:?}"
    );
}

#[test]
fn every_error_is_reported_not_just_the_first() {
    let e = errs("fn a( {} fn b( {} fn c() {}");
    assert!(e.len() >= 2, "expected several errors, got {}: {e:?}", e.len());
}

#[test]
fn errors_are_concrete_and_carry_a_span() {
    let e = errs("fn 123() {}");
    match &e[0].kind {
        ParseErrorKind::Expected { expected, found } => {
            // `.labelled("an identifier")` should replace the raw token
            // alternatives, not pile on top of them.
            assert!(
                expected.contains(&Expected::Label("an identifier")),
                "expected the label, got {expected:?}"
            );
            assert_eq!(found, &Some(Token::Int(123)));
        }
        other => panic!("expected an Expected error, got {other:?}"),
    }
    assert!(e[0].span.start < e[0].span.end, "span should cover the token");
}

#[test]
fn error_message_names_the_construct() {
    let e = errs("fn f(x: ) {}");
    let msg = e[0].to_string();
    assert!(msg.contains("a type"), "should name the construct: {msg}");
}

#[test]
fn empty_input_is_an_empty_module() {
    let m = ok("");
    assert!(m.decls.is_empty());
}

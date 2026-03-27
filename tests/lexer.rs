use spin_up::lexer::{lex, Token};

#[test]
fn test_lex_empty_input() {
    let tokens = lex("").unwrap();
    assert!(tokens.is_empty());
}

#[test]
fn test_lex_keywords() {
    let tokens = lex("import resource supplies if then else fn map filter").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::Import,
            &Token::Resource,
            &Token::Supplies,
            &Token::If,
            &Token::Then,
            &Token::Else,
            &Token::Fn,
            &Token::Map,
            &Token::Filter,
        ]
    );
}

#[test]
fn test_lex_identifier() {
    let tokens = lex("postgres").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, Token::Ident("postgres".to_string()));
}

#[test]
fn test_lex_punctuation() {
    let tokens = lex("{ } ( ) [ ] , : :: . = >= <= == !=").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::LBrace,
            &Token::RBrace,
            &Token::LParen,
            &Token::RParen,
            &Token::LBracket,
            &Token::RBracket,
            &Token::Comma,
            &Token::Colon,
            &Token::PathSep,
            &Token::Dot,
            &Token::Eq,
            &Token::Gte,
            &Token::Lte,
            &Token::EqEq,
            &Token::BangEq,
        ]
    );
}

#[test]
fn test_lex_number_literal() {
    let tokens = lex("42 3.14").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("42".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("3.14".to_string()));
}

#[test]
fn test_lex_string_literal() {
    let tokens = lex(r#""hello world""#).unwrap();
    assert_eq!(
        tokens[0].kind,
        Token::StringLit("hello world".to_string())
    );
}

#[test]
fn test_lex_comment_ignored() {
    let tokens = lex("import // this is a comment\nresource").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(kinds, &[&Token::Import, &Token::Resource]);
}

#[test]
fn test_token_spans() {
    let tokens = lex("import postgres").unwrap();
    assert_eq!(tokens[0].span, 0..6);
    assert_eq!(tokens[1].span, 7..15);
}

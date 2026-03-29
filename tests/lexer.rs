use spin_up::lexer::{Token, lex};

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
    assert_eq!(tokens[0].kind, Token::StringLit("hello world".to_string()));
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

#[test]
fn test_lex_primitive_type_keywords() {
    let input = "bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 str";
    let tokens = lex(input).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::Bool,
            &Token::U8,
            &Token::U16,
            &Token::U32,
            &Token::U64,
            &Token::U128,
            &Token::I8,
            &Token::I16,
            &Token::I32,
            &Token::I64,
            &Token::I128,
            &Token::F32,
            &Token::F64,
            &Token::Str,
        ]
    );
}

#[test]
fn test_lex_type_keyword() {
    let tokens = lex("type").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(kinds, &[&Token::Type]);
}

#[test]
fn test_lex_hex_literal() {
    let tokens = lex("0xff 0xFF 0x1A2B").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0xff".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0xFF".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("0x1A2B".to_string()));
}

#[test]
fn test_lex_binary_literal() {
    let tokens = lex("0b1010 0b0000_1111").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0b1010".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0b0000_1111".to_string()));
}

#[test]
fn test_lex_octal_literal() {
    let tokens = lex("0o77 0o755").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0o77".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0o755".to_string()));
}

#[test]
fn test_lex_underscore_separators() {
    let tokens = lex("1_000_000 0xff_ff").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("1_000_000".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0xff_ff".to_string()));
}

#[test]
fn test_lex_type_suffix() {
    let tokens = lex("42u32 3.14f64 0xffu8").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("42u32".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("3.14f64".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("0xffu8".to_string()));
}

#[test]
fn test_lex_float_exponent() {
    let tokens = lex("1.0e10 2.5E-3 1e5").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("1.0e10".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("2.5E-3".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("1e5".to_string()));
}

#[test]
fn test_lex_attribute() {
    let tokens = lex("#[lang-item]").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::HashBracket,
            &Token::Ident("lang-item".to_string()),
            &Token::RBracket
        ]
    );
}

#[test]
fn test_lex_attribute_before_type() {
    let tokens = lex("#[lang-item]\ntype IpAddr {}").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::HashBracket,
            &Token::Ident("lang-item".to_string()),
            &Token::RBracket,
            &Token::Type,
            &Token::Ident("IpAddr".to_string()),
            &Token::LBrace,
            &Token::RBrace,
        ]
    );
}

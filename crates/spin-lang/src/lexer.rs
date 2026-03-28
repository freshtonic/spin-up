use std::ops::Range;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Import,
    Resource,
    Supplies,
    If,
    Then,
    Else,
    Fn,
    Map,
    Filter,
    Self_,

    // Primitive type keywords
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Str,

    // Type definition keywords
    Record,
    Choice,

    // Literals
    Ident(String),
    Number(String),
    StringLit(String),

    // Attribute syntax
    HashBracket, // #[

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    PathSep, // ::
    Dot,
    Eq,
    EqEq,
    BangEq,
    Gte,
    Lte,
    Gt,
    Lt,
    Pipe,
    Arrow,     // ->
    Semicolon, // ;
}

#[derive(Debug, Clone)]
pub struct Spanned {
    pub kind: Token,
    pub span: Range<usize>,
}

#[derive(Debug, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}' at position {pos}")]
    UnexpectedChar { ch: char, pos: usize },
    #[error("unterminated string literal starting at position {pos}")]
    UnterminatedString { pos: usize },
}

pub fn lex(input: &str) -> Result<Vec<Spanned>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(pos, ch)) = chars.peek() {
        match ch {
            // Whitespace
            c if c.is_ascii_whitespace() => {
                chars.next();
            }
            // Line comments
            '/' if matches!(chars.clone().nth(1), Some((_, '/'))) => {
                chars.next();
                chars.next();
                while let Some(&(_, c)) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            // String literals
            '"' => {
                chars.next();
                let start = pos;
                let mut value = String::new();
                loop {
                    match chars.next() {
                        Some((_, '"')) => break,
                        Some((_, c)) => value.push(c),
                        None => return Err(LexError::UnterminatedString { pos: start }),
                    }
                }
                let end = start + value.len() + 2; // include quotes
                tokens.push(Spanned {
                    kind: Token::StringLit(value),
                    span: start..end,
                });
            }
            // Numbers
            c if c.is_ascii_digit() => {
                let start = pos;
                let mut value = String::new();

                // Check for 0x, 0b, 0o prefixes
                if c == '0'
                    && let Some((_, prefix)) = chars.clone().nth(1)
                    && (prefix == 'x' || prefix == 'b' || prefix == 'o')
                {
                    // Consume '0' and prefix
                    value.push(c);
                    chars.next();
                    value.push(prefix);
                    chars.next();

                    // Consume digits valid for the base + underscores
                    while let Some(&(_, d)) = chars.peek() {
                        if d.is_ascii_alphanumeric() || d == '_' {
                            value.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let end = start + value.len();
                    tokens.push(Spanned {
                        kind: Token::Number(value),
                        span: start..end,
                    });
                    continue;
                }

                // Decimal number: digits and underscores
                while let Some(&(_, d)) = chars.peek() {
                    if d.is_ascii_digit() || d == '_' {
                        value.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Optional decimal point followed by digits
                if let Some(&(_, '.')) = chars.peek() {
                    value.push('.');
                    chars.next();
                    while let Some(&(_, d)) = chars.peek() {
                        if d.is_ascii_digit() || d == '_' {
                            value.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }

                // Optional exponent e/E, optionally followed by +/-
                if let Some(&(_, e)) = chars.peek()
                    && (e == 'e' || e == 'E')
                {
                    value.push(e);
                    chars.next();
                    // Optional sign
                    if let Some(&(_, sign)) = chars.peek()
                        && (sign == '+' || sign == '-')
                    {
                        value.push(sign);
                        chars.next();
                    }
                    // Exponent digits
                    while let Some(&(_, d)) = chars.peek() {
                        if d.is_ascii_digit() || d == '_' {
                            value.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }

                // Optional type suffix (trailing alphabetic + digits, e.g. u32, f64, i8)
                if let Some(&(_, s)) = chars.peek()
                    && s.is_ascii_alphabetic()
                {
                    value.push(s);
                    chars.next();
                    while let Some(&(_, d)) = chars.peek() {
                        if d.is_ascii_alphanumeric() {
                            value.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }

                let end = start + value.len();
                tokens.push(Spanned {
                    kind: Token::Number(value),
                    span: start..end,
                });
            }
            // Identifiers and keywords
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = pos;
                let mut value = String::new();
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        value.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let end = start + value.len();
                let kind = match value.as_str() {
                    "import" => Token::Import,
                    "resource" => Token::Resource,
                    "supplies" => Token::Supplies,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "fn" => Token::Fn,
                    "map" => Token::Map,
                    "filter" => Token::Filter,
                    "Self" => Token::Self_,
                    "bool" => Token::Bool,
                    "u8" => Token::U8,
                    "u16" => Token::U16,
                    "u32" => Token::U32,
                    "u64" => Token::U64,
                    "u128" => Token::U128,
                    "i8" => Token::I8,
                    "i16" => Token::I16,
                    "i32" => Token::I32,
                    "i64" => Token::I64,
                    "i128" => Token::I128,
                    "f32" => Token::F32,
                    "f64" => Token::F64,
                    "str" => Token::Str,
                    "record" => Token::Record,
                    "choice" => Token::Choice,
                    _ => Token::Ident(value),
                };
                tokens.push(Spanned {
                    kind,
                    span: start..end,
                });
            }
            // Two-character operators
            ':' if matches!(chars.clone().nth(1), Some((_, ':'))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::PathSep,
                    span: pos..pos + 2,
                });
            }
            '=' if matches!(chars.clone().nth(1), Some((_, '='))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::EqEq,
                    span: pos..pos + 2,
                });
            }
            '!' if matches!(chars.clone().nth(1), Some((_, '='))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::BangEq,
                    span: pos..pos + 2,
                });
            }
            '>' if matches!(chars.clone().nth(1), Some((_, '='))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Gte,
                    span: pos..pos + 2,
                });
            }
            '<' if matches!(chars.clone().nth(1), Some((_, '='))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Lte,
                    span: pos..pos + 2,
                });
            }
            '-' if matches!(chars.clone().nth(1), Some((_, '>'))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Arrow,
                    span: pos..pos + 2,
                });
            }
            // Single-character operators
            '{' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::LBrace,
                    span: pos..pos + 1,
                });
            }
            '}' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::RBrace,
                    span: pos..pos + 1,
                });
            }
            '(' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::LParen,
                    span: pos..pos + 1,
                });
            }
            ')' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::RParen,
                    span: pos..pos + 1,
                });
            }
            '[' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::LBracket,
                    span: pos..pos + 1,
                });
            }
            ']' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::RBracket,
                    span: pos..pos + 1,
                });
            }
            ',' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Comma,
                    span: pos..pos + 1,
                });
            }
            ':' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Colon,
                    span: pos..pos + 1,
                });
            }
            '.' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Dot,
                    span: pos..pos + 1,
                });
            }
            '=' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Eq,
                    span: pos..pos + 1,
                });
            }
            '>' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Gt,
                    span: pos..pos + 1,
                });
            }
            '<' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Lt,
                    span: pos..pos + 1,
                });
            }
            '|' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Pipe,
                    span: pos..pos + 1,
                });
            }
            ';' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Semicolon,
                    span: pos..pos + 1,
                });
            }
            '#' if matches!(chars.clone().nth(1), Some((_, '['))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::HashBracket,
                    span: pos..pos + 2,
                });
            }
            _ => return Err(LexError::UnexpectedChar { ch, pos }),
        }
    }

    Ok(tokens)
}

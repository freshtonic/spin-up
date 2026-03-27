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

    // Literals
    Ident(String),
    Number(String),
    StringLit(String),

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
    Arrow, // ->
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
                while let Some(&(_, c)) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        value.push(c);
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
            _ => return Err(LexError::UnexpectedChar { ch, pos }),
        }
    }

    Ok(tokens)
}

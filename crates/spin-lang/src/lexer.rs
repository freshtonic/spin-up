use std::ops::Range;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Import,
    If,
    Then,
    Else,
    Fn,
    Map,
    Filter,
    Self_,
    Interface,
    Impl,
    For,
    Let,
    It,

    // Primitive type keywords
    Bool,
    Number_, // "number" keyword (Number_ to avoid conflict with Number literal)
    String_, // "string" keyword (String_ to avoid conflict with StringLit)

    // Collection keywords
    Set, // "Set"

    // Built-in function keywords
    Keep,
    Drop_, // "drop" (Drop_ to avoid Rust conflict)
    Count,
    Sum,
    Mean,
    Median,
    Min,
    Max,

    // Regex literal
    RegexLit(String),

    // Regex match operator
    RegexMatch, // =~

    // Arithmetic operators
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

    // Type definition keywords
    Type,

    // Literals
    Ident(String),
    Number(String),
    StringLit(String),

    // Hash-prefixed delimiters
    HashBracket, // #[
    HashBrace,   // #{
    HashParen,   // #(

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
    And,       // &&
    Or,        // ||
    Bang,      // !
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
                // Regex literal: r"pattern"
                if value == "r" && matches!(chars.peek(), Some(&(_, '"'))) {
                    chars.next(); // consume opening '"'
                    let mut pattern = String::new();
                    loop {
                        match chars.next() {
                            Some((_, '"')) => break,
                            Some((_, c)) => pattern.push(c),
                            None => return Err(LexError::UnterminatedString { pos: start }),
                        }
                    }
                    let end = start + 2 + pattern.len() + 1; // r" + pattern + "
                    tokens.push(Spanned {
                        kind: Token::RegexLit(pattern),
                        span: start..end,
                    });
                    continue;
                }

                let end = start + value.len();
                let kind = match value.as_str() {
                    "import" => Token::Import,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "fn" => Token::Fn,
                    "map" => Token::Map,
                    "filter" => Token::Filter,
                    "Self" => Token::Self_,
                    "bool" => Token::Bool,
                    "number" => Token::Number_,
                    "string" => Token::String_,
                    "Set" => Token::Set,
                    "keep" => Token::Keep,
                    "drop" => Token::Drop_,
                    "count" => Token::Count,
                    "sum" => Token::Sum,
                    "mean" => Token::Mean,
                    "median" => Token::Median,
                    "min" => Token::Min,
                    "max" => Token::Max,
                    "type" => Token::Type,
                    "interface" => Token::Interface,
                    "impl" => Token::Impl,
                    "for" => Token::For,
                    "let" => Token::Let,
                    "it" => Token::It,
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
            '=' if matches!(chars.clone().nth(1), Some((_, '~'))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::RegexMatch,
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
            '!' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Bang,
                    span: pos..pos + 1,
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
            '&' if matches!(chars.clone().nth(1), Some((_, '&'))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::And,
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
            '-' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Minus,
                    span: pos..pos + 1,
                });
            }
            '+' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Plus,
                    span: pos..pos + 1,
                });
            }
            '*' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Star,
                    span: pos..pos + 1,
                });
            }
            '/' => {
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Slash,
                    span: pos..pos + 1,
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
            '|' if matches!(chars.clone().nth(1), Some((_, '|'))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::Or,
                    span: pos..pos + 2,
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
            '#' => match chars.clone().nth(1) {
                Some((_, '[')) => {
                    chars.next();
                    chars.next();
                    tokens.push(Spanned {
                        kind: Token::HashBracket,
                        span: pos..pos + 2,
                    });
                }
                Some((_, '{')) => {
                    chars.next();
                    chars.next();
                    tokens.push(Spanned {
                        kind: Token::HashBrace,
                        span: pos..pos + 2,
                    });
                }
                Some((_, '(')) => {
                    chars.next();
                    chars.next();
                    tokens.push(Spanned {
                        kind: Token::HashParen,
                        span: pos..pos + 2,
                    });
                }
                _ => return Err(LexError::UnexpectedChar { ch, pos }),
            },
            _ => return Err(LexError::UnexpectedChar { ch, pos }),
        }
    }

    Ok(tokens)
}

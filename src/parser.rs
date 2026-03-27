use crate::ast::{Import, Item, Module};
use crate::lexer::{self, Spanned, Token};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Lex(#[from] lexer::LexError),
    #[error("expected {expected} at position {pos}, found {found}")]
    Expected {
        expected: String,
        found: String,
        pos: usize,
    },
    #[error("unexpected end of input, expected {expected}")]
    UnexpectedEof { expected: String },
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Spanned> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Spanned> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn expect_ident(&mut self) -> Result<(String, std::ops::Range<usize>), ParseError> {
        match self.advance() {
            Some(Spanned {
                kind: Token::Ident(name),
                span,
            }) => Ok((name.clone(), span.clone())),
            Some(Spanned { kind, span }) => Err(ParseError::Expected {
                expected: "identifier".to_string(),
                found: format!("{kind:?}"),
                pos: span.start,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "identifier".to_string(),
            }),
        }
    }

    fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while self.peek().is_some() {
            match &self.peek().unwrap().kind {
                Token::Import => {
                    imports.push(self.parse_import()?);
                }
                Token::Resource => {
                    items.push(Item::ResourceDef(self.parse_resource_def()?));
                }
                other => {
                    let span = &self.peek().unwrap().span;
                    return Err(ParseError::Expected {
                        expected: "import or resource".to_string(),
                        found: format!("{other:?}"),
                        pos: span.start,
                    });
                }
            }
        }

        Ok(Module { imports, items })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'import'
        let (module_name, name_span) = self.expect_ident()?;
        Ok(Import {
            module_name,
            span: start..name_span.end,
        })
    }

    fn parse_resource_def(&mut self) -> Result<crate::ast::ResourceDef, ParseError> {
        // Placeholder -- implemented in Task 9
        let span = &self.advance().unwrap().span;
        Err(ParseError::Expected {
            expected: "resource parsing not yet implemented".to_string(),
            found: "resource".to_string(),
            pos: span.start,
        })
    }
}

pub fn parse(input: &str) -> Result<Module, ParseError> {
    let tokens = lexer::lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_module()
}

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
                Token::Supplies => {
                    items.push(Item::SuppliesDef(self.parse_supplies_def()?));
                }
                other => {
                    let span = &self.peek().unwrap().span;
                    return Err(ParseError::Expected {
                        expected: "import, resource, or supplies".to_string(),
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
        let start = self.advance().unwrap().span.start; // consume 'resource'
        let (name, _) = self.expect_ident()?;
        self.expect_token(Token::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            fields.push(self.parse_field()?);
            // Optional trailing comma
            if self.check(&Token::Comma) {
                self.advance();
            }
        }

        let end = self.expect_token(Token::RBrace)?;

        Ok(crate::ast::ResourceDef {
            name,
            fields,
            span: start..end.end,
        })
    }

    fn parse_field(&mut self) -> Result<crate::ast::Field, ParseError> {
        let (name, name_span) = self.expect_ident()?;
        self.expect_token(Token::Colon)?;
        let ty = self.parse_type_expr()?;
        let end = self.previous_span_end();

        Ok(crate::ast::Field {
            name,
            ty,
            span: name_span.start..end,
        })
    }

    fn parse_type_expr(&mut self) -> Result<crate::ast::TypeExpr, ParseError> {
        // Self::Name
        if self.check(&Token::Self_) {
            self.advance();
            self.expect_token(Token::PathSep)?;
            let (name, _) = self.expect_ident()?;
            return Ok(crate::ast::TypeExpr::SelfPath(name));
        }

        let (name, _) = self.expect_ident()?;

        // module::Type
        if self.check(&Token::PathSep) {
            self.advance();
            let (type_name, _) = self.expect_ident()?;
            return Ok(crate::ast::TypeExpr::Path {
                module: name,
                name: type_name,
            });
        }

        // Type<Args>
        if self.check(&Token::Lt) {
            self.advance();
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_expr()?);
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_token(Token::Gt)?;
            return Ok(crate::ast::TypeExpr::Generic { name, args });
        }

        Ok(crate::ast::TypeExpr::Named(name))
    }

    fn parse_supplies_def(&mut self) -> Result<crate::ast::SuppliesDef, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'supplies'
        let (module, _) = self.expect_ident()?;
        self.expect_token(Token::PathSep)?;
        let (name, _) = self.expect_ident()?;
        let resource_path = crate::ast::QualifiedPath { module, name };

        self.expect_token(Token::LBrace)?;

        let mut field_assignments = Vec::new();
        while !self.check(&Token::RBrace) {
            field_assignments.push(self.parse_field_assignment()?);
            if self.check(&Token::Comma) {
                self.advance();
            }
        }

        let end = self.expect_token(Token::RBrace)?;

        Ok(crate::ast::SuppliesDef {
            resource_path,
            field_assignments,
            span: start..end.end,
        })
    }

    fn parse_field_assignment(&mut self) -> Result<crate::ast::FieldAssignment, ParseError> {
        let (name, name_span) = self.expect_ident()?;
        self.expect_token(Token::Eq)?;
        let value = self.parse_expr()?;
        let end = self.previous_span_end();

        Ok(crate::ast::FieldAssignment {
            name,
            value,
            span: name_span.start..end,
        })
    }

    fn parse_expr(&mut self) -> Result<crate::ast::Expr, ParseError> {
        match self.advance() {
            Some(Spanned {
                kind: Token::StringLit(s),
                ..
            }) => Ok(crate::ast::Expr::StringLit(s.clone())),
            Some(Spanned {
                kind: Token::Number(n),
                ..
            }) => Ok(crate::ast::Expr::Number(n.clone())),
            Some(Spanned {
                kind: Token::Ident(name),
                ..
            }) => match name.as_str() {
                "true" => Ok(crate::ast::Expr::Bool(true)),
                "false" => Ok(crate::ast::Expr::Bool(false)),
                _ => Ok(crate::ast::Expr::Ident(name.clone())),
            },
            Some(Spanned { kind, span }) => Err(ParseError::Expected {
                expected: "expression".to_string(),
                found: format!("{kind:?}"),
                pos: span.start,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "expression".to_string(),
            }),
        }
    }

    fn check(&self, expected: &Token) -> bool {
        matches!(self.peek(), Some(t) if std::mem::discriminant(&t.kind) == std::mem::discriminant(expected))
    }

    fn expect_token(&mut self, expected: Token) -> Result<std::ops::Range<usize>, ParseError> {
        match self.advance() {
            Some(Spanned { kind, span })
                if std::mem::discriminant(kind) == std::mem::discriminant(&expected) =>
            {
                Ok(span.clone())
            }
            Some(Spanned { kind, span }) => Err(ParseError::Expected {
                expected: format!("{expected:?}"),
                found: format!("{kind:?}"),
                pos: span.start,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: format!("{expected:?}"),
            }),
        }
    }

    fn previous_span_end(&self) -> usize {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span.end
        } else {
            0
        }
    }
}

pub fn parse(input: &str) -> Result<Module, ParseError> {
    let tokens = lexer::lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_module()
}

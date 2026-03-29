use crate::ast::{Attribute, Import, Item, Module, Variant};
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
            let attributes = self.parse_attributes()?;

            match &self.peek().unwrap().kind {
                Token::Import => {
                    imports.push(self.parse_import()?);
                }
                Token::Type => {
                    items.push(self.parse_type_def(attributes)?);
                }
                Token::Supplies => {
                    items.push(Item::SuppliesDef(self.parse_supplies_def()?));
                }
                other => {
                    let span = &self.peek().unwrap().span;
                    return Err(ParseError::Expected {
                        expected: "import, type, or supplies".to_string(),
                        found: format!("{other:?}"),
                        pos: span.start,
                    });
                }
            }
        }

        Ok(Module { imports, items })
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, ParseError> {
        let mut attributes = Vec::new();

        while let Some(Spanned {
            kind: Token::HashBracket,
            ..
        }) = self.peek()
        {
            let start = self.advance().unwrap().span.start; // consume '#['
            let (name, _) = self.expect_ident()?;
            let end_span = self.expect_token(Token::RBracket)?;
            attributes.push(Attribute {
                name,
                span: start..end_span.end,
            });
        }

        Ok(attributes)
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'import'
        let (module_name, name_span) = self.expect_ident()?;
        Ok(Import {
            module_name,
            span: start..name_span.end,
        })
    }

    fn parse_type_def(&mut self, attributes: Vec<Attribute>) -> Result<Item, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'type'
        let (name, _) = self.expect_ident()?;

        // Optionally parse generic type parameters: <T>, <T, U>, etc.
        let type_params = if self.check(&Token::Lt) {
            self.advance(); // consume '<'
            let mut params = Vec::new();
            loop {
                if self.check(&Token::Gt) {
                    break;
                }
                let (param, _) = self.expect_ident()?;
                params.push(param);
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_token(Token::Gt)?;
            params
        } else {
            vec![]
        };

        // Handle `type Name;` (empty product type without `=`)
        if self.check(&Token::Semicolon) {
            let end = self.expect_token(Token::Semicolon)?;
            return Ok(Item::RecordDef(crate::ast::RecordDef {
                name,
                type_params,
                attributes,
                fields: vec![],
                span: start..end.end,
            }));
        }

        self.expect_token(Token::Eq)?;

        // Handle `type Name = ;` (empty product type with `=`)
        if self.check(&Token::Semicolon) {
            let end = self.expect_token(Token::Semicolon)?;
            return Ok(Item::RecordDef(crate::ast::RecordDef {
                name,
                type_params,
                attributes,
                fields: vec![],
                span: start..end.end,
            }));
        }

        // Disambiguate product vs sum:
        // - Ident followed by Colon => product type (field: Type)
        // - Otherwise => sum type (variants)
        let is_product = match self.tokens.get(self.pos) {
            Some(Spanned {
                kind: Token::Ident(_),
                ..
            }) => matches!(
                self.tokens.get(self.pos + 1),
                Some(Spanned {
                    kind: Token::Colon,
                    ..
                })
            ),
            _ => false,
        };

        if is_product {
            let mut fields = Vec::new();
            while !self.check(&Token::Semicolon) {
                fields.push(self.parse_field()?);
                if self.check(&Token::Comma) {
                    self.advance();
                }
            }
            let end = self.expect_token(Token::Semicolon)?;
            Ok(Item::RecordDef(crate::ast::RecordDef {
                name,
                type_params,
                attributes,
                fields,
                span: start..end.end,
            }))
        } else {
            let mut variants = Vec::new();
            loop {
                if self.check(&Token::Semicolon) {
                    break;
                }
                variants.push(self.parse_variant()?);
                if self.check(&Token::Pipe) {
                    self.advance();
                } else {
                    break;
                }
            }
            let end = self.expect_token(Token::Semicolon)?;
            Ok(Item::ChoiceDef(crate::ast::ChoiceDef {
                name,
                type_params,
                attributes,
                variants,
                span: start..end.end,
            }))
        }
    }

    fn parse_variant(&mut self) -> Result<Variant, ParseError> {
        let (name, name_span) = self.expect_ident()?;
        let mut fields = Vec::new();

        if self.check(&Token::LParen) {
            self.advance();
            loop {
                if self.check(&Token::RParen) {
                    break;
                }
                fields.push(self.parse_type_expr()?);
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_token(Token::RParen)?;
        }

        let end = self.previous_span_end();
        Ok(Variant {
            name,
            fields,
            span: name_span.start..end,
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

        // Primitive type keywords
        if let Some(primitive) = self.try_parse_primitive() {
            return Ok(crate::ast::TypeExpr::Primitive(primitive));
        }

        // Array [T; N] or Slice [T]
        if self.check(&Token::LBracket) {
            self.advance();
            let element = self.parse_type_expr()?;
            if self.check(&Token::Semicolon) {
                self.advance();
                let size = self.parse_array_size()?;
                self.expect_token(Token::RBracket)?;
                return Ok(crate::ast::TypeExpr::Array {
                    element: Box::new(element),
                    size,
                });
            }
            self.expect_token(Token::RBracket)?;
            return Ok(crate::ast::TypeExpr::Slice(Box::new(element)));
        }

        // Tuple (T1, T2, ...) or Unit ()
        if self.check(&Token::LParen) {
            self.advance();
            if self.check(&Token::RParen) {
                self.advance();
                return Ok(crate::ast::TypeExpr::Unit);
            }
            let mut elements = Vec::new();
            loop {
                elements.push(self.parse_type_expr()?);
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_token(Token::RParen)?;
            return Ok(crate::ast::TypeExpr::Tuple(elements));
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

    fn try_parse_primitive(&mut self) -> Option<crate::ast::PrimitiveType> {
        let primitive = match self.peek()?.kind {
            Token::Bool => crate::ast::PrimitiveType::Bool,
            Token::U8 => crate::ast::PrimitiveType::U8,
            Token::U16 => crate::ast::PrimitiveType::U16,
            Token::U32 => crate::ast::PrimitiveType::U32,
            Token::U64 => crate::ast::PrimitiveType::U64,
            Token::U128 => crate::ast::PrimitiveType::U128,
            Token::I8 => crate::ast::PrimitiveType::I8,
            Token::I16 => crate::ast::PrimitiveType::I16,
            Token::I32 => crate::ast::PrimitiveType::I32,
            Token::I64 => crate::ast::PrimitiveType::I64,
            Token::I128 => crate::ast::PrimitiveType::I128,
            Token::F32 => crate::ast::PrimitiveType::F32,
            Token::F64 => crate::ast::PrimitiveType::F64,
            Token::Str => crate::ast::PrimitiveType::Str,
            _ => return None,
        };
        self.advance();
        Some(primitive)
    }

    fn parse_array_size(&mut self) -> Result<usize, ParseError> {
        match self.advance() {
            Some(Spanned {
                kind: Token::Number(n),
                span,
            }) => n.parse::<usize>().map_err(|_| ParseError::Expected {
                expected: "valid array size (usize)".to_string(),
                found: n.clone(),
                pos: span.start,
            }),
            Some(Spanned { kind, span }) => Err(ParseError::Expected {
                expected: "array size (number)".to_string(),
                found: format!("{kind:?}"),
                pos: span.start,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "array size (number)".to_string(),
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

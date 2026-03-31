use crate::ast::{
    AsInterfaceBlock, Attribute, BinaryOp, Expr, FieldInit, Import, Item, LetBinding, Module,
    StringPart, UnaryOp, Variant,
};
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
            // Contextual keywords: can be used as identifiers in most positions
            Some(Spanned { kind, span }) if token_as_contextual_ident(kind).is_some() => Ok((
                token_as_contextual_ident(kind).unwrap().to_string(),
                span.clone(),
            )),
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
                Token::Interface => {
                    items.push(self.parse_interface_def()?);
                }
                Token::Impl => {
                    items.push(self.parse_impl_block()?);
                }
                Token::Let => {
                    items.push(self.parse_let_binding()?);
                }
                other => {
                    let span = &self.peek().unwrap().span;
                    return Err(ParseError::Expected {
                        expected: "import, type, interface, impl, or let".to_string(),
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

            // Optionally parse attribute arguments: `(...)` with balanced parens
            let args = if self.check(&Token::LParen) {
                self.advance(); // consume '('
                let mut depth: usize = 1;
                let mut parts = Vec::new();

                while depth > 0 {
                    match self.advance() {
                        Some(Spanned {
                            kind: Token::LParen,
                            ..
                        }) => {
                            depth += 1;
                            parts.push("(".to_string());
                        }
                        Some(Spanned {
                            kind: Token::RParen,
                            ..
                        }) => {
                            depth -= 1;
                            if depth > 0 {
                                parts.push(")".to_string());
                            }
                        }
                        Some(spanned) => {
                            parts.push(token_to_string(&spanned.kind));
                        }
                        None => {
                            return Err(ParseError::UnexpectedEof {
                                expected: "closing ')'".to_string(),
                            });
                        }
                    }
                }

                Some(parts.join(" "))
            } else {
                None
            };

            let end_span = self.expect_token(Token::RBracket)?;
            attributes.push(Attribute {
                name,
                args,
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
        // - Ident (or contextual keyword) followed by Colon => product type (field: Type)
        // - HashBracket => field attribute, so product type
        // - Otherwise => sum type (variants)
        let first_is_ident_like = matches!(
            self.tokens.get(self.pos),
            Some(Spanned {
                kind: Token::Ident(_),
                ..
            })
        ) || self
            .tokens
            .get(self.pos)
            .is_some_and(|t| token_as_contextual_ident(&t.kind).is_some());
        let second_is_colon = matches!(
            self.tokens.get(self.pos + 1),
            Some(Spanned {
                kind: Token::Colon,
                ..
            })
        );
        let is_product = match self.tokens.get(self.pos) {
            _ if first_is_ident_like && second_is_colon => true,
            Some(Spanned {
                kind: Token::HashBracket,
                ..
            }) => true,
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

    fn parse_interface_def(&mut self) -> Result<Item, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'interface'
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

        self.expect_token(Token::Eq)?;

        // Parse interface fields until ';'
        let mut fields = Vec::new();
        while !self.check(&Token::Semicolon) {
            // Each field can have per-field attributes like #[default(...)]
            let field_attributes = self.parse_attributes()?;

            let (field_name, field_name_span) = self.expect_ident()?;
            self.expect_token(Token::Colon)?;
            let ty = self.parse_type_expr()?;
            let end = self.previous_span_end();

            fields.push(crate::ast::InterfaceField {
                name: field_name,
                ty,
                attributes: field_attributes,
                span: field_name_span.start..end,
            });

            if self.check(&Token::Comma) {
                self.advance();
            }
        }

        let end = self.expect_token(Token::Semicolon)?;

        Ok(Item::InterfaceDef(crate::ast::InterfaceDef {
            name,
            type_params,
            fields,
            span: start..end.end,
        }))
    }

    fn parse_impl_block(&mut self) -> Result<Item, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'impl'
        let (interface_name, _) = self.expect_ident()?;
        self.expect_token(Token::For)?;
        let (type_name, _) = self.expect_ident()?;
        self.expect_token(Token::LBrace)?;

        let mut mappings = Vec::new();
        while !self.check(&Token::RBrace) {
            let (name, name_span) = self.expect_ident()?;
            self.expect_token(Token::Colon)?;
            let value = self.parse_expr()?;
            let end = self.previous_span_end();
            mappings.push(crate::ast::FieldMapping {
                name,
                value,
                span: name_span.start..end,
            });
            if self.check(&Token::Comma) {
                self.advance();
            }
        }

        let end = self.expect_token(Token::RBrace)?;

        Ok(Item::ImplBlock(crate::ast::ImplBlock {
            interface_name,
            type_name,
            mappings,
            span: start..end.end,
        }))
    }

    fn parse_let_binding(&mut self) -> Result<Item, ParseError> {
        let start = self.advance().unwrap().span.start; // consume 'let'
        let (name, _) = self.expect_ident()?;

        let ty = if self.check(&Token::Colon) {
            self.advance(); // consume ':'
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect_token(Token::Eq)?;
        let value = self.parse_expr()?;
        let end = self.previous_span_end();

        Ok(Item::LetBinding(LetBinding {
            name,
            ty,
            value,
            span: start..end,
        }))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while let Some(op) = self.peek_binary_op(&[Token::EqEq, Token::BangEq]) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.peek_binary_op(&[Token::Lt, Token::Gt, Token::Lte, Token::Gte]) {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn peek_binary_op(&self, tokens: &[Token]) -> Option<BinaryOp> {
        let peeked = self.peek()?;
        for token in tokens {
            if std::mem::discriminant(&peeked.kind) == std::mem::discriminant(token) {
                return match &peeked.kind {
                    Token::EqEq => Some(BinaryOp::Eq),
                    Token::BangEq => Some(BinaryOp::NotEq),
                    Token::Lt => Some(BinaryOp::Lt),
                    Token::Gt => Some(BinaryOp::Gt),
                    Token::Lte => Some(BinaryOp::Lte),
                    Token::Gte => Some(BinaryOp::Gte),
                    _ => None,
                };
            }
        }
        None
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&Token::Bang) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = match self.peek() {
            Some(Spanned {
                kind: Token::StringLit(_),
                ..
            }) => {
                if let Token::StringLit(s) = &self.advance().unwrap().kind {
                    parse_string_literal(s)
                } else {
                    unreachable!()
                }
            }
            Some(Spanned {
                kind: Token::Number(_),
                ..
            }) => {
                if let Token::Number(n) = &self.advance().unwrap().kind {
                    Expr::Number(n.clone())
                } else {
                    unreachable!()
                }
            }
            Some(Spanned {
                kind: Token::It, ..
            }) => {
                self.advance();
                Expr::It
            }
            Some(Spanned {
                kind: Token::Self_, ..
            }) => {
                self.advance();
                Expr::Self_
            }
            Some(Spanned {
                kind: Token::Ident(name),
                ..
            }) => {
                let name = name.clone();
                match name.as_str() {
                    "true" => {
                        self.advance();
                        Expr::BoolLit(true)
                    }
                    "false" => {
                        self.advance();
                        Expr::BoolLit(false)
                    }
                    "None" => {
                        self.advance();
                        Expr::None_
                    }
                    _ => self.parse_ident_expr()?,
                }
            }
            Some(Spanned {
                kind: Token::RegexLit(_),
                ..
            }) => {
                if let Token::RegexLit(s) = &self.advance().unwrap().kind {
                    Expr::RegexLit(s.clone())
                } else {
                    unreachable!()
                }
            }
            Some(Spanned {
                kind: Token::LBracket,
                ..
            }) => {
                self.advance(); // consume '['
                let items = self.parse_expr_list(Token::RBracket)?;
                self.expect_token(Token::RBracket)?;
                Expr::ListLit(items)
            }
            // Contextual keywords used as identifiers in expression position
            Some(Spanned { kind, .. }) if token_as_contextual_ident(kind).is_some() => {
                self.parse_ident_expr()?
            }
            Some(Spanned { kind, span }) => {
                let pos = span.start;
                let found = format!("{kind:?}");
                return Err(ParseError::Expected {
                    expected: "expression".to_string(),
                    found,
                    pos,
                });
            }
            None => {
                return Err(ParseError::UnexpectedEof {
                    expected: "expression".to_string(),
                });
            }
        };

        // Chain field access with `.`
        self.parse_field_access_chain(expr)
    }

    fn parse_ident_expr(&mut self) -> Result<Expr, ParseError> {
        let (name, _) = self.expect_ident()?;

        // Ident::Variant(args) — VariantConstruction
        if self.check(&Token::PathSep) {
            self.advance(); // consume '::'
            let (variant, _) = self.expect_ident()?;
            self.expect_token(Token::LParen)?;
            let args = self.parse_expr_list(Token::RParen)?;
            self.expect_token(Token::RParen)?;
            return Ok(Expr::VariantConstruction {
                type_name: name,
                variant,
                args,
            });
        }

        // Ident { field: expr, ... } — TypeConstruction
        if self.check(&Token::LBrace) && self.is_field_init_block() {
            self.advance(); // consume '{'
            let (fields, as_interfaces) = self.parse_construction_body()?;
            self.expect_token(Token::RBrace)?;
            return Ok(Expr::TypeConstruction {
                type_name: name,
                fields,
                as_interfaces,
            });
        }

        // Ident(args) — NamedConstruction or Call
        if self.check(&Token::LParen) {
            self.advance(); // consume '('
            if self.is_named_arg_pattern() {
                let fields = self.parse_field_init_list(Token::RParen)?;
                self.expect_token(Token::RParen)?;
                return Ok(Expr::NamedConstruction {
                    type_name: name,
                    fields,
                });
            }
            let args = self.parse_expr_list(Token::RParen)?;
            self.expect_token(Token::RParen)?;
            return Ok(Expr::Call { name, args });
        }

        Ok(Expr::Ident(name))
    }

    /// Look ahead to determine if the block after `{` contains `Ident:` pattern
    /// (indicating field initializers) or `<as` pattern (indicating as-interface blocks).
    fn is_field_init_block(&self) -> bool {
        // Check if tokens at pos+1 and pos+2 are Ident:Colon or <as (Lt followed by Ident("as"))
        let t1 = self.tokens.get(self.pos + 1);
        let t2 = self.tokens.get(self.pos + 2);
        let t1_is_ident_like = matches!(
            t1,
            Some(Spanned {
                kind: Token::Ident(_),
                ..
            })
        ) || t1
            .is_some_and(|t| token_as_contextual_ident(&t.kind).is_some());
        let t2_is_colon = matches!(
            t2,
            Some(Spanned {
                kind: Token::Colon,
                ..
            })
        );
        (t1_is_ident_like && t2_is_colon)
            || matches!(
                (t1, t2),
                (
                    Some(Spanned {
                        kind: Token::Lt,
                        ..
                    }),
                    Some(Spanned {
                        kind: Token::Ident(name),
                        ..
                    })
                ) if name == "as"
            )
    }

    /// Check if the current position (inside parens) has `Ident Colon` pattern
    /// indicating named arguments.
    fn is_named_arg_pattern(&self) -> bool {
        let is_ident_like = matches!(
            self.tokens.get(self.pos),
            Some(Spanned {
                kind: Token::Ident(_),
                ..
            })
        ) || self
            .tokens
            .get(self.pos)
            .is_some_and(|t| token_as_contextual_ident(&t.kind).is_some());

        is_ident_like
            && matches!(
                self.tokens.get(self.pos + 1),
                Some(Spanned {
                    kind: Token::Colon,
                    ..
                })
            )
    }

    fn parse_field_init_list(&mut self, terminator: Token) -> Result<Vec<FieldInit>, ParseError> {
        let mut fields = Vec::new();
        while !self.check(&terminator) {
            let (name, name_span) = self.expect_ident()?;
            self.expect_token(Token::Colon)?;
            let value = self.parse_expr()?;
            let end = self.previous_span_end();
            fields.push(FieldInit {
                name,
                value,
                span: name_span.start..end,
            });
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        Ok(fields)
    }

    /// Parse the body of a type construction: a mix of `name: expr` field inits
    /// and `<as Interface> { ... }` blocks, terminated by `}` (which is NOT consumed).
    fn parse_construction_body(
        &mut self,
    ) -> Result<(Vec<FieldInit>, Vec<AsInterfaceBlock>), ParseError> {
        let mut fields = Vec::new();
        let mut as_interfaces = Vec::new();
        while !self.check(&Token::RBrace) {
            if self.is_as_interface_start() {
                as_interfaces.push(self.parse_as_interface_block()?);
            } else {
                let (name, name_span) = self.expect_ident()?;
                self.expect_token(Token::Colon)?;
                let value = self.parse_expr()?;
                let end = self.previous_span_end();
                fields.push(FieldInit {
                    name,
                    value,
                    span: name_span.start..end,
                });
            }
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        Ok((fields, as_interfaces))
    }

    /// Check if the current position starts an `<as Interface>` block:
    /// Token::Lt followed by Token::Ident("as").
    fn is_as_interface_start(&self) -> bool {
        matches!(
            (self.tokens.get(self.pos), self.tokens.get(self.pos + 1)),
            (
                Some(Spanned {
                    kind: Token::Lt,
                    ..
                }),
                Some(Spanned {
                    kind: Token::Ident(name),
                    ..
                })
            ) if name == "as"
        )
    }

    /// Parse `<as InterfaceName> { field: expr, ... }`.
    /// Assumes the current token is `<`.
    fn parse_as_interface_block(&mut self) -> Result<AsInterfaceBlock, ParseError> {
        let start_span = self.expect_token(Token::Lt)?; // consume '<'
        // Expect Ident("as")
        match self.advance() {
            Some(Spanned {
                kind: Token::Ident(name),
                ..
            }) if name == "as" => {}
            Some(Spanned { kind, span }) => {
                return Err(ParseError::Expected {
                    expected: "\"as\"".to_string(),
                    found: format!("{kind:?}"),
                    pos: span.start,
                });
            }
            None => {
                return Err(ParseError::UnexpectedEof {
                    expected: "\"as\"".to_string(),
                });
            }
        }
        let (interface_name, _) = self.expect_ident()?;
        self.expect_token(Token::Gt)?; // consume '>'
        self.expect_token(Token::LBrace)?; // consume '{'
        let fields = self.parse_field_init_list(Token::RBrace)?;
        let end_span = self.expect_token(Token::RBrace)?; // consume '}'
        Ok(AsInterfaceBlock {
            interface_name,
            fields,
            span: start_span.start..end_span.end,
        })
    }

    fn parse_expr_list(&mut self, terminator: Token) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = Vec::new();
        while !self.check(&terminator) {
            exprs.push(self.parse_expr()?);
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        Ok(exprs)
    }

    fn parse_field_access_chain(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        while self.check(&Token::Dot) {
            self.advance(); // consume '.'
            let (field, _) = self.expect_ident()?;
            expr = Expr::FieldAccess {
                object: Box::new(expr),
                field,
            };
        }
        Ok(expr)
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
        let attributes = self.parse_attributes()?;
        let (name, name_span) = self.expect_ident()?;
        self.expect_token(Token::Colon)?;
        let ty = self.parse_type_expr()?;
        let end = self.previous_span_end();

        let start = if let Some(attr) = attributes.first() {
            attr.span.start
        } else {
            name_span.start
        };

        Ok(crate::ast::Field {
            name,
            ty,
            attributes,
            span: start..end,
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

        // List [T]
        if self.check(&Token::LBracket) {
            self.advance();
            let element = self.parse_type_expr()?;
            self.expect_token(Token::RBracket)?;
            return Ok(crate::ast::TypeExpr::List(Box::new(element)));
        }

        // HashMap {K: V}
        if self.check(&Token::LBrace) {
            self.advance();
            let key = self.parse_type_expr()?;
            self.expect_token(Token::Colon)?;
            let value = self.parse_type_expr()?;
            self.expect_token(Token::RBrace)?;
            return Ok(crate::ast::TypeExpr::HashMap {
                key: Box::new(key),
                value: Box::new(value),
            });
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

    fn try_parse_primitive(&mut self) -> Option<crate::ast::PrimitiveType> {
        let primitive = match self.peek()?.kind {
            Token::Bool => crate::ast::PrimitiveType::Bool,
            Token::Number_ => crate::ast::PrimitiveType::Number,
            Token::String_ => crate::ast::PrimitiveType::String,
            _ => return None,
        };
        self.advance();
        Some(primitive)
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

/// Parse a string literal, detecting `${...}` interpolation sequences.
///
/// If no interpolations are found, returns `Expr::StringLit`.
/// If interpolations are present, returns `Expr::StringInterpolation` with a
/// mix of `StringPart::Literal` and `StringPart::Expr` segments.
///
/// The expression inside `${...}` is parsed as a dotted identifier path,
/// e.g. `name` becomes `Expr::Ident("name")` and `postgres.host` becomes
/// `Expr::FieldAccess { object: Ident("postgres"), field: "host" }`.
fn parse_string_literal(s: &str) -> Expr {
    if !s.contains("${") {
        return Expr::StringLit(s.to_string());
    }

    let mut parts = Vec::new();
    let mut rest = s;

    while !rest.is_empty() {
        if let Some(dollar_pos) = rest.find("${") {
            // Push any literal text before the interpolation
            if dollar_pos > 0 {
                parts.push(StringPart::Literal(rest[..dollar_pos].to_string()));
            }

            // Find the closing brace
            let after_open = &rest[dollar_pos + 2..];
            match after_open.find('}') {
                Some(close_pos) => {
                    let expr_text = &after_open[..close_pos];
                    parts.push(StringPart::Expr(parse_dotted_path(expr_text)));
                    rest = &after_open[close_pos + 1..];
                }
                None => {
                    // No closing brace found — treat remainder as literal
                    parts.push(StringPart::Literal(rest.to_string()));
                    break;
                }
            }
        } else {
            // No more interpolations — push remaining text as literal
            parts.push(StringPart::Literal(rest.to_string()));
            break;
        }
    }

    Expr::StringInterpolation(parts)
}

/// Parse a dotted path like `"postgres.host"` into a chain of `FieldAccess` expressions.
///
/// `"name"` → `Expr::Ident("name")`
/// `"a.b"` → `Expr::FieldAccess { object: Ident("a"), field: "b" }`
/// `"a.b.c"` → `Expr::FieldAccess { object: FieldAccess { object: Ident("a"), field: "b" }, field: "c" }`
fn parse_dotted_path(path: &str) -> Expr {
    let segments: Vec<&str> = path.split('.').collect();
    let mut expr = Expr::Ident(segments[0].trim().to_string());
    for segment in &segments[1..] {
        expr = Expr::FieldAccess {
            object: Box::new(expr),
            field: segment.trim().to_string(),
        };
    }
    expr
}

/// Check if a keyword token can be used as a contextual identifier.
/// These are keywords that name built-in functions and can also serve as field/type names.
fn token_as_contextual_ident(token: &Token) -> Option<&'static str> {
    match token {
        Token::Keep => Some("keep"),
        Token::Drop_ => Some("drop"),
        Token::Count => Some("count"),
        Token::Sum => Some("sum"),
        Token::Mean => Some("mean"),
        Token::Median => Some("median"),
        Token::Min => Some("min"),
        Token::Max => Some("max"),
        Token::Set => Some("Set"),
        Token::Map => Some("map"),
        Token::Filter => Some("filter"),
        Token::Number_ => Some("number"),
        Token::String_ => Some("string"),
        _ => None,
    }
}

/// Convert a token to its string representation for raw attribute argument capture.
fn token_to_string(token: &Token) -> String {
    match token {
        Token::Ident(s) => s.clone(),
        Token::Number(s) => s.clone(),
        Token::StringLit(s) => format!("\"{s}\""),
        Token::Import => "import".to_string(),
        Token::If => "if".to_string(),
        Token::Then => "then".to_string(),
        Token::Else => "else".to_string(),
        Token::Fn => "fn".to_string(),
        Token::Map => "map".to_string(),
        Token::Filter => "filter".to_string(),
        Token::Self_ => "self".to_string(),
        Token::Interface => "interface".to_string(),
        Token::Impl => "impl".to_string(),
        Token::For => "for".to_string(),
        Token::Let => "let".to_string(),
        Token::It => "it".to_string(),
        Token::Bool => "bool".to_string(),
        Token::Number_ => "number".to_string(),
        Token::String_ => "string".to_string(),
        Token::Set => "Set".to_string(),
        Token::Keep => "keep".to_string(),
        Token::Drop_ => "drop".to_string(),
        Token::Count => "count".to_string(),
        Token::Sum => "sum".to_string(),
        Token::Mean => "mean".to_string(),
        Token::Median => "median".to_string(),
        Token::Min => "min".to_string(),
        Token::Max => "max".to_string(),
        Token::RegexLit(s) => format!("r\"{s}\""),
        Token::RegexMatch => "=~".to_string(),
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Star => "*".to_string(),
        Token::Slash => "/".to_string(),
        Token::Type => "type".to_string(),
        Token::HashBracket => "#[".to_string(),
        Token::LBrace => "{".to_string(),
        Token::RBrace => "}".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::Comma => ",".to_string(),
        Token::Colon => ":".to_string(),
        Token::PathSep => "::".to_string(),
        Token::Dot => ".".to_string(),
        Token::Eq => "=".to_string(),
        Token::EqEq => "==".to_string(),
        Token::BangEq => "!=".to_string(),
        Token::Gte => ">=".to_string(),
        Token::Lte => "<=".to_string(),
        Token::Gt => ">".to_string(),
        Token::Lt => "<".to_string(),
        Token::Pipe => "|".to_string(),
        Token::And => "&&".to_string(),
        Token::Or => "||".to_string(),
        Token::Bang => "!".to_string(),
        Token::Arrow => "->".to_string(),
        Token::Semicolon => ";".to_string(),
    }
}

pub fn parse(input: &str) -> Result<Module, ParseError> {
    let tokens = lexer::lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_module()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expr, Item};

    #[test]
    fn test_parse_let_string_literal() {
        let input = r#"let x = "hello""#;
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert_eq!(l.name, "x");
                assert!(matches!(&l.value, Expr::StringLit(s) if s == "hello"));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_number() {
        let input = "let x = 42";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert!(matches!(&l.value, Expr::Number(n) if n == "42"));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_bool() {
        let input = "let x = true";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert!(matches!(&l.value, Expr::BoolLit(true)));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_none() {
        let input = "let x = None";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert!(matches!(&l.value, Expr::None_));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_ident_ref() {
        let input = "let x = my_var";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert!(matches!(&l.value, Expr::Ident(n) if n == "my_var"));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_type_construction() {
        let input = r#"let x = MyType {
  host: "localhost",
  port: 8080,
}"#;
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::TypeConstruction {
                    type_name, fields, ..
                } => {
                    assert_eq!(type_name, "MyType");
                    assert_eq!(fields.len(), 2);
                }
                other => panic!("expected TypeConstruction, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_variant_construction() {
        let input = "let x = Option::Some(42)";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::VariantConstruction {
                    type_name,
                    variant,
                    args,
                } => {
                    assert_eq!(type_name, "Option");
                    assert_eq!(variant, "Some");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected VariantConstruction, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_binary_op() {
        let input = "let x = it >= 15 && it < 17";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::BinaryOp { op, .. } => {
                    assert_eq!(*op, BinaryOp::And);
                }
                other => panic!("expected BinaryOp, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_named_construction() {
        let input = "let x = SemVer(major: 17, minor: 0)";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::NamedConstruction { type_name, fields } => {
                    assert_eq!(type_name, "SemVer");
                    assert_eq!(fields.len(), 2);
                }
                other => panic!("expected NamedConstruction, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_it_keyword() {
        let input = "let x = it";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert!(matches!(&l.value, Expr::It));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_self_field_access() {
        let input = "let x = Self.endpoint.user";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::FieldAccess { object, field } => {
                    assert_eq!(field, "user");
                    match object.as_ref() {
                        Expr::FieldAccess { object, field } => {
                            assert_eq!(field, "endpoint");
                            assert!(matches!(object.as_ref(), Expr::Self_));
                        }
                        other => panic!("expected nested FieldAccess, got {other:?}"),
                    }
                }
                other => panic!("expected FieldAccess, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_unary_not() {
        let input = "let x = !my_flag";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::UnaryOp { op, operand } => {
                    assert_eq!(*op, crate::ast::UnaryOp::Not);
                    assert!(matches!(operand.as_ref(), Expr::Ident(n) if n == "my_flag"));
                }
                other => panic!("expected UnaryOp, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_or_expression() {
        let input = "let x = a || b";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::BinaryOp { op, left, right } => {
                    assert_eq!(*op, BinaryOp::Or);
                    assert!(matches!(left.as_ref(), Expr::Ident(n) if n == "a"));
                    assert!(matches!(right.as_ref(), Expr::Ident(n) if n == "b"));
                }
                other => panic!("expected BinaryOp, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_equality() {
        let input = "let x = a == b";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::BinaryOp { op, .. } => {
                    assert_eq!(*op, BinaryOp::Eq);
                }
                other => panic!("expected BinaryOp, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_call_positional() {
        let input = "let x = Some(42)";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::Call { name, args } => {
                    assert_eq!(name, "Some");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], Expr::Number(n) if n == "42"));
                }
                other => panic!("expected Call, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_binding_with_type_annotation() {
        let input = "let port: number = 5432";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert_eq!(l.name, "port");
                assert!(l.ty.is_some());
                assert!(matches!(
                    l.ty.as_ref().unwrap(),
                    crate::ast::TypeExpr::Primitive(crate::ast::PrimitiveType::Number)
                ));
                assert!(matches!(&l.value, Expr::Number(n) if n == "5432"));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_let_binding_without_type_annotation() {
        let input = "let port = 5432";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => {
                assert_eq!(l.name, "port");
                assert!(l.ty.is_none());
                assert!(matches!(&l.value, Expr::Number(n) if n == "5432"));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_precedence_and_binds_tighter_than_or() {
        // a || b && c should parse as a || (b && c)
        let input = "let x = a || b && c";
        let module = parse(input).unwrap();
        match &module.items[0] {
            Item::LetBinding(l) => match &l.value {
                Expr::BinaryOp { op, left, right } => {
                    assert_eq!(*op, BinaryOp::Or);
                    assert!(matches!(left.as_ref(), Expr::Ident(n) if n == "a"));
                    assert!(matches!(
                        right.as_ref(),
                        Expr::BinaryOp {
                            op: BinaryOp::And,
                            ..
                        }
                    ));
                }
                other => panic!("expected BinaryOp, got {other:?}"),
            },
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }
}

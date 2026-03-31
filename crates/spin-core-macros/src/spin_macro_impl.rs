use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// Convert a proc-macro `TokenStream` into a faithful string representation
/// of .spin source code.
///
/// Rust's proc-macro tokenizer breaks input into tokens that may not preserve
/// the original spacing. This function reconstructs the source with correct
/// spacing for the .spin lexer, handling:
///
/// - `#[attr]` — Rust tokenizes as `Punct('#')` + `Group(Bracket, ...)`,
///   but spin's lexer expects `#[` as a single `HashBracket` token.
/// - Hyphenated identifiers like `lang-item` — Rust tokenizes as
///   `Ident("lang")` + `Punct('-')` + `Ident("item")`, and we must
///   reconstruct without spaces around the hyphen.
/// - `${expr}` inside string literals — these remain as a single `Literal`
///   token, so they are preserved naturally.
pub fn tokens_to_spin_source(tokens: TokenStream) -> String {
    let mut result = String::new();
    let mut iter = tokens.into_iter().peekable();

    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Punct(ref p) if p.as_char() == '#' => {
                match iter.peek() {
                    // #[...] — attribute syntax (Rust tokenizes brackets as Group)
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => {
                        let group = iter.next().unwrap();
                        if let TokenTree::Group(g) = group {
                            result.push_str("#[");
                            result.push_str(&tokens_to_spin_source(g.stream()));
                            result.push(']');
                            continue;
                        }
                    }
                    // #{...} — hashmap literal
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                        let group = iter.next().unwrap();
                        if let TokenTree::Group(g) = group {
                            result.push_str("#{");
                            result.push_str(&tokens_to_spin_source(g.stream()));
                            result.push('}');
                            continue;
                        }
                    }
                    // #(...) — set literal
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                        let group = iter.next().unwrap();
                        if let TokenTree::Group(g) = group {
                            result.push_str("#(");
                            result.push_str(&tokens_to_spin_source(g.stream()));
                            result.push(')');
                            continue;
                        }
                    }
                    _ => {}
                }
                result.push('#');
            }
            TokenTree::Punct(ref p) => {
                let ch = p.as_char();
                let spacing = p.spacing();

                // Add space before `=` if it's a standalone assignment,
                // not part of a compound operator (==, >=, <=, !=).
                // We detect compound operators by checking Joint spacing
                // of the previous punct — but since we only have the result
                // string, we check if the previous char is one that forms
                // compounds AND the current token has Joint spacing on the
                // *previous* token. Actually, Rust encodes this: Joint
                // spacing on the first punct means it's part of a compound.
                // When the previous punct was Alone-spaced, > and = are
                // separate tokens. We handle this by adding space before
                // `=` unless the result ends with a Joint-spaced compound
                // starter. Since we can't check spacing of previous tokens
                // from the result string alone, we use a different strategy:
                // always add space before `=` when not preceded by space,
                // and rely on Joint-spaced compound operators to NOT have
                // a space between them (since Joint tokens emit no space).
                if ch == '=' && !result.is_empty() && !result.ends_with([' ', '\n']) {
                    // Check if this `=` is part of a compound operator.
                    // If the previous character is `>`, `<`, `!`, or `=`,
                    // AND it was emitted without a trailing space (meaning
                    // it had Joint spacing), then this is a compound operator.
                    // We detect Joint by the absence of trailing space.
                    let prev = result.chars().last().unwrap();
                    if !matches!(prev, '>' | '<' | '!' | '=') {
                        result.push(' ');
                    }
                }

                result.push(ch);

                if spacing == Spacing::Alone {
                    // Add space after certain punctuation for readability.
                    // Also add space after `>` and `<` when Alone, to
                    // prevent accidental `>=` or `<=` with the next token.
                    match ch {
                        ',' | ';' | '=' | '>' | '<' => result.push(' '),
                        _ => {}
                    }
                }
                // Joint spacing: no space, next punct continues the sequence
            }
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Brace => ('{', '}'),
                    Delimiter::Bracket => ('[', ']'),
                    Delimiter::Parenthesis => ('(', ')'),
                    Delimiter::None => {
                        // No visible delimiters — just emit the contents
                        result.push_str(&tokens_to_spin_source(g.stream()));
                        continue;
                    }
                };
                // Add space before opening delimiter if preceded by non-space,
                // non-opener, non-hash
                if !result.is_empty()
                    && !result.ends_with(|c: char| {
                        c == ' ' || c == '\n' || c == '(' || c == '[' || c == '{'
                    })
                {
                    result.push(' ');
                }
                result.push(open);
                result.push_str(&tokens_to_spin_source(g.stream()));
                result.push(close);
            }
            TokenTree::Ident(ref i) => {
                // Add space before ident unless:
                // - at start of output
                // - preceded by opener, whitespace, hash, hyphen, or colon
                //   (hyphen: `lang-item`; colon: `spin-core::TcpPort`)
                if !result.is_empty()
                    && !result.ends_with(|c: char| {
                        c == ' '
                            || c == '\n'
                            || c == '('
                            || c == '['
                            || c == '{'
                            || c == '#'
                            || c == '-'
                            || c == ':'
                    })
                {
                    result.push(' ');
                }
                result.push_str(&i.to_string());
            }
            TokenTree::Literal(ref l) => {
                // Add space before literal unless preceded by space, opener, or `=`
                if !result.is_empty()
                    && !result.ends_with(|c: char| {
                        c == ' ' || c == '\n' || c == '(' || c == '[' || c == '{' || c == '='
                    })
                {
                    result.push(' ');
                }
                result.push_str(&l.to_string());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn simple_type_def() {
        let tokens = quote! { type Foo = x: u32; };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            spin_lang::ast::Item::RecordDef(r) => {
                assert_eq!(r.name, "Foo");
                assert_eq!(r.fields.len(), 1);
                assert_eq!(r.fields[0].name, "x");
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn attribute_without_args() {
        // This is the key test: #[lang-item] must become "#[lang-item]" in the output
        let tokens = quote! {
            #[lang-item]
            type Foo = x: u32;
        };
        let source = tokens_to_spin_source(tokens);

        // The source must contain #[ as a single token for our lexer
        assert!(
            source.contains("#["),
            "expected #[ in source, got: {source:?}"
        );
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            spin_lang::ast::Item::RecordDef(r) => {
                assert_eq!(r.attributes.len(), 1);
                assert_eq!(r.attributes[0].name, "lang-item");
                assert!(r.attributes[0].args.is_none());
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn attribute_with_args() {
        let tokens = quote! {
            #[delegate(PostgresEndpoint)]
            type Proxy = frontend: str;
        };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        match &module.items[0] {
            spin_lang::ast::Item::RecordDef(r) => {
                assert_eq!(r.attributes.len(), 1);
                assert_eq!(r.attributes[0].name, "delegate");
                assert_eq!(r.attributes[0].args.as_deref(), Some("PostgresEndpoint"));
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn let_binding_simple() {
        let tokens = quote! { let x = 42 };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
        assert!(matches!(
            &module.items[0],
            spin_lang::ast::Item::LetBinding(_)
        ));
    }

    #[test]
    fn import_with_hyphen() {
        let tokens = quote! { import spin-core };
        let source = tokens_to_spin_source(tokens);

        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].module_name, "spin-core");
    }

    #[test]
    fn multiple_items() {
        let tokens = quote! {
            type Inner = value: str;
            type Server = inner: Inner;
            interface Endpoint = host: str;
        };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 3);
    }

    #[test]
    fn impl_block() {
        let tokens = quote! {
            interface Endpoint = host: str, port: u16;
            type Server = hostname: str, port_num: u16;
            impl Endpoint for Server {
                host: self.hostname,
                port: self.port_num,
            }
        };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 3);
        assert!(matches!(
            &module.items[2],
            spin_lang::ast::Item::ImplBlock(_)
        ));
    }

    #[test]
    fn record_construction_in_let() {
        let tokens = quote! {
            type Foo = x: u32;
            let f = Foo { x: 42, }
        };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn qualified_type_path() {
        let tokens = quote! { type Postgres = port: spin-core::TcpPort; };
        let source = tokens_to_spin_source(tokens);

        let module = spin_lang::parser::parse(&source).unwrap();
        match &module.items[0] {
            spin_lang::ast::Item::RecordDef(r) => {
                assert_eq!(r.fields[0].name, "port");
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn generic_type() {
        let tokens = quote! { type Postgres = tls: Option<Self::Tls>; };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        match &module.items[0] {
            spin_lang::ast::Item::RecordDef(r) => {
                assert_eq!(r.fields[0].name, "tls");
            }
            other => panic!("expected RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn string_literal_preserved() {
        let tokens = quote! { let x = "hello world" };
        let source = tokens_to_spin_source(tokens);
        assert!(
            source.contains("\"hello world\""),
            "expected string literal in source, got: {source:?}"
        );
        let module = spin_lang::parser::parse(&source).unwrap();
        match &module.items[0] {
            spin_lang::ast::Item::LetBinding(l) => {
                assert!(matches!(
                    &l.value,
                    spin_lang::ast::Expr::StringLit(s) if s == "hello world"
                ));
            }
            other => panic!("expected LetBinding, got {other:?}"),
        }
    }

    #[test]
    fn comparison_operators() {
        let tokens = quote! { let x = Foo(val: it >= 1 && it <= 10) };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
        assert!(matches!(
            &module.items[0],
            spin_lang::ast::Item::LetBinding(_)
        ));
    }

    #[test]
    fn equality_operator() {
        let tokens = quote! { let x = Foo(level: it == 1 || it == 2) };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn not_equal_operator() {
        let tokens = quote! { let x = Foo(val: it != 0) };
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn empty_input() {
        let tokens = quote! {};
        let source = tokens_to_spin_source(tokens);
        let module = spin_lang::parser::parse(&source).unwrap();
        assert!(module.imports.is_empty());
        assert!(module.items.is_empty());
    }

    #[test]
    fn produces_same_result_as_direct_parse() {
        let tokens = quote! { type Foo = x: u32; };
        let source = tokens_to_spin_source(tokens);
        let module_from_macro = spin_lang::parser::parse(&source).unwrap();
        let module_from_parse = spin_lang::parser::parse("type Foo = x: u32;").unwrap();

        assert_eq!(module_from_macro.items.len(), module_from_parse.items.len());
        match (&module_from_macro.items[0], &module_from_parse.items[0]) {
            (spin_lang::ast::Item::RecordDef(a), spin_lang::ast::Item::RecordDef(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.fields.len(), b.fields.len());
                assert_eq!(a.fields[0].name, b.fields[0].name);
            }
            _ => panic!("expected both to be RecordDef"),
        }
    }
}

use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use spin_lang::ast::*;
use spin_lang::parser;

pub struct SpinBackend {
    client: Client,
    documents: Mutex<HashMap<Url, DocumentState>>,
}

struct DocumentState {
    source: String,
    module: Option<Module>,
}

impl SpinBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }
}

// --- Position/offset helpers ---

/// Convert a byte offset in source text to an LSP Position (line, character).
fn offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

/// Convert an LSP Position to a byte offset in source text.
fn position_to_offset(source: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut current_col = 0u32;
    for (i, ch) in source.char_indices() {
        if current_line == position.line && current_col == position.character {
            return i;
        }
        if ch == '\n' {
            if current_line == position.line {
                // Position is past end of line; return end of this line
                return i;
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
    }
    source.len()
}

/// Convert a byte range to an LSP Range.
fn span_to_range(source: &str, span: &std::ops::Range<usize>) -> Range {
    Range::new(
        offset_to_position(source, span.start),
        offset_to_position(source, span.end),
    )
}

// --- Completion helpers ---

const KEYWORDS: &[&str] = &[
    "type",
    "interface",
    "impl",
    "for",
    "let",
    "import",
    "if",
    "then",
    "else",
    "fn",
    "map",
    "filter",
    "self",
    "Self",
    "it",
    "None",
    "true",
    "false",
];

const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64",
    "str",
];

/// Collect all user-defined type names (records, choices, interfaces) from a module.
fn collect_type_names(module: &Module) -> Vec<String> {
    let mut names = Vec::new();
    for item in &module.items {
        match item {
            Item::RecordDef(r) => names.push(r.name.clone()),
            Item::ChoiceDef(c) => names.push(c.name.clone()),
            Item::InterfaceDef(i) => names.push(i.name.clone()),
            _ => {}
        }
    }
    names
}

/// Collect field names for a given type name from the module.
fn collect_fields_for_type(module: &Module, type_name: &str) -> Vec<(String, String)> {
    for item in &module.items {
        match item {
            Item::RecordDef(r) if r.name == type_name => {
                return r
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), format_type_expr(&f.ty)))
                    .collect();
            }
            Item::InterfaceDef(i) if i.name == type_name => {
                return i
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), format_type_expr(&f.ty)))
                    .collect();
            }
            _ => {}
        }
    }
    Vec::new()
}

/// Format a TypeExpr for display.
fn format_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Primitive(p) => format!("{p:?}").to_lowercase(),
        TypeExpr::Path { module, name } => format!("{module}::{name}"),
        TypeExpr::Generic { name, args } => {
            let args_str: Vec<String> = args.iter().map(format_type_expr).collect();
            format!("{name}<{}>", args_str.join(", "))
        }
        TypeExpr::SelfPath(n) => format!("Self::{n}"),
        TypeExpr::Array { element, size } => format!("[{}; {size}]", format_type_expr(element)),
        TypeExpr::Slice(element) => format!("[{}]", format_type_expr(element)),
        TypeExpr::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(format_type_expr).collect();
            format!("({})", parts.join(", "))
        }
        TypeExpr::Unit => "()".to_string(),
    }
}

/// Determine the completion context based on cursor position and source text.
enum CompletionContext {
    /// After a dot: suggest fields
    AfterDot { prefix: String },
    /// After a colon: suggest types
    AfterColon,
    /// General position: suggest keywords, types, etc.
    General,
}

fn detect_completion_context(source: &str, offset: usize) -> CompletionContext {
    let before_cursor = &source[..offset.min(source.len())];
    let trimmed = before_cursor.trim_end();
    if let Some(before_dot) = trimmed.strip_suffix('.') {
        // Find the identifier before the dot
        let ident: String = before_dot
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        CompletionContext::AfterDot { prefix: ident }
    } else if trimmed.ends_with(':') {
        CompletionContext::AfterColon
    } else {
        CompletionContext::General
    }
}

/// Build keyword completion items.
fn keyword_completions() -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .map(|kw| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}

/// Build primitive type completion items.
fn primitive_type_completions() -> Vec<CompletionItem> {
    PRIMITIVE_TYPES
        .iter()
        .map(|t| CompletionItem {
            label: t.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            ..Default::default()
        })
        .collect()
}

/// Build user-defined type completion items from a module.
fn user_type_completions(module: &Module) -> Vec<CompletionItem> {
    collect_type_names(module)
        .into_iter()
        .map(|name| CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::CLASS),
            ..Default::default()
        })
        .collect()
}

/// Build field completion items for a given type name.
fn field_completions(module: &Module, type_name: &str) -> Vec<CompletionItem> {
    collect_fields_for_type(module, type_name)
        .into_iter()
        .map(|(name, ty)| CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(ty),
            ..Default::default()
        })
        .collect()
}

/// Build all completion items for a given context.
fn build_completions(module: Option<&Module>, context: CompletionContext) -> Vec<CompletionItem> {
    match context {
        CompletionContext::AfterDot { ref prefix } => {
            if let Some(module) = module {
                // Try to find the type by looking up let bindings or "self"
                if prefix == "self" || prefix == "Self" {
                    // For self, we'd need to know the current type context.
                    // For now, return all fields from all types as a heuristic.
                    let mut items = Vec::new();
                    for item in &module.items {
                        if let Item::RecordDef(r) = item {
                            items.extend(field_completions(module, &r.name));
                        }
                    }
                    items
                } else {
                    // Try to find a let binding with that name and infer its type
                    let type_name = find_let_binding_type(module, prefix);
                    if let Some(type_name) = type_name {
                        field_completions(module, &type_name)
                    } else {
                        // Maybe prefix is a type name directly
                        field_completions(module, prefix)
                    }
                }
            } else {
                Vec::new()
            }
        }
        CompletionContext::AfterColon => {
            let mut items = primitive_type_completions();
            if let Some(module) = module {
                items.extend(user_type_completions(module));
            }
            items
        }
        CompletionContext::General => {
            let mut items = keyword_completions();
            items.extend(primitive_type_completions());
            if let Some(module) = module {
                items.extend(user_type_completions(module));
            }
            items
        }
    }
}

/// Try to find the type name of a let binding by name.
fn find_let_binding_type(module: &Module, name: &str) -> Option<String> {
    for item in &module.items {
        if let Item::LetBinding(lb) = item
            && lb.name == name
        {
            // If it has an explicit type annotation, use that
            if let Some(ref ty) = lb.ty {
                return Some(format_type_expr(ty));
            }
            // Otherwise try to infer from TypeConstruction
            if let Expr::TypeConstruction { type_name, .. } = &lb.value {
                return Some(type_name.clone());
            }
        }
    }
    None
}

// --- Hover helpers ---

/// Find what token is at the given offset in the source and produce hover text.
fn hover_at(source: &str, module: Option<&Module>, offset: usize) -> Option<String> {
    // Extract the word at the cursor position
    let word = word_at_offset(source, offset)?;

    // Check if it's a keyword
    if let Some(desc) = keyword_description(&word) {
        return Some(format!("**{}** (keyword)\n\n{}", word, desc));
    }

    // Check in the module AST
    let module = module?;

    // Check if it's a type name
    for item in &module.items {
        match item {
            Item::RecordDef(r) if r.name == word => {
                let fields: Vec<String> = r
                    .fields
                    .iter()
                    .map(|f| format!("  {}: {}", f.name, format_type_expr(&f.ty)))
                    .collect();
                return Some(format!(
                    "**type** {}\n\n```\ntype {} =\n{};\n```",
                    word,
                    word,
                    fields.join(",\n")
                ));
            }
            Item::ChoiceDef(c) if c.name == word => {
                let variants: Vec<String> = c
                    .variants
                    .iter()
                    .map(|v| {
                        if v.fields.is_empty() {
                            v.name.clone()
                        } else {
                            let args: Vec<String> = v.fields.iter().map(format_type_expr).collect();
                            format!("{}({})", v.name, args.join(", "))
                        }
                    })
                    .collect();
                return Some(format!(
                    "**type** {} (choice)\n\n```\ntype {} = {};\n```",
                    word,
                    word,
                    variants.join(" | ")
                ));
            }
            Item::InterfaceDef(i) if i.name == word => {
                let fields: Vec<String> = i
                    .fields
                    .iter()
                    .map(|f| format!("  {}: {}", f.name, format_type_expr(&f.ty)))
                    .collect();
                return Some(format!(
                    "**interface** {}\n\n```\ninterface {} =\n{};\n```",
                    word,
                    word,
                    fields.join(",\n")
                ));
            }
            _ => {}
        }
    }

    // Check if it's a field name by looking at all types
    for item in &module.items {
        match item {
            Item::RecordDef(r) => {
                for f in &r.fields {
                    if f.name == word {
                        return Some(format!(
                            "**field** `{}` of type `{}`\n\nType: `{}`",
                            word,
                            r.name,
                            format_type_expr(&f.ty)
                        ));
                    }
                }
            }
            Item::InterfaceDef(i) => {
                for f in &i.fields {
                    if f.name == word {
                        return Some(format!(
                            "**field** `{}` of interface `{}`\n\nType: `{}`",
                            word,
                            i.name,
                            format_type_expr(&f.ty)
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// Extract the word at a given byte offset.
fn word_at_offset(source: &str, offset: usize) -> Option<String> {
    if offset > source.len() {
        return None;
    }
    let bytes = source.as_bytes();

    // Find start of word
    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    // Find end of word
    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(source[start..end].to_string())
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Return a brief description for a keyword.
fn keyword_description(word: &str) -> Option<&'static str> {
    match word {
        "type" => Some("Defines a record (product type) or choice (sum type)."),
        "interface" => Some("Defines an interface that types can implement."),
        "impl" => Some("Implements an interface for a type."),
        "for" => Some("Specifies the target type in an impl block."),
        "let" => Some("Binds a value to a name."),
        "import" => Some("Imports a module."),
        "if" => Some("Conditional expression."),
        "then" => Some("Specifies the consequent in an if expression."),
        "else" => Some("Specifies the alternative in an if expression."),
        "fn" => Some("Defines a function."),
        "map" => Some("Transforms each element of a collection."),
        "filter" => Some("Filters elements of a collection by a predicate."),
        "self" => Some("Refers to the current instance."),
        "Self" => Some("Refers to the current type."),
        "it" => Some("The implicit value being constrained."),
        "None" => Some("The absence of a value (sugar for Option::None)."),
        "true" => Some("Boolean true literal."),
        "false" => Some("Boolean false literal."),
        _ => None,
    }
}

// --- Rename helpers ---

/// Collect all byte ranges where the identifier `name` occurs in the AST.
fn find_all_occurrences(module: &Module, name: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    for item in &module.items {
        find_in_item(item, name, &mut ranges);
    }
    ranges
}

fn find_in_item(item: &Item, name: &str, ranges: &mut Vec<std::ops::Range<usize>>) {
    match item {
        Item::RecordDef(r) => {
            if r.name == name {
                // The name span covers just the name part of the definition
                let name_start = r.span.start + "type ".len();
                let name_end = name_start + r.name.len();
                ranges.push(name_start..name_end);
            }
            for f in &r.fields {
                find_in_field(f, name, ranges);
            }
        }
        Item::ChoiceDef(c) => {
            if c.name == name {
                let name_start = c.span.start + "type ".len();
                let name_end = name_start + c.name.len();
                ranges.push(name_start..name_end);
            }
            for v in &c.variants {
                if v.name == name {
                    ranges.push(v.span.start..v.span.start + v.name.len());
                }
                for ty in &v.fields {
                    find_in_type_expr(ty, name, ranges);
                }
            }
        }
        Item::InterfaceDef(i) => {
            if i.name == name {
                let name_start = i.span.start + "interface ".len();
                let name_end = name_start + i.name.len();
                ranges.push(name_start..name_end);
            }
            for f in &i.fields {
                if f.name == name {
                    ranges.push(f.span.start..f.span.start + f.name.len());
                }
                find_in_type_expr(&f.ty, name, ranges);
            }
        }
        Item::ImplBlock(ib) => {
            if ib.interface_name == name {
                let name_start = ib.span.start + "impl ".len();
                let name_end = name_start + ib.interface_name.len();
                ranges.push(name_start..name_end);
            }
            if ib.type_name == name {
                // "impl InterfaceName for TypeName" - find the type name after "for "
                let for_offset =
                    ib.span.start + "impl ".len() + ib.interface_name.len() + " for ".len();
                ranges.push(for_offset..for_offset + ib.type_name.len());
            }
            for m in &ib.mappings {
                if m.name == name {
                    ranges.push(m.span.start..m.span.start + m.name.len());
                }
                find_in_expr(&m.value, name, ranges);
            }
        }
        Item::LetBinding(lb) => {
            if lb.name == name {
                let name_start = lb.span.start + "let ".len();
                let name_end = name_start + lb.name.len();
                ranges.push(name_start..name_end);
            }
            if let Some(ref ty) = lb.ty {
                find_in_type_expr(ty, name, ranges);
            }
            find_in_expr(&lb.value, name, ranges);
        }
    }
}

fn find_in_field(field: &Field, name: &str, ranges: &mut Vec<std::ops::Range<usize>>) {
    if field.name == name {
        ranges.push(field.span.start..field.span.start + field.name.len());
    }
    find_in_type_expr(&field.ty, name, ranges);
}

fn find_in_type_expr(_ty: &TypeExpr, _name: &str, _ranges: &mut Vec<std::ops::Range<usize>>) {
    // TypeExpr doesn't carry spans currently, so we can't provide accurate ranges
    // for type references. This is a known limitation.
}

// Expr variants don't carry spans yet, so `ranges` is only passed through to recursive
// calls. Once spans are added to Expr, this function will push directly to `ranges`.
#[allow(clippy::only_used_in_recursion)]
fn find_in_expr(expr: &Expr, name: &str, ranges: &mut Vec<std::ops::Range<usize>>) {
    match expr {
        Expr::Ident(ident) if ident == name => {
            // Expr doesn't carry spans currently - known limitation
        }
        Expr::FieldAccess { object, field: _ } => {
            find_in_expr(object, name, ranges);
        }
        Expr::TypeConstruction {
            type_name: _,
            fields,
            as_interfaces,
        } => {
            for fi in fields {
                find_in_expr(&fi.value, name, ranges);
            }
            for ai in as_interfaces {
                for fi in &ai.fields {
                    find_in_expr(&fi.value, name, ranges);
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            find_in_expr(left, name, ranges);
            find_in_expr(right, name, ranges);
        }
        Expr::UnaryOp { operand, .. } => {
            find_in_expr(operand, name, ranges);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                find_in_expr(arg, name, ranges);
            }
        }
        Expr::VariantConstruction { args, .. } => {
            for arg in args {
                find_in_expr(arg, name, ranges);
            }
        }
        Expr::NamedConstruction { fields, .. } => {
            for fi in fields {
                find_in_expr(&fi.value, name, ranges);
            }
        }
        _ => {}
    }
}

// --- Diagnostic conversion ---

/// Convert a ParseError into an LSP Diagnostic.
fn parse_error_to_diagnostic(error: &parser::ParseError, source: &str) -> Diagnostic {
    let (message, pos) = match error {
        parser::ParseError::Lex(lex_err) => {
            let msg = lex_err.to_string();
            let pos = extract_lex_error_pos(lex_err);
            (msg, pos)
        }
        parser::ParseError::Expected {
            expected,
            found,
            pos,
        } => (format!("expected {expected}, found {found}"), *pos),
        parser::ParseError::UnexpectedEof { expected } => (
            format!("unexpected end of input, expected {expected}"),
            source.len().saturating_sub(1),
        ),
    };

    let position = offset_to_position(source, pos);
    Diagnostic {
        range: Range::new(position, position),
        severity: Some(DiagnosticSeverity::ERROR),
        message,
        ..Default::default()
    }
}

fn extract_lex_error_pos(err: &spin_lang::lexer::LexError) -> usize {
    match err {
        spin_lang::lexer::LexError::UnexpectedChar { pos, .. } => *pos,
        spin_lang::lexer::LexError::UnterminatedString { pos } => *pos,
    }
}

// --- LanguageServer implementation ---

#[tower_lsp::async_trait]
impl LanguageServer for SpinBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "spin-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.update_document(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // We use FULL sync, so there's exactly one change with the full text
        if let Some(change) = params.content_changes.into_iter().next() {
            self.update_document(uri, change.text).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.lock().unwrap();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(&doc.source, position);
        let context = detect_completion_context(&doc.source, offset);
        let items = build_completions(doc.module.as_ref(), context);

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.lock().unwrap();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(&doc.source, position);
        let content = hover_at(&doc.source, doc.module.as_ref(), offset);

        Ok(content.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text,
            }),
            range: None,
        }))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        let docs = self.documents.lock().unwrap();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(&doc.source, position);
        let word = word_at_offset(&doc.source, offset);

        Ok(word.map(|w| PrepareRenameResponse::RangeWithPlaceholder {
            range: word_range_at_offset(&doc.source, offset),
            placeholder: w,
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let docs = self.documents.lock().unwrap();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(&doc.source, position);
        let Some(old_name) = word_at_offset(&doc.source, offset) else {
            return Ok(None);
        };

        let Some(ref module) = doc.module else {
            return Ok(None);
        };

        let occurrences = find_all_occurrences(module, &old_name);
        if occurrences.is_empty() {
            return Ok(None);
        }

        let edits: Vec<TextEdit> = occurrences
            .iter()
            .map(|span| TextEdit {
                range: span_to_range(&doc.source, span),
                new_text: new_name.clone(),
            })
            .collect();

        let mut changes = HashMap::new();
        changes.insert(uri, edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }
}

/// Get the range of the word at the given offset.
fn word_range_at_offset(source: &str, offset: usize) -> Range {
    let bytes = source.as_bytes();
    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }
    Range::new(
        offset_to_position(source, start),
        offset_to_position(source, end),
    )
}

impl SpinBackend {
    async fn update_document(&self, uri: Url, text: String) {
        let (module, diagnostics) = match parser::parse(&text) {
            Ok(module) => (Some(module), vec![]),
            Err(err) => (None, vec![parse_error_to_diagnostic(&err, &text)]),
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        let mut docs = self.documents.lock().unwrap();
        docs.insert(
            uri,
            DocumentState {
                source: text,
                module,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Position/offset conversion tests ---

    #[test]
    fn offset_to_position_start_of_file() {
        let source = "hello\nworld";
        let pos = offset_to_position(source, 0);
        assert_eq!(pos, Position::new(0, 0));
    }

    #[test]
    fn offset_to_position_middle_of_first_line() {
        let source = "hello\nworld";
        let pos = offset_to_position(source, 3);
        assert_eq!(pos, Position::new(0, 3));
    }

    #[test]
    fn offset_to_position_start_of_second_line() {
        let source = "hello\nworld";
        let pos = offset_to_position(source, 6);
        assert_eq!(pos, Position::new(1, 0));
    }

    #[test]
    fn offset_to_position_middle_of_second_line() {
        let source = "hello\nworld";
        let pos = offset_to_position(source, 8);
        assert_eq!(pos, Position::new(1, 2));
    }

    #[test]
    fn offset_to_position_end_of_file() {
        let source = "hello\nworld";
        let pos = offset_to_position(source, 11);
        assert_eq!(pos, Position::new(1, 5));
    }

    #[test]
    fn offset_to_position_past_end_clamps() {
        let source = "hello";
        let pos = offset_to_position(source, 100);
        assert_eq!(pos, Position::new(0, 5));
    }

    #[test]
    fn position_to_offset_start() {
        let source = "hello\nworld";
        assert_eq!(position_to_offset(source, Position::new(0, 0)), 0);
    }

    #[test]
    fn position_to_offset_second_line() {
        let source = "hello\nworld";
        assert_eq!(position_to_offset(source, Position::new(1, 2)), 8);
    }

    #[test]
    fn position_to_offset_past_end() {
        let source = "hello";
        assert_eq!(position_to_offset(source, Position::new(0, 100)), 5);
    }

    // --- Word extraction tests ---

    #[test]
    fn word_at_offset_middle_of_word() {
        let source = "hello world";
        assert_eq!(word_at_offset(source, 2), Some("hello".to_string()));
    }

    #[test]
    fn word_at_offset_start_of_word() {
        let source = "hello world";
        assert_eq!(word_at_offset(source, 0), Some("hello".to_string()));
    }

    #[test]
    fn word_at_offset_between_words() {
        let source = " = ";
        // At offset 1, the '=' is not an ident char, and there are no ident chars adjacent
        assert_eq!(word_at_offset(source, 1), None);
    }

    #[test]
    fn word_at_offset_second_word() {
        let source = "hello world";
        assert_eq!(word_at_offset(source, 7), Some("world".to_string()));
    }

    // --- Completion context detection tests ---

    #[test]
    fn detect_context_after_dot() {
        let source = "self.";
        let ctx = detect_completion_context(source, 5);
        assert!(matches!(ctx, CompletionContext::AfterDot { prefix } if prefix == "self"));
    }

    #[test]
    fn detect_context_after_colon() {
        let source = "name:";
        let ctx = detect_completion_context(source, 5);
        assert!(matches!(ctx, CompletionContext::AfterColon));
    }

    #[test]
    fn detect_context_general() {
        let source = "let x = ";
        let ctx = detect_completion_context(source, 8);
        assert!(matches!(ctx, CompletionContext::General));
    }

    // --- Keyword completion tests ---

    #[test]
    fn keyword_completions_includes_expected_keywords() {
        let items = keyword_completions();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"type"));
        assert!(labels.contains(&"let"));
        assert!(labels.contains(&"import"));
        assert!(labels.contains(&"interface"));
        assert!(labels.contains(&"impl"));
        assert_eq!(items.len(), KEYWORDS.len());
    }

    #[test]
    fn keyword_completions_have_correct_kind() {
        let items = keyword_completions();
        for item in &items {
            assert_eq!(item.kind, Some(CompletionItemKind::KEYWORD));
        }
    }

    // --- Primitive type completion tests ---

    #[test]
    fn primitive_type_completions_includes_expected_types() {
        let items = primitive_type_completions();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"bool"));
        assert!(labels.contains(&"u32"));
        assert!(labels.contains(&"str"));
        assert!(labels.contains(&"f64"));
        assert_eq!(items.len(), PRIMITIVE_TYPES.len());
    }

    // --- Type name collection from parsed modules ---

    #[test]
    fn collect_type_names_from_parsed_module() {
        let source = "type Foo = x: u32;\ntype Bar = A | B;\ninterface Baz = y: str;";
        let module = parser::parse(source).unwrap();
        let names = collect_type_names(&module);
        assert_eq!(names, vec!["Foo", "Bar", "Baz"]);
    }

    #[test]
    fn user_type_completions_from_parsed_module() {
        let source = "type MyType = value: u32;";
        let module = parser::parse(source).unwrap();
        let items = user_type_completions(&module);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "MyType");
        assert_eq!(items[0].kind, Some(CompletionItemKind::CLASS));
    }

    // --- Field completion tests ---

    #[test]
    fn field_completions_for_record() {
        let source = "type Server = host: str, port: u16;";
        let module = parser::parse(source).unwrap();
        let items = field_completions(&module, "Server");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "host");
        assert_eq!(items[0].detail, Some("str".to_string()));
        assert_eq!(items[1].label, "port");
        assert_eq!(items[1].detail, Some("u16".to_string()));
    }

    #[test]
    fn field_completions_for_interface() {
        let source = "interface Endpoint = host: str, port: u16;";
        let module = parser::parse(source).unwrap();
        let items = field_completions(&module, "Endpoint");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "host");
        assert_eq!(items[1].label, "port");
    }

    #[test]
    fn field_completions_for_unknown_type() {
        let source = "type Server = host: str;";
        let module = parser::parse(source).unwrap();
        let items = field_completions(&module, "NonExistent");
        assert!(items.is_empty());
    }

    // --- Build completions integration tests ---

    #[test]
    fn build_completions_general_includes_keywords_and_types() {
        let source = "type Server = host: str;";
        let module = parser::parse(source).unwrap();
        let items = build_completions(Some(&module), CompletionContext::General);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Keywords
        assert!(labels.contains(&"type"));
        assert!(labels.contains(&"let"));
        // Primitive types
        assert!(labels.contains(&"bool"));
        assert!(labels.contains(&"str"));
        // User types
        assert!(labels.contains(&"Server"));
    }

    #[test]
    fn build_completions_after_colon_includes_types_only() {
        let source = "type Server = host: str;";
        let module = parser::parse(source).unwrap();
        let items = build_completions(Some(&module), CompletionContext::AfterColon);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Should have types but not keywords like "let"
        assert!(labels.contains(&"str"));
        assert!(labels.contains(&"Server"));
        assert!(!labels.contains(&"let"));
    }

    #[test]
    fn build_completions_after_dot_for_self() {
        let source = "type Server = host: str, port: u16;";
        let module = parser::parse(source).unwrap();
        let items = build_completions(
            Some(&module),
            CompletionContext::AfterDot {
                prefix: "self".to_string(),
            },
        );
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"host"));
        assert!(labels.contains(&"port"));
    }

    // --- Hover tests ---

    #[test]
    fn hover_on_keyword() {
        let source = "type Foo = x: u32;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 1); // "t|ype"
        let text = result.unwrap();
        assert!(text.contains("**type** (keyword)"));
        assert!(text.contains("Defines a record"));
    }

    #[test]
    fn hover_on_record_type_name() {
        let source = "type Server = host: str, port: u16;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 6); // "S|erver"
        let text = result.unwrap();
        assert!(text.contains("**type** Server"));
        assert!(text.contains("host: str"));
        assert!(text.contains("port: u16"));
    }

    #[test]
    fn hover_on_interface_name() {
        let source = "interface Endpoint = host: str;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 11); // "E|ndpoint"
        let text = result.unwrap();
        assert!(text.contains("**interface** Endpoint"));
        assert!(text.contains("host: str"));
    }

    #[test]
    fn hover_on_choice_type() {
        let source = "type Color = Red | Green | Blue;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 6); // "C|olor"
        let text = result.unwrap();
        assert!(text.contains("**type** Color (choice)"));
        assert!(text.contains("Red"));
        assert!(text.contains("Green"));
        assert!(text.contains("Blue"));
    }

    #[test]
    fn hover_on_field_name() {
        let source = "type Server = host: str, port: u16;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 15); // "h|ost"
        let text = result.unwrap();
        assert!(text.contains("**field** `host`"));
        assert!(text.contains("Type: `str`"));
    }

    #[test]
    fn hover_on_nothing() {
        let source = "type Foo = x: u32;";
        let module = parser::parse(source).unwrap();
        let result = hover_at(source, Some(&module), 9); // on "=" which is not an ident
        assert!(result.is_none());
    }

    // --- Format type expression tests ---

    #[test]
    fn format_type_expr_named() {
        assert_eq!(format_type_expr(&TypeExpr::Named("Foo".to_string())), "Foo");
    }

    #[test]
    fn format_type_expr_primitive() {
        assert_eq!(
            format_type_expr(&TypeExpr::Primitive(PrimitiveType::U32)),
            "u32"
        );
    }

    #[test]
    fn format_type_expr_path() {
        assert_eq!(
            format_type_expr(&TypeExpr::Path {
                module: "core".to_string(),
                name: "Port".to_string()
            }),
            "core::Port"
        );
    }

    #[test]
    fn format_type_expr_generic() {
        assert_eq!(
            format_type_expr(&TypeExpr::Generic {
                name: "Option".to_string(),
                args: vec![TypeExpr::Primitive(PrimitiveType::U32)],
            }),
            "Option<u32>"
        );
    }

    #[test]
    fn format_type_expr_unit() {
        assert_eq!(format_type_expr(&TypeExpr::Unit), "()");
    }

    // --- Diagnostic conversion tests ---

    #[test]
    fn parse_error_to_diagnostic_expected() {
        let err = parser::ParseError::Expected {
            expected: "identifier".to_string(),
            found: ";".to_string(),
            pos: 5,
        };
        let source = "type ;";
        let diag = parse_error_to_diagnostic(&err, source);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diag.message.contains("expected identifier"));
    }

    #[test]
    fn parse_error_to_diagnostic_unexpected_eof() {
        let err = parser::ParseError::UnexpectedEof {
            expected: "=".to_string(),
        };
        let source = "type Foo";
        let diag = parse_error_to_diagnostic(&err, source);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diag.message.contains("unexpected end of input"));
    }

    // --- Keyword description tests ---

    #[test]
    fn keyword_description_known() {
        assert!(keyword_description("type").is_some());
        assert!(keyword_description("let").is_some());
        assert!(keyword_description("import").is_some());
    }

    #[test]
    fn keyword_description_unknown() {
        assert!(keyword_description("foobar").is_none());
    }

    // --- find_let_binding_type tests ---

    #[test]
    fn find_let_binding_type_with_construction() {
        let source = "type Server = host: str;\nlet s = Server { host: \"localhost\" }";
        let module = parser::parse(source).unwrap();
        let result = find_let_binding_type(&module, "s");
        assert_eq!(result, Some("Server".to_string()));
    }

    #[test]
    fn find_let_binding_type_not_found() {
        let source = "type Server = host: str;";
        let module = parser::parse(source).unwrap();
        let result = find_let_binding_type(&module, "nonexistent");
        assert_eq!(result, None);
    }

    // --- Span to range conversion tests ---

    #[test]
    fn span_to_range_single_line() {
        let source = "type Foo = x: u32;";
        let range = span_to_range(source, &(5..8));
        assert_eq!(range.start, Position::new(0, 5));
        assert_eq!(range.end, Position::new(0, 8));
    }

    #[test]
    fn span_to_range_multi_line() {
        let source = "hello\nworld";
        let range = span_to_range(source, &(0..8));
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(1, 2));
    }
}

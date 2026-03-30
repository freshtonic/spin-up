use spin_up::analysis::registry::TypeRegistry;
use spin_up::analysis::unify::unify;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::parser;

#[test]
fn test_unify_complete_impl() {
    let source = r#"
interface Endpoint = host: str, port: u16;
type Server = hostname: str, port_num: u16;
impl Endpoint for Server {
  host: self.hostname,
  port: self.port_num,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "expected no errors, got: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_missing_required_field() {
    let source = r#"
interface Endpoint = host: str, port: u16;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::MissingField { field, .. } if field == "port"
    ));
}

#[test]
fn test_unify_optional_field_can_be_omitted() {
    let source = r#"
interface Endpoint = host: str, tls: Option<TlsConfig>;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "Option fields should be omittable: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_default_field_can_be_omitted() {
    let source = r#"
interface Endpoint =
  host: str,
  #[default("localhost")]
  fallback: str,
;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "Default fields should be omittable: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_unknown_interface_error() {
    let source = r#"
type Server = hostname: str;
impl NonExistent for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::UnknownInterface { name } if name == "NonExistent"
    ));
}

#[test]
fn test_unify_type_mismatch_in_impl() {
    let source = r#"
interface Endpoint = host: str;
type Server = hostname: u32;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::TypeMismatch { expected, found }
        if expected == "str" && found == "u32"
    ));
}

#[test]
fn test_unify_compatible_types_in_impl() {
    let source = r#"
interface Endpoint = host: str, port: u16;
type Server = hostname: str, port_num: u16;
impl Endpoint for Server {
  host: self.hostname,
  port: self.port_num,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "expected no errors, got: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_string_literal_matches_str() {
    let source = r#"
interface Endpoint = host: str;
type Server = x: u32;
impl Endpoint for Server {
  host: "localhost",
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "string literal should match str: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_bool_literal_mismatch() {
    let source = r#"
interface Endpoint = host: str;
type Server = x: u32;
impl Endpoint for Server {
  host: true,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok(), "bool literal should not match str");
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::TypeMismatch { expected, found }
        if expected == "str" && found == "bool"
    ));
}

#[test]
fn test_unify_none_matches_option() {
    let source = r#"
interface Endpoint = tls: Option<TlsConfig>;
type Server = x: u32;
impl Endpoint for Server {
  tls: None,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "None should match Option<T>: {:?}",
        diags.errors()
    );
}

#[test]
fn test_unify_chained_field_access() {
    let source = r#"
type Inner = value: str;
type Server = inner: Inner;
interface Endpoint = host: str;
impl Endpoint for Server {
  host: self.inner.value,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "chained field access should resolve: {:?}",
        diags.errors()
    );
}

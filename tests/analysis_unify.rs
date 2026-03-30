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

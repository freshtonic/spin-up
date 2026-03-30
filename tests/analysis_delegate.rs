use spin_up::analysis::delegate::check_delegates;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::parser;

#[test]
fn valid_delegate_produces_no_errors() {
    let source = r#"
interface Endpoint = host: str, port: u16;

#[delegate(Endpoint)]
type Proxy =
  #[target(Endpoint)]
  frontend: Endpoint,
  backend: Endpoint,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(
        diags.is_ok(),
        "valid delegate should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn delegate_without_target_emits_invalid_delegate() {
    let source = r#"
interface Endpoint = host: str;

#[delegate(Endpoint)]
type Proxy =
  frontend: Endpoint,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidDelegate { .. }
    ));
}

#[test]
fn delegate_with_unknown_interface_emits_unknown_interface() {
    let source = r#"
#[delegate(NonExistent)]
type Proxy =
  #[target(NonExistent)]
  frontend: str,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::UnknownInterface { .. }
    ));
}

#[test]
fn type_without_delegates_produces_no_errors() {
    let source = "type Foo = x: u32;";
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(diags.is_ok());
}

#[test]
fn multiple_target_fields_for_same_delegate_emits_invalid_delegate() {
    let source = r#"
interface Endpoint = host: str;

#[delegate(Endpoint)]
type Proxy =
  #[target(Endpoint)]
  frontend: Endpoint,
  #[target(Endpoint)]
  backend: Endpoint,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidDelegate { .. }
    ));
}

#[test]
fn target_field_type_mismatch_emits_invalid_delegate() {
    let source = r#"
interface Endpoint = host: str;

#[delegate(Endpoint)]
type Proxy =
  #[target(Endpoint)]
  frontend: u32,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidDelegate { .. }
    ));
}

#[test]
fn target_without_delegate_emits_invalid_delegate() {
    let source = r#"
interface Endpoint = host: str;

type Proxy =
  #[target(Endpoint)]
  frontend: Endpoint,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidDelegate { .. }
    ));
}

#[test]
fn delegate_with_impl_block_for_field_type_is_valid() {
    let source = r#"
interface Endpoint = host: str;
type MyFrontend = host: str;
impl Endpoint for MyFrontend {
  host: self.host,
}

#[delegate(Endpoint)]
type Proxy =
  #[target(Endpoint)]
  frontend: MyFrontend,
;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_delegates(&registry);
    assert!(
        diags.is_ok(),
        "delegate with impl block should pass: {:?}",
        diags.errors()
    );
}

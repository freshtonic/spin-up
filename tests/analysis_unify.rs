use spin_up::analysis::registry::TypeRegistry;
use spin_up::analysis::unify::unify;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn test_unify_complete_impl() {
    let module = spin! {
        interface Endpoint = host: string, port: number;
        type Server = hostname: string, port_num: number;
        impl Endpoint for Server {
            host: self.hostname,
            port: self.port_num,
        }
    };
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
    let module = spin! {
        interface Endpoint = host: string, port: number;
        type Server = hostname: string;
        impl Endpoint for Server {
            host: self.hostname,
        }
    };
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
    let module = spin! {
        interface Endpoint = host: string, tls: Option<TlsConfig>;
        type Server = hostname: string;
        impl Endpoint for Server {
            host: self.hostname,
        }
    };
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
    let module = spin! {
        interface Endpoint =
            host: string,
            #[default("localhost")]
            fallback: string,
        ;
        type Server = hostname: string;
        impl Endpoint for Server {
            host: self.hostname,
        }
    };
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
    let module = spin! {
        type Server = hostname: string;
        impl NonExistent for Server {
            host: self.hostname,
        }
    };
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
    let module = spin! {
        interface Endpoint = host: string;
        type Server = hostname: number;
        impl Endpoint for Server {
            host: self.hostname,
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::TypeMismatch { expected, found }
        if expected == "string" && found == "number"
    ));
}

#[test]
fn test_unify_compatible_types_in_impl() {
    let module = spin! {
        interface Endpoint = host: string, port: number;
        type Server = hostname: string, port_num: number;
        impl Endpoint for Server {
            host: self.hostname,
            port: self.port_num,
        }
    };
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
    let module = spin! {
        interface Endpoint = host: string;
        type Server = x: number;
        impl Endpoint for Server {
            host: "localhost",
        }
    };
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
    let module = spin! {
        interface Endpoint = host: string;
        type Server = x: number;
        impl Endpoint for Server {
            host: true,
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok(), "bool literal should not match str");
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::TypeMismatch { expected, found }
        if expected == "string" && found == "bool"
    ));
}

#[test]
fn test_unify_none_matches_option() {
    let module = spin! {
        interface Endpoint = tls: Option<TlsConfig>;
        type Server = x: number;
        impl Endpoint for Server {
            tls: None,
        }
    };
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
    let module = spin! {
        type Inner = value: string;
        type Server = inner: Inner;
        interface Endpoint = host: string;
        impl Endpoint for Server {
            host: self.inner.value,
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(
        diags.is_ok(),
        "chained field access should resolve: {:?}",
        diags.errors()
    );
}

use spin_up::ast::{Expr, Item};
use spin_up::spin;

#[test]
fn test_spin_macro_type_def() {
    let module = spin! {
        type Foo = x: number, y: string;
    };
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Foo");
            assert_eq!(r.fields.len(), 2);
            assert_eq!(r.fields[0].name, "x");
            assert_eq!(r.fields[1].name, "y");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_spin_macro_interface_and_impl() {
    let module = spin! {
        interface Endpoint = host: string, port: number;
        type Server = hostname: string, port_num: number;
        impl Endpoint for Server {
            host: self.hostname,
            port: self.port_num,
        }
    };
    assert_eq!(module.items.len(), 3);
    assert!(matches!(&module.items[0], Item::InterfaceDef(_)));
    assert!(matches!(&module.items[1], Item::RecordDef(_)));
    assert!(matches!(&module.items[2], Item::ImplBlock(_)));
}

#[test]
fn test_spin_macro_let_binding() {
    let module = spin! {
        let x = 42
    };
    assert_eq!(module.items.len(), 1);
    assert!(matches!(&module.items[0], Item::LetBinding(_)));
}

#[test]
fn test_spin_macro_complex() {
    let module = spin! {
        type Inner = value: string;
        type Server = inner: Inner;
        interface Endpoint = host: string;
        impl Endpoint for Server {
            host: self.inner.value,
        }
        let server = Server {
            inner: Inner {
                value: "localhost",
            },
        }
    };
    assert_eq!(module.items.len(), 5);
}

#[test]
fn test_spin_macro_produces_same_result_as_parse() {
    let module_from_macro = spin! {
        type Foo = x: number;
    };
    let module_from_parse = spin_up::parser::parse("type Foo = x: number;").unwrap();

    // Both should produce a single RecordDef with the same name and fields
    assert_eq!(module_from_macro.items.len(), module_from_parse.items.len());
    match (&module_from_macro.items[0], &module_from_parse.items[0]) {
        (Item::RecordDef(a), Item::RecordDef(b)) => {
            assert_eq!(a.name, b.name);
            assert_eq!(a.fields.len(), b.fields.len());
            assert_eq!(a.fields[0].name, b.fields[0].name);
        }
        _ => panic!("expected both to be RecordDef"),
    }
}

// --- Tests proving proc-macro removes previous limitations ---

#[test]
fn test_spin_macro_with_attributes() {
    let module = spin! {
        #[lang-item]
        type Foo = x: number;
    };
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert!(r.attributes[0].args.is_none());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_spin_macro_with_attribute_args() {
    let module = spin! {
        #[delegate(PostgresEndpoint)]
        type Proxy = frontend: string;
    };
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "delegate");
            assert_eq!(r.attributes[0].args.as_deref(), Some("PostgresEndpoint"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_spin_macro_with_string_interpolation() {
    let module = spin! {
        let x = "hello ${name}"
    };
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert!(
                matches!(&l.value, Expr::StringInterpolation(_)),
                "expected StringInterpolation, got {:?}",
                l.value
            );
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_spin_macro_with_dotted_interpolation() {
    let module = spin! {
        let x = "host: ${postgres.host}"
    };
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert!(
                matches!(&l.value, Expr::StringInterpolation(_)),
                "expected StringInterpolation, got {:?}",
                l.value
            );
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

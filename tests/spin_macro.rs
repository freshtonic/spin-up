use spin_up::ast::Item;
use spin_up::spin;

#[test]
fn test_spin_macro_type_def() {
    let module = spin! {
        type Foo = x: u32, y: str;
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
        interface Endpoint = host: str, port: u16;
        type Server = hostname: str, port_num: u16;
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
        type Inner = value: str;
        type Server = inner: Inner;
        interface Endpoint = host: str;
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
        type Foo = x: u32;
    };
    let module_from_parse = spin_up::parser::parse("type Foo = x: u32;").unwrap();

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

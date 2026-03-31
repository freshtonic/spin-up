use spin_up::analysis::as_interface::check_as_interfaces;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn valid_as_interface() {
    let module = spin! {
        interface Endpoint = port: number;
        type Server = name: string;
        impl Endpoint for Server {
            port: 8080,
        }
        let x = Server {
            name: "foo",
            <as Endpoint> {
                port: 9090,
            }
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_as_interfaces(&registry);
    assert!(
        diags.is_ok(),
        "valid as-interface should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn as_interface_unknown_interface() {
    let module = spin! {
        type Server = name: string;
        let x = Server {
            name: "foo",
            <as NonExistent> {
                port: 9090,
            }
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_as_interfaces(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::UnknownInterface { name } if name == "NonExistent"
    ));
}

#[test]
fn as_interface_type_does_not_implement() {
    let module = spin! {
        interface Endpoint = port: number;
        type Server = name: string;
        let x = Server {
            name: "foo",
            <as Endpoint> {
                port: 9090,
            }
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_as_interfaces(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidAsInterface { type_name, interface }
            if type_name == "Server" && interface == "Endpoint"
    ));
}

#[test]
fn no_as_interfaces_passes() {
    let module = spin! {
        type Server = name: string;
        let x = Server { name: "foo" }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_as_interfaces(&registry);
    assert!(diags.is_ok());
}

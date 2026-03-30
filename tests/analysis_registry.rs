use spin_up::analysis::registry::TypeRegistry;
use spin_up::spin;

#[test]
fn test_registry_registers_type() {
    let module = spin! { type Foo = x: u32; };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_type("Foo").is_some());
}

#[test]
fn test_registry_registers_interface() {
    let module = spin! { interface Bar = x: u32; };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_interface("Bar").is_some());
}

#[test]
fn test_registry_registers_let_binding() {
    let module = spin! { let x = 42 };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_binding("x").is_some());
}

#[test]
fn test_registry_registers_impl() {
    let module = spin! {
        impl Foo for Bar {
            x: self.x,
        }
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    let impls = registry.lookup_impls_for_type("Bar");
    assert_eq!(impls.len(), 1);
}

#[test]
fn test_registry_unknown_type_returns_none() {
    let registry = TypeRegistry::new();
    assert!(registry.lookup_type("Unknown").is_none());
}

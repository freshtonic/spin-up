use spin_up::builtins;
use spin_up::parser;

#[test]
fn test_builtin_module_exists() {
    let source = builtins::get_module_source("spin-core-net");
    assert!(source.is_some());
}

#[test]
fn test_builtin_module_not_found() {
    let source = builtins::get_module_source("spin-core-nonexistent");
    assert!(source.is_none());
}

#[test]
fn test_builtin_spin_core_net_parses() {
    let source = builtins::get_module_source("spin-core-net").unwrap();
    let module = parser::parse(source).unwrap();
    // Should have 6 items: IpAddrV4, IpAddrV6, IpAddr, SocketAddrV4, SocketAddrV6, SocketAddr
    assert_eq!(module.items.len(), 6);
}

#[test]
fn test_builtin_module_names_list() {
    let names = builtins::builtin_module_names();
    assert!(names.contains(&"spin-core-net"));
}

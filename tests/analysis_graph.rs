use spin_up::analysis::graph::build_dependency_graph;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn test_graph_simple_dependency() {
    let module = spin! {
        type Database = port: u16;
        let db = Database { port: 5432 }
        let app = MyApp { database: db }
        type MyApp = database: Database;
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let graph = build_dependency_graph(&registry);
    assert!(
        graph.diagnostics.is_ok(),
        "expected no errors, got: {:?}",
        graph.diagnostics.errors()
    );
    let order = graph.topological_order();
    // db should come before app
    let db_pos = order.iter().position(|n| n == "db").unwrap();
    let app_pos = order.iter().position(|n| n == "app").unwrap();
    assert!(db_pos < app_pos);
}

#[test]
fn test_graph_detects_cycle() {
    let module = spin! {
        let a = Foo { dep: b }
        let b = Bar { dep: a }
        type Foo = dep: Bar;
        type Bar = dep: Foo;
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let graph = build_dependency_graph(&registry);
    assert!(!graph.diagnostics.is_ok());
    assert!(matches!(
        &graph.diagnostics.errors()[0].kind,
        DiagnosticKind::CyclicDependency { .. }
    ));
}

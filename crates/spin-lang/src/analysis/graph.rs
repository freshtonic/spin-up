use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::Expr;
use crate::diagnostics::{DiagnosticKind, Diagnostics};

use super::registry::TypeRegistry;

/// A dependency graph built from let-bindings.
///
/// Nodes are binding names; directed edges point from a binding to the
/// bindings it depends on. A valid graph is a DAG whose topological
/// order gives a safe evaluation sequence.
#[derive(Debug)]
pub struct DependencyGraph {
    /// Adjacency list: node -> set of nodes it depends on.
    edges: HashMap<String, HashSet<String>>,
    /// All node names (binding names), preserving insertion order is not
    /// required — topological sort produces the canonical order.
    nodes: HashSet<String>,
    /// Topological order computed during construction (empty when a cycle
    /// is detected).
    topo_order: Vec<String>,
    /// Diagnostics collected during graph construction.
    pub diagnostics: Diagnostics,
}

impl DependencyGraph {
    /// Return all node names (binding names) in the graph.
    pub fn nodes(&self) -> &HashSet<String> {
        &self.nodes
    }

    /// Return the adjacency list: each node maps to the set of bindings
    /// it depends on.
    pub fn edges(&self) -> &HashMap<String, HashSet<String>> {
        &self.edges
    }

    /// Return the topological order of bindings (dependencies before
    /// dependents). Empty when the graph contains a cycle.
    pub fn topological_order(&self) -> &[String] {
        &self.topo_order
    }
}

/// Build a dependency graph from all let-bindings in the registry.
///
/// For each binding the expression tree is walked to discover references
/// to other known bindings (via `Expr::Ident`). A topological sort is
/// then attempted; if the graph contains a cycle a `CyclicDependency`
/// diagnostic is emitted.
pub fn build_dependency_graph(registry: &TypeRegistry) -> DependencyGraph {
    let bindings = registry.all_bindings();
    let binding_names: HashSet<String> = bindings.keys().cloned().collect();

    let mut edges: HashMap<String, HashSet<String>> = HashMap::new();
    let mut nodes: HashSet<String> = HashSet::new();

    for (name, binding) in bindings {
        nodes.insert(name.clone());
        let mut deps = HashSet::new();
        collect_ident_refs(&binding.value, &binding_names, &mut deps);
        edges.insert(name.clone(), deps);
    }

    let mut diagnostics = Diagnostics::new();
    let topo_order = topological_sort(&nodes, &edges, &mut diagnostics);

    DependencyGraph {
        edges,
        nodes,
        topo_order,
        diagnostics,
    }
}

/// Recursively walk an expression tree and collect every `Expr::Ident`
/// whose name matches a known binding.
fn collect_ident_refs(expr: &Expr, known: &HashSet<String>, out: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name) => {
            if known.contains(name) {
                out.insert(name.clone());
            }
        }
        Expr::TypeConstruction {
            fields,
            as_interfaces,
            ..
        } => {
            for field in fields {
                collect_ident_refs(&field.value, known, out);
            }
            for block in as_interfaces {
                for field in &block.fields {
                    collect_ident_refs(&field.value, known, out);
                }
            }
        }
        Expr::VariantConstruction { args, .. } => {
            for arg in args {
                collect_ident_refs(arg, known, out);
            }
        }
        Expr::NamedConstruction { fields, .. } => {
            for field in fields {
                collect_ident_refs(&field.value, known, out);
            }
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_ident_refs(arg, known, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_ident_refs(left, known, out);
            collect_ident_refs(right, known, out);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_ident_refs(operand, known, out);
        }
        Expr::FieldAccess { object, .. } => {
            collect_ident_refs(object, known, out);
        }
        // Leaf nodes that cannot contain binding references.
        Expr::StringLit(_)
        | Expr::Number(_)
        | Expr::BoolLit(_)
        | Expr::It
        | Expr::Self_
        | Expr::None_ => {}
    }
}

/// Kahn's algorithm for topological sorting.
///
/// Returns the sorted order on success. On cycle detection, emits a
/// `CyclicDependency` diagnostic and returns a partial (empty) order.
fn topological_sort(
    nodes: &HashSet<String>,
    edges: &HashMap<String, HashSet<String>>,
    diagnostics: &mut Diagnostics,
) -> Vec<String> {
    // in_degree[node] = number of nodes that depend on `node`
    // Wait — Kahn's uses *incoming* edges. Our `edges` map is
    // node -> {things node depends on}, i.e. node -> its *dependencies*.
    // An edge from A to B means "A depends on B", so B must come first.
    // In Kahn's terms: B -> A is the direction of the DAG edge (B before A).
    // in_degree of A = number of dependencies A has.

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    // reverse_edges: dependency -> vec of dependents
    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in nodes {
        in_degree.entry(node.as_str()).or_insert(0);
    }

    for (node, deps) in edges {
        *in_degree.entry(node.as_str()).or_insert(0) += deps.len();
        for dep in deps {
            reverse.entry(dep.as_str()).or_default().push(node.as_str());
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    for (node, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(node);
        }
    }

    // Sort the initial queue for deterministic output.
    let mut sorted_start: Vec<&str> = queue.drain(..).collect();
    sorted_start.sort();
    for item in sorted_start {
        queue.push_back(item);
    }

    let mut order: Vec<String> = Vec::new();

    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        if let Some(dependents) = reverse.get(node) {
            let mut sorted_dependents: Vec<&str> = dependents.clone();
            sorted_dependents.sort();
            for dependent in sorted_dependents {
                let deg = in_degree.get_mut(dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(dependent);
                }
            }
        }
    }

    if order.len() != nodes.len() {
        // Cycle detected — collect the nodes that were not placed.
        let placed: HashSet<&str> = order.iter().map(String::as_str).collect();
        let cycle_members: Vec<String> = nodes
            .iter()
            .filter(|n| !placed.contains(n.as_str()))
            .cloned()
            .collect();

        diagnostics.error(
            DiagnosticKind::CyclicDependency {
                cycle: cycle_members,
            },
            0..0,
            "<dependency-graph>",
        );

        // Return empty order — the graph is invalid.
        return Vec::new();
    }

    order
}

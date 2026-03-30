use std::collections::HashMap;

use crate::ast::{ChoiceDef, ImplBlock, InterfaceDef, Item, LetBinding, Module, RecordDef};

/// Distinguishes record types from choice (sum) types in the registry.
#[derive(Debug, Clone)]
pub enum TypeDef {
    Record(RecordDef),
    Choice(ChoiceDef),
}

/// Stores all known types, interfaces, let-bindings, and impl blocks
/// discovered during module registration.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    types: HashMap<String, TypeDef>,
    interfaces: HashMap<String, InterfaceDef>,
    bindings: HashMap<String, LetBinding>,
    impls: Vec<ImplBlock>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Walk every item in `module` and register it by name.
    pub fn register_module(&mut self, _name: &str, module: &Module) {
        for item in &module.items {
            match item {
                Item::RecordDef(r) => {
                    self.types
                        .insert(r.name.clone(), TypeDef::Record(r.clone()));
                }
                Item::ChoiceDef(c) => {
                    self.types
                        .insert(c.name.clone(), TypeDef::Choice(c.clone()));
                }
                Item::InterfaceDef(i) => {
                    self.interfaces.insert(i.name.clone(), i.clone());
                }
                Item::ImplBlock(imp) => {
                    self.impls.push(imp.clone());
                }
                Item::LetBinding(lb) => {
                    self.bindings.insert(lb.name.clone(), lb.clone());
                }
            }
        }
    }

    pub fn lookup_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    pub fn lookup_interface(&self, name: &str) -> Option<&InterfaceDef> {
        self.interfaces.get(name)
    }

    pub fn lookup_binding(&self, name: &str) -> Option<&LetBinding> {
        self.bindings.get(name)
    }

    /// Return all impl blocks in the registry.
    pub fn all_impls(&self) -> &[ImplBlock] {
        &self.impls
    }

    /// Return all let-bindings in the registry.
    pub fn all_bindings(&self) -> &HashMap<String, LetBinding> {
        &self.bindings
    }

    /// Return all impl blocks whose implementing type matches `type_name`.
    pub fn lookup_impls_for_type(&self, type_name: &str) -> Vec<&ImplBlock> {
        self.impls
            .iter()
            .filter(|imp| imp.type_name == type_name)
            .collect()
    }
}

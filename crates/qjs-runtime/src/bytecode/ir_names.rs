use std::collections::BTreeSet;

use super::ir::{ClassElementDef, ClassPrivateElementDef, Op};

pub(super) fn collect_nested_global_names_from_ops(code: &[Op], names: &mut BTreeSet<String>) {
    for op in code {
        match op {
            Op::NewFunction { bytecode, .. } => {
                for name in bytecode.closure_referenced_global_names() {
                    names.insert(name);
                }
            }
            Op::NewClass {
                constructor,
                elements,
                ..
            } => {
                for name in constructor.bytecode.closure_referenced_global_names() {
                    names.insert(name);
                }
                for element in elements {
                    collect_class_element_global_names(element, names);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn collect_nested_written_binding_names_from_ops(
    code: &[Op],
    names: &mut BTreeSet<String>,
) {
    for op in code {
        match op {
            Op::NewFunction { bytecode, .. } => {
                for name in bytecode.closure_written_binding_names() {
                    names.insert(name);
                }
            }
            Op::NewClass {
                constructor,
                elements,
                ..
            } => {
                for name in constructor.bytecode.closure_written_binding_names() {
                    names.insert(name);
                }
                for element in elements {
                    collect_class_element_written_names(element, names);
                }
            }
            _ => {}
        }
    }
}

fn collect_class_element_global_names(element: &ClassElementDef, names: &mut BTreeSet<String>) {
    match element {
        ClassElementDef::Method(def) => {
            for name in def.bytecode.closure_referenced_global_names() {
                names.insert(name);
            }
        }
        ClassElementDef::Field(def) => {
            if let Some(initializer) = &def.initializer {
                for name in initializer.bytecode.closure_referenced_global_names() {
                    names.insert(name);
                }
            }
        }
        ClassElementDef::Private(def) => collect_private_class_element_global_names(def, names),
        ClassElementDef::StaticBlock(def) => {
            for name in def.bytecode.closure_referenced_global_names() {
                names.insert(name);
            }
        }
    }
}

fn collect_private_class_element_global_names(
    element: &ClassPrivateElementDef,
    names: &mut BTreeSet<String>,
) {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(initializer) = initializer {
                for name in initializer.bytecode.closure_referenced_global_names() {
                    names.insert(name);
                }
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => {
            for name in def.bytecode.closure_referenced_global_names() {
                names.insert(name);
            }
        }
    }
}

fn collect_class_element_written_names(element: &ClassElementDef, names: &mut BTreeSet<String>) {
    match element {
        ClassElementDef::Method(def) => {
            for name in def.bytecode.closure_written_binding_names() {
                names.insert(name);
            }
        }
        ClassElementDef::Field(def) => {
            if let Some(initializer) = &def.initializer {
                for name in initializer.bytecode.closure_written_binding_names() {
                    names.insert(name);
                }
            }
        }
        ClassElementDef::Private(def) => collect_private_class_element_written_names(def, names),
        ClassElementDef::StaticBlock(def) => {
            for name in def.bytecode.closure_written_binding_names() {
                names.insert(name);
            }
        }
    }
}

fn collect_private_class_element_written_names(
    element: &ClassPrivateElementDef,
    names: &mut BTreeSet<String>,
) {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(initializer) = initializer {
                for name in initializer.bytecode.closure_written_binding_names() {
                    names.insert(name);
                }
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => {
            for name in def.bytecode.closure_written_binding_names() {
                names.insert(name);
            }
        }
    }
}

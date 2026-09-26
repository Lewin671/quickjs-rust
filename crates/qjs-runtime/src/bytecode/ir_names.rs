use std::collections::BTreeSet;

use super::ir::{ClassElementDef, ClassPrivateElementDef, Op};

pub(super) fn collect_nested_global_names_from_ops<'a>(
    code: &'a [Op],
    names: &mut BTreeSet<&'a str>,
) {
    for op in code {
        match op {
            Op::NewFunction { bytecode, .. } => {
                for name in bytecode.closure_referenced_global_names_ref() {
                    names.insert(name.as_str());
                }
            }
            Op::NewClass { definition } => {
                for name in definition
                    .constructor
                    .bytecode
                    .closure_referenced_global_names_ref()
                {
                    names.insert(name.as_str());
                }
                for element in &definition.elements {
                    collect_class_element_global_names(element, names);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn collect_nested_written_binding_names_from_ops<'a>(
    code: &'a [Op],
    names: &mut BTreeSet<&'a str>,
) {
    for op in code {
        match op {
            Op::NewFunction { bytecode, .. } => {
                for name in bytecode.closure_written_binding_names_ref() {
                    names.insert(name.as_str());
                }
            }
            Op::NewClass { definition } => {
                for name in definition
                    .constructor
                    .bytecode
                    .closure_written_binding_names_ref()
                {
                    names.insert(name.as_str());
                }
                for element in &definition.elements {
                    collect_class_element_written_names(element, names);
                }
            }
            _ => {}
        }
    }
}

fn collect_class_element_global_names<'a>(
    element: &'a ClassElementDef,
    names: &mut BTreeSet<&'a str>,
) {
    match element {
        ClassElementDef::Method(def) => {
            for name in def.bytecode.closure_referenced_global_names_ref() {
                names.insert(name.as_str());
            }
        }
        ClassElementDef::Field(def) => {
            if let Some(initializer) = &def.initializer {
                for name in initializer.bytecode.closure_referenced_global_names_ref() {
                    names.insert(name.as_str());
                }
            }
        }
        ClassElementDef::Private(def) => collect_private_class_element_global_names(def, names),
        ClassElementDef::StaticBlock(def) => {
            for name in def.bytecode.closure_referenced_global_names_ref() {
                names.insert(name.as_str());
            }
        }
    }
}

fn collect_private_class_element_global_names<'a>(
    element: &'a ClassPrivateElementDef,
    names: &mut BTreeSet<&'a str>,
) {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(initializer) = initializer {
                for name in initializer.bytecode.closure_referenced_global_names_ref() {
                    names.insert(name.as_str());
                }
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => {
            for name in def.bytecode.closure_referenced_global_names_ref() {
                names.insert(name.as_str());
            }
        }
    }
}

fn collect_class_element_written_names<'a>(
    element: &'a ClassElementDef,
    names: &mut BTreeSet<&'a str>,
) {
    match element {
        ClassElementDef::Method(def) => {
            for name in def.bytecode.closure_written_binding_names_ref() {
                names.insert(name.as_str());
            }
        }
        ClassElementDef::Field(def) => {
            if let Some(initializer) = &def.initializer {
                for name in initializer.bytecode.closure_written_binding_names_ref() {
                    names.insert(name.as_str());
                }
            }
        }
        ClassElementDef::Private(def) => collect_private_class_element_written_names(def, names),
        ClassElementDef::StaticBlock(def) => {
            for name in def.bytecode.closure_written_binding_names_ref() {
                names.insert(name.as_str());
            }
        }
    }
}

fn collect_private_class_element_written_names<'a>(
    element: &'a ClassPrivateElementDef,
    names: &mut BTreeSet<&'a str>,
) {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(initializer) = initializer {
                for name in initializer.bytecode.closure_written_binding_names_ref() {
                    names.insert(name.as_str());
                }
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => {
            for name in def.bytecode.closure_written_binding_names_ref() {
                names.insert(name.as_str());
            }
        }
    }
}

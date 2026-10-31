//! Arguments demand follows lexical arrows and the scopes at each operation.
//! Ordinary children, fields and static blocks own separate argument boundaries;
//! an unrelated child's eval never creates an arguments object in its parent.

use super::{compiler::Compiler, ir::Op};

impl Compiler {
    pub(super) fn arguments_demand(&self) -> bool {
        self.code.iter().enumerate().any(|(pc, op)| {
            let observes = match op {
                Op::CallDirectEval { .. } | Op::CallDirectEvalSpread { .. } => true,
                Op::LoadGlobal(name)
                | Op::TypeofGlobal(name)
                | Op::StoreGlobalStrict(name)
                | Op::DeleteIdent(name) => name == "arguments",
                Op::StoreGlobalSloppy { name, .. }
                | Op::StoreLocalOrGlobalSloppy { name, .. }
                | Op::LoadIdentWith { name, .. }
                | Op::LoadResolvedIdentWith { name, .. }
                | Op::TypeofIdentWith { name, .. }
                | Op::DeleteIdentWith { name, .. } => name == "arguments",
                Op::LoadLocal(slot)
                | Op::LoadLocalOrUndefined(slot)
                | Op::StoreLocal(slot)
                | Op::AssignLocal(slot) => self
                    .locals
                    .get(*slot)
                    .is_some_and(|local| local.name == "arguments"),
                Op::NewFunction {
                    lexical_arguments: true,
                    bytecode,
                    ..
                } => bytecode.binding_plan.needs_lexical_arguments,
                _ => false,
            };
            observes && self.arguments_reaches_owner(pc)
        })
    }

    pub(super) fn ensure_implicit_arguments(&mut self) -> usize {
        if let Some(slot) = self.scope_plan.implicit_arguments {
            return slot.0;
        }
        let own_var = self
            .local_slots
            .get("arguments")
            .copied()
            .filter(|&slot| self.locals[slot].hoisted && self.scope_plan.variable.0 == 0);
        let slot = own_var.unwrap_or_else(|| self.local_slot("\0implicit_arguments", false));
        self.locals[slot].from_env = false;
        let variable = self.scope_plan.variable;
        self.scope_plan.variable = crate::binding::ScopeId(0);
        self.scope_plan.declare("arguments", slot, false);
        self.scope_plan.variable = variable;
        self.scope_plan.implicit_arguments = Some(crate::binding::LocalSlot(slot));
        slot
    }

    /// Late demand may synthesize the implicit slot after earlier references
    /// were lowered as free names. Wire those operations without changing PCs.
    pub(super) fn wire_arguments_slot(&mut self) {
        let Some(slot) = self.scope_plan.implicit_arguments else {
            return;
        };
        for op in &mut self.code {
            match op {
                Op::LoadGlobal(name) if name == "arguments" => *op = Op::LoadLocal(slot.0),
                Op::TypeofGlobal(name) if name == "arguments" => {
                    *op = Op::TypeofIdentWith {
                        name: name.clone(),
                        slot: Some(slot.0),
                    };
                }
                Op::DeleteIdent(name) if name == "arguments" => {
                    *op = Op::DeleteIdentWith {
                        name: name.clone(),
                        slot: Some(slot.0),
                    };
                }
                Op::StoreGlobalStrict(name) if name == "arguments" => *op = Op::AssignLocal(slot.0),
                Op::StoreGlobalSloppy { name, .. } | Op::StoreLocalOrGlobalSloppy { name, .. }
                    if name == "arguments" =>
                {
                    *op = Op::AssignLocal(slot.0)
                }
                Op::LoadIdentWith {
                    name, slot: target, ..
                }
                | Op::LoadResolvedIdentWith {
                    name, slot: target, ..
                }
                | Op::StoreIdentWith {
                    name, slot: target, ..
                }
                | Op::StoreResolvedIdentWith {
                    name, slot: target, ..
                }
                | Op::TypeofIdentWith { name, slot: target }
                | Op::DeleteIdentWith { name, slot: target }
                | Op::ResolveIdentWith {
                    name, slot: target, ..
                } if name == "arguments" && target.is_none() => {
                    *target = Some(slot.0);
                }
                _ => {}
            }
        }
    }

    fn arguments_reaches_owner(&self, pc: usize) -> bool {
        let mut scope = self
            .scope_plan
            .pc_scopes
            .get(pc)
            .copied()
            .unwrap_or(crate::binding::ScopeId(0));
        loop {
            if let Some((_, slot, _)) = self
                .scope_plan
                .declarations
                .iter()
                .rev()
                .find(|(name, _, owner)| name == "arguments" && *owner == scope)
            {
                let local = &self.locals[slot.0];
                return self.scope_plan.implicit_arguments == Some(*slot)
                    || local.is_received_upvalue()
                    || local.hoisted && !local.parameter && !local.hoisted_function;
            }
            let Some(parent) = self.scope_plan.scopes[scope.0].parent else {
                return true;
            };
            scope = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{compiler::compile_script, ir::Bytecode};
    use super::*;

    fn function(source: &str) -> std::rc::Rc<Bytecode> {
        let script = qjs_parser::parse_script(source).unwrap();
        let top = compile_script(&script).unwrap();
        top.code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn parameter_only_arrows_and_ordinary_child_eval_need_no_parent_arguments() {
        for source in [
            "function f(x) { return y => x + y; }",
            "function f(x) { return function g() { return eval('arguments[0]'); }; }",
            "function f(x) { class C { m() { return eval('arguments[0]'); } } return C; }",
            "function f(arguments) { return () => eval('arguments'); }",
            "function f() { let arguments = 3; return () => eval('arguments'); }",
        ] {
            let body = function(source);
            assert!(!body.needs_arguments_object(), "{source}");
            assert!(
                !body
                    .binding_plan
                    .bindings
                    .iter()
                    .any(|binding| { binding.kind == crate::binding::BindingKind::Arguments }),
                "{source}"
            );
        }
    }

    #[test]
    fn lexical_arrow_demand_reaches_the_nearest_real_arguments_owner() {
        for source in [
            "function f(x) { return () => () => arguments[0]; }",
            "function f(x) { return () => () => eval('arguments[0]'); }",
            "function f(x = eval('arguments[0]')) { let arguments = 3; return x; }",
            "function f() { var arguments; return () => arguments; }",
        ] {
            let body = function(source);
            assert!(body.needs_arguments_object(), "{source}");
            assert_eq!(
                body.binding_plan
                    .bindings
                    .iter()
                    .filter(|binding| { binding.kind == crate::binding::BindingKind::Arguments })
                    .count(),
                1,
                "{source}"
            );
        }
    }

    #[test]
    fn lexical_arguments_shadowing_preserves_default_eval_and_escaping_arrows() {
        assert_eq!(
            crate::eval(
                r#"
            function outer(value = eval('arguments[0]')) {
                let arguments = ['body'];
                return [value, (() => () => eval('arguments[0]'))()()];
            }
            function nested(x) { return function(y) { return eval('arguments[0]'); }; }
            function own(x) { return () => () => eval('arguments[0]'); }
            [outer('parameter').join(':'), nested(1)(2), own(3)()()].join('|');
        "#
            ),
            Ok(crate::Value::String("parameter:body|2|3".into()))
        );
    }
}

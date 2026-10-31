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

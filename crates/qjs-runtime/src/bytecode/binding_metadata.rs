//! Finish immutable scope metadata from the existing compiler's declarations.
//! Descriptors share the executable body's slot and received-cell ordering.

use super::ir::Bytecode;

impl Bytecode {
    pub(super) fn from_compiler(
        compiler: super::compiler::Compiler,
        parameters: Option<(Vec<usize>, &qjs_ast::FunctionParams)>,
        global_lexical_names: Vec<String>,
    ) -> Self {
        let (slots, params) = match parameters {
            Some((slots, params)) => (Some(slots), Some(params)),
            None => (None, None),
        };
        let mut bytecode = Self::with_scope_global_lexical_names_and_strict_with_parameter_slots(
            compiler.constants,
            compiler.locals,
            compiler.code,
            compiler.global_scope,
            global_lexical_names,
            compiler.strict,
            slots,
            Some((compiler.scope_plan, compiler.direct_eval_source)),
        );
        if let Some(params) = params {
            bytecode.binding_plan.parameters =
                super::binding_plan::ParameterInfo::compile(&bytecode, params);
        }
        bytecode
    }

    /// Consumes the declaration identities recorded by this body's compiler.
    /// No syntax is reparsed and no live values enter this plan.
    pub(super) fn install_scope_plan(
        &mut self,
        mut builder: crate::binding::ScopePlanBuilder,
        eval_declarations: bool,
    ) {
        use crate::binding::{
            BindingId, BindingKind, BindingMeta, InitPolicy, LocalSlot, WritePolicy,
        };
        let mut bindings = Vec::new();
        let mut slots = vec![None; self.locals.len()];
        let mut declarations = vec![Vec::new(); builder.scopes.len()];
        for (slot, local) in self.locals.iter().enumerate() {
            if local.compiler_temporary || local.name == "this" {
                continue;
            }
            let declaration = builder
                .declaration_by_slot
                .get(slot)
                .copied()
                .flatten()
                .map(|index| &builder.declarations[index]);
            if declaration.is_none() && local.name.starts_with('\0') {
                continue;
            }
            let id = BindingId(bindings.len());
            slots[slot] = Some(id);
            let name = declaration.map_or_else(|| local.name.clone(), |(name, _, _)| name.clone());
            let kind = if local.is_received_upvalue() {
                BindingKind::Received
            } else if local.parameter {
                BindingKind::Parameter
            } else if local.catch_binding {
                BindingKind::Catch
            } else if local.hoisted_function {
                BindingKind::Function
            } else if local.hoisted {
                BindingKind::Var
            } else {
                BindingKind::Lexical
            };
            if !local.is_received_upvalue() && !local.sloppy_global_fallback {
                let scope = declaration.map_or(builder.variable, |(_, _, scope)| *scope);
                let descriptor = &mut builder.scopes[scope.0];
                declarations[scope.0].push(id);
                descriptor.lookup.insert(name.clone(), id);
            }
            bindings.push(BindingMeta {
                id,
                name,
                kind,
                mutable: local.mutable,
                strict_write: if local.mutable {
                    WritePolicy::Mutable
                } else {
                    WritePolicy::Throw
                },
                deletable: eval_declarations && local.hoisted && !local.parameter,
                initialization: if local.parameter {
                    InitPolicy::Parameter
                } else if local.hoisted {
                    InitPolicy::Undefined
                } else {
                    InitPolicy::Uninitialized
                },
                slot: LocalSlot(slot),
            });
        }
        self.binding_plan.bindings = bindings.into_boxed_slice();
        self.binding_plan.local_bindings = slots.into_boxed_slice();
        for (scope, bindings) in builder.scopes.iter_mut().zip(declarations) {
            scope.declarations = bindings.into_boxed_slice();
        }
        self.binding_plan.scopes = builder.scopes.into_boxed_slice();
        self.binding_plan.exposure = crate::binding::EvalExposurePlan {
            sites: builder.sites.into_boxed_slice(),
        };
        self.binding_plan
            .validate(self.locals.len(), self.code.len());
    }
}

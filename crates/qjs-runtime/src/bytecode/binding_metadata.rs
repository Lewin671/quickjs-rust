//! Finish immutable scope metadata from the existing compiler's declarations.
//! Descriptors share the executable body's slot and received-cell ordering.

use super::ir::Bytecode;

impl Bytecode {
    pub(super) fn from_compiler(
        compiler: super::compiler::Compiler,
        parameters: Option<(Vec<usize>, &qjs_ast::FunctionParams)>,
        global_lexical_names: Vec<String>,
    ) -> Self {
        let lexical_arguments = !compiler.arguments_owner && compiler.arguments_demand();
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
        bytecode.binding_plan.needs_lexical_arguments = lexical_arguments;
        if let Some(params) = params {
            bytecode.binding_plan.parameters =
                super::binding_plan::ParameterInfo::compile(&bytecode, params);
        }
        bytecode
    }

    pub(crate) fn scope_descriptors(&self) -> std::rc::Rc<crate::binding::ScopeDescriptors> {
        self.binding_plan.scopes.clone()
    }

    pub(crate) fn scope_declarations(
        &self,
        scope: crate::binding::ScopeId,
    ) -> impl Iterator<Item = crate::binding::BindingMetaRef> + '_ {
        self.binding_plan.scopes[scope.0]
            .declarations
            .iter()
            .map(|&id| crate::binding::BindingMetaRef {
                owner: self.binding_plan.bindings.clone(),
                id,
            })
    }

    pub(crate) fn mark_script_lexical_routes(&mut self) {
        if let Some(root) = self.binding_plan.scopes.first() {
            for &id in &root.declarations {
                let meta = &self.binding_plan.bindings[id.0];
                if meta.kind == crate::binding::BindingKind::Lexical {
                    self.binding_plan.realm_lexical_slots[meta.slot.0] = true;
                    self.binding_plan.has_realm_lexicals = true;
                }
            }
        }
    }

    pub(crate) fn binding_meta(&self, slot: usize) -> Option<crate::binding::BindingMetaRef> {
        self.binding_plan
            .local_bindings
            .get(slot)
            .copied()
            .flatten()
            .map(|id| crate::binding::BindingMetaRef {
                owner: self.binding_plan.bindings.clone(),
                id,
            })
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
            } else if builder
                .implicit_arguments
                .is_some_and(|implicit| implicit.0 == slot)
            {
                BindingKind::Arguments
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
            if !(local.is_received_upvalue()
                || local.sloppy_global_fallback
                || self.global_scope && local.hoisted)
            {
                let scope = declaration.map_or(builder.variable, |(_, _, scope)| *scope);
                let descriptor = &mut builder.scopes[scope.0];
                if self.global_scope && scope.0 == 0 && kind == BindingKind::Lexical {
                    self.binding_plan.realm_lexical_slots[slot] = true;
                    self.binding_plan.has_realm_lexicals = true;
                }
                declarations[scope.0].push(id);
                descriptor.lookup.insert(name.clone(), id);
            }
            if kind == BindingKind::Arguments {
                self.binding_plan.arguments_slot = Some(slot);
                self.binding_plan.cell_slots.push(slot);
                self.binding_plan.dynamic_cell_slots.push(slot);
                self.binding_plan.initial_slots[slot] = super::binding_plan::InitialSlot::Arguments;
            }
            if name == "arguments" && kind == BindingKind::Lexical {
                self.binding_plan.initial_slots[slot] = super::binding_plan::InitialSlot::Empty;
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
                } else if local.hoisted || kind == BindingKind::Arguments {
                    InitPolicy::Undefined
                } else {
                    InitPolicy::Uninitialized
                },
                slot: LocalSlot(slot),
            });
        }
        self.binding_plan.cell_slots.sort_unstable();
        self.binding_plan.cell_slots.dedup();
        self.binding_plan.dynamic_cell_slots.sort_unstable();
        self.binding_plan.dynamic_cell_slots.dedup();
        self.binding_plan.bindings = std::rc::Rc::new(crate::binding::BindingMetadata(
            bindings.into_boxed_slice(),
            crate::binding::MetadataOrigin::Compiled,
        ));
        self.binding_plan.local_bindings = slots.into_boxed_slice();
        self.binding_plan.cell_slots.retain(|&slot| {
            self.binding_plan.local_bindings[slot].is_some() || self.locals[slot].name == "this"
        });
        self.binding_plan.dynamic_cell_slots.retain(|&slot| {
            self.binding_plan.local_bindings[slot].is_some() || self.locals[slot].name == "this"
        });
        for (scope, bindings) in builder.scopes.iter_mut().zip(declarations) {
            scope.declarations = bindings.into_boxed_slice();
        }
        self.binding_plan.scopes = std::rc::Rc::new(crate::binding::ScopeDescriptors(
            builder.scopes.into_boxed_slice(),
        ));
        builder
            .pc_scopes
            .resize(self.code.len(), *builder.active.last().expect("root scope"));
        self.binding_plan.pc_scopes = builder.pc_scopes.into_boxed_slice();
        self.binding_plan.exposure = crate::binding::EvalExposurePlan {
            sites: builder.sites.into_boxed_slice(),
        };
        self.binding_plan
            .validate(self.locals.len(), self.code.len());
    }
}

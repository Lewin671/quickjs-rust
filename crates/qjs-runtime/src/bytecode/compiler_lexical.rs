use std::collections::HashMap;

use qjs_ast::{BindingPattern, ForInLeft, ForInit, FunctionParams, Stmt, SwitchCase, VarKind};

use crate::RuntimeError;

use super::{
    compiler::Compiler,
    ir::{Bytecode, Local},
};

pub(super) struct LexicalCapture {
    pub(super) name: String,
    pub(super) storage_name: String,
    pub(super) slot: usize,
}

impl Compiler {
    pub(super) fn declare_lexical_slot(&mut self, name: &str, mutable: bool) -> usize {
        let storage_name = self.lexical_storage_name(name);
        self.declare_lexical_slot_with_storage_name(name, &storage_name, mutable)
    }

    pub(super) fn declare_lexical_slot_with_storage_name(
        &mut self,
        name: &str,
        storage_name: &str,
        mutable: bool,
    ) -> usize {
        if let Some(slot) = self.current_lexical_scope().get(name) {
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: storage_name.to_owned(),
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable,
            from_env: false,
            sloppy_global_fallback: false,
        });
        let scope = self.current_lexical_scope_mut();
        scope.insert(name.to_owned(), slot);
        scope.insert(storage_name.to_owned(), slot);
        slot
    }

    pub(super) fn declare_captured_lexical_slot(&mut self, name: &str, mutable: bool) -> usize {
        self.declare_captured_lexical_slot_with_storage_name(name, name, mutable)
    }

    pub(super) fn declare_captured_lexical_slot_with_storage_name(
        &mut self,
        name: &str,
        storage_name: &str,
        mutable: bool,
    ) -> usize {
        if let Some(slot) = self.current_lexical_scope().get(name) {
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: storage_name.to_owned(),
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable,
            from_env: true,
            sloppy_global_fallback: false,
        });
        let scope = self.current_lexical_scope_mut();
        scope.insert(name.to_owned(), slot);
        scope.insert(storage_name.to_owned(), slot);
        slot
    }

    pub(super) fn resolve_local_slot(&self, name: &str) -> Option<usize> {
        let lexical = self
            .lexical_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied());
        if lexical.is_some() {
            return lexical;
        }
        // Global-scope `var`/function bindings live in the realm, not in
        // frame slots: every identifier reference compiles to a global op so
        // eval'd code and deferred jobs share the same binding.
        if self.global_scope && self.global_hoisted.contains(name) {
            return None;
        }
        self.local_slots
            .get(name)
            .copied()
            .filter(|slot| !self.locals[*slot].sloppy_global_fallback)
    }

    pub(super) fn assignment_slot(&mut self, name: &str) -> usize {
        if let Some(slot) = self.resolve_local_slot(name) {
            return slot;
        }
        if let Some(slot) = self.local_slots.get(name) {
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: name.to_owned(),
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable: true,
            from_env: false,
            sloppy_global_fallback: true,
        });
        self.local_slots.insert(name.to_owned(), slot);
        slot
    }

    pub(super) fn with_lexical_scope<T>(
        &mut self,
        compile: impl FnOnce(&mut Self) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        self.lexical_scopes.push(HashMap::new());
        let result = compile(self);
        self.lexical_scopes
            .pop()
            .expect("lexical scope stack should be balanced");
        result
    }

    pub(super) fn current_lexical_slots_for_names(&self, names: &[String]) -> Vec<usize> {
        let Some(scope) = self.lexical_scopes.last() else {
            return Vec::new();
        };
        let mut slots = Vec::new();
        for name in names {
            if let Some(slot) = scope.get(name)
                && !slots.contains(slot)
            {
                slots.push(*slot);
            }
        }
        slots
    }

    pub(super) fn active_lexical_captures(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> Vec<LexicalCapture> {
        let mut captures = Vec::new();
        for name in function_bytecode
            .global_names()
            .iter()
            .map(String::as_str)
            .chain(function_bytecode.local_names())
        {
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_ok()
            {
                continue;
            }
            if let Some(slot) = self.resolve_active_lexical_slot(name)
                && !captures
                    .iter()
                    .any(|capture: &LexicalCapture| capture.slot == slot)
            {
                captures.push(LexicalCapture {
                    name: name.to_owned(),
                    storage_name: self.locals[slot].name.clone(),
                    slot,
                });
            }
        }
        captures
    }

    pub(super) fn predeclare_current_scope_lexicals(&mut self, body: &[Stmt]) {
        for (name, mutable) in current_scope_lexical_declared_bindings(body) {
            self.declare_lexical_slot(&name, mutable);
        }
    }

    fn current_lexical_scope(&self) -> &HashMap<String, usize> {
        self.lexical_scopes
            .last()
            .expect("compiler should always have a lexical scope")
    }

    fn current_lexical_scope_mut(&mut self) -> &mut HashMap<String, usize> {
        self.lexical_scopes
            .last_mut()
            .expect("compiler should always have a lexical scope")
    }

    fn resolve_active_lexical_slot(&self, name: &str) -> Option<usize> {
        self.lexical_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn lexical_storage_name(&self, name: &str) -> String {
        if self
            .lexical_scopes
            .iter()
            .rev()
            .skip(1)
            .any(|scope| scope.contains_key(name))
            || self.local_slots.contains_key(name)
            || self.locals.iter().any(|local| local.name == name)
        {
            format!("\0lexical:{}:{}", name, self.locals.len())
        } else {
            name.to_owned()
        }
    }
}

pub(super) fn catch_param_annex_b_blocked_names(param: Option<&BindingPattern>) -> Vec<String> {
    match param {
        Some(BindingPattern::Identifier { .. }) | None => Vec::new(),
        Some(pattern) => pattern.names(),
    }
}

pub(super) fn for_init_lexical_names(init: &ForInit) -> Vec<String> {
    match init {
        ForInit::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            declarations,
            ..
        } => declarations
            .iter()
            .flat_map(|declaration| declaration.binding.names())
            .collect(),
        ForInit::VarDecl { .. } | ForInit::Expr(_) => Vec::new(),
    }
}

pub(super) fn for_in_left_lexical_names(left: &ForInLeft) -> Vec<String> {
    match left {
        ForInLeft::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            binding,
            ..
        } => binding.names(),
        ForInLeft::VarDecl { .. } | ForInLeft::Target(_) => Vec::new(),
    }
}

pub(super) fn is_lexical_for_in_left(left: &ForInLeft) -> bool {
    matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
            ..
        }
    )
}

pub(super) fn switch_lexical_declared_names(cases: &[SwitchCase], strict: bool) -> Vec<String> {
    let mut names = Vec::new();
    for case in cases {
        names.extend(switch_case_lexical_declared_names(&case.consequent, strict));
    }
    names
}

pub(super) fn switch_lexical_declared_bindings(
    cases: &[SwitchCase],
    strict: bool,
) -> Vec<(String, bool)> {
    let mut names = Vec::new();
    for case in cases {
        names.extend(switch_case_lexical_declared_bindings(
            &case.consequent,
            strict,
        ));
    }
    names
}

pub(super) fn switch_annex_b_blocked_names(cases: &[SwitchCase], strict: bool) -> Vec<String> {
    let mut names = Vec::new();
    for case in cases {
        names.extend(annex_b_blocked_names(&case.consequent, strict));
    }
    names
}

pub(super) fn annex_b_blocked_names(body: &[Stmt], strict: bool) -> Vec<String> {
    let mut names = switch_case_lexical_declared_names(body, strict);
    for stmt in body {
        if let Stmt::FunctionDecl {
            name,
            is_generator: false,
            is_async: false,
            ..
        } = stmt
            && !names.iter().any(|existing| existing == name)
        {
            names.push(name.clone());
        }
    }
    names
}

pub(super) fn lexical_declared_names(body: &[Stmt]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                declarations,
                ..
            } => names.extend(
                declarations
                    .iter()
                    .flat_map(|declaration| declaration.binding.names()),
            ),
            Stmt::For {
                init: Some(init), ..
            } => names.extend(for_init_lexical_names(init)),
            Stmt::ForIn { left, .. } | Stmt::ForOf { left, .. } => {
                names.extend(for_in_left_lexical_names(left));
            }
            Stmt::Switch { cases, .. } => names.extend(switch_lexical_declared_names(cases, false)),
            Stmt::ClassDecl { name, .. } => names.push(name.clone()),
            _ => {}
        }
    }
    names
}

fn switch_case_lexical_declared_names(body: &[Stmt], strict: bool) -> Vec<String> {
    switch_case_lexical_declared_bindings(body, strict)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn switch_case_lexical_declared_bindings(body: &[Stmt], strict: bool) -> Vec<(String, bool)> {
    let mut names = current_scope_lexical_declared_bindings(body);
    for stmt in body {
        if let Stmt::FunctionDecl {
            name,
            is_generator,
            is_async,
            ..
        } = stmt
            && (strict || *is_generator || *is_async)
        {
            names.push((name.clone(), true));
        }
    }
    names
}

pub(super) fn current_scope_lexical_declared_bindings(body: &[Stmt]) -> Vec<(String, bool)> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                declarations,
                ..
            } => {
                let mutable = matches!(
                    stmt,
                    Stmt::VarDecl {
                        kind: VarKind::Let,
                        ..
                    }
                );
                names.extend(
                    declarations
                        .iter()
                        .flat_map(|declaration| declaration.binding.names())
                        .map(|name| (name, mutable)),
                );
            }
            Stmt::ClassDecl { name, .. } => names.push((name.clone(), true)),
            _ => {}
        }
    }
    names
}

/// Whether a loop body declares a lexical binding (`let`/`const`/`class`/
/// `using`) within the same function scope (not inside a nested function).
///
/// Each loop iteration must give the body's lexical bindings a fresh
/// per-iteration environment so closures created in one iteration capture that
/// iteration's bindings (ES2023 14.7 `CreatePerIterationEnvironment`). The
/// `for`-head case is handled by re-homing the head slots; this detects the
/// same need for lexicals declared in the loop *body* of `while`, `do`/`while`,
/// and `for(;;)` loops. Nested function and class *bodies* are their own
/// scopes, so the walk does not descend into them.
pub(super) fn stmt_declares_capturable_lexical(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::VarDecl { kind, .. } => kind.is_lexical(),
        Stmt::ClassDecl { .. } => true,
        Stmt::Block { body, .. } => body.iter().any(stmt_declares_capturable_lexical),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_declares_capturable_lexical(consequent)
                || alternate
                    .as_deref()
                    .is_some_and(stmt_declares_capturable_lexical)
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::Labelled { body, .. }
        | Stmt::With { body, .. } => stmt_declares_capturable_lexical(body),
        Stmt::Switch { cases, .. } => cases
            .iter()
            .any(|case| case.consequent.iter().any(stmt_declares_capturable_lexical)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(stmt_declares_capturable_lexical)
                || handler.as_ref().is_some_and(|handler| {
                    handler.body.iter().any(stmt_declares_capturable_lexical)
                })
                || finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.iter().any(stmt_declares_capturable_lexical))
        }
        _ => false,
    }
}

pub(super) fn function_body_annex_b_blocked_names(
    params: &FunctionParams,
    body: &[Stmt],
) -> Vec<String> {
    let mut names = function_param_names(params);
    names.extend(lexical_declared_names(body));
    names
}

pub(super) fn function_param_names(params: &FunctionParams) -> Vec<String> {
    let mut names = params.names();
    if !names.iter().any(|name| name == "arguments") {
        names.push("arguments".to_owned());
    }
    names
}

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
        self.declare_lexical_slot_with_storage_name(name, name, mutable)
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

    /// Resolves an identifier read to an indexed binding when the binding is
    /// known at compile time. Global `var`/function writes deliberately keep
    /// using the global operations so descriptor and strictness checks remain
    /// centralized, while reads can use the shared realm cell installed for
    /// their otherwise vestigial local slot.
    pub(super) fn resolve_identifier_load_slot(&self, name: &str) -> Option<usize> {
        self.resolve_local_slot(name).or_else(|| {
            (self.global_scope && self.global_hoisted.contains(name))
                .then(|| self.local_slots.get(name).copied())
                .flatten()
        })
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
        let global_names = function_bytecode.global_names();
        let written_names = function_bytecode.written_binding_names();
        for name in global_names
            .iter()
            .map(String::as_str)
            .chain(written_names.iter().map(String::as_str))
            .chain(function_bytecode.local_names())
        {
            // Compiler temporaries are owned by the function whose prologue or
            // body created them. In particular, nested rest/default-parameter
            // functions can both have a `\0\0rest_argument` snapshot; capturing
            // the parent's same-named temporary makes the inner prologue read
            // the outer argument array and corrupts every later upvalue.
            if name.starts_with("\0\0") {
                continue;
            }
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_ok()
            {
                continue;
            }
            let read_only_global = !function_bytecode.contains_direct_eval()
                && !written_names.iter().any(|written| written == name)
                && (global_names.iter().any(|global| global == name)
                    || function_bytecode
                        .local_slot(name)
                        .is_some_and(|slot| function_bytecode.local_is_from_env(slot)));
            if let Some(slot) = self.resolve_active_capture_slot(name, read_only_global)
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
        // Lexical `this` and every `super` operation use the surrounding
        // `this` binding implicitly, so they may have no identifier load/store
        // for the ordinary name scan above to discover. Arrow functions omit
        // `this` from `function_local_names`; force that outer slot into their
        // received-upvalue plan. Ordinary methods/constructors retain their own
        // `this` local and do not enter this branch.
        if function_bytecode.uses_lexical_this()
            && function_local_names
                .binary_search_by(|local| local.as_str().cmp("this"))
                .is_err()
            && let Some(slot) = self.resolve_active_capture_slot("this", false)
            && !captures
                .iter()
                .any(|capture: &LexicalCapture| capture.slot == slot)
        {
            captures.push(LexicalCapture {
                name: "this".to_owned(),
                storage_name: self.locals[slot].name.clone(),
                slot,
            });
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

    /// Resolves bindings that have moved onto shared cells through T016 S4.
    /// Lexical scopes win over same-named function-environment bindings;
    /// parameters and body `var`/function declarations are otherwise
    /// capturable even though they live in `local_slots` rather than
    /// `lexical_scopes`. A read-only reference to a statically known global
    /// `var` may receive the realm's shared cell too; any function that writes
    /// the name keeps the global operations so descriptor checks remain on the
    /// name-addressed path.
    fn resolve_active_capture_slot(&self, name: &str, read_only_global: bool) -> Option<usize> {
        self.resolve_active_lexical_slot(name).or_else(|| {
            if self.global_scope
                && !self.direct_eval_source
                && read_only_global
                && self.global_hoisted.contains(name)
            {
                return self.local_slots.get(name).copied();
            }
            (!self.global_scope && !self.direct_eval_source)
                .then(|| self.resolve_local_slot(name))
                .flatten()
        })
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

#[cfg(test)]
mod tests {
    use super::super::{ir::Op, upvalue_resolver};
    use super::*;

    #[test]
    fn shadowed_lexicals_keep_source_names_and_distinct_slots() {
        let mut compiler = Compiler::function_compiler(false, false);
        let outer = compiler.declare_lexical_slot("value", true);
        let inner = compiler
            .with_lexical_scope(|compiler| Ok(compiler.declare_lexical_slot("value", true)))
            .expect("nested lexical declaration should compile");

        assert_ne!(outer, inner);
        assert_eq!(compiler.locals[outer].name, "value");
        assert_eq!(compiler.locals[inner].name, "value");
    }

    #[test]
    fn nested_function_captures_parent_parameter_by_slot() {
        let script = qjs_parser::parse_script(
            "function outer(value) { return function read() { return value; }; }",
        )
        .expect("source should parse");
        let top = super::super::compiler::compile_script(&script).expect("source should compile");
        let outer = top
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    bytecode,
                    ..
                } if name == "outer" => Some(bytecode),
                _ => None,
            })
            .expect("outer function should be emitted");
        let parameter_slot = outer
            .local_slot("value")
            .expect("outer parameter should have a slot");
        assert!(outer.local_is_parameter(parameter_slot));

        let captures = outer
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    lexical_captures,
                    ..
                } if name == "read" => Some(lexical_captures),
                _ => None,
            })
            .expect("nested function should be emitted");
        assert_eq!(captures, &vec![("value".to_owned(), parameter_slot)]);
        assert_eq!(
            upvalue_resolver::resolve_upvalues(outer).cell_slots,
            vec![parameter_slot]
        );
    }

    #[test]
    fn nested_function_captures_parent_var_by_slot() {
        let script = qjs_parser::parse_script(
            "function outer() { var value = 1; return function read() { return value; }; }",
        )
        .expect("source should parse");
        let top = super::super::compiler::compile_script(&script).expect("source should compile");
        let outer = top
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    bytecode,
                    ..
                } if name == "outer" => Some(bytecode),
                _ => None,
            })
            .expect("outer function should be emitted");
        let var_slot = outer
            .local_slot("value")
            .expect("outer var should have a slot");
        assert!(outer.local_is_body_hoist_only(var_slot));

        let captures = outer
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    lexical_captures,
                    ..
                } if name == "read" => Some(lexical_captures),
                _ => None,
            })
            .expect("nested function should be emitted");
        assert_eq!(captures, &vec![("value".to_owned(), var_slot)]);
        assert_eq!(
            upvalue_resolver::resolve_upvalues(outer).cell_slots,
            vec![var_slot]
        );
    }

    #[test]
    fn global_var_reads_use_the_shared_slot_but_writes_stay_global() {
        let script =
            qjs_parser::parse_script("var value = 1; value; value *= 2; value++; typeof value;")
                .expect("source should parse");
        let bytecode =
            super::super::compiler::compile_script(&script).expect("source should compile");
        let value_slot = bytecode
            .local_slot("value")
            .expect("global var should retain an indexed slot");

        assert!(
            bytecode
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadLocal(slot) if *slot == value_slot))
        );
        assert!(
            bytecode
                .code
                .iter()
                .any(|op| matches!(op, Op::StoreGlobalSloppy(name) if name == "value"))
        );
        assert!(
            !bytecode
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadGlobal(name) if name == "value"))
        );
    }

    #[test]
    fn nested_read_only_global_uses_realm_cell_but_writer_stays_global() {
        let script = qjs_parser::parse_script(
            "var value = 1; function read() { return value; } function write() { value = 2; return value; }",
        )
        .expect("source should parse");
        let top = super::super::compiler::compile_script(&script).expect("source should compile");

        let nested = |function_name: &str| {
            top.code
                .iter()
                .find_map(|op| match op {
                    Op::NewFunction {
                        name: Some(name),
                        bytecode,
                        lexical_captures,
                        ..
                    } if name == function_name => Some((bytecode, lexical_captures)),
                    _ => None,
                })
                .expect("nested function should be emitted")
        };

        let (reader, reader_captures) = nested("read");
        assert_eq!(reader_captures.len(), 1);
        assert_eq!(reader_captures[0].0, "value");
        let reader_slot = reader
            .local_slot("value")
            .expect("reader should receive the realm cell in a slot");
        assert!(reader.local_is_from_env(reader_slot));
        assert!(
            reader
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadLocal(slot) if *slot == reader_slot))
        );

        let (writer, writer_captures) = nested("write");
        assert!(writer_captures.is_empty());
        assert!(
            writer
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadGlobal(name) if name == "value"))
        );
        assert!(writer.code.iter().any(|op| matches!(
            op,
            Op::StoreLocalOrGlobalSloppy { name, .. } if name == "value"
        )));
    }

    #[test]
    fn nested_direct_eval_keeps_read_only_globals_name_addressed() {
        let script = qjs_parser::parse_script(
            "var value = 1; function read() { eval('var value = 2'); return value; }",
        )
        .expect("source should parse");
        let top = super::super::compiler::compile_script(&script).expect("source should compile");
        let (reader, captures) = top
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    bytecode,
                    lexical_captures,
                    ..
                } if name == "read" => Some((bytecode, lexical_captures)),
                _ => None,
            })
            .expect("nested function should be emitted");

        assert!(captures.is_empty());
        assert!(reader.contains_direct_eval());
        assert!(
            reader
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadGlobal(name) if name == "value"))
        );
    }

    #[test]
    fn direct_eval_var_capture_uses_the_name_to_cell_deopt_path() {
        let script = qjs_parser::parse_script("var value = 1; function read() { return value; }")
            .expect("source should parse");
        let bytecode = super::super::compiler::compile_direct_eval_script(&script, false)
            .expect("direct eval source should compile");
        let captures = bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    lexical_captures,
                    ..
                } if name == "read" => Some(lexical_captures),
                _ => None,
            })
            .expect("nested function should be emitted");
        assert!(captures.is_empty());
    }
}

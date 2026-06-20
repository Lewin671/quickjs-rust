use std::rc::Rc;

use qjs_ast::{AssignmentOp, BinaryOp, Expr, ForInit, Literal, Stmt, UnaryOp, VarKind};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::Compiler;
use super::compiler_lexical::{for_init_lexical_names, stmt_declares_capturable_lexical};
use super::ir::Op;
use super::util::parse_number_literal;

enum OptionalChainStep<'a> {
    Member(&'a qjs_ast::MemberProperty),
    Call(&'a [qjs_ast::CallArgument]),
}

struct OptionalChainEntry<'a> {
    kind: OptionalChainStep<'a>,
    optional: bool,
}

impl Compiler {
    pub(super) fn compile_if(
        &mut self,
        test: &Expr,
        consequent: &Stmt,
        alternate: Option<&Stmt>,
    ) -> Result<(), RuntimeError> {
        self.compile_expr(test)?;
        let else_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        self.compile_if_clause_stmt(consequent)?;
        let end_jump = self.emit(Op::Jump(usize::MAX));
        let else_target = self.code.len();
        self.patch_jump(else_jump, else_target);
        self.emit(Op::Pop);
        if let Some(alternate) = alternate {
            self.compile_if_clause_stmt(alternate)?;
        } else {
            self.emit_load_undefined();
        }
        let end_target = self.code.len();
        self.patch_jump(end_jump, end_target);
        Ok(())
    }

    fn compile_if_clause_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        if let Stmt::FunctionDecl { name, .. } = stmt {
            if self.annex_b_function_name_blocked(name) {
                self.emit_load_undefined();
                return Ok(());
            }
            return self.with_lexical_scope(|compiler| {
                compiler
                    .with_annex_b_blocked_function_names(std::slice::from_ref(name), |compiler| {
                        compiler.compile_function_decl(stmt)
                    })
            });
        }
        self.compile_stmt(stmt)
    }

    pub(super) fn compile_while(&mut self, test: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        let loop_start = self.code.len();
        self.compile_expr(test)?;
        let exit_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        self.push_loop(result_slot);
        let body_start = self.code.len();
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
        let context = self.pop_loop();
        // A `continue` (and the fall-through after the body) must pass through
        // the per-iteration environment refresh, so the refresh — when needed —
        // becomes the continue target rather than the loop top.
        let continue_target = self.emit_loop_iteration_scope(body, body_start, loop_start);
        self.emit(Op::Jump(loop_start));
        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        self.patch_loop_continues(&context, continue_target);
        Ok(())
    }

    /// Emits a per-iteration captured-environment refresh for `while`/`do`-`while`
    /// loops whose body declares lexical bindings captured by a closure, so each
    /// iteration's closures capture that iteration's bindings. Returns the
    /// continue target: the refresh instruction when one is emitted, otherwise
    /// `fallback` (the loop's normal continue point). `for`-head loops re-home
    /// their head slots through the existing `iteration_slots` path instead.
    fn emit_loop_iteration_scope(
        &mut self,
        body: &Stmt,
        body_start: usize,
        fallback: usize,
    ) -> usize {
        if !self.loop_body_needs_iteration_scope(body, body_start) {
            return fallback;
        }
        let target = self.code.len();
        self.emit(Op::FreshIterationScope(Vec::new()));
        target
    }

    /// Whether each iteration of a loop with the given `body` (compiled from
    /// `body_start` onward in `self.code`) needs a fresh captured environment:
    /// the body both declares a lexical binding and creates a closure that
    /// could capture it. Loops that create no closure, or whose body declares
    /// no lexical, keep the single shared environment to avoid a per-iteration
    /// clone on the hot path.
    fn loop_body_needs_iteration_scope(&self, body: &Stmt, body_start: usize) -> bool {
        stmt_declares_capturable_lexical(body)
            && self.code[body_start..]
                .iter()
                .any(|op| matches!(op, Op::NewFunction { .. } | Op::NewClass { .. }))
    }

    pub(super) fn compile_do_while(
        &mut self,
        body: &Stmt,
        test: &Expr,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        let loop_start = self.code.len();
        self.push_loop(result_slot);
        let body_start = self.code.len();
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
        let context = self.pop_loop();
        // The refresh (when needed) sits before the test so both the
        // fall-through and a `continue` (which targets the test) run it.
        let test_start = self.code.len();
        let continue_target = self.emit_loop_iteration_scope(body, body_start, test_start);
        self.compile_expr(test)?;
        let exit_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        self.emit(Op::Jump(loop_start));
        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        self.patch_loop_continues(&context, continue_target);
        Ok(())
    }

    pub(super) fn compile_for(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        if matches!(
            init,
            Some(ForInit::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                ..
            })
        ) {
            return self.with_lexical_scope(|compiler| {
                compiler.compile_for_scoped(init, test, update, body)
            });
        }
        self.compile_for_scoped(init, test, update, body)
    }

    fn compile_for_scoped(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        if for_init_has_using(init) {
            return self.compile_for_with_disposal(init, test, update, body);
        }
        if let Some(init) = init {
            self.compile_for_init(init)?;
            self.emit(Op::Pop);
        }
        self.compile_for_loop_after_init(init, test, update, body)
    }

    fn compile_for_with_disposal(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        self.emit(Op::EnterDisposableScope);
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));

        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });
        self.disposable_scope_depth += 1;
        let body_result = (|| {
            if let Some(init) = init {
                self.compile_for_init(init)?;
                self.emit(Op::Pop);
            }
            self.compile_for_loop_after_init_with_result(init, test, update, body, result_slot)
        })();
        self.disposable_scope_depth -= 1;
        body_result?;
        self.emit(Op::ExitTry);
        let normal_jump = self.emit(Op::Jump(usize::MAX));

        let finally_target = self.compile_dispose_finally(for_init_has_await_using(init));
        if let Op::EnterTry { finally, .. } = &mut self.code[enter] {
            *finally = Some(finally_target);
        }
        self.patch_jump(normal_jump, finally_target);
        self.emit(Op::LoadLocal(result_slot));
        Ok(())
    }

    fn compile_for_loop_after_init(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.compile_for_loop_after_init_with_result(init, test, update, body, result_slot)
    }

    fn compile_for_loop_after_init_with_result(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
        result_slot: usize,
    ) -> Result<(), RuntimeError> {
        let loop_start = self.code.len();
        let exit_jump = if let Some(test) = test {
            self.compile_expr(test)?;
            let jump = self.emit(Op::JumpIfFalse(usize::MAX));
            self.emit(Op::Pop);
            Some(jump)
        } else {
            None
        };
        let blocked = init.map_or_else(Vec::new, for_init_lexical_names);
        let iteration_slots: Vec<usize> = if matches!(
            init,
            Some(ForInit::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                ..
            })
        ) {
            blocked
                .iter()
                .filter_map(|name| self.resolve_local_slot(name))
                .collect()
        } else {
            Vec::new()
        };
        let body_start = self.code.len();
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.push_loop(result_slot);
            compiler.compile_stmt(body)?;
            compiler.emit(Op::StoreLocal(result_slot));
            Ok(())
        })?;
        let context = self.pop_loop();
        let update_start = self.code.len();
        if !iteration_slots.is_empty() {
            // A lexical for-head already refreshes the captured environment each
            // iteration, which also covers lexicals declared in the body.
            self.emit(Op::FreshIterationScope(iteration_slots));
        } else if self.loop_body_needs_iteration_scope(body, body_start) {
            // A non-lexical head (`for(var ...)`, `for(;;)`) still needs a fresh
            // environment when the body declares captured lexicals.
            self.emit(Op::FreshIterationScope(Vec::new()));
        }
        if let Some(update) = update {
            self.compile_expr(update)?;
            self.emit(Op::Pop);
        }
        self.emit(Op::Jump(loop_start));
        let exit = self.code.len();
        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump, exit);
            self.emit(Op::Pop);
        }
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_scoped_loop_completion(result_slot, &cleanup_slots, &context);
        self.patch_loop_continues(&context, update_start);
        Ok(())
    }

    fn compile_for_init(&mut self, init: &ForInit) -> Result<(), RuntimeError> {
        match init {
            ForInit::Expr(expr) => self.compile_expr(expr),
            ForInit::VarDecl {
                kind, declarations, ..
            } => {
                for declaration in declarations {
                    for name in declaration.binding.names() {
                        let slot = self.declare_var_kind_slot(&name, *kind);
                        if matches!(
                            kind,
                            VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing
                        ) {
                            self.emit(Op::ClearLocal(slot));
                        }
                    }
                    if let Some(init) = &declaration.init {
                        self.compile_declaration_init(&declaration.binding, init)?;
                        if self.disposable_scope_depth > 0 {
                            match kind {
                                VarKind::Using => {
                                    self.emit(Op::RegisterDisposable);
                                }
                                VarKind::AwaitUsing => {
                                    self.emit(Op::RegisterAsyncDisposable);
                                }
                                _ => {}
                            }
                        }
                        self.compile_binding_initializer(&declaration.binding, *kind)?;
                    } else {
                        self.compile_binding_uninitialized(&declaration.binding, *kind)?;
                    }
                }
                self.emit_load_undefined();
                Ok(())
            }
        }
    }

    pub(super) fn compile_expr(&mut self, expr: &Expr) -> Result<(), RuntimeError> {
        match expr {
            Expr::Literal(literal) => self.compile_literal(literal),
            Expr::Identifier { name, .. } => {
                let slot = self.resolve_local_slot(name);
                if self.identifier_needs_with_resolution(slot) {
                    self.emit(Op::LoadIdentWith {
                        name: name.clone(),
                        slot,
                    });
                } else if let Some(slot) = slot {
                    self.emit(Op::LoadLocal(slot));
                } else {
                    self.emit(Op::LoadGlobal(name.clone()));
                }
                Ok(())
            }
            Expr::This { .. } => {
                self.emit(Op::LoadGlobal("this".to_owned()));
                Ok(())
            }
            Expr::Sequence { expressions, .. } => self.compile_sequence(expressions),
            Expr::Unary {
                op: UnaryOp::Typeof,
                argument,
                ..
            } => self.compile_typeof(argument),
            Expr::Unary {
                op: UnaryOp::Delete,
                argument,
                ..
            } => self.compile_delete(argument),
            Expr::Unary { op, argument, .. } => {
                self.compile_expr(argument)?;
                self.emit(Op::Unary(*op));
                Ok(())
            }
            Expr::Binary {
                left, op, right, ..
            } => self.compile_binary(left, *op, right),
            Expr::Template {
                parts, expressions, ..
            } => self.compile_template(parts, expressions),
            Expr::TaggedTemplate {
                tag,
                cooked,
                raw,
                expressions,
                ..
            } => self.compile_tagged_template(tag, cooked, raw, expressions),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => self.compile_conditional(test, consequent, alternate),
            Expr::Assignment {
                target,
                op: AssignmentOp::Assign,
                value,
                ..
            } => self.compile_assign(target, value),
            Expr::Assignment {
                target, op, value, ..
            } => self.compile_compound_assign(target, *op, value),
            Expr::Update {
                target, op, prefix, ..
            } => self.compile_update(target, *op, *prefix),
            Expr::Array { elements, .. } => self.compile_array(elements),
            Expr::Object { properties, .. } => self.compile_object(properties),
            Expr::Call {
                callee, arguments, ..
            } => self.compile_call(callee, arguments),
            Expr::New {
                callee,
                arguments,
                span,
            } => self.compile_new(callee, arguments, *span),
            Expr::NewTarget { .. } => {
                self.emit(Op::LoadNewTarget);
                Ok(())
            }
            Expr::Function {
                name,
                params,
                body,
                constructable,
                lexical_this,
                lexical_arguments,
                is_generator,
                is_async,
                span,
                ..
            } => {
                let source_text = self.function_source_text(*span);
                let is_strict = self.strict || is_strict_function_body(body);
                let local_names =
                    collect_function_local_names(name.as_ref(), params, body, !lexical_arguments);
                let (bytecode, lexical_captures) = self.compile_nested_function_body(
                    params,
                    body,
                    is_strict,
                    *is_generator,
                    *is_async,
                    &local_names,
                )?;
                self.emit(Op::NewFunction {
                    name: name.clone(),
                    has_name_binding: name.is_some(),
                    params: params.clone(),
                    local_names,
                    lexical_captures,
                    bytecode: Rc::new(bytecode),
                    // A generator or async function is never constructable.
                    constructable: *constructable && !*is_generator && !*is_async,
                    is_strict,
                    lexical_this: *lexical_this,
                    lexical_arguments: *lexical_arguments,
                    is_generator: *is_generator,
                    is_async: *is_async,
                    source_text,
                });
                Ok(())
            }
            Expr::Class { name, body, .. } => self.compile_class_expression(name.as_deref(), body),
            Expr::Member {
                object, property, ..
            } => {
                if Self::member_chain_has_optional(expr) {
                    return self.compile_optional_chain(expr);
                }
                if matches!(object.as_ref(), Expr::Super { .. }) {
                    return self.compile_super_member(property);
                }
                if let qjs_ast::MemberProperty::Private(name) = property {
                    self.compile_expr(object)?;
                    self.emit(Op::GetPrivate(name.clone()));
                    return Ok(());
                }
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.emit(Op::GetProp);
                Ok(())
            }
            Expr::OptionalMember { .. } | Expr::OptionalCall { .. } => {
                self.compile_optional_chain(expr)
            }
            Expr::PrivateIn { name, object, .. } => {
                self.compile_expr(object)?;
                self.emit(Op::PrivateIn(name.clone()));
                Ok(())
            }
            Expr::Yield {
                argument, delegate, ..
            } => {
                if *delegate {
                    // `yield* expr` delegates to the iterable produced by
                    // `expr` (ES2023 14.4.14). The whole delegation algorithm
                    // (iterator-get, next/return/throw forwarding, and outer
                    // suspension on each inner result) lives in the VM op so
                    // the suspend/resume snapshot stays a plain re-entry point.
                    match argument {
                        Some(argument) => self.compile_expr(argument)?,
                        None => self.emit_load_undefined(),
                    }
                    let iterator_slot = self.temp_local("yield_delegate_iterator");
                    let next_slot = self.temp_local("yield_delegate_next");
                    self.emit(Op::YieldDelegate {
                        iterator_slot,
                        next_slot,
                        async_delegate: self.async_generator_body,
                    });
                    return Ok(());
                }
                match argument {
                    Some(argument) => self.compile_expr(argument)?,
                    None => self.emit_load_undefined(),
                }
                self.emit(Op::Yield);
                Ok(())
            }
            Expr::Await { argument, .. } => {
                // `await expr` suspends the async function (or async generator)
                // body at a dedicated `Op::Await`. The driver resolves the
                // awaited value and resumes the body via the job queue with the
                // fulfillment value or an injected throw. Keeping `Await`
                // distinct from `Yield` lets an async generator route an await
                // suspension to a promise reaction and a `yield` suspension to
                // its consumer.
                self.compile_expr(argument)?;
                self.emit(Op::Await);
                Ok(())
            }
            Expr::ImportCall {
                specifier, options, ..
            } => {
                // Per EvaluateImportCall the specifier is evaluated before the
                // options argument; emit the specifier first so it sits below
                // the options on the stack for the op.
                self.compile_expr(specifier)?;
                if let Some(options) = options {
                    self.compile_expr(options)?;
                }
                self.emit(Op::ImportCall {
                    has_options: options.is_some(),
                });
                Ok(())
            }
            Expr::ImportMeta { .. } => {
                self.emit(Op::ImportMeta);
                Ok(())
            }
            Expr::Super { span } => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "SyntaxError: 'super' keyword unexpected at byte {}",
                    span.start
                ),
            }),
        }
    }

    /// Compiles an expression that supplies a value to a named binding,
    /// assignment, or property, applying NamedEvaluation (ES2023 §8.3.4): an
    /// anonymous function, arrow, generator, async, or class expression takes
    /// `name` as its `name` property. Any other expression (including a *named*
    /// function/class expression, which keeps its own name) compiles exactly
    /// like `compile_expr`.
    pub(super) fn compile_named_expr(
        &mut self,
        expr: &Expr,
        name: &str,
    ) -> Result<(), RuntimeError> {
        match expr {
            Expr::Function {
                name: None,
                params,
                body,
                constructable,
                lexical_this,
                lexical_arguments,
                is_generator,
                is_async,
                span,
                ..
            } => {
                let source_text = self.function_source_text(*span);
                let is_strict = self.strict || is_strict_function_body(body);
                let local_names =
                    collect_function_local_names(None, params, body, !lexical_arguments);
                let (bytecode, lexical_captures) = self.compile_nested_function_body(
                    params,
                    body,
                    is_strict,
                    *is_generator,
                    *is_async,
                    &local_names,
                )?;
                self.emit(Op::NewFunction {
                    name: Some(name.to_owned()),
                    has_name_binding: false,
                    params: params.clone(),
                    local_names,
                    lexical_captures,
                    bytecode: Rc::new(bytecode),
                    constructable: *constructable && !*is_generator && !*is_async,
                    is_strict,
                    lexical_this: *lexical_this,
                    lexical_arguments: *lexical_arguments,
                    is_generator: *is_generator,
                    is_async: *is_async,
                    source_text,
                });
                Ok(())
            }
            Expr::Class {
                name: None, body, ..
            } => self.compile_class(Some(name), body),
            _ => self.compile_expr(expr),
        }
    }

    pub(super) fn compile_function_without_name_binding(
        &mut self,
        expr: &Expr,
        display_name: &str,
    ) -> Result<(), RuntimeError> {
        let Expr::Function {
            params,
            body,
            constructable,
            lexical_this,
            lexical_arguments,
            is_generator,
            is_async,
            span,
            ..
        } = expr
        else {
            return self.compile_expr(expr);
        };
        let source_text = self.function_source_text(*span);
        let is_strict = self.strict || is_strict_function_body(body);
        let local_names = collect_function_local_names(None, params, body, !lexical_arguments);
        let (bytecode, lexical_captures) = self.compile_nested_function_body(
            params,
            body,
            is_strict,
            *is_generator,
            *is_async,
            &local_names,
        )?;
        self.emit(Op::NewFunction {
            name: Some(display_name.to_owned()),
            has_name_binding: false,
            params: params.clone(),
            local_names,
            lexical_captures,
            bytecode: Rc::new(bytecode),
            constructable: *constructable && !*is_generator && !*is_async,
            is_strict,
            lexical_this: *lexical_this,
            lexical_arguments: *lexical_arguments,
            is_generator: *is_generator,
            is_async: *is_async,
            source_text,
        });
        Ok(())
    }

    /// Compiles `super.x` / `super[expr]` property reads.
    fn compile_super_member(
        &mut self,
        property: &qjs_ast::MemberProperty,
    ) -> Result<(), RuntimeError> {
        match property {
            qjs_ast::MemberProperty::Named(name) => {
                self.emit(Op::SuperGet { key: name.clone() });
            }
            qjs_ast::MemberProperty::Computed(expr) => {
                self.emit(Op::RequireSuperThis);
                self.compile_expr(expr)?;
                self.emit(Op::SuperGetComputed);
            }
            qjs_ast::MemberProperty::Private(name) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("SyntaxError: super.#{name} is not allowed"),
                });
            }
        }
        Ok(())
    }

    fn compile_optional_chain(&mut self, expr: &Expr) -> Result<(), RuntimeError> {
        let mut chain = Vec::new();
        let base = Self::collect_optional_chain(expr, &mut chain);
        self.compile_optional_chain_steps(base, &chain)
    }

    /// Compiles `callee(arguments)` where `callee` is (or contains) an optional
    /// member chain, e.g. `obj?.method()` or `a?.b.c()`. The trailing call is
    /// not itself optional, but it must run inside the chain so a short-circuited
    /// member yields `undefined` instead of calling, and a method call keeps the
    /// member's object as its `this`.
    pub(super) fn compile_optional_chain_call<'a>(
        &mut self,
        callee: &'a Expr,
        arguments: &'a [qjs_ast::CallArgument],
    ) -> Result<(), RuntimeError> {
        let mut chain = Vec::new();
        let base = Self::collect_optional_chain(callee, &mut chain);
        chain.push(OptionalChainEntry {
            kind: OptionalChainStep::Call(arguments),
            optional: false,
        });
        self.compile_optional_chain_steps(base, &chain)
    }

    fn compile_optional_chain_steps<'a>(
        &mut self,
        base: &Expr,
        chain: &[OptionalChainEntry<'a>],
    ) -> Result<(), RuntimeError> {
        let mut end_jumps = Vec::new();
        let mut index = 0;

        // `super.method?.()` / `super.method(...)` needs super-method dispatch
        // (the call's `this` is the current `this`, not the resolved value), so
        // when the chain base is a super property immediately followed by a
        // call, resolve it with SuperMethod (leaving `[this, callee]`) and
        // dispatch via CallResolved instead of treating `super.method` as a
        // standalone value.
        if let Expr::Member {
            object, property, ..
        } = base
            && matches!(object.as_ref(), Expr::Super { .. })
            && let Some(first) = chain.first()
            && let OptionalChainStep::Call(arguments) = &first.kind
        {
            self.compile_super_method(property)?;
            self.compile_optional_resolved_call(first.optional, arguments, &mut end_jumps)?;
            index = 1;
        } else {
            self.compile_expr(base)?;
        }

        while index < chain.len() {
            let step = &chain[index];
            // A member immediately followed by a call is a method call: the
            // member's object must be preserved as `this`, so resolve the method
            // while keeping the receiver and dispatch via CallResolved rather
            // than dropping the object with a plain GetProp + Call.
            if let OptionalChainStep::Member(property) = &step.kind
                && let Some(next) = chain.get(index + 1)
                && let OptionalChainStep::Call(arguments) = &next.kind
            {
                self.compile_optional_method_call(
                    step.optional,
                    property,
                    next.optional,
                    arguments,
                    &mut end_jumps,
                )?;
                index += 2;
                continue;
            }
            if step.optional {
                let access_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
                self.emit(Op::Pop);
                self.emit_load_undefined();
                end_jumps.push(self.emit(Op::Jump(usize::MAX)));
                let access_target = self.code.len();
                self.patch_jump(access_jump, access_target);
            }
            match &step.kind {
                OptionalChainStep::Member(property) => {
                    self.compile_member_access_from_stack(property)?;
                }
                OptionalChainStep::Call(arguments) => {
                    self.compile_optional_call_from_stack(arguments)?;
                }
            }
            index += 1;
        }

        let end_target = self.code.len();
        for jump in end_jumps {
            self.patch_jump(jump, end_target);
        }
        Ok(())
    }

    /// Emits a method call inside an optional chain, keeping the receiver as
    /// `this`. On entry the stack holds `[receiver]`; on exit `[result]`.
    /// `member_optional`/`call_optional` short-circuit the whole chain (pushing
    /// `undefined`) when the receiver or the resolved method is nullish.
    fn compile_optional_method_call(
        &mut self,
        member_optional: bool,
        property: &qjs_ast::MemberProperty,
        call_optional: bool,
        arguments: &[qjs_ast::CallArgument],
        end_jumps: &mut Vec<usize>,
    ) -> Result<(), RuntimeError> {
        if member_optional {
            let access_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
            self.emit(Op::Pop);
            self.emit_load_undefined();
            end_jumps.push(self.emit(Op::Jump(usize::MAX)));
            let access_target = self.code.len();
            self.patch_jump(access_jump, access_target);
        }
        // Resolve the method while keeping the receiver: [obj] -> [obj, method].
        self.emit(Op::Dup);
        if let qjs_ast::MemberProperty::Private(name) = property {
            self.emit(Op::GetPrivate(name.clone()));
        } else {
            self.compile_member_key(property)?;
            self.emit(Op::GetProp);
        }
        self.compile_optional_resolved_call(call_optional, arguments, end_jumps)
    }

    /// Dispatches a call whose receiver and callee are already on the stack as
    /// `[receiver, callee]` (a resolved method or super method). When
    /// `call_optional`, a nullish callee short-circuits the chain, discarding
    /// both stack entries before yielding `undefined`.
    fn compile_optional_resolved_call(
        &mut self,
        call_optional: bool,
        arguments: &[qjs_ast::CallArgument],
        end_jumps: &mut Vec<usize>,
    ) -> Result<(), RuntimeError> {
        if call_optional {
            let call_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
            self.emit(Op::Pop);
            self.emit(Op::Pop);
            self.emit_load_undefined();
            end_jumps.push(self.emit(Op::Jump(usize::MAX)));
            let call_target = self.code.len();
            self.patch_jump(call_jump, call_target);
        }
        let has_spread = arguments
            .iter()
            .any(|a| matches!(a, qjs_ast::CallArgument::Spread(_)));
        if has_spread {
            self.compile_argument_array(arguments)?;
            self.emit(Op::CallResolvedSpread);
        } else {
            for argument in arguments {
                let qjs_ast::CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::CallResolved(arguments.len()));
        }
        Ok(())
    }

    fn compile_optional_call_from_stack(
        &mut self,
        arguments: &[qjs_ast::CallArgument],
    ) -> Result<(), RuntimeError> {
        let has_spread = arguments
            .iter()
            .any(|a| matches!(a, qjs_ast::CallArgument::Spread(_)));
        if has_spread {
            self.compile_argument_array(arguments)?;
            self.emit(Op::CallSpread);
        } else {
            for argument in arguments {
                let qjs_ast::CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::Call(arguments.len()));
        }
        Ok(())
    }

    pub(super) fn member_chain_has_optional(expr: &Expr) -> bool {
        match expr {
            Expr::OptionalMember { .. } | Expr::OptionalCall { .. } => true,
            Expr::Member { object, .. } => Self::member_chain_has_optional(object),
            Expr::Call { callee, .. } => Self::member_chain_has_optional(callee),
            _ => false,
        }
    }

    fn collect_optional_chain<'a>(
        expr: &'a Expr,
        chain: &mut Vec<OptionalChainEntry<'a>>,
    ) -> &'a Expr {
        match expr {
            // `super.x` / `super[x]` is a leaf base of the chain: it must be
            // compiled through the dedicated super-property path (which resolves
            // against the home object's prototype), not as a plain object on the
            // stack followed by GetProp.
            Expr::Member { object, .. } if matches!(object.as_ref(), Expr::Super { .. }) => expr,
            Expr::Member {
                object, property, ..
            } => {
                let base = Self::collect_optional_chain(object, chain);
                chain.push(OptionalChainEntry {
                    kind: OptionalChainStep::Member(property),
                    optional: false,
                });
                base
            }
            Expr::OptionalMember {
                object, property, ..
            } => {
                let base = Self::collect_optional_chain(object, chain);
                chain.push(OptionalChainEntry {
                    kind: OptionalChainStep::Member(property),
                    optional: true,
                });
                base
            }
            Expr::OptionalCall {
                callee, arguments, ..
            } => {
                let base = Self::collect_optional_chain(callee, chain);
                chain.push(OptionalChainEntry {
                    kind: OptionalChainStep::Call(arguments),
                    optional: true,
                });
                base
            }
            // `super(...)` is a SuperCall and must be compiled as a leaf base
            // (not split into a bare Super plus a call step), so `super()?.a`
            // works.
            Expr::Call { callee, .. } if matches!(callee.as_ref(), Expr::Super { .. }) => expr,
            // A plain call within an optional chain (`a?.b.c(x).d`) is part of
            // the same chain: it must be collected as a step so a short-circuit
            // at any earlier link skips it (and its arguments) rather than
            // splitting the chain and evaluating the tail on `undefined`.
            Expr::Call {
                callee, arguments, ..
            } => {
                let base = Self::collect_optional_chain(callee, chain);
                chain.push(OptionalChainEntry {
                    kind: OptionalChainStep::Call(arguments),
                    optional: false,
                });
                base
            }
            _ => expr,
        }
    }

    fn compile_member_access_from_stack(
        &mut self,
        property: &qjs_ast::MemberProperty,
    ) -> Result<(), RuntimeError> {
        if let qjs_ast::MemberProperty::Private(name) = property {
            self.emit(Op::GetPrivate(name.clone()));
        } else {
            self.compile_member_key(property)?;
            self.emit(Op::GetProp);
        }
        Ok(())
    }

    fn compile_literal(&mut self, literal: &Literal) -> Result<(), RuntimeError> {
        let value = match literal {
            Literal::Number { raw, .. } => Value::Number(parse_number_literal(raw)?),
            Literal::BigInt { raw, .. } => Value::BigInt(crate::bigint::parse_bigint_literal(raw)?),
            Literal::String { value, .. } => Value::String(value.clone().into()),
            Literal::Boolean { value, .. } => Value::Boolean(*value),
            Literal::Null { .. } => Value::Null,
        };
        let slot = self.const_slot(value);
        self.emit(Op::LoadConst(slot));
        Ok(())
    }

    fn compile_sequence(&mut self, expressions: &[Expr]) -> Result<(), RuntimeError> {
        if expressions.is_empty() {
            self.emit_load_undefined();
            return Ok(());
        }
        for (index, expression) in expressions.iter().enumerate() {
            self.compile_expr(expression)?;
            if index + 1 != expressions.len() {
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    fn compile_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
    ) -> Result<(), RuntimeError> {
        match op {
            BinaryOp::LogicalAnd => self.compile_short_circuit(left, right, Op::JumpIfFalse),
            BinaryOp::LogicalOr => self.compile_short_circuit(left, right, Op::JumpIfTrue),
            BinaryOp::NullishCoalescing => {
                self.compile_short_circuit(left, right, Op::JumpIfNotNullish)
            }
            _ => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                self.emit(Op::Binary(op));
                Ok(())
            }
        }
    }

    fn compile_template(
        &mut self,
        parts: &[String],
        expressions: &[Expr],
    ) -> Result<(), RuntimeError> {
        let first = self.const_slot(Value::String(
            parts.first().cloned().unwrap_or_default().into(),
        ));
        self.emit(Op::LoadConst(first));
        for (index, expression) in expressions.iter().enumerate() {
            self.compile_expr(expression)?;
            self.emit(Op::ToString);
            self.emit(Op::Binary(BinaryOp::Add));
            let part = self.const_slot(Value::String(
                parts.get(index + 1).cloned().unwrap_or_default().into(),
            ));
            self.emit(Op::LoadConst(part));
            self.emit(Op::Binary(BinaryOp::Add));
        }
        Ok(())
    }

    fn compile_tagged_template(
        &mut self,
        tag: &Expr,
        cooked: &[String],
        raw: &[String],
        expressions: &[Expr],
    ) -> Result<(), RuntimeError> {
        if let Expr::Member {
            object, property, ..
        } = tag
        {
            self.compile_expr(object)?;
            self.compile_member_key(property)?;
            self.emit(Op::NewTemplateObject {
                cooked: cooked.to_vec(),
                raw: raw.to_vec(),
            });
            for expression in expressions {
                self.compile_expr(expression)?;
            }
            self.emit(Op::CallMethod(expressions.len() + 1));
            return Ok(());
        }

        self.compile_expr(tag)?;
        self.emit(Op::NewTemplateObject {
            cooked: cooked.to_vec(),
            raw: raw.to_vec(),
        });
        for expression in expressions {
            self.compile_expr(expression)?;
        }
        self.emit(Op::Call(expressions.len() + 1));
        Ok(())
    }

    fn compile_short_circuit(
        &mut self,
        left: &Expr,
        right: &Expr,
        jump: fn(usize) -> Op,
    ) -> Result<(), RuntimeError> {
        self.compile_expr(left)?;
        let end_jump = self.emit(jump(usize::MAX));
        self.emit(Op::Pop);
        self.compile_expr(right)?;
        let end = self.code.len();
        self.patch_jump(end_jump, end);
        Ok(())
    }

    fn compile_conditional(
        &mut self,
        test: &Expr,
        consequent: &Expr,
        alternate: &Expr,
    ) -> Result<(), RuntimeError> {
        self.compile_expr(test)?;
        let else_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        self.compile_expr(consequent)?;
        let end_jump = self.emit(Op::Jump(usize::MAX));
        let else_target = self.code.len();
        self.patch_jump(else_jump, else_target);
        self.emit(Op::Pop);
        self.compile_expr(alternate)?;
        let end = self.code.len();
        self.patch_jump(end_jump, end);
        Ok(())
    }
}

fn for_init_has_using(init: Option<&ForInit>) -> bool {
    matches!(
        init,
        Some(ForInit::VarDecl {
            kind: VarKind::Using | VarKind::AwaitUsing,
            ..
        })
    )
}

fn for_init_has_await_using(init: Option<&ForInit>) -> bool {
    matches!(
        init,
        Some(ForInit::VarDecl {
            kind: VarKind::AwaitUsing,
            ..
        })
    )
}

use std::rc::Rc;

use qjs_ast::{AssignmentOp, BinaryOp, Expr, ForInit, Literal, Stmt, UnaryOp, VarKind};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::{Compiler, for_init_lexical_names};
use super::ir::Op;
use super::util::parse_number_literal;

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
        if let Stmt::FunctionDecl { .. } = stmt {
            return self.compile_function_decl(stmt);
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
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
        let context = self.pop_loop();
        self.emit(Op::Jump(loop_start));
        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        self.patch_loop_continues(&context, loop_start);
        Ok(())
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
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
        let context = self.pop_loop();
        let test_start = self.code.len();
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
        self.patch_loop_continues(&context, test_start);
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
                kind: VarKind::Let | VarKind::Const,
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
        if let Some(init) = init {
            self.compile_for_init(init)?;
            self.emit(Op::Pop);
        }
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
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
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.push_loop(result_slot);
            compiler.compile_stmt(body)?;
            compiler.emit(Op::StoreLocal(result_slot));
            Ok(())
        })?;
        let context = self.pop_loop();
        let update_start = self.code.len();
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
                        if matches!(kind, VarKind::Let | VarKind::Const) {
                            self.emit(Op::ClearLocal(slot));
                        }
                    }
                    if let Some(init) = &declaration.init {
                        self.compile_expr(init)?;
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
                if let Some(slot) = self.resolve_local_slot(name) {
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
                callee, arguments, ..
            } => self.compile_new(callee, arguments),
            Expr::Function {
                name,
                params,
                body,
                constructable,
                lexical_this,
                lexical_arguments,
                ..
            } => {
                let is_strict = self.strict || is_strict_function_body(body);
                let bytecode =
                    super::compiler::compile_function_body_with_strict(params, body, is_strict)?;
                let local_names =
                    collect_function_local_names(name.as_ref(), params, body, !lexical_arguments);
                self.emit(Op::NewFunction {
                    name: name.clone(),
                    params: params.clone(),
                    local_names,
                    bytecode: Rc::new(bytecode),
                    constructable: *constructable,
                    is_strict,
                    lexical_this: *lexical_this,
                    lexical_arguments: *lexical_arguments,
                });
                Ok(())
            }
            Expr::Class { name, body, .. } => self.compile_class(name.as_deref(), body),
            Expr::Member {
                object, property, ..
            } => {
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
            Expr::PrivateIn { name, object, .. } => {
                self.compile_expr(object)?;
                self.emit(Op::PrivateIn(name.clone()));
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

    fn compile_literal(&mut self, literal: &Literal) -> Result<(), RuntimeError> {
        let value = match literal {
            Literal::Number { raw, .. } => Value::Number(parse_number_literal(raw)?),
            Literal::BigInt { raw, .. } => Value::BigInt(crate::bigint::parse_bigint_literal(raw)?),
            Literal::String { value, .. } => Value::String(value.clone()),
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
        let first = self.const_slot(Value::String(parts.first().cloned().unwrap_or_default()));
        self.emit(Op::LoadConst(first));
        for (index, expression) in expressions.iter().enumerate() {
            self.compile_expr(expression)?;
            self.emit(Op::ToString);
            self.emit(Op::Binary(BinaryOp::Add));
            let part = self.const_slot(Value::String(
                parts.get(index + 1).cloned().unwrap_or_default(),
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

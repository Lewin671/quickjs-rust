use std::rc::Rc;

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, Expr, ForInit, Literal, Stmt, UnaryOp, VarKind,
};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::Compiler;
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
        self.reset_current_result_slot();
        self.compile_stmt(consequent)?;
        let end_jump = self.emit(Op::Jump(usize::MAX));
        let else_target = self.code.len();
        self.patch_jump(else_jump, else_target);
        self.emit(Op::Pop);
        if let Some(alternate) = alternate {
            self.reset_current_result_slot();
            self.compile_stmt(alternate)?;
        } else {
            self.emit_load_undefined();
        }
        let end_target = self.code.len();
        self.patch_jump(end_jump, end_target);
        Ok(())
    }

    pub(super) fn compile_while(&mut self, test: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("loop_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        let loop_start = self.code.len();
        self.compile_expr(test)?;
        let exit_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
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

    fn reset_current_result_slot(&mut self) {
        if let Some(result_slot) = self.current_result_slot() {
            self.emit_load_undefined();
            self.emit(Op::StoreLocal(result_slot));
        }
    }

    pub(super) fn compile_with(&mut self, object: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
        if self.strict {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: with statements are not allowed in strict mode".to_owned(),
            });
        }
        self.compile_expr(object)?;
        let enter = self.emit(Op::EnterWith(usize::MAX));
        let result_slot = self.temp_local("with_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.dynamic_scope_depth += 1;
        self.push_completion(result_slot);
        let result = self.compile_stmt(body);
        self.pop_completion();
        self.dynamic_scope_depth -= 1;
        result?;
        self.emit(Op::StoreLocal(result_slot));
        self.emit(Op::ExitWith);
        self.emit(Op::LoadLocal(result_slot));
        let after = self.code.len();
        if let Op::EnterWith(target) = &mut self.code[enter] {
            *target = after;
        }
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
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
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
        let restores = init
            .map(|init| self.for_init_shadow_saves(init))
            .unwrap_or_default();
        let cleanup_slots = init
            .map(|init| self.for_init_cleanup_slots(init))
            .unwrap_or_default();
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
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.push_loop(result_slot);
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
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
        let cleanup_start = self.code.len();
        self.emit_clear_locals(&cleanup_slots);
        self.emit_restores(&restores);
        self.emit(Op::LoadLocal(result_slot));
        self.patch_loop_breaks(&context, cleanup_start);
        self.patch_loop_continues(&context, update_start);
        Ok(())
    }

    fn compile_for_init(&mut self, init: &ForInit) -> Result<(), RuntimeError> {
        match init {
            ForInit::Expr(expr) => self.compile_expr(expr),
            ForInit::VarDecl {
                kind, declarations, ..
            } => {
                let is_hoisted = *kind == VarKind::Var;
                for declaration in declarations {
                    self.validate_strict_binding_name(&declaration.name)?;
                    let slot = if *kind == VarKind::Const {
                        self.immutable_local_slot(&declaration.name, is_hoisted)
                    } else {
                        self.local_slot(&declaration.name, is_hoisted)
                    };
                    if let Some(init) = &declaration.init {
                        if self.dynamic_scope_depth > 0 {
                            self.emit(Op::ResolveName(declaration.name.clone()));
                        }
                        self.compile_expr(init)?;
                        if self.dynamic_scope_depth > 0 {
                            self.emit(Op::StoreName {
                                name: declaration.name.clone(),
                                strict: self.strict,
                            });
                        } else {
                            self.emit(Op::InitLocal(slot));
                        }
                    } else if *kind != VarKind::Var || self.direct_eval {
                        self.emit_load_undefined();
                        self.emit(Op::InitLocal(slot));
                    }
                }
                self.emit_load_undefined();
                Ok(())
            }
            ForInit::Binding {
                kind, target, init, ..
            } => {
                self.ensure_target_local_slots(target, *kind == VarKind::Var);
                let value_slot = self.temp_local("for_init_binding");
                self.compile_expr(init)?;
                self.emit(Op::StoreLocal(value_slot));
                self.compile_init_value(target, value_slot)?;
                self.emit(Op::Pop);
                self.emit_load_undefined();
                Ok(())
            }
        }
    }

    fn for_init_cleanup_slots(&mut self, init: &ForInit) -> Vec<usize> {
        let mut slots = Vec::new();
        match init {
            ForInit::VarDecl {
                kind, declarations, ..
            } if *kind != VarKind::Var => {
                slots.extend(
                    declarations
                        .iter()
                        .map(|declaration| self.local_slot(&declaration.name, false)),
                );
            }
            ForInit::Binding { kind, target, .. } if *kind != VarKind::Var => {
                self.collect_target_local_slots(target, false, &mut slots);
            }
            _ => {}
        }
        slots.sort_unstable();
        slots.dedup();
        slots
    }

    fn for_init_shadow_saves(&mut self, init: &ForInit) -> Vec<(usize, usize)> {
        let mut restores = Vec::new();
        match init {
            ForInit::VarDecl {
                kind, declarations, ..
            } if *kind != VarKind::Var => {
                for declaration in declarations {
                    self.save_for_init_shadow(&declaration.name, &mut restores);
                }
            }
            ForInit::Binding { kind, target, .. } if *kind != VarKind::Var => {
                self.collect_target_shadow_saves(target, &mut restores);
            }
            _ => {}
        }
        restores
    }

    fn collect_target_shadow_saves(
        &mut self,
        target: &AssignmentTarget,
        restores: &mut Vec<(usize, usize)>,
    ) {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                self.save_for_init_shadow(name, restores);
            }
            AssignmentTarget::Array { elements, .. } => {
                for element in elements.iter().flatten() {
                    self.collect_target_shadow_saves(&element.target, restores);
                }
            }
            AssignmentTarget::Object { properties, .. } => {
                for property in properties {
                    self.collect_target_shadow_saves(&property.target, restores);
                }
            }
            AssignmentTarget::Member { .. } => {}
        }
    }

    fn save_for_init_shadow(&mut self, name: &str, restores: &mut Vec<(usize, usize)>) {
        if let Some(slot) = self.local_slots.get(name).copied()
            && !restores.iter().any(|(candidate, _)| *candidate == slot)
        {
            let saved_slot = self.temp_local("for_init_shadow");
            self.emit(Op::LoadLocalOrUndefined(slot));
            self.emit(Op::StoreLocal(saved_slot));
            restores.push((slot, saved_slot));
        }
    }

    pub(super) fn compile_expr(&mut self, expr: &Expr) -> Result<(), RuntimeError> {
        match expr {
            Expr::Literal(literal) => self.compile_literal(literal),
            Expr::Identifier { name, .. } => {
                if self.dynamic_scope_depth > 0 {
                    self.emit(Op::LoadName(name.clone()));
                } else if let Some(slot) = self.local_slots.get(name) {
                    self.emit(Op::LoadLocal(*slot));
                } else {
                    self.emit(Op::LoadName(name.clone()));
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
                ..
            } => {
                let is_strict = self.strict || is_strict_function_body(body);
                let bytecode =
                    super::compiler::compile_function_body_with_strict(params, body, is_strict)?;
                let local_names = collect_function_local_names(name.as_ref(), params, body);
                self.emit(Op::NewFunction {
                    name: name.clone(),
                    params: params.clone(),
                    local_names,
                    bytecode: Rc::new(bytecode),
                    constructable: *constructable,
                    is_strict,
                });
                Ok(())
            }
            Expr::Member {
                object, property, ..
            } => {
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.emit(Op::GetProp);
                Ok(())
            }
        }
    }

    fn compile_literal(&mut self, literal: &Literal) -> Result<(), RuntimeError> {
        let value = match literal {
            Literal::Number { raw, .. } => Value::Number(parse_number_literal(raw)?),
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

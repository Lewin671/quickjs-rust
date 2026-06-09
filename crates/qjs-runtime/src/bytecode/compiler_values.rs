use std::rc::Rc;

use qjs_ast::{
    ArrayElement, CallArgument, Expr, MemberProperty, ObjectProperty, ObjectPropertyKey, Stmt,
    VarKind,
};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::Compiler;
use super::ir::{ArrayElementKind, Op};
use super::util::unsupported_stmt;

impl Compiler {
    pub(super) fn compile_hoisted_function_decls(
        &mut self,
        body: &[Stmt],
    ) -> Result<(), RuntimeError> {
        for stmt in body {
            if let Stmt::FunctionDecl { name, .. } = stmt
                && (!self.annex_b_function_name_blocked(name)
                    || self.annex_b_arguments_function_name_blocked(name))
            {
                self.compile_function_decl(stmt)?;
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    pub(super) fn compile_array(&mut self, elements: &[ArrayElement]) -> Result<(), RuntimeError> {
        let mut element_kinds = Vec::with_capacity(elements.len());
        for element in elements {
            match element {
                ArrayElement::Expr(expr) => {
                    self.compile_expr(expr)?;
                    element_kinds.push(ArrayElementKind::Expr);
                }
                ArrayElement::Elision => {
                    element_kinds.push(ArrayElementKind::Elision);
                }
                ArrayElement::Spread(expr) => {
                    self.compile_expr(expr)?;
                    element_kinds.push(ArrayElementKind::Spread);
                }
            }
        }
        self.emit(Op::NewArray {
            elements: element_kinds,
        });
        Ok(())
    }

    pub(super) fn compile_object(
        &mut self,
        properties: &[ObjectProperty],
    ) -> Result<(), RuntimeError> {
        for property in properties {
            match &property.key {
                ObjectPropertyKey::Literal(key) => {
                    let slot = self.const_slot(Value::String(key.clone()));
                    self.emit(Op::LoadConst(slot));
                }
                ObjectPropertyKey::Computed(expr) => self.compile_expr(expr)?,
            }
            self.compile_expr(&property.value)?;
        }
        self.emit(Op::NewObject(
            properties.iter().map(|property| property.kind).collect(),
        ));
        Ok(())
    }

    pub(super) fn compile_member_key(
        &mut self,
        property: &MemberProperty,
    ) -> Result<(), RuntimeError> {
        match property {
            MemberProperty::Named(name) => {
                let slot = self.const_slot(Value::String(name.clone()));
                self.emit(Op::LoadConst(slot));
                Ok(())
            }
            MemberProperty::Computed(expr) => self.compile_expr(expr),
        }
    }

    pub(super) fn compile_delete(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        let Expr::Member {
            object, property, ..
        } = argument
        else {
            let slot = self.const_slot(Value::Boolean(true));
            self.emit(Op::LoadConst(slot));
            return Ok(());
        };
        self.compile_expr(object)?;
        self.compile_member_key(property)?;
        self.emit(Op::DeleteProp);
        Ok(())
    }

    pub(super) fn compile_call(
        &mut self,
        callee: &Expr,
        arguments: &[CallArgument],
    ) -> Result<(), RuntimeError> {
        let has_spread = arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)));
        if let Expr::Member {
            object, property, ..
        } = callee
        {
            self.compile_expr(object)?;
            self.compile_member_key(property)?;
            if has_spread {
                self.compile_argument_array(arguments)?;
                self.emit(Op::CallMethodSpread);
                return Ok(());
            }
            for argument in arguments {
                let CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::CallMethod(arguments.len()));
            return Ok(());
        }

        self.compile_expr(callee)?;
        if has_spread {
            self.compile_argument_array(arguments)?;
            self.emit(Op::CallSpread);
            return Ok(());
        }
        for argument in arguments {
            let CallArgument::Expr(argument) = argument else {
                unreachable!("spread arguments are handled above");
            };
            self.compile_expr(argument)?;
        }
        self.emit(Op::Call(arguments.len()));
        Ok(())
    }

    pub(super) fn compile_new(
        &mut self,
        callee: &Expr,
        arguments: &[CallArgument],
    ) -> Result<(), RuntimeError> {
        self.compile_expr(callee)?;
        if arguments
            .iter()
            .any(|argument| matches!(argument, CallArgument::Spread(_)))
        {
            self.compile_argument_array(arguments)?;
            self.emit(Op::NewSpread);
            return Ok(());
        }
        for argument in arguments {
            let CallArgument::Expr(argument) = argument else {
                unreachable!("spread arguments are handled above");
            };
            self.compile_expr(argument)?;
        }
        self.emit(Op::New(arguments.len()));
        Ok(())
    }

    fn compile_argument_array(&mut self, arguments: &[CallArgument]) -> Result<(), RuntimeError> {
        let elements = arguments
            .iter()
            .map(|argument| match argument {
                CallArgument::Spread(expr) => ArrayElement::Spread(expr.clone()),
                CallArgument::Expr(expr) => ArrayElement::Expr(expr.clone()),
            })
            .collect::<Vec<_>>();
        self.compile_array(&elements)
    }

    pub(super) fn compile_function_decl(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        let Stmt::FunctionDecl {
            name, params, body, ..
        } = stmt
        else {
            return Err(unsupported_stmt(stmt));
        };
        let blocked_arguments = self.annex_b_arguments_function_name_blocked(name);
        if self.annex_b_function_name_blocked(name) && !blocked_arguments {
            self.emit_load_undefined();
            return Ok(());
        }
        let is_strict = self.strict || is_strict_function_body(body);
        let bytecode = super::compiler::compile_function_body_with_strict(params, body, is_strict)?;
        let local_names = collect_function_local_names(Some(name), params, body, true);
        self.emit(Op::NewFunction {
            name: Some(name.clone()),
            params: params.clone(),
            local_names,
            bytecode: Rc::new(bytecode),
            constructable: true,
            is_strict,
            lexical_this: false,
            lexical_arguments: false,
        });
        if blocked_arguments {
            let slot = self.declare_lexical_slot(name, true);
            self.emit(Op::StoreLocal(slot));
        } else if self.global_scope {
            let slot = self.local_slot(name, true);
            self.emit_store_var_binding(slot, name, VarKind::Var);
        } else {
            let slot = self.local_slot(name, true);
            self.emit(Op::StoreLocal(slot));
        }
        self.emit_load_undefined();
        Ok(())
    }
}

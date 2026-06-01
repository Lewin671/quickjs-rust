use qjs_ast::{Expr, MemberProperty, ObjectProperty, ObjectPropertyKey, Stmt};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::ir::Op;
use super::util::unsupported_stmt;

impl Compiler {
    pub(super) fn compile_array(&mut self, elements: &[Expr]) -> Result<(), RuntimeError> {
        for element in elements {
            self.compile_expr(element)?;
        }
        self.emit(Op::NewArray(elements.len()));
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
        self.emit(Op::NewObject(properties.len()));
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

    pub(super) fn compile_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
    ) -> Result<(), RuntimeError> {
        if let Expr::Member {
            object, property, ..
        } = callee
        {
            self.compile_expr(object)?;
            self.compile_member_key(property)?;
            for argument in arguments {
                self.compile_expr(argument)?;
            }
            self.emit(Op::CallMethod(arguments.len()));
            return Ok(());
        }

        self.compile_expr(callee)?;
        for argument in arguments {
            self.compile_expr(argument)?;
        }
        self.emit(Op::Call(arguments.len()));
        Ok(())
    }

    pub(super) fn compile_new(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
    ) -> Result<(), RuntimeError> {
        self.compile_expr(callee)?;
        for argument in arguments {
            self.compile_expr(argument)?;
        }
        self.emit(Op::New(arguments.len()));
        Ok(())
    }

    pub(super) fn compile_function_decl(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        let Stmt::FunctionDecl {
            name, params, body, ..
        } = stmt
        else {
            return Err(unsupported_stmt(stmt));
        };
        let slot = self.local_slot(name, true);
        self.emit(Op::NewFunction {
            name: Some(name.clone()),
            params: params.clone(),
            body: body.clone(),
            constructable: true,
        });
        self.emit(Op::StoreLocal(slot));
        self.emit_load_undefined();
        Ok(())
    }
}

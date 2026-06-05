use std::rc::Rc;

use qjs_ast::{ClassMethod, Expr, MemberProperty, ObjectProperty, ObjectPropertyKey, Stmt};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::Compiler;
use super::ir::Op;
use super::util::unsupported_stmt;

impl Compiler {
    pub(super) fn compile_hoisted_function_decls(
        &mut self,
        body: &[Stmt],
    ) -> Result<(), RuntimeError> {
        for stmt in body {
            if let Stmt::FunctionDecl { .. } = stmt {
                self.compile_function_decl(stmt)?;
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    pub(super) fn compile_array(&mut self, elements: &[Option<Expr>]) -> Result<(), RuntimeError> {
        let mut holes = Vec::new();
        for (index, element) in elements.iter().enumerate() {
            if let Some(element) = element {
                self.compile_expr(element)?;
            } else {
                holes.push(index);
                self.emit_load_undefined();
            }
        }
        self.emit(Op::NewArray {
            count: elements.len(),
            holes,
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
        if let Expr::Identifier { name, .. } = argument {
            if self.dynamic_scope_depth == 0 && self.local_slots.contains_key(name) {
                let slot = self.const_slot(Value::Boolean(false));
                self.emit(Op::LoadConst(slot));
            } else {
                self.emit(Op::DeleteName(name.clone()));
            }
            return Ok(());
        }
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
        let is_strict = self.strict || is_strict_function_body(body);
        let bytecode = super::compiler::compile_function_body_with_strict(params, body, is_strict)?;
        let local_names = collect_function_local_names(Some(name), params, body);
        self.emit(Op::NewFunction {
            name: Some(name.clone()),
            params: params.clone(),
            local_names,
            bytecode: Rc::new(bytecode),
            constructable: true,
            is_strict,
        });
        self.emit(Op::StoreLocal(slot));
        self.emit_load_undefined();
        Ok(())
    }

    pub(super) fn compile_class_decl(
        &mut self,
        name: &str,
        methods: &[ClassMethod],
    ) -> Result<(), RuntimeError> {
        let class_slot = self.local_slot(name, true);
        let bytecode = super::compiler::compile_function_body(&[], &[])?;
        self.emit(Op::NewFunction {
            name: Some(name.to_owned()),
            params: Vec::new(),
            local_names: vec!["arguments".to_owned(), "this".to_owned()],
            bytecode: Rc::new(bytecode),
            constructable: true,
            is_strict: self.strict,
        });
        self.emit(Op::StoreLocal(class_slot));

        for method in methods {
            if method.is_static {
                self.emit(Op::LoadLocal(class_slot));
            } else {
                self.emit(Op::LoadLocal(class_slot));
                let prototype = self.const_slot(Value::String("prototype".to_owned()));
                self.emit(Op::LoadConst(prototype));
                self.emit(Op::GetProp);
            }
            let key = self.const_slot(Value::String(method.name.clone()));
            self.emit(Op::LoadConst(key));
            let is_strict = self.strict || is_strict_function_body(&method.body);
            let bytecode = super::compiler::compile_function_body_with_strict(
                &method.params,
                &method.body,
                is_strict,
            )?;
            self.emit(Op::NewFunction {
                name: Some(method.name.clone()),
                params: method.params.clone(),
                local_names: collect_function_local_names(
                    Some(&method.name),
                    &method.params,
                    &method.body,
                ),
                bytecode: Rc::new(bytecode),
                constructable: false,
                is_strict,
            });
            self.emit(Op::SetProp { strict: false });
            self.emit(Op::Pop);
        }

        self.emit_load_undefined();
        Ok(())
    }
}

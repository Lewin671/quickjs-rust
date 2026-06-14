use std::rc::Rc;

use qjs_ast::{
    ArrayElement, CallArgument, Expr, Literal, MemberProperty, ObjectProperty, ObjectPropertyKey,
    ObjectPropertyKind, Span, Stmt, VarKind,
};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
};

use super::compiler::Compiler;
use super::ir::{ArrayElementKind, ObjectPropertyMeta, Op};
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
        self.emit(Op::NewObjectLiteral);
        for property in properties {
            if matches!(property.kind, ObjectPropertyKind::Spread) {
                self.compile_expr(&property.value)?;
                self.emit(Op::CopyObjectSpread);
                continue;
            }
            match &property.key {
                ObjectPropertyKey::Literal(key) => {
                    let slot = self.const_slot(Value::String(key.clone()));
                    self.emit(Op::LoadConst(slot));
                    // `{ f: <anon> }` names a plain property value after the
                    // key via NamedEvaluation. Object method syntax also gets
                    // that display name, but without a function-body name
                    // binding.
                    if matches!(property.kind, ObjectPropertyKind::Data)
                        && !property.is_proto_setter
                    {
                        if let qjs_ast::Expr::Function {
                            name: Some(function_name),
                            constructable: false,
                            ..
                        } = &property.value
                            && function_name == key
                        {
                            self.compile_function_without_name_binding(&property.value, key)?;
                            self.emit(Op::DefineObjectProperty(ObjectPropertyMeta {
                                kind: property.kind,
                                is_proto_setter: property.is_proto_setter,
                            }));
                            continue;
                        }
                        self.compile_named_expr(&property.value, key)?;
                        self.emit(Op::DefineObjectProperty(ObjectPropertyMeta {
                            kind: property.kind,
                            is_proto_setter: property.is_proto_setter,
                        }));
                        continue;
                    }
                    if matches!(
                        property.kind,
                        ObjectPropertyKind::Getter | ObjectPropertyKind::Setter
                    ) {
                        let prefix = match property.kind {
                            ObjectPropertyKind::Getter => "get ",
                            ObjectPropertyKind::Setter => "set ",
                            _ => unreachable!("accessor kind checked above"),
                        };
                        self.compile_function_without_name_binding(
                            &property.value,
                            &format!("{prefix}{key}"),
                        )?;
                        self.emit(Op::DefineObjectProperty(ObjectPropertyMeta {
                            kind: property.kind,
                            is_proto_setter: property.is_proto_setter,
                        }));
                        continue;
                    }
                }
                ObjectPropertyKey::Computed(expr) => self.compile_expr(expr)?,
            }
            self.compile_expr(&property.value)?;
            self.emit(Op::DefineObjectProperty(ObjectPropertyMeta {
                kind: property.kind,
                is_proto_setter: property.is_proto_setter,
            }));
        }
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
            // Private members never reach the ordinary key path; callers route
            // them to dedicated private ops. Reaching here is a compiler bug.
            MemberProperty::Private(name) => Err(RuntimeError {
                thrown: None,
                message: format!("private member #{name} used as an ordinary property key"),
            }),
        }
    }

    pub(super) fn compile_delete(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        match argument {
            Expr::Member {
                object, property, ..
            } => {
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.emit(Op::DeleteProp {
                    is_strict: self.strict,
                });
            }
            Expr::Identifier { name, .. } => {
                if self.strict {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "SyntaxError: cannot delete identifier `{name}` in strict mode"
                        ),
                    });
                }
                if self.inside_with() {
                    let slot = self.resolve_local_slot(name);
                    self.emit(Op::DeleteIdentWith {
                        name: name.clone(),
                        slot,
                    });
                } else {
                    self.emit(Op::DeleteIdent(name.clone()));
                }
            }
            _ => {
                self.compile_expr(argument)?;
                self.emit(Op::Pop);
                let slot = self.const_slot(Value::Boolean(true));
                self.emit(Op::LoadConst(slot));
            }
        }
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

        // `super(...)` invokes the parent constructor.
        if matches!(callee, Expr::Super { .. }) {
            if has_spread {
                self.compile_argument_array(arguments)?;
                self.emit(Op::SuperCallSpread);
                return Ok(());
            }
            for argument in arguments {
                let CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::SuperCall(arguments.len()));
            return Ok(());
        }

        // `super.x(...)` calls a parent-prototype method with the current
        // `this` as the receiver.
        if let Expr::Member {
            object, property, ..
        } = callee
            && matches!(object.as_ref(), Expr::Super { .. })
        {
            // Resolve the callee from the parent prototype; this leaves
            // `[this_value, callee]` on the stack.
            self.compile_super_method(property)?;
            if has_spread {
                self.compile_argument_array(arguments)?;
                self.emit(Op::CallResolvedSpread);
                return Ok(());
            }
            for argument in arguments {
                let CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::CallResolved(arguments.len()));
            return Ok(());
        }

        // `obj.#m(...)` calls a private method with `obj` as the receiver.
        if let Expr::Member {
            object,
            property: MemberProperty::Private(name),
            ..
        } = callee
        {
            // Leave `[receiver, callee]` on the stack for `CallResolved`.
            self.compile_expr(object)?;
            self.emit(Op::Dup);
            self.emit(Op::GetPrivate(name.clone()));
            if has_spread {
                self.compile_argument_array(arguments)?;
                self.emit(Op::CallResolvedSpread);
                return Ok(());
            }
            for argument in arguments {
                let CallArgument::Expr(argument) = argument else {
                    unreachable!("spread arguments are handled above");
                };
                self.compile_expr(argument)?;
            }
            self.emit(Op::CallResolved(arguments.len()));
            return Ok(());
        }

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

        let direct_eval = matches!(callee, Expr::Identifier { name, .. } if name == "eval");
        self.compile_expr(callee)?;
        if has_spread {
            self.compile_argument_array(arguments)?;
            if direct_eval {
                self.emit(Op::CallDirectEvalSpread);
            } else {
                self.emit(Op::CallSpread);
            }
            return Ok(());
        }
        for argument in arguments {
            let CallArgument::Expr(argument) = argument else {
                unreachable!("spread arguments are handled above");
            };
            self.compile_expr(argument)?;
        }
        if direct_eval {
            self.emit(Op::CallDirectEval(arguments.len()));
        } else {
            self.emit(Op::Call(arguments.len()));
        }
        Ok(())
    }

    pub(super) fn compile_new(
        &mut self,
        callee: &Expr,
        arguments: &[CallArgument],
        span: Span,
    ) -> Result<(), RuntimeError> {
        if let Err(error) = validate_regexp_literal_new(callee, arguments, span) {
            self.regexp_literal_error = true;
            return Err(error);
        }
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

    /// Resolves a `super.m` / `super[expr]` method, leaving `[this, callee]`
    /// on the stack for a following `CallResolved`.
    fn compile_super_method(&mut self, property: &MemberProperty) -> Result<(), RuntimeError> {
        match property {
            MemberProperty::Named(name) => {
                self.emit(Op::SuperMethod { key: name.clone() });
            }
            MemberProperty::Computed(expr) => {
                self.emit(Op::RequireSuperThis);
                self.compile_expr(expr)?;
                self.emit(Op::SuperMethodComputed);
            }
            MemberProperty::Private(name) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("SyntaxError: super.#{name} is not allowed"),
                });
            }
        }
        Ok(())
    }

    pub(super) fn compile_argument_array(
        &mut self,
        arguments: &[CallArgument],
    ) -> Result<(), RuntimeError> {
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
            name,
            params,
            body,
            is_generator,
            is_async,
            ..
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
        let local_names = collect_function_local_names(Some(name), params, body, true);
        let (bytecode, lexical_captures) = self.compile_nested_function_body(
            params,
            body,
            is_strict,
            *is_generator,
            *is_async,
            &local_names,
        )?;
        self.emit(Op::NewFunction {
            name: Some(name.clone()),
            has_name_binding: true,
            params: params.clone(),
            local_names,
            lexical_captures,
            bytecode: Rc::new(bytecode),
            // A generator or async function is never constructable.
            constructable: !*is_generator && !*is_async,
            is_strict,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: *is_generator,
            is_async: *is_async,
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

/// Reports a regexp literal whose pattern or flags are invalid as an early
/// (parse-phase) error.
///
/// The parser desugars a `/pattern/flags` literal into a `new RegExp(...)`
/// expression. Unlike a hand-written `new RegExp(...)` call, the literal must
/// fail at parse time, so its validity is checked statically here during
/// bytecode compilation instead of when the constructor runs. The desugared
/// shape is recognized by its single-character `RegExp` callee whose span
/// coincides with the start of the `new` expression (a real constructor call
/// would carry the six-character `RegExp` identifier span placed after `new`).
fn validate_regexp_literal_new(
    callee: &Expr,
    arguments: &[CallArgument],
    span: Span,
) -> Result<(), RuntimeError> {
    let Expr::Identifier {
        name,
        span: callee_span,
    } = callee
    else {
        return Ok(());
    };
    if name != "RegExp" || callee_span.start != span.start || callee_span.end != span.start + 1 {
        return Ok(());
    }
    let mut literals = arguments.iter().filter_map(|argument| match argument {
        CallArgument::Expr(Expr::Literal(Literal::String { value, .. })) => Some(value.as_str()),
        _ => None,
    });
    let Some(pattern) = literals.next() else {
        return Ok(());
    };
    let flags = literals.next().unwrap_or("");
    crate::regexp::validate_regexp_literal(pattern, flags)
}

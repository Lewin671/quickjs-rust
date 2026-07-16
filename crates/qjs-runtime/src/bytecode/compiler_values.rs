use std::rc::Rc;

use qjs_ast::{
    ArrayElement, CallArgument, Expr, Literal, MemberProperty, ObjectProperty, ObjectPropertyKey,
    ObjectPropertyKind, Span, Stmt, VarKind,
};

use crate::{
    RuntimeError, Value,
    function::{collect_function_local_names, is_strict_function_body},
    module::DEFAULT_BINDING,
    value::ObjectLiteralShape,
};

use super::compiler::Compiler;
use super::ir::{ArrayElementKind, ComputedNameKind, ObjectPropertyMeta, Op};
use super::util::{parse_number_literal, unsupported_stmt};
use super::vm_props::array_index_from_number;

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
        if properties.iter().all(|property| {
            matches!(property.kind, ObjectPropertyKind::Data)
                && !property.is_proto_setter
                && matches!(property.key, ObjectPropertyKey::Literal(_))
        }) {
            let mut keys = Vec::with_capacity(properties.len());
            for property in properties {
                let ObjectPropertyKey::Literal(key) = &property.key else {
                    unreachable!("static object literal key checked above")
                };
                keys.push(Rc::from(key.as_str()));
                if let qjs_ast::Expr::Function {
                    name: Some(function_name),
                    constructable: false,
                    ..
                } = &property.value
                    && function_name == key
                {
                    self.compile_function_without_name_binding(&property.value, key)?;
                } else {
                    self.compile_named_expr(&property.value, key)?;
                }
            }
            self.emit(Op::NewObjectDataLiteral {
                shape: ObjectLiteralShape::new(keys),
            });
            return Ok(());
        }

        self.emit(Op::NewObjectLiteral);
        for property in properties {
            if matches!(property.kind, ObjectPropertyKind::Spread) {
                self.compile_expr(&property.value)?;
                self.emit(Op::CopyObjectSpread);
                continue;
            }
            match &property.key {
                ObjectPropertyKey::Literal(key) => {
                    let slot = self.const_slot(Value::String(key.clone().into()));
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
                ObjectPropertyKey::Computed(expr) => {
                    self.compile_expr(expr)?;
                    // ToPropertyKey is part of evaluating the PropertyName and
                    // must run before the value (a computed key's `toString` is
                    // observable before the value expression). The later
                    // SetComputedFunctionName / DefineObjectProperty re-key is
                    // idempotent on the resolved key.
                    self.emit(Op::ToPropertyKey);
                    self.compile_expr(&property.value)?;
                    // A computed key names an anonymous function/accessor value
                    // via SetFunctionName (a Symbol key becomes "[description]").
                    let name_kind = match property.kind {
                        ObjectPropertyKind::Getter => Some(ComputedNameKind::Getter),
                        ObjectPropertyKind::Setter => Some(ComputedNameKind::Setter),
                        ObjectPropertyKind::Data
                            if is_anonymous_function_definition(&property.value) =>
                        {
                            Some(ComputedNameKind::Plain)
                        }
                        _ => None,
                    };
                    if let Some(kind) = name_kind {
                        self.emit(Op::SetComputedFunctionName(kind));
                    }
                    self.emit(Op::DefineObjectProperty(ObjectPropertyMeta {
                        kind: property.kind,
                        is_proto_setter: property.is_proto_setter,
                    }));
                    continue;
                }
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
                let slot = self.const_slot(Value::String(name.clone().into()));
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

    pub(super) fn compile_member_get(
        &mut self,
        property: &MemberProperty,
    ) -> Result<(), RuntimeError> {
        match property {
            MemberProperty::Named(name) => {
                let cache = if let Some(Op::LoadLocal(slot)) = self.code.last() {
                    let slot = *slot;
                    self.code.pop();
                    super::ir::NamedPropertyCache::for_local(slot)
                } else {
                    Default::default()
                };
                self.emit(Op::GetPropNamed {
                    key: Rc::from(name.as_str()),
                    cache,
                });
                return Ok(());
            }
            MemberProperty::Computed(expr) => {
                if let Expr::Literal(Literal::Number { raw, .. }) = expr.as_ref() {
                    let number = parse_number_literal(raw)?;
                    if let Some(index) = array_index_from_number(number) {
                        // Array indices occupy at most 32 bits. On wider hosts,
                        // keep a fused local slot in the existing operand's
                        // upper half so the hot `Op` layout stays unchanged.
                        let encoded = if usize::BITS > u32::BITS
                            && let Some(Op::LoadLocal(slot)) = self.code.last()
                            && let Some(encoded) = slot
                                .checked_add(1)
                                .and_then(|slot| slot.checked_shl(u32::BITS))
                                .map(|slot| slot | index)
                        {
                            self.code.pop();
                            encoded
                        } else {
                            index
                        };
                        self.emit(Op::GetPropIndex(encoded));
                        return Ok(());
                    }
                }
            }
            MemberProperty::Private(_) => {}
        }
        self.compile_member_key(property)?;
        self.emit(Op::GetProp);
        Ok(())
    }

    pub(super) fn compile_delete(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        match argument {
            Expr::Member {
                object, property, ..
            } => {
                // Deleting a SuperReference is a runtime ReferenceError, thrown
                // before the property key is evaluated (so `delete super[expr]`
                // never runs `expr`). Emit the throw without compiling either
                // operand.
                if matches!(object.as_ref(), Expr::Super { .. }) {
                    self.emit(Op::ThrowReferenceError(
                        "Unsupported reference to 'super'".to_owned(),
                    ));
                    return Ok(());
                }
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
                let slot = self.resolve_local_slot(name);
                if self.identifier_needs_with_resolution(slot) {
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

        // `obj?.method(...)` / `a?.b.c(...)`: the callee is an optional member
        // chain, so the call must run inside that chain to short-circuit when a
        // link is nullish and to keep the member's object as `this`.
        if Self::member_chain_has_optional(callee) {
            return self.compile_optional_chain_call(callee, arguments);
        }

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
            self.emit(Op::Dup);
            self.compile_member_get(property)?;
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

        // A bare-identifier call resolved through a `with` scope uses the with
        // binding object as the `this` value when the name is found on it (and
        // undefined otherwise), per GetThisValue of an object-environment
        // reference. `eval` keeps its dedicated direct-eval path.
        if let Expr::Identifier { name, .. } = callee
            && name != "eval"
        {
            let slot = self.resolve_local_slot(name);
            if self.identifier_needs_with_resolution(slot) {
                let object_slot = self.temp_local("with_call_object");
                self.emit(Op::ResolveIdentWith {
                    name: name.clone(),
                    slot,
                    object_slot,
                });
                // Receiver (with object or undefined), then the resolved callee.
                self.emit(Op::LoadLocal(object_slot));
                self.emit(Op::LoadResolvedIdentWith {
                    name: name.clone(),
                    slot,
                    object_slot,
                    is_strict: self.strict,
                });
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
        }

        let direct_eval = matches!(callee, Expr::Identifier { name, .. } if name == "eval");
        self.compile_expr(callee)?;
        if has_spread {
            self.compile_argument_array(arguments)?;
            if direct_eval {
                self.emit(Op::CallDirectEvalSpread {
                    is_strict: self.strict,
                });
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
            self.emit(Op::CallDirectEval {
                argc: arguments.len(),
                is_strict: self.strict,
            });
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
    pub(super) fn compile_super_method(
        &mut self,
        property: &MemberProperty,
    ) -> Result<(), RuntimeError> {
        match property {
            MemberProperty::Named(name) => {
                self.emit(Op::SuperMethod { key: name.clone() });
            }
            MemberProperty::Computed(expr) => {
                self.emit(Op::SuperReference);
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
            span,
            ..
        } = stmt
        else {
            return Err(unsupported_stmt(stmt));
        };
        let source_text = self.function_source_text(*span);
        let blocked_arguments = self.annex_b_arguments_function_name_blocked(name);
        if self.annex_b_function_name_blocked(name) && !blocked_arguments {
            return self.compile_block_scoped_function_decl(
                name,
                params,
                body,
                *is_generator,
                *is_async,
            );
        }
        let is_strict = self.strict || is_strict_function_body(body);
        let is_anonymous_default_export = name == DEFAULT_BINDING;
        let function_name = if is_anonymous_default_export {
            "default"
        } else {
            name
        };
        // A FunctionDeclaration's name belongs to the surrounding declaration
        // environment. References from its body must therefore capture that
        // outer binding (which is also what makes an exported declaration a
        // live binding), rather than creating a separate function-name cell.
        // Named FunctionExpressions pass their name here instead and retain
        // their distinct inner immutable binding.
        let local_names = collect_function_local_names(None, params, body, true);
        let (bytecode, lexical_captures) = self.compile_nested_function_body(
            params,
            body,
            is_strict,
            *is_generator,
            *is_async,
            &local_names,
        )?;
        self.emit(Op::NewFunction {
            name: Some(function_name.to_owned()),
            has_name_binding: false,
            immutable_name_binding: false,
            params: Rc::new(params.clone()),
            local_names: Rc::new(local_names),
            lexical_captures,
            bytecode: Rc::new(bytecode),
            // A generator or async function is never constructable.
            constructable: !*is_generator && !*is_async,
            is_strict,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: *is_generator,
            is_async: *is_async,
            source_text,
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

    fn compile_block_scoped_function_decl(
        &mut self,
        name: &String,
        params: &qjs_ast::FunctionParams,
        body: &[Stmt],
        is_generator: bool,
        is_async: bool,
    ) -> Result<(), RuntimeError> {
        let lexical_slot = self.declare_lexical_slot(name, true);
        let is_strict = self.strict || is_strict_function_body(body);
        let local_names = collect_function_local_names(None, params, body, true);
        let (bytecode, lexical_captures) = self.compile_nested_function_body(
            params,
            body,
            is_strict,
            is_generator,
            is_async,
            &local_names,
        )?;
        self.emit(Op::NewFunction {
            name: Some(name.to_owned()),
            has_name_binding: false,
            immutable_name_binding: false,
            params: Rc::new(params.clone()),
            local_names: Rc::new(local_names),
            lexical_captures,
            bytecode: Rc::new(bytecode),
            constructable: !is_generator && !is_async,
            is_strict,
            lexical_this: false,
            lexical_arguments: false,
            is_generator,
            is_async,
            // The block-scoped Annex B path does not carry the source span;
            // toString falls back to the [native code] form.
            source_text: None,
        });
        self.emit(Op::Dup);
        self.emit(Op::StoreLocal(lexical_slot));
        // Annex B B.3.3 hoists a block-scoped function declaration into the
        // enclosing var scope only in sloppy mode; strict code keeps it purely
        // block-scoped (lexical).
        if self.strict || self.annex_b_function_name_blocked_by_outer_scope(name) {
            self.emit(Op::Pop);
        } else {
            let var_slot = self.local_slot(name, true);
            self.emit_store_var_binding(var_slot, name, VarKind::Var);
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

/// Whether `expr` is an anonymous function definition (an unnamed function,
/// arrow, or class) that participates in NamedEvaluation / SetFunctionName.
fn is_anonymous_function_definition(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Function { name: None, .. } | Expr::Class { name: None, .. }
    )
}

#[cfg(test)]
mod tests {
    use super::super::{compiler, ir::Op};

    #[test]
    fn static_member_reads_use_named_property_op() {
        let script = qjs_parser::parse_script(
            "let object = { value: 1 }; let key = 'value'; object.value; ({ value: 2 }).value; object[key];",
        )
        .expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");

        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| {
                    matches!(op, Op::GetPropNamed { key, cache } if key.as_ref() == "value" && cache.local_slot().is_some())
                })
                .count(),
            1
        );
        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| {
                    matches!(op, Op::GetPropNamed { key, cache } if key.as_ref() == "value" && cache.local_slot().is_none())
                })
                .count(),
            1
        );
        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| matches!(op, Op::GetProp))
                .count(),
            1
        );
    }

    #[test]
    fn numeric_literal_member_reads_use_index_property_op() {
        let script = qjs_parser::parse_script(
            "let value = [1]; value[0]; value[0x1]; value[1.5]; value[-0]; value[key];",
        )
        .expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");

        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| {
                    matches!(op, Op::GetPropIndex(encoded) if encoded & u32::MAX as usize <= 1)
                })
                .count(),
            2
        );
        if usize::BITS > u32::BITS {
            assert_eq!(
                bytecode
                    .code
                    .iter()
                    .filter(
                        |op| matches!(op, Op::GetPropIndex(encoded) if encoded >> u32::BITS != 0)
                    )
                    .count(),
                2
            );
        }
        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| matches!(op, Op::GetProp))
                .count(),
            3
        );
    }

    #[test]
    fn static_data_object_literals_use_bulk_allocation_op() {
        let script = qjs_parser::parse_script(
            "({ a: side(1), b: 2, a: 3 }); ({ [key]: 1 }); ({ __proto__: value }); ({ get x() {} }); ({ ...source });",
        )
        .expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");

        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| {
                    matches!(op, Op::NewObjectDataLiteral { shape } if shape.input_len() == 3)
                })
                .count(),
            1
        );
        assert_eq!(
            bytecode
                .code
                .iter()
                .filter(|op| matches!(op, Op::NewObjectLiteral))
                .count(),
            4
        );
    }
}

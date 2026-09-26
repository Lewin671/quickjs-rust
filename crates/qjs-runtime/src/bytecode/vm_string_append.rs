use qjs_ast::BinaryOp;

use crate::{RuntimeError, Value, operations};

use super::{ir::Op, vm::Vm};

impl Vm<'_> {
    /// Drops only engine-internal mirrors before a compound string assignment.
    /// The following store bytecode restores the binding immediately, with an
    /// optional preceding `Dup` when the assignment value remains observable.
    /// Any real JavaScript alias keeps the Rc shared and therefore immutable.
    pub(super) fn prepare_compound_string_reuse(&mut self, expected: &crate::JsString) -> bool {
        // A handle, not a copy of the instruction: cloning the store op
        // cloned its binding name, an allocation on every string `+`.
        let bytecode = self.bytecode.clone();
        let store_ip = match bytecode.code.get(self.ip) {
            Some(Op::Dup) => self.ip + 1,
            _ => self.ip,
        };
        match bytecode.code.get(store_ip) {
            Some(Op::AssignLocal(slot)) => self.detach_matching_local_string(*slot, expected),
            Some(Op::StoreGlobalStrict(name)) | Some(Op::StoreGlobalSloppy { name, .. }) => {
                self.detach_matching_realm_string(name, expected)
            }
            Some(Op::StoreLocalOrGlobalSloppy { slot, name }) => {
                self.detach_matching_local_string(*slot, expected)
                    || self.detach_matching_realm_string(name, expected)
            }
            _ => false,
        }
    }

    fn detach_matching_local_string(&mut self, slot: usize, expected: &crate::JsString) -> bool {
        if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
            return false;
        }
        let Some(local_meta) = self.bytecode.locals.get(slot) else {
            return false;
        };
        let name = local_meta.name.as_str();
        if !local_meta.mutable
            || self.env.has_module_import(name)
            || self.env.is_immutable_lexical_binding(name)
            || self.env.is_immutable_function_name(name)
            || self.local_slot_targets_non_writable_global(slot, name)
        {
            return false;
        }
        if self.slot_is_authoritative(slot)
            && let Some(Some(local)) = self.locals.get_mut(slot)
            && matches!(local, Value::String(current) if crate::JsString::ptr_eq(current, expected))
        {
            *local = Value::Undefined;
            self.release_dead_completion_copies(expected);
            return true;
        }
        self.detach_matching_shared_string(slot, expected)
    }

    /// Drops copies of `expected` parked in the frame's dead completion
    /// temporaries. `if (c) text += part;` inside a function-body loop stores
    /// the statement's value in the block's and the loop's result slots, which
    /// nothing can read; left there, they kept the buffer shared and every
    /// append copied the whole string.
    pub(super) fn release_dead_completion_copies(&mut self, expected: &crate::JsString) {
        if expected.is_unique() || self.bytecode.dead_completion_slots.is_empty() {
            return;
        }
        let frame = &mut self.current;
        for slot in frame.bytecode.dead_completion_slots.iter().copied() {
            if let Some(Some(local)) = frame.locals.get_mut(slot)
                && matches!(&*local, Value::String(current) if crate::JsString::ptr_eq(current, expected))
            {
                *local = Value::Undefined;
            }
        }
    }

    /// Temporarily clears the engine's internal mirrors of one shared binding
    /// value so `Rc::unwrap_or_clone` can reclaim its allocation. Any actual
    /// JavaScript alias keeps an Rc alive and therefore still forces a copy,
    /// preserving string immutability. The completed assignment immediately
    /// restores the slot/cell/realm mirrors through the normal store path.
    fn detach_matching_shared_string(&mut self, slot: usize, expected: &crate::JsString) -> bool {
        if self.direct_eval_with_stack {
            return false;
        }
        let Some(cell) = self
            .local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .cloned()
        else {
            return false;
        };
        let matches = cell.with_value(|value| {
            matches!(value, Value::String(current) if crate::JsString::ptr_eq(current, expected))
        });
        if !matches {
            return false;
        }

        let name = self.bytecode.locals[slot].name.clone();
        let realm_cell = self.env.is_realm_binding_cell(&name, &cell);
        let global_this = if realm_cell {
            self.cached_global_this()
        } else {
            None
        };
        if let Some(property) = global_this
            .as_ref()
            .and_then(|global_this| global_this.own_property(&name))
            && (property.is_accessor() || !property.writable)
        {
            return false;
        }

        cell.set(Value::Undefined);
        if let Some(Some(local)) = self.locals.get_mut(slot)
            && matches!(local, Value::String(current) if crate::JsString::ptr_eq(current, expected))
        {
            *local = Value::Undefined;
        }
        if realm_cell {
            // `cell` (already set to `Value::Undefined` above) *is* the
            // realm's canonical binding for `name` when `realm_cell` is
            // true — the realm's binding storage has no separate raw map to
            // mirror this into anymore, only the globalThis own-property
            // value below is a distinct JS-observable storage.
            if let Some(global_this) = global_this
                && global_this
                    .own_property(&name)
                    .is_some_and(|property| {
                        matches!(property.value, Value::String(current) if crate::JsString::ptr_eq(&current, expected))
                    })
            {
                global_this.set(name, Value::Undefined);
            }
        }
        true
    }

    fn detach_matching_realm_string(&mut self, name: &str, expected: &crate::JsString) -> bool {
        if self.env.has_module_import(name)
            || self.env.is_immutable_lexical_binding(name)
            || self.env.is_immutable_function_name(name)
            || self.env.has_local_binding(name)
            || self
                .bytecode
                .local_slot(name)
                .is_some_and(|slot| self.locals.get(slot).is_some_and(Option::is_some))
        {
            return false;
        }
        let realm_matches = self.realm.get_value(name).is_some_and(|value| {
            matches!(value, Value::String(current) if crate::JsString::ptr_eq(&current, expected))
        });
        if !realm_matches {
            return false;
        }
        let global_this = self.cached_global_this();
        if let Some(property) = global_this
            .as_ref()
            .and_then(|global_this| global_this.own_property(name))
            && (property.is_accessor() || !property.writable)
        {
            return false;
        }
        let cell = self.env.realm_binding_cell(name);
        if let Some(cell) = &cell
            && !cell.with_value(|value| {
                matches!(value, Value::String(current) if crate::JsString::ptr_eq(current, expected))
            })
        {
            return false;
        }
        if let Some(cell) = cell {
            // `cell` *is* the realm's canonical binding for `name` — no
            // separate raw map entry to mirror this into.
            cell.set(Value::Undefined);
        }
        if let Some(global_this) = global_this
            && global_this
                .own_property(name)
                .is_some_and(|property| {
                    matches!(property.value, Value::String(current) if crate::JsString::ptr_eq(&current, expected))
                })
        {
            global_this.set(name.to_owned(), Value::Undefined);
        }
        true
    }

    pub(super) fn run_string_append_op(&mut self, op: Op) -> Result<(), RuntimeError> {
        let (result, discard) = match op {
            Op::AppendStringLiteralLocal {
                slot,
                value,
                discard,
            } => (self.append_string_literal_local(slot, &value), discard),
            Op::AppendStringLiteralGlobal {
                name,
                value,
                is_strict,
                discard,
            } => (
                self.append_string_literal_global(&name, &value, is_strict),
                discard,
            ),
            _ => unreachable!("string append dispatcher received a non-append opcode"),
        };
        if let Some(value) = self.handle_runtime_result(result)?
            && !discard
        {
            self.stack.push(value);
        }
        Ok(())
    }

    fn append_string_literal_local(
        &mut self,
        slot: usize,
        suffix: &str,
    ) -> Result<Value, RuntimeError> {
        let local_meta = self
            .bytecode
            .locals
            .get(slot)
            .cloned()
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
        if !local_meta.mutable {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // The append opcode mutates the local string in place as a fast path.
        // A received capture's authoritative value is its shared cell, not the
        // compatibility slot snapshot left from function entry; refresh that
        // one slot before taking the mutable string reference.
        let shared_value = self.upvalue_slot_value(slot);
        if shared_value.is_none()
            && let Some(Some(Value::String(current))) = self.locals.get(slot)
            && !current.is_unique()
        {
            let expected = current.clone();
            self.release_dead_completion_copies(&expected);
        }
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        if let Some(value) = shared_value {
            *local = Some(value);
        }
        let Some(value) = local else {
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{}`", local_meta.name),
            });
        };
        let Value::String(string) = value else {
            let left = value.clone();
            let mut env = self.current_env();
            let result = operations::eval_binary(
                left,
                BinaryOp::Add,
                Value::String(suffix.to_owned().into()),
                &mut env,
            )?;
            self.apply_env(env);
            self.store_local(slot, result.clone())?;
            return Ok(result);
        };
        string.make_mut().push_str(suffix);
        let result = Value::String(string.clone());
        if let Some(upvalue) = self.local_upvalues.get(slot).and_then(Option::as_ref) {
            upvalue.set(result.clone());
        }
        self.write_through_module_live_binding(&local_meta.name, result.clone());
        if local_meta.from_env || self.bytecode.local_is_body_hoist_only(slot) {
            let name = local_meta.name.clone();
            if self.env.has_local_binding(&name) {
                self.env.insert(name, result.clone());
            }
        }
        let syncs_global_var = (local_meta.from_env && !local_meta.hoisted)
            || (self.bytecode.global_scope
                && self.bytecode.local_is_body_hoist_only(slot)
                && !self.bytecode.local_is_compiler_temporary(slot));
        let global_this = syncs_global_var
            .then(|| self.cached_global_this())
            .flatten();
        if let Some(global_this) = global_this
            && global_this.has_own_property(&local_meta.name)
        {
            global_this
                .append_string_property(&local_meta.name, suffix)
                .unwrap_or_else(|| {
                    global_this.set(local_meta.name.clone(), result.clone());
                    result.clone()
                });
            if self.realm.contains(&local_meta.name) {
                // A top-level reader may already hold the realm binding's
                // shared cell even when this older from-env frame still uses a
                // compatibility slot. Route the mirror through CallEnv so the
                // cell cannot retain the pre-append string.
                self.env.insert_realm(local_meta.name, result.clone());
            }
        }
        Ok(result)
    }

    fn append_string_literal_global(
        &mut self,
        name: &str,
        suffix: &str,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        if self.env.has_local_binding(name) {
            return self.append_string_literal_global_via_store(name, suffix, is_strict);
        }
        {
            // The cell is the sole storage for this binding, so mutating it
            // in place needs no separate cell refresh afterward — unlike the
            // old two-map realm model, where `Rc::make_mut` on the raw map's
            // copy could detach the value from an already-captured cell.
            let appended = self.realm.cell(name).and_then(|cell| {
                cell.with_value_mut(|value| {
                    let Value::String(string) = value else {
                        return None;
                    };
                    string.make_mut().push_str(suffix);
                    Some(value.clone())
                })
            });
            if let Some(result) = appended {
                if let Some(global_this) = self.cached_global_this()
                    && global_this.has_own_property(name)
                {
                    global_this
                        .append_string_property(name, suffix)
                        .unwrap_or_else(|| {
                            global_this.set(name.to_owned(), result.clone());
                            result.clone()
                        });
                }
                self.write_through_module_live_binding(name, result.clone());
                return Ok(result);
            }
        }

        self.append_string_literal_global_via_store(name, suffix, is_strict)
    }

    fn append_string_literal_global_via_store(
        &mut self,
        name: &str,
        suffix: &str,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        let left = self.load_global(name)?;
        let mut env = self.current_env();
        let result = operations::eval_binary(
            left,
            BinaryOp::Add,
            Value::String(suffix.to_owned().into()),
            &mut env,
        )?;
        self.apply_env(env);
        if is_strict {
            self.store_global_strict(name, result.clone())?;
        } else {
            self.store_global_sloppy(name, result.clone())?;
        }
        Ok(result)
    }
}

/// Appends the string form of a primitive that needs no `ToPrimitive`,
/// borrowing a string operand rather than copying it into a buffer of its
/// own first; `false`, with `out` unchanged, for anything else.
pub(super) fn push_primitive(out: &mut String, value: &Value) -> bool {
    match value {
        Value::String(value) => out.push_str(value),
        Value::Number(number) => crate::number::push_number_js_string(out, *number),
        Value::BigInt(value) => out.push_str(&value.to_string()),
        Value::Boolean(true) => out.push_str("true"),
        Value::Boolean(false) => out.push_str("false"),
        Value::Null => out.push_str("null"),
        Value::Undefined => out.push_str("undefined"),
        _ => return false,
    }
    true
}

/// Whether [`push_primitive`] appends `value`.
pub(super) fn is_appendable(value: &Value) -> bool {
    is_plain_primitive(value)
}

fn is_plain_primitive(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_)
            | Value::Number(_)
            | Value::BigInt(_)
            | Value::Boolean(_)
            | Value::Null
            | Value::Undefined
    )
}

/// `receiver.concat(...arguments)` on a string receiver when every argument
/// is a primitive whose ToString runs no user code: the result built in one
/// buffer, numbers formatted straight into it. `None` otherwise.
pub(crate) fn concat_string_with_primitives(
    receiver: &crate::JsString,
    arguments: &[Value],
) -> Option<Value> {
    if !arguments.iter().all(is_plain_primitive) {
        return None;
    }
    let extra: usize = arguments
        .iter()
        .map(|value| match value {
            Value::String(text) => text.len(),
            _ => 24,
        })
        .sum();
    let mut text = String::with_capacity(receiver.len() + extra);
    text.push_str(receiver);
    for value in arguments {
        push_primitive(&mut text, value);
    }
    Some(Value::String(text.into()))
}

/// `left + right` when one side is a string and neither needs
/// `ToPrimitive`: the concatenation, extending the left string's own buffer
/// when nothing else holds it -- the intermediate of `a + b + c` is such a
/// buffer -- and borrowing the right operand. Hands both operands back
/// otherwise.
pub(crate) fn concat_primitives(left: Value, right: Value) -> Result<Value, (Value, Value)> {
    if !(matches!(left, Value::String(_)) || matches!(right, Value::String(_)))
        || !is_plain_primitive(&left)
        || !is_plain_primitive(&right)
    {
        return Err((left, right));
    }
    let right_len = match &right {
        Value::String(right) => right.len(),
        _ => 16,
    };
    let mut out = match left {
        // Grown in place: the buffer and its box are both reused.
        Value::String(mut left) if left.is_unique() => {
            let text = left.make_mut();
            text.reserve(right_len);
            push_primitive(text, &right);
            return Ok(Value::String(left));
        }
        Value::String(left) => {
            let mut text = String::with_capacity(left.len() + right_len);
            text.push_str(&left);
            text
        }
        left => {
            let mut text = String::with_capacity(16 + right_len);
            push_primitive(&mut text, &left);
            text
        }
    };
    push_primitive(&mut out, &right);
    Ok(Value::String(out.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

    #[test]
    fn primitive_concatenation_extends_an_unshared_left_string() {
        let text = |value: &Value| match value {
            Value::String(text) => text.as_str().to_owned(),
            other => panic!("expected a string, got {other:?}"),
        };
        let left = crate::JsString::from("ab".to_owned());
        let before = left.clone();
        // Shared: the original keeps its text.
        let shared = concat_primitives(Value::String(left), Value::Number(1.5)).unwrap();
        assert_eq!(text(&shared), "ab1.5");
        assert_eq!(before.as_str(), "ab");
        // Unshared: extended where it is.
        let Value::String(unique) = shared else {
            unreachable!()
        };
        let grown = concat_primitives(Value::String(unique), Value::Boolean(true)).unwrap();
        assert_eq!(text(&grown), "ab1.5true");
        let appended = concat_primitives(grown, Value::Null).unwrap();
        assert_eq!(text(&appended), "ab1.5truenull");
        for (left, right, expected) in [
            (
                Value::Number(2.0),
                Value::String("x".to_owned().into()),
                "2x",
            ),
            (
                Value::Undefined,
                Value::String("!".to_owned().into()),
                "undefined!",
            ),
            (
                Value::String("s".to_owned().into()),
                Value::Undefined,
                "sundefined",
            ),
        ] {
            assert_eq!(text(&concat_primitives(left, right).unwrap()), expected);
        }
        // Neither side a string, or an object that needs `ToPrimitive`.
        assert!(concat_primitives(Value::Number(1.0), Value::Number(2.0)).is_err());
        let object = eval("({ toString() { return 'o'; } })").unwrap();
        assert!(concat_primitives(Value::String("a".to_owned().into()), object).is_err());
        assert_eq!(
            eval("var o = { toString() { return 'o'; } }; var s = 'a' + 1; s + o + s;"),
            Ok(Value::String("a1oa1".to_owned().into()))
        );
    }

    #[test]
    fn captured_global_string_append_releases_realm_read_before_sync() {
        assert_eq!(
            eval(
                "var trace = ''; function outer() { return function() { trace += '1'; }; } outer()(); trace;"
            ),
            Ok(Value::String("1".to_owned().into()))
        );
    }

    #[test]
    fn discarded_dynamic_string_append_keeps_direct_store_shape() {
        let script = qjs_parser::parse_script(
            "function join(parts) { let result = ''; for (let index = 0; index < parts.length; index++) { result += parts[index]; } return result; }",
        )
        .expect("script should parse");
        let bytecode = compiler::compile_script(&script).expect("script should compile");
        let join = bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction {
                    name: Some(name),
                    bytecode,
                    ..
                } if name == "join" => Some(bytecode),
                _ => None,
            })
            .expect("join bytecode should be present");

        assert!(
            join.code
                .windows(2)
                .any(|ops| matches!(ops, [Op::Binary(BinaryOp::Add), Op::AssignLocal(_)])),
            "discarded compound assignment should store directly: {:#?}",
            join.code
        );
    }

    /// `if (c) text += part;` in a function-body loop parks each result in
    /// the block's and loop's completion slots. Those are dead in a function
    /// body, so the append must release them and keep the buffer unique; a
    /// genuine alias of the string still forces a copy.
    #[test]
    fn conditional_string_append_in_function_loop_stays_unique() {
        let script = qjs_parser::parse_script(
            "function build(n) { var text = ''; for (var i = 0; i < n; i++) { if (i >= 0) text += 'a'; } return text; }",
        )
        .expect("script should parse");
        let bytecode = compiler::compile_script(&script).expect("script should compile");
        let build = bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode),
                _ => None,
            })
            .expect("build bytecode should be present");
        assert_eq!(
            build.dead_completion_slots.len(),
            2,
            "block and loop result slots: {:#?}",
            build.locals
        );
        assert!(bytecode.dead_completion_slots.is_empty());
        assert_eq!(
            eval(
                "function build(n) { var text = ''; for (var i = 0; i < n; i++) { if (i % 2) text += 'b'; else text += 'a'; } return text; } build(6);"
            ),
            Ok(Value::String("ababab".into()))
        );
        // With a real alias the completion slot is not the only other owner,
        // and the alias must keep its old value.
        assert_eq!(
            eval(
                "function build(n) { var text = '', seen; for (var i = 0; i < n; i++) { if (i >= 0) seen = text += 'a'; } return seen + ':' + text; } build(3);"
            ),
            Ok(Value::String("aaa:aaa".into()))
        );
        assert_eq!(
            eval(
                "function build(n) { var text = '', first; for (var i = 0; i < n; i++) { if (i == 0) first = text; if (i >= 0) text += 'a'; } return first + ':' + text; } build(3);"
            ),
            Ok(Value::String(":aaa".into()))
        );
    }

    #[test]
    fn discarded_dynamic_string_append_preserves_alias_and_capture() {
        assert_eq!(
            eval(
                "function collect(parts) {\
                    let result = '';\
                    const initial = result;\
                    function read() { return result; }\
                    for (let index = 0; index < parts.length; index++) {\
                        result += parts[index];\
                    }\
                    return initial + ':' + read() + ':' + result;\
                }\
                collect(['a', 'bc', 'd']);",
            ),
            Ok(Value::String(":abcd:abcd".into()))
        );
    }
}

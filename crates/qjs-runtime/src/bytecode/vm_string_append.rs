use qjs_ast::BinaryOp;

use crate::{GLOBAL_THIS_BINDING, RuntimeError, Value, operations};

use super::{ir::Op, vm::Vm, vm_bindings::is_compiler_temporary};

impl Vm<'_> {
    /// Drops only engine-internal mirrors before a compound string assignment.
    /// The following `Dup` + store bytecodes restore the binding immediately;
    /// any real JavaScript alias keeps the Rc shared and therefore immutable.
    pub(super) fn prepare_compound_string_reuse(&mut self, expected: &std::rc::Rc<String>) -> bool {
        if !matches!(self.bytecode.code.get(self.ip), Some(Op::Dup)) {
            return false;
        }
        match self.bytecode.code.get(self.ip + 1).cloned() {
            Some(Op::AssignLocal(slot)) => self.detach_matching_local_string(slot, expected),
            Some(Op::StoreGlobalStrict(name)) | Some(Op::StoreGlobalSloppy(name)) => {
                self.detach_matching_realm_string(&name, expected)
            }
            Some(Op::StoreLocalOrGlobalSloppy { slot, name }) => {
                self.detach_matching_local_string(slot, expected)
                    || self.detach_matching_realm_string(&name, expected)
            }
            _ => false,
        }
    }

    fn detach_matching_local_string(
        &mut self,
        slot: usize,
        expected: &std::rc::Rc<String>,
    ) -> bool {
        if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
            return false;
        }
        let Some(local_meta) = self.bytecode.locals.get(slot) else {
            return false;
        };
        let name = local_meta.name.clone();
        if !local_meta.mutable
            || self.env.has_module_import(&name)
            || self.env.is_immutable_lexical_binding(&name)
            || self.env.is_immutable_function_name(&name)
            || self.local_slot_targets_non_writable_global(slot, &name)
        {
            return false;
        }
        if self.slot_is_authoritative(slot)
            && let Some(Some(local)) = self.locals.get_mut(slot)
            && matches!(local, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
        {
            *local = Value::Undefined;
            return true;
        }
        self.detach_matching_shared_string(slot, expected)
    }

    /// Temporarily clears the engine's internal mirrors of one shared binding
    /// value so `Rc::unwrap_or_clone` can reclaim its allocation. Any actual
    /// JavaScript alias keeps an Rc alive and therefore still forces a copy,
    /// preserving string immutability. The completed assignment immediately
    /// restores the slot/cell/realm mirrors through the normal store path.
    fn detach_matching_shared_string(
        &mut self,
        slot: usize,
        expected: &std::rc::Rc<String>,
    ) -> bool {
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
            matches!(value, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
        });
        if !matches {
            return false;
        }

        let name = self.bytecode.locals[slot].name.clone();
        let realm_cell = self.env.is_realm_binding_cell(&name, &cell);
        let global_this = if realm_cell {
            match self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned() {
                Some(Value::Object(global_this)) => Some(global_this),
                _ => None,
            }
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
            && matches!(local, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
        {
            *local = Value::Undefined;
        }
        if realm_cell {
            if let Some(binding) = self.realm.borrow_mut().get_mut(&name)
                && matches!(binding, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
            {
                *binding = Value::Undefined;
            }
            if let Some(global_this) = global_this
                && global_this
                    .own_property(&name)
                    .is_some_and(|property| {
                        matches!(property.value, Value::String(current) if std::rc::Rc::ptr_eq(&current, expected))
                    })
            {
                global_this.set(name, Value::Undefined);
            }
        }
        true
    }

    fn detach_matching_realm_string(&mut self, name: &str, expected: &std::rc::Rc<String>) -> bool {
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
        let realm_matches = self.realm.borrow().get(name).is_some_and(|value| {
            matches!(value, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
        });
        if !realm_matches {
            return false;
        }
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned() {
            Some(Value::Object(global_this)) => Some(global_this),
            _ => None,
        };
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
                matches!(value, Value::String(current) if std::rc::Rc::ptr_eq(current, expected))
            })
        {
            return false;
        }
        if let Some(cell) = cell {
            cell.set(Value::Undefined);
        }
        if let Some(binding) = self.realm.borrow_mut().get_mut(name) {
            *binding = Value::Undefined;
        }
        if let Some(global_this) = global_this
            && global_this
                .own_property(name)
                .is_some_and(|property| {
                    matches!(property.value, Value::String(current) if std::rc::Rc::ptr_eq(&current, expected))
                })
        {
            global_this.set(name.to_owned(), Value::Undefined);
        }
        true
    }

    pub(super) fn run_string_append_op(&mut self, op: Op) -> Result<(), RuntimeError> {
        let result = match op {
            Op::AppendStringLiteralLocal { slot, value } => {
                self.append_string_literal_local(slot, &value)
            }
            Op::AppendStringLiteralGlobal {
                name,
                value,
                is_strict,
            } => self.append_string_literal_global(&name, &value, is_strict),
            _ => unreachable!("string append dispatcher received a non-append opcode"),
        };
        if let Some(value) = self.handle_runtime_result(result)? {
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
        std::rc::Rc::make_mut(string).push_str(suffix);
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
                && !is_compiler_temporary(&local_meta.name));
        let global_this = syncs_global_var
            .then(|| self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned())
            .flatten();
        if let Some(Value::Object(global_this)) = global_this
            && global_this.has_own_property(&local_meta.name)
        {
            global_this
                .append_string_property(&local_meta.name, suffix)
                .unwrap_or_else(|| {
                    global_this.set(local_meta.name.clone(), result.clone());
                    result.clone()
                });
            if self.realm.borrow().contains_key(&local_meta.name) {
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
            let mut realm = self.realm.borrow_mut();
            if let Some(Value::String(string)) = realm.get_mut(name) {
                std::rc::Rc::make_mut(string).push_str(suffix);
                let result = Value::String(string.clone());
                drop(realm);
                if let Some(Value::Object(global_this)) =
                    self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
                    && global_this.has_own_property(name)
                {
                    global_this
                        .append_string_property(name, suffix)
                        .unwrap_or_else(|| {
                            global_this.set(name.to_owned(), result.clone());
                            result.clone()
                        });
                }
                // `Rc::make_mut` can detach the realm string from a cell's
                // earlier clone. Refresh that cell after the in-place append.
                self.env.insert_realm(name.to_owned(), result.clone());
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

pub(super) fn primitive_append_suffix(value: Value) -> Result<String, Value> {
    Ok(match value {
        Value::Number(number) => crate::number::number_to_js_string(number),
        Value::BigInt(value) => value.to_string(),
        Value::String(value) => std::rc::Rc::unwrap_or_clone(value),
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        value => return Err(value),
    })
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn captured_global_string_append_releases_realm_read_before_sync() {
        assert_eq!(
            eval(
                "var trace = ''; function outer() { return function() { trace += '1'; }; } outer()(); trace;"
            ),
            Ok(Value::String("1".to_owned().into()))
        );
    }
}

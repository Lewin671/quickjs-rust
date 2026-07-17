use qjs_ast::BinaryOp;

use crate::{GLOBAL_THIS_BINDING, RuntimeError, Value, operations};

use super::{ir::Op, vm::Vm, vm_bindings::is_compiler_temporary};

impl Vm<'_> {
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
        if syncs_global_var
            && let Some(Value::Object(global_this)) =
                self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
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
            self.store_global_strict(name.to_owned(), result.clone())?;
        } else {
            self.store_global_sloppy(name.to_owned(), result.clone())?;
        }
        Ok(result)
    }
}

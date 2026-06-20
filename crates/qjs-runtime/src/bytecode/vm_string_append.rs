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
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        if !local_meta.mutable {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
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
        self.write_through_captured(&local_meta.name, result.clone());
        if local_meta.from_env || self.bytecode.local_is_body_hoist_only(slot) {
            let name = local_meta.name.clone();
            if self.env.locals().contains_key(&name) {
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
                self.realm
                    .borrow_mut()
                    .insert(local_meta.name, result.clone());
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
                return Ok(result);
            }
        }

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

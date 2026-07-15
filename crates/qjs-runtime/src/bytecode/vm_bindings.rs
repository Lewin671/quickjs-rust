use std::collections::{HashMap, HashSet};

use crate::{
    GLOBAL_THIS_BINDING, Property, PropertyKey, RuntimeError, Value,
    function::{CallEnv, Upvalue},
    is_truthy,
    object::boxed_primitive,
    property::has_property,
    property_value, property_value_key,
    symbol::unscopables_symbol,
};

use super::util::typeof_value;
use super::{
    ir::{Bytecode, Op},
    vm::{Slot, Vm},
    vm_props::get_property,
    vm_set::set_property_key,
};

impl Vm<'_> {
    pub(super) fn enter_body_deopt_scope(&mut self) {
        let Some(parameter_bindings) = self.env.deopt_bindings().cloned() else {
            return;
        };
        let split_slots = self
            .bytecode
            .locals
            .iter()
            .enumerate()
            .filter_map(|(slot, local)| {
                let marker = format!(
                    "{}{}",
                    crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                    local.name
                );
                (local.hoisted && !local.parameter && parameter_bindings.contains_key(&marker))
                    .then_some(slot)
            })
            .collect::<Vec<_>>();
        if split_slots.is_empty() {
            return;
        }
        let body_bindings = crate::function::DynamicBindings::new();
        for (name, upvalue) in parameter_bindings.cells() {
            if split_slots
                .iter()
                .any(|slot| self.bytecode.locals[*slot].name == name)
            {
                continue;
            }
            body_bindings.insert_cell(name, upvalue);
        }
        for slot in split_slots {
            let name = self.bytecode.locals[slot].name.clone();
            let value = Value::Undefined;
            let upvalue = Upvalue::new(value.clone());
            self.locals[slot] = Some(value);
            self.local_upvalues[slot] = Some(upvalue.clone());
            body_bindings.insert_cell(name, upvalue);
        }
        self.env.set_deopt_bindings(body_bindings);
    }

    /// Executes a `with`-related opcode: scope push/pop and the with-aware
    /// identifier load/store/typeof. Centralizing the stack interaction here
    /// keeps the main bytecode loop terse.
    pub(super) fn run_with_op(&mut self, op: Op) -> Result<(), RuntimeError> {
        match op {
            Op::EnterWith => {
                let value = self.pop()?;
                let result: Result<Value, RuntimeError> = match value {
                    Value::Null | Value::Undefined => Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: cannot convert null or undefined to object".to_owned(),
                    }),
                    Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => {
                        let env = self.realm_env();
                        Ok(boxed_primitive(value, &env)
                            .expect("primitive value should box to object"))
                    }
                    other => Ok(other),
                };
                if let Some(object) = self.handle_runtime_result(result)? {
                    self.with_stack.push(object);
                }
            }
            Op::ExitWith => {
                self.with_stack.pop();
            }
            Op::LoadIdentWith {
                name,
                slot,
                is_strict,
            } => {
                let result = self.load_ident_with(&name, slot, is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::ResolveIdentWith {
                name, object_slot, ..
            } => {
                let result = self.resolve_ident_with(&name, object_slot);
                self.handle_runtime_result(result)?;
            }
            Op::LoadResolvedIdentWith {
                name,
                slot,
                object_slot,
                is_strict,
            } => {
                let result = self.load_resolved_ident_with(&name, slot, object_slot, is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::StoreIdentWith {
                name,
                slot,
                is_strict,
            } => {
                let value = self.pop()?;
                let result = self.store_ident_with(&name, slot, is_strict, value);
                self.handle_runtime_result(result)?;
            }
            Op::StoreResolvedIdentWith {
                name,
                slot,
                object_slot,
                is_strict,
            } => {
                let value = self.pop()?;
                let result =
                    self.store_resolved_ident_with(&name, slot, object_slot, is_strict, value);
                self.handle_runtime_result(result)?;
            }
            Op::TypeofIdentWith { name, slot } => {
                let result = self.typeof_ident_with(&name, slot);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::DeleteIdentWith { name, slot } => {
                self.delete_ident_with(&name, slot)?;
            }
            _ => unreachable!("run_with_op received a non-with opcode"),
        }
        Ok(())
    }

    pub(super) fn initialize_script_global_bindings(
        bytecode: &Bytecode,
        globals: &mut HashMap<String, Value>,
    ) -> Result<(), RuntimeError> {
        let global_this = globals
            .get(GLOBAL_THIS_BINDING)
            .and_then(|value| match value {
                Value::Object(object) => Some(object.clone()),
                _ => None,
            });
        if let Some(global_this) = &global_this {
            for name in bytecode.global_lexical_names() {
                if global_this
                    .own_property(name)
                    .is_some_and(|property| !property.configurable)
                {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "SyntaxError: global lexical declaration `{name}` conflicts with an existing var binding"
                        ),
                    });
                }
            }
        }
        for name in bytecode.hoisted_local_names() {
            if let Some(property) = global_this
                .as_ref()
                .and_then(|object| object.own_property(name))
            {
                globals.insert(name.to_owned(), property.value);
            } else {
                globals.entry(name.to_owned()).or_insert(Value::Undefined);
                if let Some(global_this) = &global_this {
                    global_this.define_property(
                        name.to_owned(),
                        Property::data(Value::Undefined, true, true, false),
                    );
                }
            }
        }
        Ok(())
    }

    pub(super) fn initial_slots(bytecode: &Bytecode, env: &CallEnv) -> Vec<Slot> {
        bytecode
            .locals
            .iter()
            .map(|local| {
                // Global-scope `var`/function bindings live in the realm; the
                // vestigial slot stays empty so captures and loads route to
                // the shared cell instead of a frozen copy.
                if local.hoisted && bytecode.global_scope {
                    None
                } else if !local.from_env
                    && crate::function::is_call_frame_binding(&local.name)
                    && let Some(value) = env.get_local(&local.name)
                {
                    // A declaring function's materialized `this`/`arguments`
                    // slot is seeded by call setup. It is deliberately not an
                    // indexed upvalue; a nested arrow captures the resulting
                    // local cell as an ordinary ParentLocal source.
                    Some(value)
                } else if local.from_env
                    && let Some(value) = env.get_local(&local.name)
                    && !matches!(
                        &value,
                        Value::Function(function) if function.is_uninitialized_lexical_marker()
                    )
                {
                    // Only a binding the caller passed in the frame's locals
                    // layer seeds a from_env slot; realm globals stay in the
                    // shared cell so closures observe live values.
                    Some(value)
                } else if local.from_env
                    && !local.hoisted
                    && !crate::function::is_call_frame_binding(&local.name)
                    && let Some(value) = env.get_realm(&local.name)
                {
                    // A script-level `var`/function binding is realm-backed,
                    // not an incoming lexical upvalue. Seed the compatibility
                    // slot from the live realm; stores below synchronize the
                    // global object and realm rather than creating a closure
                    // snapshot.
                    Some(value)
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn initial_local_upvalues(
        bytecode: &Bytecode,
        _locals: &[Slot],
        upvalues: &[Upvalue],
        env: &CallEnv,
    ) -> Vec<Option<Upvalue>> {
        let mut next_received = 0;
        let mut local_upvalues = vec![None; bytecode.locals.len()];
        let direct_eval_frame = matches!(
            env.get_local(crate::DIRECT_EVAL_BINDING),
            Some(Value::Boolean(true))
        );
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if let Some(upvalue) = env.module_import_cell(&local.name) {
                local_upvalues[slot] = Some(upvalue);
                if local.is_received_upvalue() {
                    next_received += 1;
                }
                continue;
            }
            if local.sloppy_global_fallback {
                local_upvalues[slot] = env.realm_binding_cell(&local.name);
                continue;
            }
            if local.is_received_upvalue() {
                if let Some(upvalue) = upvalues.get(next_received) {
                    local_upvalues[slot] = Some(upvalue.clone());
                } else if env.deopt_bindings().is_some() {
                    local_upvalues[slot] = env
                        .deopt_bindings()
                        .and_then(|bindings| bindings.cell(&local.name))
                        .or_else(|| env.frame_binding_cell(&local.name));
                }
                next_received += 1;
            }
        }
        let plan = super::upvalue_resolver::resolve_upvalues(bytecode);
        let mut cell_slots = plan.cell_slots;
        if bytecode.needs_arguments_object() {
            cell_slots.extend(
                bytecode
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, local)| local.parameter.then_some(slot)),
            );
        }
        if bytecode.contains_direct_eval()
            || bytecode.contains_with()
            || env.deopt_bindings().is_some()
        {
            cell_slots.extend(
                bytecode
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, local)| {
                        (!(local.sloppy_global_fallback || bytecode.global_scope && local.hoisted))
                            .then_some(slot)
                    }),
            );
            cell_slots.sort_unstable();
            cell_slots.dedup();
        }
        cell_slots.extend(
            bytecode
                .locals
                .iter()
                .enumerate()
                .filter_map(|(slot, local)| {
                    (bytecode.local_slot(&local.name) == Some(slot)
                        && env.module_live_binding_cell(&local.name).is_some())
                    .then_some(slot)
                }),
        );
        cell_slots.sort_unstable();
        cell_slots.dedup();
        for slot in cell_slots {
            let Some(local) = bytecode.locals.get(slot) else {
                continue;
            };
            if local.is_received_upvalue() {
                continue;
            }
            if bytecode.global_scope
                && local.hoisted
                && let Some(upvalue) = env.realm_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if local.parameter
                && bytecode.needs_arguments_object()
                && let Some(upvalue) = env.frame_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if bytecode.local_slot(&local.name) == Some(slot)
                && let Some(upvalue) = env.module_live_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if direct_eval_frame
                && local.hoisted
                && !bytecode
                    .global_lexical_names()
                    .iter()
                    .any(|name| name == &local.name)
                && !env.has_frame_binding(&local.name)
                && let Some(upvalue) = env
                    .deopt_bindings()
                    .and_then(|bindings| bindings.cell(&local.name))
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            let value = _locals
                .get(slot)
                .and_then(Option::as_ref)
                .cloned()
                .unwrap_or_else(
                    || Value::Function(crate::Function::uninitialized_lexical_marker()),
                );
            local_upvalues[slot] = Some(Upvalue::new(value));
        }
        local_upvalues
    }

    pub(super) fn initial_authoritative_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<Upvalue>],
        env: &CallEnv,
    ) -> u128 {
        bytecode
            .locals
            .iter()
            .enumerate()
            .take(u128::BITS as usize)
            .filter_map(|(slot, local)| {
                (!bytecode.global_scope
                    && !local.sloppy_global_fallback
                    && local_upvalues.get(slot).is_some_and(Option::is_none)
                    && env.slot_is_authoritative(&local.name))
                .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
    }

    pub(super) fn refresh_authoritative_slots(&mut self) {
        self.authoritative_slots =
            Self::initial_authoritative_slots(self.bytecode, &self.local_upvalues, &self.env);
    }

    /// Keeps a module's exported binding cell current for name-based writes.
    pub(super) fn write_through_module_live_binding(&self, name: &str, value: Value) {
        if let Some(binding) = self.env.module_live_binding_cell(name) {
            binding.set(value);
        }
    }

    /// Slot-addressed module live-binding update.
    pub(super) fn write_through_module_live_binding_slot(&self, slot: usize, value: &Value) {
        if let Some(name) = self.bytecode.locals.get(slot).map(|local| &local.name) {
            // Module live bindings describe the module's top-level declaration
            // slot. A nested lexical may reuse the same source name but owns a
            // distinct cell and must never update the export by coincidence.
            if self.bytecode.local_slot(name) == Some(slot)
                && let Some(binding) = self.env.module_live_binding_cell(name)
            {
                binding.set(value.clone());
            }
        }
    }

    pub(super) fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        // `this` belongs to the frame for function bodies (arrows inherit it
        // through capture). Module bodies provide their `this` binding through
        // the environment chain instead, while derived constructors without a
        // completed `super(...)` stay in their `this` TDZ.
        if name == "this" && !self.bytecode.global_scope {
            if let Some(value) = &self.direct_this {
                return Ok(value.clone());
            }
            if let Some(value) = self.env.get_local(name) {
                return Ok(value);
            }
            if !self.env.has_local_binding(crate::SUPER_CONSTRUCTOR_BINDING)
                && !self
                    .env
                    .has_local_binding(crate::ACTIVE_CONSTRUCTOR_BINDING)
                && let Some(value) = self.env.get(name)
            {
                return Ok(value);
            }
            return Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing `this`"
                    .to_owned(),
            });
        }
        if self.env.is_immutable_function_name(name)
            && let Some(value) = self.env.get_local(name)
        {
            return Ok(value);
        }
        let deoptimized_sloppy_global = if !self.bytecode.global_scope
            && let Some(slot) = self.bytecode.local_slot(name)
            && self.bytecode.local_is_sloppy_global_fallback(slot)
            && let Some(value) = self.local_slot_value(slot)
        {
            if !value.is_uninitialized_lexical_marker() {
                return Ok(value);
            }
            true
        } else {
            false
        };
        if let Some(value) = self.env.module_import_value(name) {
            if value.is_uninitialized_lexical_marker() {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            return Ok(value);
        }
        if self.bytecode.global_scope
            && let Some(slot) = self.bytecode.local_slot(name)
            && self.bytecode.local_is_body_hoist_only(slot)
            && !is_compiler_temporary(name)
            && let Some(value) = self.global_this_property(name)
        {
            return Ok(value);
        }
        // A "global" name may actually be a caller-scope binding carried in this
        // frame's own locals layer (e.g. an outer `var`/`let` the body closes
        // over); check that first, then the shared realm, then a property
        // created directly on `globalThis` (`this.x = 1` and realm bindings
        // share one global namespace).
        // Reconfiguring a captured sloppy global into an accessor detaches its
        // realm cell. The frame still owns that now-invalidated cell, so skip
        // its marker and resolve the accessor on `globalThis` below.
        if !deoptimized_sloppy_global && let Some(value) = self.env.get(name) {
            if matches!(
                &value,
                Value::Function(function) if function.is_uninitialized_lexical_marker()
            ) {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            return Ok(value);
        }
        if let Some(value) = self.global_this_own_value(name)? {
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{name}`"),
        })
    }

    /// Reads an own property of `globalThis` by name, invoking any getter, when
    /// the name resolves to a property created directly on the global object
    /// (e.g. `this.x = 1`, a realm binding, or `Object.defineProperty`). Returns
    /// `None` when no such own property exists, so the caller can decide whether
    /// that is a ReferenceError (a bare read) or "undefined" (a `typeof`).
    pub(super) fn global_this_own_value(
        &mut self,
        name: &str,
    ) -> Result<Option<Value>, RuntimeError> {
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => global_this.clone(),
            _ => return Ok(None),
        };
        if !global_this.has_own_property(name) {
            return Ok(None);
        }
        let mut env = self.current_env();
        let value = property_value(Value::Object(global_this), name, &mut env)?;
        self.apply_env(env);
        Ok(Some(value))
    }

    pub(super) fn load_new_target(&self) -> Value {
        self.env
            .get(crate::NEW_TARGET_BINDING)
            .unwrap_or(Value::Undefined)
    }

    /// Reads an own property of the realm's `globalThis` object, if any.
    pub(super) fn global_this_property(&self, name: &str) -> Option<Value> {
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        }?;
        global_this
            .own_property(name)
            .map(|property| property.value)
    }

    fn has_realm_or_global_this_binding(&self, name: &str) -> bool {
        self.realm.borrow().contains_key(name) || self.global_this_property(name).is_some()
    }

    fn store_realm_or_global_this_sloppy(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if self.env.is_immutable_lexical_binding(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        if let Some(property) = self.global_this_own_property(&name)
            && !property.writable
        {
            return Ok(());
        }
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
            self.write_through_module_live_binding(&name, value);
            self.sync_marked_dynamic_global(&name);
            return Ok(());
        }
        self.invalidate_array_prototype_cache(&name);
        if self.realm.borrow().contains_key(&name) {
            self.env.insert_realm(name.clone(), value.clone());
            if self.env.has_local_binding(&name) {
                self.env.insert(name.clone(), value.clone());
            }
            self.write_through_module_live_binding(&name, value.clone());
            let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
                Some(Value::Object(global_this)) => Some(global_this.clone()),
                _ => None,
            };
            if let Some(global_this) = global_this
                && global_this.has_own_property(&name)
            {
                global_this.set(name.clone(), value);
            }
            self.sync_marked_dynamic_global(&name);
            return Ok(());
        }
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        };
        if let Some(global_this) = global_this {
            global_this.set(name.clone(), value.clone());
        }
        self.env.insert_realm(name.clone(), value.clone());
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
        }
        self.write_through_module_live_binding(&name, value);
        self.sync_marked_dynamic_global(&name);
        Ok(())
    }

    /// Returns the full own-property descriptor of a `globalThis` property so
    /// callers can inspect attribute flags such as `writable`.
    pub(super) fn global_this_own_property(&self, name: &str) -> Option<Property> {
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        }?;
        global_this.own_property(name)
    }

    pub(super) fn local_slot_targets_non_writable_global(&self, slot: usize, name: &str) -> bool {
        let is_global_shadow = self.bytecode.global_scope
            && self.bytecode.local_is_body_hoist_only(slot)
            && !is_compiler_temporary(name);
        let is_sloppy_fallback = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback);
        (is_global_shadow || is_sloppy_fallback)
            && self
                .global_this_own_property(name)
                .is_some_and(|property| !property.writable)
    }

    /// Resolves an identifier inside a `with` body to the innermost with-object
    /// that binds `name` (an own-or-inherited property not filtered out by the
    /// object's `Symbol.unscopables`). Returns `None` when no with-object binds
    /// it, in which case the caller falls back to ordinary scope resolution.
    fn with_binding_object(&self, name: &str) -> Result<Option<Value>, RuntimeError> {
        let env = self.realm_env();
        for object in self.with_stack.iter().rev() {
            if !has_property(object.clone(), &env, name)? {
                continue;
            }
            if self.is_unscopable(object, name)? {
                continue;
            }
            return Ok(Some(object.clone()));
        }
        Ok(None)
    }

    /// Whether `name` is excluded from a with-object's bindings by its
    /// `Symbol.unscopables` (a property whose value is truthy).
    fn is_unscopable(&self, object: &Value, name: &str) -> Result<bool, RuntimeError> {
        let mut env = self.current_env();
        let Some(symbol) = unscopables_symbol(&env) else {
            return Ok(false);
        };
        let unscopables =
            property_value_key(object.clone(), &PropertyKey::Symbol(symbol), &mut env)?;
        match unscopables {
            Value::Object(_) | Value::Function(_) | Value::Array(_) => {
                let blocked = get_property(unscopables, name, &mut env)?;
                Ok(is_truthy(&blocked))
            }
            _ => Ok(false),
        }
    }

    pub(super) fn load_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            // GetBindingValue re-checks HasProperty (step 2) before the Get
            // (step 4); the binding may have been deleted by the @@unscopables
            // getter. A false result throws in strict mode and otherwise yields
            // undefined for the loose with-binding.
            if !has_property(object.clone(), &env, name)? {
                self.apply_env(env);
                if is_strict {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                return Ok(Value::Undefined);
            }
            let value = get_property(object, name, &mut env)?;
            self.apply_env(env);
            return Ok(value);
        }
        match slot {
            Some(slot) => self.load_local(slot),
            None => self.load_global(name),
        }
    }

    pub(super) fn store_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        is_strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            // SetMutableBinding step 1 re-checks HasProperty (observable on a
            // Proxy, and the binding may have been deleted by the @@unscopables
            // getter). A false result throws only in strict mode; the Set in
            // step 3 runs otherwise, recreating the property in sloppy mode.
            if !has_property(object.clone(), &env, name)? && is_strict {
                self.apply_env(env);
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            set_property_key(
                object,
                PropertyKey::String(name.to_owned()),
                value,
                &mut env,
            )?;
            self.apply_env(env);
            return Ok(());
        }
        match slot {
            Some(slot) => self.assign_local(slot, value),
            None if is_strict => self.store_global_strict(name.to_owned(), value),
            None => {
                self.store_global_sloppy(name.to_owned(), value)?;
                self.record_sloppy_global_name(name);
                Ok(())
            }
        }
    }

    pub(super) fn resolve_ident_with(
        &mut self,
        name: &str,
        object_slot: usize,
    ) -> Result<(), RuntimeError> {
        let value = self.with_binding_object(name)?.unwrap_or(Value::Undefined);
        self.store_local(object_slot, value)
    }

    pub(super) fn load_resolved_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        match self.load_local(object_slot)? {
            Value::Undefined => match slot {
                Some(slot) => self.load_local(slot),
                None => self.load_global(name),
            },
            object => {
                let mut env = self.current_env();
                // GetBindingValue re-checks HasProperty (step 2) before the Get;
                // a false result throws in strict mode, else yields undefined.
                if !has_property(object.clone(), &env, name)? {
                    self.apply_env(env);
                    if is_strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    }
                    return Ok(Value::Undefined);
                }
                let value = get_property(object, name, &mut env)?;
                self.apply_env(env);
                Ok(value)
            }
        }
    }

    pub(super) fn store_resolved_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        match self.load_local(object_slot)? {
            Value::Undefined => match slot {
                Some(slot) => self.assign_local(slot, value),
                None if is_strict => self.store_global_strict(name.to_owned(), value),
                None => {
                    self.store_global_sloppy(name.to_owned(), value)?;
                    self.record_sloppy_global_name(name);
                    Ok(())
                }
            },
            object => {
                let mut env = self.current_env();
                // SetMutableBinding step 1: HasProperty always runs (observable
                // on a Proxy); a false result throws only in strict mode, while
                // the Set in step 3 still recreates the property in sloppy mode.
                if !has_property(object.clone(), &env, name)? && is_strict {
                    self.apply_env(env);
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                set_property_key(
                    object,
                    PropertyKey::String(name.to_owned()),
                    value,
                    &mut env,
                )?;
                self.apply_env(env);
                Ok(())
            }
        }
    }

    pub(super) fn typeof_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<Value, RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            let value = get_property(object, name, &mut env)?;
            self.apply_env(env);
            return Ok(Value::String(typeof_value(value).into()));
        }
        let value = match slot {
            Some(slot) => self.load_local(slot)?,
            None => {
                if let Some(value) = self.env.module_import_value(name) {
                    if value.is_uninitialized_lexical_marker() {
                        return Err(RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    }
                    value
                } else {
                    self.env.get(name).unwrap_or(Value::Undefined)
                }
            }
        };
        let value = if matches!(
            &value,
            Value::Function(function) if function.is_uninitialized_lexical_marker()
        ) {
            Value::Undefined
        } else {
            value
        };
        Ok(Value::String(typeof_value(value).into()))
    }

    #[inline(always)]
    pub(super) fn load_local(&mut self, slot: usize) -> Result<Value, RuntimeError> {
        if self.slot_is_authoritative(slot) {
            return match self.locals.get(slot) {
                Some(Some(value)) => self.checked_local_value(slot, value.clone()),
                Some(None) => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                }),
                None => Err(RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                }),
            };
        }
        self.load_local_slow(slot)
    }

    #[inline(never)]
    fn load_local_slow(&mut self, slot: usize) -> Result<Value, RuntimeError> {
        if let Some(cell) = self.local_upvalues.get(slot).and_then(Option::as_ref)
            && let Some(local) = self.bytecode.locals.get(slot)
            && self.env.is_realm_binding_cell(&local.name, cell)
            && !self.realm.borrow().contains_key(&local.name)
        {
            let name = local.name.clone();
            if let Some(value) = self.global_this_own_value(&name)? {
                return Ok(value);
            }
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if let Some(value) = self.upvalue_slot_value(slot) {
            return self.checked_local_value(slot, value);
        }
        if let Some(local) = self.bytecode.locals.get(slot)
            && local.from_env
            && let Some(value) = self.env.module_import_value(&local.name)
        {
            if value.is_uninitialized_lexical_marker() {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{}`", local.name),
                });
            }
            return Ok(value);
        }
        match self.locals.get(slot) {
            Some(Some(value)) => self.checked_local_value(slot, value.clone()),
            Some(None) => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn local_slot_value(&self, slot: usize) -> Option<Value> {
        self.upvalue_slot_value(slot)
            .or_else(|| self.locals.get(slot).and_then(Option::as_ref).cloned())
    }

    pub(super) fn upvalue_slot_value(&self, slot: usize) -> Option<Value> {
        self.local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .map(Upvalue::get)
    }

    fn checked_local_value(&self, slot: usize, value: Value) -> Result<Value, RuntimeError> {
        if matches!(
            &value,
            Value::Function(function) if function.is_uninitialized_lexical_marker()
        ) {
            if is_compiler_temporary(&self.bytecode.locals[slot].name) {
                return Ok(Value::Undefined);
            }
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            });
        }
        Ok(value)
    }

    pub(super) fn load_local_or_undefined(&self, slot: usize) -> Result<Value, RuntimeError> {
        if let Some(value) = self.upvalue_slot_value(slot) {
            return Ok(value);
        }
        if let Some(local) = self.bytecode.locals.get(slot)
            && local.from_env
            && let Some(value) = self.env.module_import_value(&local.name)
        {
            return Ok(value);
        }
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Ok(Value::Undefined),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    #[inline(always)]
    pub(super) fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable)
        {
            self.locals[slot] = Some(value);
            return Ok(());
        }
        self.store_local_slow(slot, value)
    }

    #[inline(never)]
    fn store_local_slow(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        // Read only the `Copy` slot metadata up front so the hot local write
        // never clones the `Local` (its owned `name` would be a heap
        // allocation on every assignment); the binding name is resolved by
        // reference, and only on the cold capture/global-sync paths.
        let (mutable, from_env, hoisted, module_import, immutable_env_binding) = {
            let local_meta = self.bytecode.locals.get(slot).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
            (
                local_meta.mutable,
                local_meta.from_env,
                local_meta.hoisted,
                self.env.has_module_import(&local_meta.name),
                local_meta.from_env
                    && !local_meta.parameter
                    && !local_meta.hoisted
                    && (self.env.is_immutable_lexical_binding(&local_meta.name)
                        || self.env.is_immutable_function_name(&local_meta.name)),
            )
        };
        if module_import {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        let upvalue_initialized = self
            .local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .map(|upvalue| upvalue.get())
            .is_some_and(|value| {
                !matches!(
                    value,
                    Value::Function(function) if function.is_uninitialized_lexical_marker()
                )
            });
        if !mutable && (local.is_some() || upvalue_initialized) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        if (local.is_some() || upvalue_initialized) && immutable_env_binding {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        *local = Some(value.clone());
        let uses_shared_cell =
            if let Some(upvalue) = self.local_upvalues.get(slot).and_then(Option::as_ref) {
                upvalue.set(value.clone());
                true
            } else {
                false
            };
        if !uses_shared_cell {
            // Binding classes not migrated to cells still use the coexistence
            // snapshot/writeback path. A cell-backed lexical must not also write
            // by name: same-named shadowed bindings are distinct slots/cells.
            self.write_through_module_live_binding_slot(slot, &value);
        } else if !from_env {
            // A declaring frame still mirrors its own cell into the coexistence
            // map for module live exports and not-yet-migrated consumers. A
            // received upvalue must never take this name-keyed path: its parent
            // binding is already updated through the shared cell, and a
            // same-named outer binding can be a different cell.
            self.write_through_module_live_binding_slot(slot, &value);
        }
        if self.bytecode.global_scope
            && self.persist_global_lexicals
            && !hoisted
            && self.bytecode.local_slot(&self.bytecode.locals[slot].name) == Some(slot)
            && self
                .bytecode
                .global_lexical_names()
                .iter()
                .any(|name| name == &self.bytecode.locals[slot].name)
            && !is_compiler_temporary(&self.bytecode.locals[slot].name)
        {
            let name = self.bytecode.locals[slot].name.clone();
            self.env
                .set_global_lexical_value(name.clone(), value.clone());
            self.env.mark_global_lexical_binding(name.clone());
            if !mutable {
                self.env.mark_immutable_lexical_binding(name);
            }
        }
        if !uses_shared_cell && (from_env || self.bytecode.local_is_body_hoist_only(slot)) {
            let name = self.bytecode.locals[slot].name.clone();
            if self.env.has_local_binding(&name) {
                self.env.insert(name, value.clone());
            }
        }
        let shared_realm_cell = self
            .local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|cell| {
                self.env
                    .is_realm_binding_cell(&self.bytecode.locals[slot].name, cell)
            });
        let syncs_global_var = (from_env && !hoisted && (!uses_shared_cell || shared_realm_cell))
            || (self.bytecode.global_scope
                && self.bytecode.local_is_body_hoist_only(slot)
                && !is_compiler_temporary(&self.bytecode.locals[slot].name));
        // Resolve `globalThis` into a local first so the `self.realm` borrow is
        // released before the body re-borrows it mutably (an `if let` chain
        // would otherwise hold the immutable borrow across the body and panic on
        // the `borrow_mut` below).
        let global_this = if syncs_global_var {
            match self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned() {
                Some(Value::Object(global_this)) => Some(global_this),
                _ => None,
            }
        } else {
            None
        };
        if let Some(global_this) = global_this
            && global_this.has_own_property(&self.bytecode.locals[slot].name)
        {
            let name = self.bytecode.locals[slot].name.clone();
            global_this.set(name.clone(), value.clone());
            if self.realm.borrow().contains_key(&name) {
                self.env.insert_realm(name.clone(), value.clone());
            }
            if self.env.has_local_binding(&name) {
                self.env.insert(name, value);
            }
        }
        Ok(())
    }

    pub(super) fn assign_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable)
        {
            return match self.locals.get_mut(slot) {
                Some(Some(Value::Function(function)))
                    if function.is_uninitialized_lexical_marker() =>
                {
                    Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "ReferenceError: undefined identifier `{}`",
                            self.bytecode.locals[slot].name
                        ),
                    })
                }
                Some(Some(local)) => {
                    *local = value;
                    Ok(())
                }
                Some(None) => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                }),
                None => Err(RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                }),
            };
        }
        let current = self
            .upvalue_slot_value(slot)
            .or_else(|| self.locals.get(slot).and_then(Option::as_ref).cloned());
        match current {
            Some(Value::Function(function)) if function.is_uninitialized_lexical_marker() => {
                Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                })
            }
            Some(_) => self.store_local(slot, value),
            None => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
        }
    }

    /// True when this frame has no name-addressed state that can supersede the
    /// indexed local. Captures, dynamic scope, modules, globals, and sloppy
    /// fallback bindings all retain the full synchronization path.
    pub(super) fn slot_is_authoritative(&self, slot: usize) -> bool {
        slot < u128::BITS as usize && self.authoritative_slots & (1_u128 << slot) != 0
    }

    pub(super) fn store_local_or_global_sloppy(
        &mut self,
        slot: usize,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if self.env.has_module_import(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // The inner name of a named function expression is immutable; a sloppy
        // assignment to it is a silent no-op.
        if self.env.is_immutable_function_name(&name) {
            return Ok(());
        }
        let is_sloppy_global_fallback = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback);
        match self.locals.get(slot) {
            Some(Some(_)) => {
                if self.local_slot_targets_non_writable_global(slot, &name) {
                    return Ok(());
                }
                if is_sloppy_global_fallback || self.has_realm_or_global_this_binding(&name) {
                    let syncs_global_snapshot = is_sloppy_global_fallback
                        && self.captured_or_local_matches_global_this(&name);
                    if syncs_global_snapshot {
                        self.record_sloppy_global_name(&name);
                    }
                    self.store_realm_or_global_this_sloppy(name.clone(), value.clone())?;
                    self.store_local(slot, value)?;
                    if syncs_global_snapshot && let Some(value) = self.locals[slot].clone() {
                        self.sync_global_this_own_property(&name, value);
                    }
                } else {
                    self.store_local(slot, value)?;
                }
                Ok(())
            }
            Some(None) => {
                if is_sloppy_global_fallback {
                    if self.local_slot_targets_non_writable_global(slot, &name) {
                        return Ok(());
                    }
                    let syncs_global_snapshot = self.captured_or_local_matches_global_this(&name);
                    if syncs_global_snapshot {
                        self.record_sloppy_global_name(&name);
                    }
                    self.store_realm_or_global_this_sloppy(name.clone(), value.clone())?;
                    self.store_local(slot, value)?;
                    if syncs_global_snapshot && let Some(value) = self.locals[slot].clone() {
                        self.sync_global_this_own_property(&name, value);
                    }
                    return Ok(());
                }
                self.store_global_sloppy(name.clone(), value)?;
                self.record_sloppy_global_name(&name);
                let global_value = self.load_global(&name)?;
                let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                })?;
                *local = Some(global_value);
                Ok(())
            }
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn clear_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let deactivated_cell = self
            .local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .cloned();
        let deactivates_lexical = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| !local.hoisted);
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        let refresh_upvalue = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| !local.from_env)
            && self
                .local_upvalues
                .get(slot)
                .and_then(Option::as_ref)
                .is_some_and(|upvalue| local.is_some() || !upvalue.is_shared());
        *local = None;
        if refresh_upvalue && let Some(upvalue) = self.local_upvalues.get_mut(slot) {
            *upvalue = Some(Upvalue::new(Value::Function(
                crate::Function::uninitialized_lexical_marker(),
            )));
        }
        if let Some(name) = self.bytecode.local_name_at(slot) {
            if deactivates_lexical && let Some(cell) = &deactivated_cell {
                self.env.remove_deopt_cell_if(name, cell);
            }
            if self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.catch_binding)
            {
                self.env.remove(name);
                self.env.unmark_catch_binding(name);
            }
        }
        Ok(())
    }

    pub(super) fn define_global_var(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let Some(Value::Object(global_this)) =
            self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
        else {
            return Err(RuntimeError {
                thrown: None,
                message: "global object binding is missing".to_owned(),
            });
        };
        if global_this.has_own_property(&name) {
            global_this.set(name.clone(), value.clone());
            let value = global_this
                .own_property(&name)
                .map(|property| property.value)
                .unwrap_or(value);
            self.invalidate_array_prototype_cache(&name);
            self.env.insert_realm(name.clone(), value.clone());
            if self.env.has_local_binding(&name) {
                self.env.insert(name.clone(), value.clone());
            }
            self.clear_global_var_local(&name);
            self.write_through_module_live_binding(&name, value);
            return Ok(());
        }
        global_this.define_property(
            name.clone(),
            Property::data(value.clone(), true, true, false),
        );
        self.invalidate_array_prototype_cache(&name);
        self.env.insert_realm(name.clone(), value.clone());
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
        }
        self.clear_global_var_local(&name);
        self.write_through_module_live_binding(&name, value);
        Ok(())
    }

    fn clear_global_var_local(&mut self, name: &str) {
        if !self.bytecode.global_scope {
            return;
        }
        let Some(slot) = self.bytecode.local_slot(name) else {
            return;
        };
        if self.bytecode.local_is_body_hoist_only(slot)
            && let Some(local) = self.locals.get_mut(slot)
        {
            *local = None;
        }
    }
    /// Resolves a dynamic environment name to the innermost currently-active
    /// frame slot. Static bytecode is already slot-indexed; this reverse lookup
    /// is only for `CallEnv` round-trips used by direct eval and native calls.
    /// Later slots correspond to inner lexical declarations, so searching in
    /// reverse preserves shadowing without encoding slot identity in the name.
    fn active_local_slot_for_env_name(&self, name: &str) -> Option<usize> {
        self.bytecode
            .locals
            .iter()
            .enumerate()
            .rev()
            .find(|(slot, local)| {
                local.name == name
                    && (self.locals.get(*slot).is_some_and(Option::is_some)
                        || self.local_upvalues.get(*slot).is_some_and(Option::is_some))
            })
            .map(|(slot, _)| slot)
            .or_else(|| self.bytecode.local_slot(name))
    }

    pub(super) fn apply_env(&mut self, env: CallEnv) {
        // The realm layer is shared by `Rc`, so global writes are already live.
        // Write each non-realm local back to its slot, to the frame's own
        // internal/caller-scope binding layer, or (for a genuinely new binding)
        // to the shared realm.
        let direct_parameter_eval_values = env
            .deopt_bindings()
            .map(|bindings| {
                bindings
                    .names()
                    .into_iter()
                    .filter_map(|name| {
                        name.strip_prefix(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX)
                            .and_then(|parameter| {
                                bindings
                                    .get(parameter)
                                    .map(|value| (parameter.to_owned(), value))
                            })
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let locals = env.into_binding_snapshot();
        let direct_parameter_eval_vars = locals
            .keys()
            .filter_map(|name| {
                name.strip_prefix(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX)
                    .map(str::to_owned)
            })
            .collect::<HashSet<_>>();
        for (name, mut value) in locals {
            if name.starts_with(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX) {
                self.env.insert_deopt(name, value);
                continue;
            }
            if self.in_parameter_prologue() && direct_parameter_eval_vars.contains(&name) {
                if let Some(parameter_value) = direct_parameter_eval_values.get(&name) {
                    value = parameter_value.clone();
                }
                if let Some(parameter_var_slot) = self.active_local_slot_for_env_name(&name) {
                    if let Some(local) = self.locals.get_mut(parameter_var_slot) {
                        *local = Some(value.clone());
                    }
                    if let Some(upvalue) = self
                        .local_upvalues
                        .get(parameter_var_slot)
                        .and_then(Option::as_ref)
                    {
                        upvalue.set(value.clone());
                    }
                }
                self.env.insert_deopt(name, value);
                continue;
            }
            if let Some(index) = self.active_local_slot_for_env_name(&name) {
                if self.in_parameter_prologue()
                    && !self.bytecode.local_is_parameter(index)
                    && (is_call_frame_binding(&name) || !self.bytecode.local_is_from_env(index))
                {
                    self.env.insert(name, value);
                    continue;
                }
                let syncs_global_this = self.bytecode.local_is_sloppy_global_fallback(index)
                    || (self.bytecode.global_scope
                        && self.bytecode.local_is_body_hoist_only(index)
                        && !is_compiler_temporary(&name));
                let value = if syncs_global_this {
                    self.global_this_property(&name).unwrap_or(value)
                } else {
                    value
                };
                if self.locals[index].is_some()
                    || self.bytecode.local_is_from_env(index)
                    || syncs_global_this
                {
                    if self.locals[index]
                        .as_ref()
                        .is_some_and(|current| !is_uninitialized_lexical_value(current))
                        && is_uninitialized_lexical_value(&value)
                    {
                        continue;
                    }
                    self.locals[index] = Some(value.clone());
                    if let Some(upvalue) = self.local_upvalues.get(index).and_then(Option::as_ref) {
                        upvalue.set(value.clone());
                    }
                    self.write_through_module_live_binding(&name, value.clone());
                    let realm_backed_slot = (self.bytecode.global_scope
                        && self.bytecode.local_is_body_hoist_only(index)
                        && !is_compiler_temporary(&name))
                        || self
                            .local_upvalues
                            .get(index)
                            .and_then(Option::as_ref)
                            .is_some_and(|cell| self.env.is_realm_binding_cell(&name, cell));
                    if realm_backed_slot && self.realm.borrow().contains_key(&name) {
                        self.env.insert_realm(name, value);
                    } else if syncs_global_this {
                        self.sync_global_this_own_property(&name, value);
                    }
                } else if self.env.has_local_binding(&name) {
                    self.env.insert(name.clone(), value.clone());
                    self.write_through_module_live_binding(&name, value);
                }
            } else if self.env.has_local_binding(&name)
                || (self.in_parameter_prologue()
                    && !is_call_frame_binding(&name)
                    && !is_compiler_temporary(&name))
            {
                self.env.insert(name, value);
            } else if self.realm.borrow().contains_key(&name) {
                // Already a realm binding (shared cell) — leave it; a mutation
                // would have hit the cell directly.
            } else {
                self.env.insert(name, value);
            }
        }
    }

    fn sync_global_this_own_property(&self, name: &str, value: Value) {
        let Some(Value::Object(global_this)) =
            self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
        else {
            return;
        };
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
            self.env.insert_realm(name.to_owned(), value);
        }
    }

    fn captured_or_local_matches_global_this(&self, name: &str) -> bool {
        let Some(global_value) = self.global_this_property(name) else {
            return false;
        };
        self.env.get_local(name) == Some(global_value)
    }

    /// `delete identifier` in non-strict mode (sloppy): attempts to delete the
    /// binding. Declared variables (var/let/const/function in any scope) cannot
    /// be deleted (returns false). Global properties on `globalThis` that are
    /// configurable can be deleted (returns true). A non-existent binding also
    /// returns true.
    pub(super) fn delete_ident(&mut self, name: &str) -> bool {
        let is_sloppy_global = self
            .bytecode
            .sloppy_global_assignment_names()
            .contains(&name.to_owned());
        // Local scope bindings (var/let/const/param) are never deletable,
        // but sloppy global assignments that happen to occupy a local slot
        // are configurable properties on globalThis and CAN be deleted.
        if !is_sloppy_global {
            if let Some(slot) = self.bytecode.local_slot(name) {
                if self.locals[slot].is_some() {
                    if self.bytecode.local_is_eval_deletable(slot) {
                        self.locals[slot] = None;
                        self.local_upvalues[slot] = None;
                        self.env.remove(name);
                        return true;
                    }
                    return false;
                }
            }
            // Non-global frame locals (e.g. captured from outer function scope)
            // are also undeletable.
            if self.env.get_local(name).is_some() {
                return false;
            }
        }
        // For globals, check the globalThis property descriptor. Only
        // configurable properties (bare assignments like `x = 1`) can be
        // deleted. `var` declarations are non-configurable.
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned() {
            Some(Value::Object(obj)) => obj,
            _ => return true,
        };
        if !global_this.has_own_property(name) {
            // Name exists in realm but not on globalThis — it's a lexical
            // binding from a script-level `let`/`const`; undeletable.
            if self.realm.borrow().contains_key(name) {
                return false;
            }
            return true;
        }
        let deleted = global_this.delete_own_property(name);
        if deleted {
            self.env.remove_realm(name);
            // Clear the cached local slot if the sloppy global was mirrored there.
            if let Some(slot) = self.bytecode.local_slot(name) {
                if let Some(local) = self.locals.get_mut(slot) {
                    *local = None;
                }
                if let Some(upvalue) = self.local_upvalues.get_mut(slot) {
                    *upvalue = None;
                }
            }
        }
        deleted
    }

    /// `delete identifier` inside a `with` body in non-strict mode: checks the
    /// with-object stack first (deletes from the first binding object), then
    /// falls back to `delete_ident` for local/global scope.
    pub(super) fn delete_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<(), RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            let result = super::vm_props::delete_property_key(
                object,
                &PropertyKey::String(name.to_owned()),
                &mut env,
            )?;
            self.apply_env(env);
            self.stack.push(result);
        } else {
            // Fall back to ordinary identifier deletion.
            let result = if let Some(s) = slot {
                if self.locals[s].is_some() {
                    false
                } else {
                    self.delete_ident(name)
                }
            } else {
                self.delete_ident(name)
            };
            self.stack.push(Value::Boolean(result));
        }
        Ok(())
    }

    pub(super) fn record_sloppy_global_name(&mut self, name: &str) {
        if !self
            .sloppy_global_names
            .iter()
            .any(|existing| existing == name)
        {
            self.sloppy_global_names.push(name.to_owned());
        }
    }

    /// Replaces each captured loop binding with a fresh per-iteration cell.
    pub(super) fn fresh_iteration_scope(&mut self, slots: &[usize]) {
        for &slot in slots {
            if let Some(upvalue) = self.local_upvalues.get_mut(slot)
                && upvalue.is_some()
            {
                let value = self
                    .locals
                    .get(slot)
                    .and_then(Option::as_ref)
                    .cloned()
                    .unwrap_or_else(|| {
                        Value::Function(crate::Function::uninitialized_lexical_marker())
                    });
                *upvalue = Some(Upvalue::new(value));
            }
        }
    }

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
        Ok(())
    }
}

pub(super) fn is_compiler_temporary(name: &str) -> bool {
    name.starts_with("\0\0")
}

pub(super) fn is_call_frame_binding(name: &str) -> bool {
    matches!(
        name,
        crate::GLOBAL_THIS_BINDING
            | crate::DIRECT_EVAL_STRICT_BINDING
            | crate::DIRECT_EVAL_ARGUMENTS_BINDING
            | crate::DIRECT_EVAL_FUNCTION_CONTEXT_BINDING
            | crate::FIELD_INITIALIZER_EVAL_BINDING
            | crate::HOME_OBJECT_BINDING
            | crate::NEW_TARGET_BINDING
            | crate::SUPER_CONSTRUCTOR_BINDING
            | crate::ACTIVE_CONSTRUCTOR_BINDING
            | "this"
            | "arguments"
    )
}

fn is_uninitialized_lexical_value(value: &Value) -> bool {
    matches!(value, Value::Function(function) if function.is_uninitialized_lexical_marker())
}

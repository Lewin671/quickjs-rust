//! A class definition is a continuation in its enclosing activation. Keys
//! execute on that activation's operand stack; only the partially built class
//! and its private-name overlay are retained across yield/await. No key thunk
//! or synthetic call frame is created.

use super::{
    ir::{ClassDefinition, ClassElementDef, ClassMemberKeyDef, ClassPrivateElementDef},
    vm::Vm,
    vm_class::{ClassHeritage, function_prototype_value, install_field_value},
};
use crate::{
    Function, ObjectRef, PropertyKey, RuntimeError, Value,
    function::{BindingCell, CompiledUserFunction, InstanceFieldInitializer},
    to_property_key_value,
};
use std::{collections::HashMap, rc::Rc};

pub(super) struct ClassKeyContinuation {
    definition: Rc<ClassDefinition>,
    name_cell: Option<BindingCell>,
    state: Option<ClassState>,
    pending: Vec<PendingStaticItem>,
    saved_execution: (crate::context::ExecutionMode, bool),
}

struct ClassState {
    constructor: Function,
    prototype: ObjectRef,
    creation: Rc<crate::context::CreationContext>,
}

enum PendingStaticItem {
    Field(usize, PropertyKey),
    Private(usize),
    Block(usize),
}

impl Vm<'_, '_> {
    pub(super) fn execution_is_strict(&self) -> bool {
        self.env.execution_state().1
    }

    pub(super) fn enter_class(&mut self, definition: Rc<ClassDefinition>) {
        let saved_execution = self.env.execution_state();
        self.env
            .set_execution_state(crate::context::ExecutionMode::ClassKey, true);
        let name_cell = definition
            .name_slot
            .map(|slot| self.ensure_upvalue_for_parent_slot(slot));
        self.cold_mut().class_regions.push(ClassKeyContinuation {
            definition,
            saved_execution,
            name_cell,
            state: None,
            pending: Vec::new(),
        });
    }

    pub(super) fn begin_class(&mut self) -> Result<(), RuntimeError> {
        // Move the owned payload out while callbacks run. The enclosing private
        // overlay remains selected until this class's environment exists.
        let mut region = self
            .cold_mut()
            .class_regions
            .pop()
            .expect("class region entered");
        let result = self.build_class_state(&region);
        if let Ok(state) = &result {
            region.state = Some(ClassState {
                constructor: state.constructor.clone(),
                prototype: state.prototype.clone(),
                creation: state.creation.clone(),
            });
        }
        self.cold_mut().class_regions.push(region);
        result.map(|_| ())
    }

    fn build_class_state(
        &mut self,
        region: &ClassKeyContinuation,
    ) -> Result<ClassState, RuntimeError> {
        let definition = &region.definition;
        let name = definition.name.as_deref();
        let constructor = &definition.constructor;
        let has_heritage = definition.has_heritage;
        // Resolve the parent
        // constructor and the prototype the new class prototype inherits from.
        let heritage = if has_heritage {
            let value = self.pop()?;
            let mut heritage_env = self.context();
            let heritage = ClassHeritage::resolve(value, &mut heritage_env)?;
            Some(heritage)
        } else {
            None
        };
        let prototype_parent = match &heritage {
            Some(ClassHeritage::Null) => None,
            Some(ClassHeritage::Parent(parent)) => parent.prototype.clone(),
            None => self.cached_object_prototype().map(crate::Prototype::Object),
        };

        let super_constructor = match &heritage {
            Some(ClassHeritage::Parent(parent)) => Some(parent.constructor.clone()),
            Some(ClassHeritage::Null) => function_prototype_value(&self.context()),
            _ => None,
        };
        let private = self.prepare_class_private_environment(&definition.private_elements);
        let mut creation = self.current_creation_context();
        let same_private = match (&creation.private, &private) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same(b),
            _ => false,
        };
        if !same_private {
            Rc::make_mut(&mut creation).private = private.clone();
        }
        let class_name_upvalue = region.name_cell.clone();
        let constructor_upvalues = self.captured_upvalues_for_function_with_override(
            &constructor.bytecode,
            &constructor.lexical_captures,
            name.zip(class_name_upvalue.as_ref()),
        );
        let constructor_function = Function::new_user_compiled(CompiledUserFunction {
            name: constructor.name.clone(),
            has_name_binding: false,
            params: std::rc::Rc::new(constructor.params.clone()),
            creation: creation.clone(),
            bytecode: constructor.bytecode.clone(),
            source_text: None,
            local_names: constructor.local_names.clone().into(),
            constructable: true,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            lexical_call: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: true,
            is_derived_constructor: has_heritage,
            owner_kind: crate::context::OwnerKind::Function,
            home_object: None,
            super_constructor: super_constructor.clone(),
            lexical_scope: self.prepare_scope_cursor(),
            upvalues: constructor_upvalues,
        });
        // Static-side inheritance: a subclass constructor inherits the parent
        // constructor's static members through its [[Prototype]], which is the
        // parent constructor itself. Storing the function directly gives
        // `Object.getPrototypeOf(Sub) === Super` reference identity and lets
        // inherited static methods and static `super.x` resolve live (rather
        // than against a definition-time snapshot).
        let constructor_parent_slot = match &heritage {
            Some(ClassHeritage::Parent(heritage_parent)) => match &heritage_parent.constructor {
                Value::Function(parent) => Some(crate::Prototype::Function(parent.clone())),
                Value::Object(parent) => Some(crate::Prototype::Object(parent.clone())),
                _ => None,
            },
            Some(ClassHeritage::Null) => {
                function_prototype_value(&self.context()).and_then(|value| match value {
                    Value::Object(parent) => Some(crate::Prototype::Object(parent)),
                    _ => None,
                })
            }
            None => None,
        };
        if let Some(parent_slot) = constructor_parent_slot {
            let _ = constructor_function.set_internal_prototype_slot(Some(parent_slot));
        }

        let prototype = ObjectRef::with_prototype_slot(
            HashMap::new(),
            prototype_parent,
            self.env.realm().atoms(),
        );
        constructor_function.install_class_prototype(prototype.clone());

        // The constructor's home object is its prototype; static `super.x`
        // resolves through it.
        constructor_function.set_home_object(Value::Object(prototype.clone()));

        if let Some(private) = private {
            prototype.set_private_environment(private);
        }
        let mut queued = Vec::new();
        for private in &definition.private_elements {
            self.queue_instance_private_method_brand(private, &constructor_function, &mut queued);
        }
        Ok(ClassState {
            constructor: constructor_function,
            prototype,
            creation,
        })
    }

    pub(super) fn current_creation_context(&mut self) -> Rc<crate::context::CreationContext> {
        self.cold()
            .and_then(|cold| {
                cold.class_regions
                    .iter()
                    .rev()
                    .find_map(|region| region.state.as_ref().map(|state| state.creation.clone()))
            })
            .unwrap_or_else(|| self.env.creation_context())
    }

    pub(super) fn class_key_private_environment(
        &self,
    ) -> Option<crate::private::PrivateEnvironment> {
        self.cold().and_then(|cold| {
            cold.class_regions.iter().rev().find_map(|region| {
                region
                    .state
                    .as_ref()
                    .and_then(|state| state.creation.private.clone())
            })
        })
    }

    pub(super) fn define_class_element(&mut self, index: usize) -> Result<(), RuntimeError> {
        let region = self
            .cold()
            .and_then(|cold| cold.class_regions.last())
            .expect("class region entered");
        let definition = region.definition.clone();
        let state = region.state.as_ref().expect("heritage resolved");
        let constructor = state.constructor.clone();
        let prototype = state.prototype.clone();
        let name = definition.name.as_deref();
        let element = &definition.elements[index];
        let key_def = match element {
            ClassElementDef::Method(def) => Some(&def.key),
            ClassElementDef::Field(def) => Some(&def.key),
            _ => None,
        };
        let key = match key_def {
            Some(ClassMemberKeyDef::Literal(key)) => Some(PropertyKey::String(key.clone())),
            Some(ClassMemberKeyDef::Computed) => {
                let value = self.pop()?;
                let mut env = self.context();
                let key = to_property_key_value(value, &mut env)?;
                Some(key)
            }
            None => None,
        };
        let pending = match element {
            ClassElementDef::Method(method) => {
                self.install_method(
                    method,
                    key.expect("method key"),
                    &prototype,
                    &constructor,
                    name,
                )?;
                None
            }
            ClassElementDef::Field(field) if field.is_static => {
                Some(PendingStaticItem::Field(index, key.expect("field key")))
            }
            ClassElementDef::Field(field) => {
                let init = self.build_field_initializer(field, &prototype, &constructor, name);
                constructor.push_instance_public_field(InstanceFieldInitializer::new(
                    key.expect("field key"),
                    init,
                ));
                None
            }
            ClassElementDef::Private(private) => {
                self.queue_instance_private_element(private, &prototype, &constructor, name);
                matches!(
                    private,
                    ClassPrivateElementDef::Field {
                        is_static: true,
                        ..
                    }
                )
                .then_some(PendingStaticItem::Private(index))
            }
            ClassElementDef::StaticBlock(_) => Some(PendingStaticItem::Block(index)),
        };
        if let Some(pending) = pending {
            self.cold_mut()
                .class_regions
                .last_mut()
                .expect("class region entered")
                .pending
                .push(pending);
        }
        Ok(())
    }

    pub(super) fn finish_class(&mut self) -> Result<Value, RuntimeError> {
        let region = self
            .cold_mut()
            .class_regions
            .last_mut()
            .expect("class region entered");
        let definition = region.definition.clone();
        let state = region.state.as_ref().expect("heritage resolved");
        let constructor = state.constructor.clone();
        let prototype = state.prototype.clone();
        let pending = std::mem::take(&mut region.pending);
        let result = self.finish_class_initialization(&definition, constructor, prototype, pending);
        let region = self.cold_mut().class_regions.pop().expect("class region");
        self.env
            .set_execution_state(region.saved_execution.0, region.saved_execution.1);
        result
    }

    pub(super) fn restore_class_depth(&mut self, depth: usize) {
        while self
            .cold()
            .is_some_and(|cold| cold.class_regions.len() > depth)
        {
            let region = self.cold_mut().class_regions.pop().expect("class region");
            self.env
                .set_execution_state(region.saved_execution.0, region.saved_execution.1);
        }
    }

    pub(super) fn class_inner_upvalue(
        &mut self,
        constructor: &Function,
        name: Option<&str>,
    ) -> Option<BindingCell> {
        let name = name?;
        if let Some(region) = self.cold().and_then(|cold| cold.class_regions.last()) {
            return region.name_cell.clone();
        }
        constructor.bytecode.as_ref().and_then(|bytecode| {
            bytecode
                .received_upvalue_names()
                .zip(&constructor.upvalues)
                .find_map(|(candidate, cell)| (candidate == name).then(|| cell.clone()))
        })
    }

    fn finish_class_initialization(
        &mut self,
        definition: &ClassDefinition,
        constructor: Function,
        prototype: ObjectRef,
        pending: Vec<PendingStaticItem>,
    ) -> Result<Value, RuntimeError> {
        let name = definition.name.as_deref();
        if let Some(slot) = definition.name_slot {
            self.store_local(slot, Value::Function(constructor.clone()))?;
        }
        self.install_private_elements(
            &definition.private_elements,
            &prototype,
            &constructor,
            name,
        )?;
        for pending in pending {
            match pending {
                PendingStaticItem::Field(index, key) => {
                    let ClassElementDef::Field(field) = &definition.elements[index] else {
                        unreachable!()
                    };
                    let initializer =
                        self.build_field_initializer(field, &prototype, &constructor, name);
                    let value = match initializer {
                        Some(thunk) => self
                            .run_field_initializer(&thunk, Value::Function(constructor.clone()))?,
                        None => Value::Undefined,
                    };
                    install_field_value(
                        &Value::Function(constructor.clone()),
                        key,
                        value,
                        &mut self.context(),
                    )?;
                }
                PendingStaticItem::Private(index) => {
                    let ClassElementDef::Private(private) = &definition.elements[index] else {
                        unreachable!()
                    };
                    self.initialize_static_private_field(private, &prototype, &constructor, name)?;
                }
                PendingStaticItem::Block(index) => {
                    let ClassElementDef::StaticBlock(block) = &definition.elements[index] else {
                        unreachable!()
                    };
                    self.run_static_block(block, &constructor, name)?;
                }
            }
        }
        Ok(Value::Function(constructor))
    }
}

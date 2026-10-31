//! Lazy arguments values and mapped indices, with entry-captured intrinsics.
//! Distinct binding cells can share one deferred object factory; no read or
//! mutation of a binding may duplicate that factory or retarget its aliases.
use crate::{
    Function, NativeFunction, Property, Prototype, RuntimeError, Value, object_prototype, symbol,
};

use super::BindingCell;

const CROSS_REALM_FUNCTION_PROTOTYPE: &str = "__quickjsRustRealmFunctionPrototype";

/// One implicit arguments binding's unmaterialized value. Captures are typed,
/// positional and fixed at entry; materialization cannot execute JavaScript.
pub(crate) struct ArgumentsRecipe {
    creation: std::rc::Rc<crate::context::CreationContext>,
    values: Box<[Value]>,
    mapped: Box<[Option<BindingCell>]>,
    prototype: Option<Prototype>,
    iterator: Option<(crate::ObjectRef, Property)>,
    callee: Value,
    restricted: bool,
}

/// Distinct parameter/body bindings may share the value of the one arguments
/// object. Sharing this factory does not alias their assignable binding cells.
pub(crate) struct LazyArguments {
    recipe: std::cell::RefCell<Option<ArgumentsRecipe>>,
    value: std::cell::OnceCell<Value>,
}
impl std::fmt::Debug for LazyArguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyArguments")
            .field("materialized", &self.value.get().is_some())
            .finish_non_exhaustive()
    }
}
impl LazyArguments {
    pub(crate) fn new(recipe: ArgumentsRecipe) -> Self {
        Self {
            recipe: std::cell::RefCell::new(Some(recipe)),
            value: std::cell::OnceCell::new(),
        }
    }
    pub(crate) fn value(&self) -> Value {
        self.value
            .get_or_init(|| {
                let recipe = self
                    .recipe
                    .borrow_mut()
                    .take()
                    .expect("arguments factory runs once");
                recipe.materialize()
            })
            .clone()
    }
}

impl std::fmt::Debug for ArgumentsRecipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArgumentsRecipe")
            .field("argc", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl ArgumentsRecipe {
    pub(crate) fn prepare(
        inputs: CallInputs<'_>,
        bytecode: &crate::Bytecode,
        locals: &[Option<Value>],
        cells: &mut [Option<BindingCell>],
        env: &crate::context::Context<'_>,
    ) -> Self {
        crate::diagnostics::count!(arguments_recipes);
        let function = inputs.function;
        let mapped: Box<[Option<BindingCell>]> = inputs
            .arguments
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let slot = mapped_argument_parameter(function, index)?;
                Some(
                    cells[slot]
                        .get_or_insert_with(|| {
                            BindingCell::with_metadata(
                                bytecode
                                    .binding_meta(slot)
                                    .expect("mapped parameter metadata"),
                                locals[slot].clone(),
                            )
                        })
                        .clone(),
                )
            })
            .collect();
        let restricted = function.is_strict || !function.params.is_simple();
        Self {
            creation: env.creation_context(),
            values: inputs
                .arguments
                .iter()
                .zip(mapped.iter())
                .map(|(value, cell)| {
                    if cell.is_some() {
                        Value::Undefined
                    } else {
                        value.clone()
                    }
                })
                .collect(),
            mapped,
            prototype: object_prototype(env).map(Prototype::Object),
            iterator: symbol::iterator_symbol(env)
                .map(|key| (key, arguments_iterator_property(function, env))),
            callee: if restricted {
                restricted_callee_value(function, env)
            } else {
                Value::Function(function.clone())
            },
            restricted,
        }
    }

    pub(crate) fn materialize(self) -> Value {
        crate::diagnostics::count!(arguments_materializations);
        let realm = &self.creation.realm;
        let width = self.values.len() + 2 + usize::from(self.iterator.is_some());
        let prediction = realm.predict_arguments_layout(width, self.prototype.as_ref());
        let mut sequence =
            crate::value::layout::DataSequence::new(prediction, realm.atoms(), self.prototype);
        sequence.insert_property(
            "length",
            Property::data(Value::Number(self.values.len() as f64), false, true, true),
        );
        for (index, (value, mapped)) in self
            .values
            .into_vec()
            .into_iter()
            .zip(self.mapped.into_vec())
            .enumerate()
        {
            if let Some(parameter) = mapped {
                sequence.insert_property(
                    &index.to_string(),
                    mapped_argument_property(realm, parameter, realm.atoms()),
                );
            } else {
                sequence.insert(&index.to_string(), value);
            }
        }
        let callee = if self.restricted {
            Property::accessor(Some(self.callee.clone()), Some(self.callee), false, false)
        } else {
            Property::non_enumerable(self.callee)
        };
        sequence.insert_property("callee", callee);
        if let Some((key, property)) = self.iterator {
            sequence.insert_symbol(&key, property);
        }
        let object = crate::value::OrderedDataPropertyBuilder::finish_unobserved(sequence);
        object.set_to_string_tag("Arguments");
        realm.retain_arguments_layout(&object);
        Value::Object(object)
    }
}

fn mapped_argument_parameter(function: &Function, index: usize) -> Option<usize> {
    if function.is_strict || !function.params.is_simple() {
        return None;
    }
    function.bytecode.as_ref()?.mapped_argument_slot(index)
}

fn mapped_argument_property(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    domain: &crate::property::atom::AtomDomain,
) -> Property {
    Property::accessor(
        Some(mapped_argument_getter(realm, parameter.clone(), domain)),
        Some(mapped_argument_setter(realm, parameter, domain)),
        true,
        true,
    )
}

fn mapped_argument_getter(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    _domain: &crate::property::atom::AtomDomain,
) -> Value {
    let mut getter = Function::new_native(
        Some("[[MappedArgumentGet]]"),
        0,
        NativeFunction::MappedArgumentGet,
        false,
        realm,
    );
    getter.push_upvalue(parameter);
    Value::Function(getter)
}

fn mapped_argument_setter(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    _domain: &crate::property::atom::AtomDomain,
) -> Value {
    let mut setter = Function::new_native(
        Some("[[MappedArgumentSet]]"),
        1,
        NativeFunction::MappedArgumentSet,
        false,
        realm,
    );
    setter.push_upvalue(parameter);
    Value::Function(setter)
}

fn restricted_callee_value(function: &Function, env: &crate::context::Context<'_>) -> Value {
    // Reuse the realm's shared %ThrowTypeError% so the strict `callee` poison
    // getter is the same object as `Function.prototype.arguments`/`caller`'s.
    cross_realm_throw_type_error(function).unwrap_or_else(|| {
        env.get_realm(super::THROW_TYPE_ERROR_INTRINSIC)
            .unwrap_or_else(|| {
                Value::Function(Function::new_native(
                    Some("ThrowTypeError"),
                    0,
                    NativeFunction::ThrowTypeError,
                    false,
                    env.realm(),
                ))
            })
    })
}

fn cross_realm_throw_type_error(function: &Function) -> Option<Value> {
    let crate::Property {
        value: Value::Object(prototype),
        ..
    } = function.own_property(super::CROSS_REALM_TYPE_ERROR_PROTOTYPE)?
    else {
        return None;
    };
    if let Some(crate::Property {
        value: Value::Function(throw_type_error),
        ..
    }) = prototype.own_property(super::CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC)
    {
        return Some(Value::Function(throw_type_error));
    }

    let target = Value::Function(Function::new_native(
        Some(""),
        0,
        NativeFunction::RealmThrowTypeError,
        false,
        function.creation_realm(),
    ));
    let throw_type_error = Function::new_bound(
        target,
        Value::Undefined,
        vec![Value::Object(prototype.clone())],
        0,
        function.property_domain(),
    );
    if let Some(property) = function.own_property(CROSS_REALM_FUNCTION_PROTOTYPE) {
        if let Some(function_prototype) = cross_realm_function_prototype(&property.value) {
            // The host's synthetic Realm functions execute on the caller's
            // native Realm. Preserve the creation Realm's %Function.prototype%
            // on the synthesized %ThrowTypeError% instead of letting its
            // implicit prototype resolve through that caller Realm.
            let _ = throw_type_error.set_internal_prototype_slot(Some(function_prototype));
        }
    }
    throw_type_error.freeze();
    let value = Value::Function(throw_type_error);
    prototype.define_property(
        super::CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC.to_owned(),
        Property::fixed_non_enumerable(value.clone()),
    );
    Some(value)
}

fn cross_realm_function_prototype(value: &Value) -> Option<Prototype> {
    let Value::Function(prototype) = value else {
        return None;
    };
    Some(Prototype::Function(prototype.clone()))
}

fn arguments_iterator_property(function: &Function, env: &crate::context::Context<'_>) -> Property {
    // Reuse the shared %Array.prototype.values% so `arguments[Symbol.iterator]`
    // has the same identity as `Array.prototype.values`, falling back to a fresh
    // native if the realm intrinsic is somehow absent.
    let values = cross_realm_array_proto_values(function)
        .or_else(|| {
            function
                .realm()
                .and_then(|realm| realm.get_value(super::ARRAY_PROTO_VALUES_INTRINSIC))
        })
        .or_else(|| env.get_realm(super::ARRAY_PROTO_VALUES_INTRINSIC))
        .unwrap_or_else(|| {
            Value::Function(Function::new_native(
                Some("[Symbol.iterator]"),
                0,
                NativeFunction::ArrayPrototypeValues,
                false,
                env.realm(),
            ))
        });
    Property::non_enumerable(values)
}

fn cross_realm_array_proto_values(function: &Function) -> Option<Value> {
    if !function.has_dynamic_function_realm && !function.has_dynamic_function_realm_override.get() {
        return None;
    }
    match function
        .own_property(super::CROSS_REALM_ARRAY_PROTO_VALUES_INTRINSIC)?
        .value
    {
        Value::Function(values) => Some(Value::Function(values)),
        _ => None,
    }
}

pub(crate) fn native_mapped_argument_get(function: &Function) -> Result<Value, RuntimeError> {
    function
        .upvalues
        .first()
        .map_or(Ok(Value::Undefined), BindingCell::read)
}

pub(crate) fn native_mapped_argument_set(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    if let Some(parameter) = function.upvalues.first() {
        parameter.assign(
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            false,
        )?;
    }
    Ok(Value::Undefined)
}

/// Borrowed raw call inputs. The plan assigns slots; no name/value map exists.
#[derive(Clone, Copy)]
pub(crate) struct CallInputs<'a> {
    pub(crate) function: &'a Function,
    pub(crate) arguments: &'a [Value],
}

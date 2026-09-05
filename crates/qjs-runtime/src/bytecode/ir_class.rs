//! Cold, immutable class-definition payloads carried by `Op::NewClass`.
//!
//! These describe a class body once at compile time; evaluation reads them
//! through `Rc<Bytecode>` handles. They live apart from the opcode set so the
//! instruction module stays reviewable.

use std::rc::Rc;

use qjs_ast::FunctionParams;

use super::ir::Bytecode;

/// Cold, immutable payload used when evaluating a class definition.
#[derive(Clone, Debug)]
pub(super) struct ClassDefinition {
    pub(super) name: Option<String>,
    pub(super) constructor: ClassConstructorDef,
    /// Class elements (methods, accessors, and fields) in source order.
    pub(super) elements: Vec<ClassElementDef>,
    /// Private elements (fields, methods, accessors) in source order. These are
    /// not ordinary properties; they install into per-object private storage
    /// keyed by fresh per-evaluation private-name identities.
    pub(super) private_elements: Vec<ClassPrivateElementDef>,
    /// Computed member keys in source order. Most are pre-evaluated by the
    /// surrounding bytecode and left on the stack; keys that need the class
    /// private environment are deferred until `NewClass` runs.
    pub(super) computed_keys: Vec<ClassComputedKeyDef>,
    /// Whether the class has an `extends` heritage clause. When set, the
    /// heritage value was pushed onto the stack before this op.
    pub(super) has_heritage: bool,
}

/// Compiled definition of a class constructor.
#[derive(Clone, Debug)]
pub(super) struct ClassConstructorDef {
    pub(super) name: Option<String>,
    pub(super) params: FunctionParams,
    pub(super) local_names: Vec<String>,
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
}

/// Whether a class member key is a literal name or a computed expression.
#[derive(Clone, Debug)]
pub(super) enum ClassMemberKeyDef {
    /// A statically known string key.
    Literal(String),
    /// A computed key evaluated by `NewClass` in class-element order.
    Computed,
}

/// The kind of a class method member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ClassMethodKind {
    Method,
    Getter,
    Setter,
}

/// A class element in source order: a method/accessor or a field. Both kinds
/// may carry a computed key evaluated by `NewClass`.
#[derive(Clone, Debug)]
pub(super) enum ClassElementDef {
    Method(ClassMethodDef),
    Field(ClassFieldDef),
    /// A private field/method/accessor placeholder kept in source order so
    /// instance initialization can interleave private and public elements.
    Private(ClassPrivateElementDef),
    /// A `static { ... }` initialization block, run at class definition with
    /// `this` = the constructor, in source order with static fields.
    StaticBlock(ClassStaticBlockDef),
}

/// Compiled `static { ... }` block: a parameterless thunk whose body runs with
/// `this` = the constructor (its home object is the constructor too, so
/// `super.x` resolves against the constructor's prototype).
#[derive(Clone, Debug)]
pub(super) struct ClassStaticBlockDef {
    pub(super) local_names: Vec<String>,
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
}

/// A private class element in source order. Private names are keyed by source
/// text (`name`, without the `#`); a fresh identity is minted at class
/// evaluation. Accessor halves for the same name merge into one binding.
#[derive(Clone, Debug)]
pub(super) enum ClassPrivateElementDef {
    /// A private field. The initializer thunk runs at construction (instance)
    /// or class definition (static); `None` installs `undefined`.
    Field {
        name: String,
        is_static: bool,
        initializer: Option<ClassFieldInitializerDef>,
    },
    /// A private method shared by all instances/the constructor.
    Method {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
    /// A private getter half.
    Getter {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
    /// A private setter half.
    Setter {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
}

#[derive(Clone, Debug)]
pub(super) enum ClassComputedKeyDef {
    Precomputed,
    Deferred {
        local_names: Vec<String>,
        lexical_captures: Vec<(String, usize)>,
        bytecode: Rc<Bytecode>,
    },
}

/// Compiled definition of a class method or accessor.
#[derive(Clone, Debug)]
pub(super) struct ClassMethodDef {
    pub(super) key: ClassMemberKeyDef,
    pub(super) method_kind: ClassMethodKind,
    pub(super) is_static: bool,
    /// Function `name`, when statically known. Computed keys derive the name
    /// from the evaluated key at runtime.
    pub(super) name: Option<String>,
    pub(super) params: FunctionParams,
    pub(super) local_names: Vec<String>,
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
    pub(super) source_text: Option<Rc<str>>,
    /// Whether the method is a generator method (`*m() {}`).
    pub(super) is_generator: bool,
    /// Whether the method is an async method (`async m() {}`).
    pub(super) is_async: bool,
}

/// Compiled definition of a public class field. The initializer is compiled
/// as a thunk evaluated with `this` bound (the instance for an instance field,
/// the constructor for a static field); `None` installs `undefined`.
#[derive(Clone, Debug)]
pub(super) struct ClassFieldDef {
    pub(super) key: ClassMemberKeyDef,
    pub(super) is_static: bool,
    pub(super) initializer: Option<ClassFieldInitializerDef>,
}

/// Compiled field initializer thunk: a parameterless function body returning
/// the field value.
#[derive(Clone, Debug)]
pub(super) struct ClassFieldInitializerDef {
    pub(super) local_names: Vec<String>,
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
}

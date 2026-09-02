//! The wide compact tier: whole-body register execution for bodies that read
//! and write named properties, call methods, and loop.
//!
//! The numeric compact tier (`compact_fn`) admits only stack, local, binary
//! and call operations, and its dispatch loop is small enough that every
//! opcode added to it measurably slows the recursive bodies it exists for:
//! carrying the named-property operations in that same `match` cost the
//! recursive sentinel 20%, whether the extra arms were filtered before the
//! match, kept out of line, or packed into the same eight-byte operation
//! word. So this tier is a second executor with its own operation set, its
//! own compiler and its own frame driver, and the numeric tier's code is
//! untouched.
//!
//! A body is admitted here when the numeric tier declines it and every one
//! of its operations is in this tier's larger set. The set is a superset, so
//! a numeric body also compiles here, which is what lets this driver run an
//! admitted callee in a window of its own register stack whether or not that
//! callee would have run on the numeric tier.

use std::rc::Rc;

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::bytecode::ir::Bytecode;
use crate::bytecode::named_property_cache::NamedPropertyCache;

mod activation;
mod compile;
#[cfg(test)]
mod tests;

pub(in crate::bytecode) use activation::try_run_standalone;

/// Register-addressed form of the subset of `Op` this tier admits.
///
/// The numeric operations are the same as `CompactOp`'s; the rest carry a
/// receiver or a named-property site index into the program's side tables,
/// which keeps the operation word at eight bytes.
#[derive(Debug, Clone, Copy)]
enum WideOp {
    LoadConst {
        dst: u16,
        index: u32,
    },
    Move {
        dst: u16,
        src: u16,
    },
    LoadUpvalueLocal {
        dst: u16,
        slot: u16,
    },
    Drop {
        src: u16,
    },
    Binary {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    Call {
        dst: u16,
        base: u16,
        argc: u8,
    },
    Return {
        src: u16,
    },
    /// Duplicates a register, which the compiler emits to keep a method-call
    /// receiver live while the method is loaded.
    Dup {
        src: u16,
        dst: u16,
    },
    /// Loads the activation's seeded `this`. Admission requires a receiver to
    /// be seeded, exactly like `Vm.direct_this`, so this never fabricates one.
    LoadThis {
        dst: u16,
    },
    /// Reads the named property `named_reads[index]` of register `obj` into
    /// `dst`, through the same per-site cache as `Op::GetPropNamed`.
    GetPropNamed {
        dst: u16,
        obj: u16,
        index: u16,
    },
    /// Writes `value` to the named property `named_writes[index]` of register
    /// `obj`, leaving the assigned value in `obj`, with the same per-site
    /// cache and strictness as `Op::SetPropNamed`.
    SetPropNamed {
        obj: u16,
        value: u16,
        index: u16,
    },
    /// Calls the callee in register `base` with the receiver in the register
    /// immediately below it and `argc` arguments above it, writing the result
    /// to `dst`: `Op::CallResolved`'s `[receiver, callee, args...]` shape.
    CallResolved {
        dst: u16,
        base: u16,
        argc: u8,
    },
    /// Jumps when the register is truthy; the condition is not consumed.
    JumpIfTruthy {
        cond: u16,
        target: u32,
    },
    Unary {
        dst: u16,
        op: UnaryOp,
        src: u16,
    },
    Typeof {
        dst: u16,
        src: u16,
    },
    /// `ToNumeric` in place; the identity on a number.
    ToNumeric {
        dst: u16,
    },
    /// `++`/`--` in place.
    Update {
        dst: u16,
        op: UpdateOp,
    },
    /// `obj[key]` with a computed key; the result replaces `obj`'s register.
    GetProp {
        dst: u16,
        obj: u16,
        key: u16,
    },
    /// `obj[index]` with a constant index, the fused `Op::GetPropIndex`.
    GetPropIndex {
        dst: u16,
        obj: u16,
        index: u16,
    },
    /// Reads the global `global_names[index]` through the same resolution
    /// `Vm::load_global` applies to a slot-only frame.
    LoadGlobal {
        dst: u16,
        index: u16,
    },
    /// `new base(args...)`: constructs through the direct-leaf construct path
    /// or the general `construct_function`, writing the instance to `dst`.
    New {
        dst: u16,
        base: u16,
        argc: u8,
    },
    /// An array literal of `count` expression elements held in the registers
    /// starting at `base`; the array replaces `base`.
    NewArray {
        dst: u16,
        base: u16,
        count: u16,
    },
    /// Re-enters a lexical declaration's temporal dead zone.
    ClearLocal {
        slot: u16,
    },
    /// Reads a lexical local, throwing the interpreter's ReferenceError when
    /// it is still in its temporal dead zone.
    MoveChecked {
        dst: u16,
        src: u16,
    },
    /// Assigns a lexical local that must already be initialized.
    AssignChecked {
        dst: u16,
        src: u16,
    },
}

const _: () = assert!(std::mem::size_of::<WideOp>() == 8);

/// The payload of a named-property read site.
#[derive(Debug, Clone)]
pub(super) struct NamedReadSite {
    pub(super) key: Rc<str>,
    pub(super) cache: NamedPropertyCache,
}

/// The payload of a named-property write site.
#[derive(Debug, Clone)]
pub(super) struct NamedWriteSite {
    pub(super) key: Rc<str>,
    pub(super) cache: Option<NamedPropertyCache>,
    pub(super) is_strict: bool,
}

#[derive(Clone)]
pub(in crate::bytecode) struct WideProgram {
    ops: Vec<WideOp>,
    named_reads: Vec<NamedReadSite>,
    named_writes: Vec<NamedWriteSite>,
    /// Total register file width: locals followed by former stack slots.
    register_count: usize,
    /// Locals this body reads through indexed storage. Entry declines unless
    /// the frame reports every one of them as authoritative.
    required_authoritative_slots: u128,
    /// Whether the body reads `this`; such a body is admitted only when the
    /// activation seeds a receiver.
    requires_this: bool,
    /// Global names read by `LoadGlobal`, by side-table index.
    global_names: Vec<String>,
    /// The frame's own `let`/`const` slots. An activation seeds each with the
    /// temporal-dead-zone marker, exactly as the interpreter starts such a
    /// slot uninitialized, so a read that precedes the declaration's
    /// `ClearLocal`/store on some path still throws.
    lexical_slots: Vec<u16>,
    /// The marker every cleared lexical register holds; one allocation per
    /// program rather than one per clear.
    tdz_marker: crate::Value,
}

impl std::fmt::Debug for WideProgram {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WideProgram")
            .field("ops", &self.ops.len())
            .field("register_count", &self.register_count)
            .field("requires_this", &self.requires_this)
            .finish()
    }
}

/// Returns this body's wide program, compiling it on first use. A body that
/// cannot be represented caches `None`.
pub(super) fn program_for(bytecode: &Bytecode) -> Option<&WideProgram> {
    bytecode
        .compact_wide_program
        .get_or_init(|| compile::compile(bytecode))
        .as_ref()
}

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
mod loop_frame;
#[cfg(test)]
mod tests;

pub(super) use activation::try_run_in_caller_env;
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
    /// Leaves the rest of this activation to the interpreter, which resumes
    /// at bytecode instruction `ip` with this activation's locals and its
    /// `depth` operand-stack registers. At a probed backedge the jump itself
    /// follows: once no accelerator claims that loop the exit declines and
    /// execution falls through to it. At a computed store (`Op::SetProp`)
    /// the exit first tries the store as a plain one and, if it is,
    /// continues with the next operation instead.
    Exit {
        ip: u32,
        depth: u16,
    },
    /// Throws the register's value. Admitted bodies contain no handlers, so a
    /// throw always leaves the frame, exactly as a thrown callee error does.
    Throw {
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
    /// `this.key`: the named read `named_reads[index]` of the activation's
    /// receiver, borrowed where the activation keeps it.
    GetPropThis {
        dst: u16,
        index: u16,
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
    pub(super) creation: super::creation_cache::CreationCache,
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
    /// Registers `0..local_registers` hold locals; the operand stack follows.
    local_registers: u16,
    /// The locals an exit hands to the interpreter frame (see `WideOp::Exit`).
    own_locals: u128,
    /// Whether any path can exit; only such programs keep exit statistics.
    has_exits: bool,
    /// Activations and exits of this program, counted until it is judged.
    activations: std::cell::Cell<u32>,
    exits: std::cell::Cell<u32>,
    /// Set once a program has exited on most activations: an exit costs the
    /// interpreter frame the general path would have built plus the work run
    /// here first, so such a body is left to the general path from then on.
    exit_heavy: std::cell::Cell<bool>,
    /// The unconditional backward jumps that exit only so a loop accelerator
    /// can claim the loop, by ascending instruction index. Each such exit is
    /// followed by the jump itself (see `WideOp::Exit`).
    probed_backedges: Box<[ProbedBackedge]>,
    /// One bit per probed backedge, in `probed_backedges` order, set once no
    /// accelerator claimed that loop: the loop then runs here.
    native_backedges: std::cell::Cell<u64>,
    /// The wide instruction each bytecode instruction begins at, and the
    /// operand-stack depth there (`u16::MAX` where unreachable): where a
    /// typed loop program run from an exit hands the activation back.
    ip_to_pc: Box<[u32]>,
    ip_depth: Box<[u16]>,
}

/// A backward jump whose exit is probed (`WideProgram::probed_backedges`).
#[derive(Clone, Copy, Debug)]
struct ProbedBackedge {
    /// The bytecode instruction index of the jump.
    ip: u32,
    /// The index of the wide jump that follows the exit.
    jump_pc: u32,
    /// The operand-stack depth at the jump.
    depth: u16,
}

/// Activations observed before a program's exit rate is judged.
const EXIT_JUDGEMENT_ACTIVATIONS: u32 = 64;

impl WideProgram {
    /// Counts one activation; `false` once the program has proved exit-heavy.
    #[inline]
    pub(super) fn admit_activation(&self) -> bool {
        if !self.has_exits {
            return true;
        }
        if self.exit_heavy.get() {
            return false;
        }
        let activations = self.activations.get();
        if activations < EXIT_JUDGEMENT_ACTIVATIONS {
            self.activations.set(activations + 1);
        }
        true
    }

    /// The wide instruction to continue at for bytecode instruction `ip`
    /// with `depth` operand-stack values, if the tier can continue there.
    pub(super) fn resume_pc(&self, ip: usize, depth: usize) -> Option<usize> {
        let expected = *self.ip_depth.get(ip)?;
        (usize::from(expected) == depth)
            .then(|| self.ip_to_pc.get(ip).map(|&pc| pc as usize))
            .flatten()
    }

    /// The probe index of the backedge exit at `ip`, if it is one.
    pub(super) fn probed_backedge(&self, ip: u32) -> Option<usize> {
        self.probed_backedges
            .binary_search_by_key(&ip, |site| site.ip)
            .ok()
            .filter(|&index| index < u64::BITS as usize)
    }

    /// Where the tier continues a loop an interpreter frame handed back at
    /// the probed backedge `index`: at the jump that follows its exit.
    pub(super) fn backedge_jump_pc(&self, index: usize) -> usize {
        self.probed_backedges[index].jump_pc as usize
    }

    pub(super) fn backedge_is_native(&self, index: usize) -> bool {
        self.native_backedges.get() & (1 << index) != 0
    }

    pub(super) fn keep_backedge_native(&self, index: usize) {
        self.native_backedges
            .set(self.native_backedges.get() | (1 << index));
    }

    /// Counts one exit and judges the program after enough activations: at
    /// three exits in four it is exit-heavy.
    pub(super) fn record_exit(&self) {
        let exits = self.exits.get().saturating_add(1);
        self.exits.set(exits);
        let activations = self.activations.get();
        if activations >= EXIT_JUDGEMENT_ACTIVATIONS && exits.saturating_mul(4) >= activations * 3 {
            self.exit_heavy.set(true);
        }
    }
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
        .get_or_init(|| {
            #[cfg(feature = "perf-counters")]
            if std::env::var_os("QJS_CF_TRACE").is_some() {
                let mut decline = compile::Decline::default();
                let program = compile::compile_traced(bytecode, &mut decline);
                trace_decline(bytecode, program.is_none().then_some(&decline));
                return program;
            }
            compile::compile(bytecode)
        })
        .as_ref()
}

/// Diagnostic builds with `QJS_CF_TRACE=1` name every body this tier
/// compiles (`CFOK`) or declines (`CFDECLINE`, with the instruction and the
/// reason). Bodies are identified by their parameter names and length; join
/// the lines with `nested_vm_constructions` to find which callees still take
/// the general call path.
#[cfg(feature = "perf-counters")]
fn trace_decline(bytecode: &Bytecode, decline: Option<&compile::Decline>) {
    let params = bytecode.parameter_names().join(",");
    let len = bytecode.code.len();
    match decline {
        None => eprintln!("CFOK wide params=({params}) len={len}"),
        Some(decline) => {
            let op = decline
                .ip
                .and_then(|ip| bytecode.code.get(ip).map(|op| format!("ip {ip} op {op:?}")))
                .unwrap_or_else(|| "whole body".to_string());
            eprintln!(
                "CFDECLINE wide params=({params}) len={len} {op}: {}",
                decline.reason
            );
        }
    }
}

/// Whether an interpreter frame continuing a wide activation of `bytecode`
/// may hand it back at the backward jump `ip` with `depth` operand-stack
/// values: the jump must be a probed backedge of the same depth.
pub(in crate::bytecode) fn hands_back_at(bytecode: &Bytecode, ip: usize, depth: usize) -> bool {
    let Some(program) = bytecode.compact_wide_program.get().and_then(Option::as_ref) else {
        return false;
    };
    u32::try_from(ip)
        .ok()
        .and_then(|ip| program.probed_backedge(ip))
        .is_some_and(|index| usize::from(program.probed_backedges[index].depth) == depth)
}

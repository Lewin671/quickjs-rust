//! A shape-independent executor for loop regions.
//!
//! The specialized loop tiers each match one exact opcode sequence, so a loop
//! that computes the same thing in a different shape — an `if`/`else` in the
//! body, an extra temporary, a different operator order — runs on the general
//! interpreter at roughly 4.5ns per opcode. This module accepts *any* loop
//! region built from a whitelist of opcodes, compiles it once into a register
//! program, and runs that program with no operand stack, no `Value` boxing for
//! arithmetic, and no per-opcode dispatch through the main match.
//!
//! Values live in two register files. The scalar one holds unboxed numbers,
//! booleans, and `undefined` — the arithmetic this tier exists to accelerate.
//! The boxed one holds any `Value`, which is what an object receiver is and what
//! a property read produces. A stack entry always lives in the register with its
//! depth's index, so paths that meet at a join agree on where each value is, and
//! a backward jump inside the region — a nested loop — needs only a check that
//! the state it delivers matches the state recorded there.
//!
//! Admission is conservative and checked three times, and it turns on what a
//! region can *execute* rather than on how much of its work is boxed. At
//! compile time the region's opcodes must all be in the whitelist and its stack
//! behaviour must be statically consistent. At entry every slot the program
//! reads must hold a representable value, every slot it writes must be an
//! authoritative frame slot, and every receiver must be a dense array. Mid-run,
//! each operation that cannot be completed without observable behaviour hands
//! the loop back.
//!
//! Handing back is what makes a region with side effects safe to accelerate: the
//! interpreter resumes at the exact bytecode instruction that stopped, with the
//! operand stack that instruction expects rebuilt from the registers, so the
//! operations that already ran are exactly the ones it does not repeat.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use std::{
    cell::{OnceCell, RefCell},
    fmt,
    rc::Rc,
};

use crate::Value;

#[cfg(test)]
mod branchy_nested_tests;
mod compile;
mod execute;
mod helper_graph;
mod register_packing;

pub(super) use compile::compile_all;
pub(super) use execute::try_run_typed_loop;

/// Registers are addressed with 16 bits, which bounds a compiled region.
const MAX_REGISTERS: usize = 1 << 12;

/// Operand-stack depth a region may reach. Every stack entry lives in the
/// register with its depth's index, in whichever register file its class names,
/// so two paths that meet at a join agree on where each value is without any
/// phi bookkeeping. A deeper region declines.
const MAX_STACK_DEPTH: usize = 48;

/// Longest region accepted, so compilation stays a bounded one-time cost.
const MAX_REGION_OPS: usize = 512;

/// Iterations run before handing control back, so a program cannot make the
/// engine unresponsive any longer than the interpreter would.
const MAX_NATIVE_ITERATIONS: u64 = 1 << 28;

/// An unboxed loop value. Only these three types take part; anything else
/// declines admission or deoptimizes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Typed {
    Number(f64),
    Boolean(bool),
    Undefined,
}

impl Typed {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(number) => Some(Self::Number(*number)),
            Value::Boolean(value) => Some(Self::Boolean(*value)),
            Value::Undefined => Some(Self::Undefined),
            _ => None,
        }
    }

    fn to_value(self) -> Value {
        match self {
            Self::Number(number) => Value::Number(number),
            Self::Boolean(value) => Value::Boolean(value),
            Self::Undefined => Value::Undefined,
        }
    }

    fn number(self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    fn is_truthy(self) -> bool {
        match self {
            Self::Number(number) => number != 0.0 && !number.is_nan(),
            Self::Boolean(value) => value,
            Self::Undefined => false,
        }
    }

    /// `ToNumeric` for the admitted types, which never observes user code.
    fn to_numeric(self) -> Self {
        match self {
            Self::Number(_) => self,
            Self::Boolean(value) => Self::Number(f64::from(u8::from(value))),
            Self::Undefined => Self::Number(f64::NAN),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TypedOp {
    Move {
        dst: u16,
        src: u16,
    },
    ToNumeric {
        dst: u16,
        src: u16,
    },
    Binary {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    Unary {
        dst: u16,
        op: UnaryOp,
        src: u16,
    },
    Update {
        dst: u16,
        op: UpdateOp,
        src: u16,
    },
    /// Reads one element of a dense array held in a frame slot. The receiver
    /// stays a slot rather than a register because registers hold only unboxed
    /// values.
    DenseRead {
        dst: u16,
        receiver: u16,
        index: u16,
    },
    /// Overwrites one in-bounds element of a dense array held in a frame slot.
    DenseWrite {
        receiver: u16,
        index: u16,
        value: u16,
    },
    /// Publishes one scalar value to a prevalidated sloppy fallback global.
    StoreSloppyGlobal {
        target: u16,
        value: u16,
    },
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    /// Moves a value between boxed registers.
    MoveBoxed {
        dst: u16,
        src: u16,
    },
    /// Narrows a boxed register to a scalar one, deoptimizing when the value is
    /// not one of the representable types.
    Unbox {
        dst: u16,
        src: u16,
    },
    /// Widens a scalar register into a boxed one.
    Box {
        dst: u16,
        src: u16,
    },
    /// Reads an own data property of a boxed object register, remembering the
    /// storage slot and interned name it resolved to so later iterations
    /// revalidate by pointer instead of resolving the name again.
    GetNamed {
        dst: u16,
        object: u16,
        name: u16,
        cache: u16,
    },
    /// Overwrites an existing own data property of a boxed object register,
    /// through the site's (name, slot) cache.
    SetNamed {
        object: u16,
        name: u16,
        value: u16,
        cache: u16,
    },
    /// `GetNamed` whose result the region consumes as a scalar: the property
    /// value lands in the scalar file, and anything the scalar file cannot
    /// hold deoptimizes at the read.
    GetNamedTyped {
        dst: u16,
        object: u16,
        name: u16,
        cache: u16,
    },
    /// `SetNamed` of a scalar register.
    SetNamedTyped {
        object: u16,
        name: u16,
        value: u16,
        cache: u16,
    },
    /// Reads one element of a dense array held in a boxed register.
    ElementRead {
        dst: u16,
        receiver: u16,
        index: u16,
    },
    /// Reads `receiver[key]` where the key is a boxed value rather than an
    /// array index. `ElementRead` conflates "this read's result must be boxed"
    /// with "this read has array semantics"; a dictionary access needs the
    /// first without the second.
    ComputedRead {
        dst: u16,
        receiver: u16,
        key: u16,
    },
    /// Writes `receiver[key] = value` under a boxed key, overwriting an
    /// existing own data property only.
    ComputedWrite {
        receiver: u16,
        key: u16,
        value: u16,
    },
    /// Calls a pure numeric intrinsic — a `Math` function whose whole effect is
    /// a floating-point computation — after checking at run time that the callee
    /// really is that intrinsic.
    CallNumericNative {
        dst: u16,
        callee: u16,
        first: u16,
        second: u16,
        arity: u8,
    },
    /// Calls a callee whose entire body one of the closed-form leaf evaluators
    /// can answer, keeping the call's receiver, callee, arguments, and result in
    /// registers instead of on the operand stack.
    ///
    /// This is the only operation here that runs a user function, and it is
    /// admitted in the one form that preserves this tier's rule that an
    /// operation either succeeds or stops the program before becoming
    /// observable. The closed-form evaluators answer with `Option<Value>`: they
    /// compute a body they have already proven total, and a body they have not
    /// proven yields `None` rather than running. There is therefore no state in
    /// which the callee has half-executed and the loop has to unwind it.
    ///
    /// The receiver is kept rather than dropped, because a receiver-property
    /// body is exactly what the second evaluator answers.
    CallClosedFormLeaf {
        dst: u16,
        receiver: u16,
        callee: u16,
        first: u16,
        second: u16,
        arity: u8,
    },
    /// A compound element access's operand check, on the register at the
    /// top of the abstract stack. `Coercible` deoptimizes on `null` or
    /// `undefined` (the interpreter raises the TypeError);
    /// `PropertyKey` deoptimizes on anything but a number, string or
    /// boolean, whose key conversion the consuming access performs itself.
    /// Anything else is a no-op, which is why the operand stays in place.
    Guard {
        src: u16,
        boxed: bool,
        kind: GuardKind,
    },
    /// `receiver.push(value)` where the callee register holds the realm's
    /// `Array.prototype.push` and the receiver is an ordinary dense array:
    /// the element is appended in place and the new length lands in the
    /// scalar file. Anything else deoptimizes to the interpreter's call.
    ArrayPush {
        dst: u16,
        receiver: u16,
        callee: u16,
        value: u16,
    },
    /// Leaves the loop: the condition value goes back on the operand stack,
    /// because the instruction at the loop's exit pops it.
    Exit {
        cond: u16,
        exit_ip: u32,
    },
    /// Compares two boxed operands without running anything the property
    /// protocol could observe, producing a scalar boolean.
    ///
    /// Unboxing an operand to compare it only works for numbers, so `a == b`
    /// over two objects deoptimized on its first iteration -- the shape of
    /// every identity search in JavaScript. The runtime side reuses the
    /// interpreter's own predicates, so the two agree by construction, and any
    /// pair that would need `ToPrimitive` deoptimizes instead of guessing.
    BoxedEquality {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    /// Leaves the region unconditionally, handing the instruction at `exit_ip`
    /// back to the interpreter with the operand stack this operation's site
    /// describes.
    ///
    /// This is what lets a `return` inside a loop body be compiled at all: a
    /// search loop leaves through its result, not through its header test, and
    /// without this the whole region declined. Resuming *at* the instruction
    /// rather than after it is the same contract [`Exit`] uses, so the
    /// interpreter runs the return itself and nothing is replayed.
    Leave {
        exit_ip: u32,
    },
}

/// Where the interpreter resumes when a program stops mid-region, and what the
/// operand stack has to look like when it does.
///
/// Every program operation carries the bytecode instruction it belongs to and
/// the abstract stack that instruction starts from. Because a stack entry always
/// lives in the register with its depth's index, materializing that stack is a
/// matter of reading `depth` registers, picking the boxed file for the depths
/// `boxed` marks. Resuming at the instruction — rather than at the loop header —
/// is what makes a program with side effects safe to abandon halfway: the
/// operations that already ran are exactly the ones the interpreter will not
/// repeat.
#[derive(Clone, Copy, Debug)]
struct DeoptSite {
    ip: u32,
    /// Range of this site's entries in the program's `site_entries`.
    start: u32,
    len: u8,
}

/// Which register file a value lives in. Scalars are unboxed numbers, booleans,
/// and `undefined`; boxed registers hold any `Value`, which is what a property
/// read produces and what an object receiver has to be.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Class {
    Scalar,
    Boxed,
}

/// Per-entry storage for one native loop run. The executor owns this while a
/// program is active, then clears and returns it to that program's tiny pool.
/// Keeping it program-local means no temporary state crosses bytecode bodies.
#[derive(Default)]
struct TypedLoopScratch {
    registers: Vec<Typed>,
    receivers: Vec<crate::ArrayRef>,
    boxed: Vec<Value>,
    sloppy_global_writes: Vec<super::vm_bindings::TypedLoopSloppyGlobalWrite>,
}

impl TypedLoopScratch {
    fn clear(&mut self) {
        self.registers.clear();
        self.receivers.clear();
        self.boxed.clear();
        self.sloppy_global_writes.clear();
    }
}

impl fmt::Debug for TypedLoopScratch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TypedLoopScratch")
            .field("register_count", &self.registers.len())
            .field("receiver_count", &self.receivers.len())
            .field("boxed_count", &self.boxed.len())
            .field(
                "sloppy_global_write_count",
                &self.sloppy_global_writes.len(),
            )
            .finish()
    }
}

/// The literal shapes one property-read site has resolved.
///
/// The one-way `caches` entry is not enough on a polymorphic site:
/// `heterogeneous_property_read` rotates three distinct object-literal shapes
/// through one read, so it missed every iteration and fell back to resolving
/// the name -- a hash lookup and a `memcmp` per access, which the profile
/// charged 27% of that sentinel to. Shape identity is an `Rc` pointer
/// comparison, so scanning a few is far cheaper than resolving the name.
/// One remembered shape and the slot the property occupies in it.
type ShapeWay = (Rc<crate::value::ObjectLiteralShape>, usize);

/// A property resolved on a receiver's immediate prototype.
///
/// A method call site reads its callee from the prototype on every iteration,
/// and walking there costs a hash lookup and a `memcmp` per level. Remembering
/// the holder makes the repeat visit a pointer comparison plus a slot read.
///
/// Validity has three parts, all cheap: the receiver must still miss the name
/// on its own (so nothing shadows it), its prototype must still be the
/// remembered holder, and the holder's property revision must be unchanged.
#[derive(Clone, Debug)]
pub(super) struct InheritedWay {
    holder: crate::ObjectRef,
    revision: u64,
    slot: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ShapeWays {
    /// The (interned name, slot) pair the site last resolved on a small
    /// object. Objects built by the same code site share the interned name,
    /// so the pointer comparison serves all of them. This used to live in
    /// the per-entry scratch and was cleared at every loop entry; an inner
    /// loop that runs a few iterations per entry -- nbody's pair loop -- then
    /// re-resolved every field access by name scan on each entry.
    pub(super) slot: Option<(Rc<str>, usize)>,
    /// Boxed deliberately, against `clippy::box_collection`. Most sites are
    /// answered by the one-way cache and never resolve a shape at all, so this
    /// keeps their entry one pointer instead of three. Measured, not assumed:
    /// unboxed costs `prototype_method_call` 3.9% against 2.6% boxed, and that
    /// sentinel never reaches the shape path.
    #[allow(clippy::box_collection)]
    ways: Option<Box<Vec<ShapeWay>>>,
    /// The prototype resolution for this site, if it has one. Boxed for the
    /// same reason: a site answered by an own property never allocates it.
    inherited_way: Option<Box<InheritedWay>>,
    /// The value last read from one exact receiver whose storage has no
    /// stable slot to cache -- a builtin such as `Math`, whose property table
    /// is dynamic. Validated by receiver identity and the receiver's property
    /// revision, which every own-property write or layout change advances.
    /// `Math.abs(i)` in a loop otherwise resolved `abs` by hash lookup and
    /// name comparison on every iteration.
    exact_way: Option<Box<ExactWay>>,
}

/// A value read from one specific receiver at one property revision.
#[derive(Clone, Debug)]
pub(super) struct ExactWay {
    holder: crate::value::ObjectWeakRef,
    revision: u64,
    value: Value,
}

impl ExactWay {
    pub(super) fn read(&self, receiver: &crate::ObjectRef) -> Option<Value> {
        (self.holder.ptr_eq(receiver) && receiver.property_revision() == self.revision)
            .then(|| self.value.clone())
    }
}

impl InheritedWay {
    /// Re-reads the remembered prototype property, or `None` if anything the
    /// resolution depended on has moved.
    pub(super) fn read(&self, receiver: &crate::ObjectRef) -> Option<Value> {
        let crate::value::Prototype::Object(prototype) = receiver.prototype_slot()? else {
            return None;
        };
        if !prototype.ptr_eq(&self.holder) || prototype.property_revision() != self.revision {
            return None;
        }
        self.holder.own_data_slot_value(self.slot)
    }
}

impl ShapeWays {
    /// How many distinct shapes one site remembers. Past a handful a site is
    /// megamorphic and the linear scan stops paying for itself.
    const WAYS: usize = 4;

    pub(super) fn inherited(&self) -> Option<&InheritedWay> {
        self.inherited_way.as_deref()
    }

    pub(super) fn record_inherited(
        &mut self,
        holder: crate::ObjectRef,
        revision: u64,
        slot: usize,
    ) {
        self.inherited_way = Some(Box::new(InheritedWay {
            holder,
            revision,
            slot,
        }));
    }

    pub(super) fn entries(&self) -> &[ShapeWay] {
        self.ways.as_deref().map_or(&[], Vec::as_slice)
    }

    pub(super) fn exact(&self) -> Option<&ExactWay> {
        self.exact_way.as_deref()
    }

    pub(super) fn record_exact(&mut self, holder: &crate::ObjectRef, value: Value) {
        self.exact_way = Some(Box::new(ExactWay {
            holder: holder.downgrade(),
            revision: holder.property_revision(),
            value,
        }));
    }

    pub(super) fn record(&mut self, shape: Rc<crate::value::ObjectLiteralShape>, slot: usize) {
        let ways = self.ways.get_or_insert_with(Box::default);
        if ways.len() < Self::WAYS {
            ways.push((shape, slot));
        }
    }
}

/// What a [`TypedOp::Guard`] checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GuardKind {
    Coercible,
    PropertyKey,
}

/// A compiled loop region.
#[derive(Clone, Debug)]
pub(super) struct TypedLoopProgram {
    header: usize,
    backedge: usize,
    ops: Vec<TypedOp>,
    /// Resume information for each operation, parallel to `ops`.
    sites: Vec<DeoptSite>,
    /// Operand-stack entries the sites name, bottom to top, as the register
    /// holding each one and the file it lives in.
    site_entries: Vec<(Class, u16)>,
    register_count: usize,
    /// Register holding each referenced frame slot, as (register, slot).
    local_slots: Vec<(u16, u32)>,
    /// Slots that must be written back when the loop ends, each with the
    /// register holding its value. Several slots may share one register: a
    /// completion temporary that only ever receives one expression's value needs
    /// no register of its own.
    written_locals: Vec<(u16, u32)>,
    /// Slots that must hold a dense-readable array on entry.
    receiver_slots: Vec<u32>,
    /// Global bindings the region reads, in register order. The region does
    /// not write any of these names and cannot run observable code, so reading
    /// each one once on entry is equivalent to reading it per iteration.
    global_reads: Vec<(u16, String)>,
    /// Existing sloppy fallback globals written by the region. Their matching
    /// reads use the frame-slot register rather than a hoisted global read, and
    /// each write is synchronized immediately by the executor.
    sloppy_global_writes: Vec<(u32, String)>,
    /// Number of boxed registers, which hold objects and any value a property
    /// read produces.
    boxed_count: usize,
    /// Boxed register holding each referenced frame slot, as (register, slot).
    boxed_locals: Vec<(u16, u32)>,
    /// Boxed local registers which occur as the callee of an unbound numeric
    /// native call. These alone may hold a Function rather than an ordinary
    /// object at native-loop entry; the call operation rechecks the exact
    /// intrinsic before it can execute.
    /// Boxed registers written back to their slots when the loop ends.
    written_boxed_locals: Vec<u16>,
    /// Global bindings read into boxed registers.
    boxed_global_reads: Vec<(u16, String)>,
    /// Property names the program reads or writes, addressed by index.
    names: Vec<Rc<str>>,
    /// Registers seeded once with a constant, as (register, value): a constant
    /// never changes, so it costs nothing per iteration.
    constant_registers: Vec<(u16, Typed)>,
    /// Boxed registers seeded once with a constant no scalar register can hold.
    boxed_constant_registers: Vec<(u16, Value)>,
    /// Number of property-access cache entries the run needs.
    cache_count: usize,
    /// Created only after the first native entry, so a program that compiles
    /// but never runs in this tier pays no pool allocation. One cleared bundle
    /// then serves the common sequential-entry case without retaining scratch
    /// storage for every recursive invocation.
    scratch_pool: OnceCell<Rc<RefCell<Vec<TypedLoopScratch>>>>,
    /// Shape ways per property site, kept on the program rather than in the
    /// per-activation scratch.
    ///
    /// Two reasons. Adding a sixth vector to the scratch cost
    /// `prototype_method_call` 3.6%: that sentinel declines this tier on
    /// essentially every backedge, and a declined backedge still moves the
    /// scratch twice. And a site's shapes are a property of the site, not of
    /// one activation, so keeping them here lets a short loop that is entered
    /// repeatedly reuse what it learned. Shape identity and the property
    /// revision are re-checked on every read, so a stale entry misses rather
    /// than reading a wrong slot.
    shape_caches: RefCell<Vec<ShapeWays>>,
    /// The bodies `helper_sites` resolved to, rebuilt at every entry.
    ///
    /// Reached only from `call_closed_form_leaf`, which already has the
    /// program. Threading it through `execute` instead measured 16% on
    /// `heterogeneous_property_read`: one more loop-carried live value costs
    /// this executor every opcode, whether or not the value is used.
    helper_graphs: RefCell<helper_graph::HelperGraph>,
    /// One entry per helper call site, in site order, naming the frame
    /// slot its callee is read from. Entry resolves each one and flattens the
    /// body it points at.
    helper_sites: Vec<HelperSite>,
}

/// Where one helper call finds its callee at loop entry.
#[derive(Clone, Copy, Debug)]
struct HelperSite {
    callee_slot: u32,
    arity: u8,
}

impl TypedLoopProgram {
    const MAX_POOLED_SCRATCH_BUNDLES: usize = 1;

    pub(super) fn header(&self) -> usize {
        self.header
    }

    pub(super) fn backedge(&self) -> usize {
        self.backedge
    }

    fn slot_for_boxed_register(&self, register: u16) -> Option<u32> {
        self.boxed_locals
            .iter()
            .find(|(candidate, _)| *candidate == register)
            .map(|(_, slot)| *slot)
    }

    fn take_scratch(&self) -> TypedLoopScratch {
        self.scratch_pool
            .get_or_init(|| Rc::new(RefCell::new(Vec::new())))
            .borrow_mut()
            .pop()
            .unwrap_or_default()
    }

    fn recycle_scratch(&self, mut scratch: TypedLoopScratch) {
        scratch.clear();
        let mut pooled = self
            .scratch_pool
            .get_or_init(|| Rc::new(RefCell::new(Vec::new())))
            .borrow_mut();
        if pooled.len() < Self::MAX_POOLED_SCRATCH_BUNDLES {
            pooled.push(scratch);
        }
    }
}

#[cfg(test)]
mod tests;

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
    /// Overwrites an existing own data property of a boxed object register.
    SetNamed {
        object: u16,
        name: u16,
        value: u16,
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
    caches: Vec<Option<(Rc<str>, usize)>>,
}

impl TypedLoopScratch {
    fn clear(&mut self) {
        self.registers.clear();
        self.receivers.clear();
        self.boxed.clear();
        self.sloppy_global_writes.clear();
        self.caches.clear();
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
            .field("cache_count", &self.caches.len())
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

    pub(super) fn record(&mut self, shape: Rc<crate::value::ObjectLiteralShape>, slot: usize) {
        let ways = self.ways.get_or_insert_with(Box::default);
        if ways.len() < Self::WAYS {
            ways.push((shape, slot));
        }
    }
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
mod tests {
    use crate::{Value, eval};

    fn nested_function(source: &str) -> crate::bytecode::Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = crate::bytecode::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                super::super::ir::Op::NewFunction { bytecode, .. } => {
                    Some(bytecode.as_ref().clone())
                }
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    #[test]
    fn typed_loop_scratch_pool_reuses_only_cleared_storage() {
        let bytecode = nested_function(
            "function run(n) { var total = 0; for (var i = 0; i < n; i++) { total += i; } return total; }",
        );
        let programs = super::compile_all(&bytecode);
        let program = programs.first().expect("loop should be admitted");
        assert!(program.scratch_pool.get().is_none());

        let mut first = program.take_scratch();
        first.registers.push(super::Typed::Number(1.0));
        first.boxed.push(Value::Number(2.0));
        first.caches.push(Some((std::rc::Rc::from("value"), 3)));
        let register_capacity = first.registers.capacity();
        let boxed_capacity = first.boxed.capacity();
        let cache_capacity = first.caches.capacity();
        program.recycle_scratch(first);

        let reused = program.take_scratch();
        assert!(reused.registers.is_empty());
        assert!(reused.receivers.is_empty());
        assert!(reused.boxed.is_empty());
        assert!(reused.sloppy_global_writes.is_empty());
        assert!(reused.caches.is_empty());
        assert!(reused.registers.capacity() >= register_capacity);
        assert!(reused.boxed.capacity() >= boxed_capacity);
        assert!(reused.caches.capacity() >= cache_capacity);
        program.recycle_scratch(reused);

        // A nested call receives fresh storage when the one sequential-entry
        // slot is occupied, and its later return cannot grow the pool.
        let mut active = program.take_scratch();
        active.registers.push(super::Typed::Number(4.0));
        let nested = program.take_scratch();
        assert!(nested.registers.is_empty());
        program.recycle_scratch(nested);
        assert_eq!(active.registers, vec![super::Typed::Number(4.0)]);
        program.recycle_scratch(active);
        assert_eq!(
            program
                .scratch_pool
                .get()
                .expect("taking scratch should initialize the pool")
                .borrow()
                .len(),
            1
        );
    }

    #[test]
    fn typed_loop_compacts_register_files_to_used_stack_depth() {
        let scalar_bytecode = nested_function(
            "function run(n) { var total = 0; for (var i = 0; i < n; i++) { total += i; } return total; }",
        );
        let scalar_program = super::compile_all(&scalar_bytecode)
            .into_iter()
            .next()
            .expect("scalar loop should be admitted");
        assert!(scalar_program.register_count < super::MAX_STACK_DEPTH);
        assert_eq!(scalar_program.boxed_count, 0);

        let dense_bytecode = nested_function(
            "function run(n) { var values = [1, 2, 4]; var total = 0; for (var i = 0; i < n; i++) { total += values[i % 3]; } return total; }",
        );
        let dense_program = super::compile_all(&dense_bytecode)
            .into_iter()
            .next()
            .expect("dense loop should be admitted");
        assert!(dense_program.register_count < super::MAX_STACK_DEPTH);
        assert!(dense_program.boxed_count > 0);
        assert!(dense_program.boxed_count < super::MAX_STACK_DEPTH);
        assert_eq!(
            eval(
                "function run(n) { var values = [1, 2, 4]; var total = 0; for (var i = 0; i < n; i++) { total += values[i % 3]; } return total; } run(6);"
            ),
            Ok(Value::Number(14.0))
        );
    }

    /// Every case here is a loop the typed tier accepts. The expected values are
    /// what the interpreter produces for the same source, so a divergence in the
    /// register program shows up as a failing assertion rather than a silent
    /// wrong answer.
    #[test]
    fn typed_loops_match_interpreted_results() {
        // Arithmetic, an if/else body, and a shift — the shape the specialized
        // tiers decline because of the branch.
        assert_eq!(
            eval(
                "function run(n) { var a = 0, b = 1, c = 0;\
                   for (var i = 0; i < n; i++) {\
                     if (a > b) { c = a - (b >> i); b = (a >> i) + b; a = c; }\
                     else { c = a + (b >> i); b = -(a >> i) + b; a = c; }\
                   }\
                   return a + ':' + b + ':' + c; }\
                 run(40);"
            ),
            Ok(Value::String("2:1:2".to_owned().into()))
        );
        // An element read from a dense array in a local slot.
        assert_eq!(
            eval(
                "function run(n) { var table = [1, 2, 4, 8, 16], total = 0;\
                   for (var i = 0; i < n; i++) { total = total + table[i % 5]; }\
                   return total; }\
                 run(20);"
            ),
            Ok(Value::Number(124.0))
        );
        // Every admitted operator, so the register program's semantics are
        // pinned against the interpreter's.
        assert_eq!(
            eval(
                "function run() { var s = 0;\
                   for (var i = 1; i < 12; i++) {\
                     s += i * 3 - 1;\
                     s += i / 2;\
                     s += i % 4;\
                     s += i ** 2;\
                     s += (i << 2) | (i >> 1);\
                     s += (i & 6) ^ (i >>> 1);\
                     s += -i;\
                     s += ~i;\
                     if (i < 5) s += 1;\
                     if (i <= 5) s += 1;\
                     if (i > 5) s += 1;\
                     if (i >= 5) s += 1;\
                     if (i === 5) s += 1;\
                     if (i !== 5) s += 1;\
                     if (!(i === 5)) s += 1;\
                   }\
                   return s; }\
                 run();"
            ),
            Ok(Value::Number(980.0))
        );
        // A zero-iteration loop leaves everything untouched.
        assert_eq!(
            eval(
                "function run() { var s = 7; for (var i = 0; i < 0; i++) { s = s + 1; } return s + ':' + i; } run();"
            ),
            Ok(Value::String("7:0".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_write_dense_elements() {
        // A branchy read-modify-write over a dense array: the shape the
        // specialized tiers decline, and the reason the element-assignment
        // idiom is recognized as a unit.
        assert_eq!(
            eval(
                "function run() { var a = [0, 1, 2, 3, 4];                   for (var r = 0; r < 3; r++) {                     for (var i = 0; i < 5; i++) {                       if (a[i] > 1) { a[i] = a[i] - 1; } else { a[i] = a[i] + 1; }                     }                   }                   return a.join(','); }                 run();"
            ),
            Ok(Value::String("1,2,1,2,1".to_owned().into()))
        );
        // The assignment's value is the expression's value.
        assert_eq!(
            eval(
                "function run() { var a = [1, 2, 3], last = 0;                   for (var i = 0; i < 3; i++) { last = (a[i] = a[i] * 2); }                   return a.join(',') + ':' + last; }                 run();"
            ),
            Ok(Value::String("2,4,6:6".to_owned().into()))
        );
        // Growth, a frozen array, a hole, and an own indexed descriptor all stay
        // on the observable path.
        assert_eq!(
            eval(
                "function run() { var a = [1]; for (var i = 0; i < 4; i++) { a[i] = i; } return a.join(','); } run();"
            ),
            Ok(Value::String("0,1,2,3".to_owned().into()))
        );
        assert_eq!(
            eval(
                "function run() { var a = Object.freeze([1, 2]);                   for (var i = 0; i < 2; i++) { a[i] = 9; }                   return a.join(','); }                 run();"
            ),
            Ok(Value::String("1,2".to_owned().into()))
        );
        assert_eq!(
            eval(
                "function run() { var a = [1, 2];                   Object.defineProperty(a, '1', { value: 5, writable: false, configurable: true, enumerable: true });                   for (var i = 0; i < 2; i++) { a[i] = i + 10; }                   return a.join(','); }                 run();"
            ),
            Ok(Value::String("10,5".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_write_dense_elements_with_computed_scalar_indices() {
        let source = "function run(n) { var values = [0, 1, 2, 3, 4, 5, 6, 7]; for (var i = 0; i < n; i++) { values[(i + 1) & 7] = values[(i + 3) & 7] + i; } return values.join(','); }";
        let bytecode = nested_function(source);
        assert_eq!(
            super::compile_all(&bytecode).len(),
            1,
            "{:#?}",
            bytecode.code
        );
        assert_eq!(
            eval(&format!("{source} run(8);")),
            Ok(Value::String("12,3,5,7,9,11,5,9".to_owned().into()))
        );
        // A computed index that grows the Array declines at the write and
        // replays the first uncommitted assignment through ordinary semantics.
        assert_eq!(
            eval(
                "function run() { var values = [1]; for (var i = 0; i < 3; i++) { values[(i + 1) & 3] = i; } return values.join(','); } run();"
            ),
            Ok(Value::String("1,0,1,2".to_owned().into()))
        );
        // The key is evaluated before the value. The value mutates `i`, so
        // using its register directly for the dense write would store at a
        // different element than ordinary JavaScript.
        let order_source = "function run(n) { var values = [0, 0, 0, 0]; for (var i = 0; i < n; i++) { values[(i + 1) & 3] = (i = i + 1); } return values.join(',') + ':' + i; }";
        assert_eq!(super::compile_all(&nested_function(order_source)).len(), 1);
        assert_eq!(
            eval(&format!("{order_source} run(4);")),
            Ok(Value::String("0,1,0,3:4".to_owned().into()))
        );
    }

    #[test]
    fn typed_loop_computed_index_with_assignment_stays_interpreted() {
        let source = "function run() { var values = [0, 0, 0, 0], key = 0; for (var i = 0; i < 4; i++) { values[key = (i + 1) & 3] = i; } return values.join(',') + ':' + key; }";
        let bytecode = nested_function(source);
        assert!(
            super::compile_all(&bytecode).is_empty(),
            "{:#?}",
            bytecode.code
        );
        assert_eq!(
            eval(&format!("{source} run();")),
            Ok(Value::String("3,0,1,2:0".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_read_globals_only_when_that_is_equivalent() {
        // A global read is hoisted to loop entry, which is equivalent because
        // the region provably writes no global and cannot call user code.
        assert_eq!(
            eval(
                "var k = 3; function run(n) { var s = 0;                   for (var i = 0; i < n; i++) { if (i % 7 > k) { s += k; } else { s -= k; } }                   return s; }                 run(14);"
            ),
            Ok(Value::Number(-6.0))
        );
        // Each entry re-reads, so a value changed between loops is picked up.
        assert_eq!(
            eval(
                "var k = 1; function run() { var s = 0;                   for (var i = 0; i < 3; i++) { if (i > 0) { s += k; } else { s -= k; } }                   return s; }                 var first = run(); k = 10; first + ':' + run();"
            ),
            Ok(Value::String("1:10".to_owned().into()))
        );
        // A region that writes a global keeps the observable path, so every
        // iteration sees the previous one's write.
        assert_eq!(
            eval(
                "var g = 5; function run() { var s = 0; for (var i = 0; i < 4; i++) { s += g; g = g + 1; } return s + ':' + g; } run();"
            ),
            Ok(Value::String("26:9".to_owned().into()))
        );
        // An accessor on the global object is called once per read, so the
        // region declines.
        assert_eq!(
            eval(
                "var calls = 0;                 Object.defineProperty(globalThis, 'probe', { get: function () { calls++; return 2; }, configurable: true });                 function run() { var s = 0; for (var i = 0; i < 5; i++) { if (i > 1) { s += probe; } else { s -= probe; } } return s; }                 var result = run(); result + ':' + calls;"
            ),
            Ok(Value::String("2:5".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_sync_existing_sloppy_numeric_globals() {
        let source = "var typedLoopSloppyProbe = 0; function run(n) { var total = typedLoopSloppyProbe = 0; for (var i = 0; i < n; i++) { typedLoopSloppyProbe = typedLoopSloppyProbe + Math.pow(i, 0); total = total + typedLoopSloppyProbe; } return total + ':' + typedLoopSloppyProbe; }";
        let bytecode = nested_function(source);
        assert_eq!(super::compile_all(&bytecode).len(), 1, "{bytecode:#?}");
        assert_eq!(
            eval(&format!("{source} run(5);")),
            Ok(Value::String("15:5".to_owned().into()))
        );

        // A native-call guard can fail after a completed sloppy-global write.
        // Resume at the exact call site so the generic callee and subsequent
        // string additions run once, without repeating the completed store.
        let deopt_source = "function run(n) { var total = typedLoopDeoptProbe = 0, box = { f: function () { return 'x'; } }; for (var i = 0; i < n; i++) { typedLoopDeoptProbe = typedLoopDeoptProbe + 1; total = total + box.f(i); total = total + typedLoopDeoptProbe; } return total + ':' + typedLoopDeoptProbe; }";
        assert_eq!(
            super::compile_all(&nested_function(deopt_source)).len(),
            1,
            "{deopt_source}"
        );
        assert_eq!(
            eval(&format!("{deopt_source} run(2);")),
            Ok(Value::String("0x1x2:2".to_owned().into()))
        );

        // A read-only descriptor must retain sloppy assignment's silent
        // failure rather than entering the prevalidated sink path.
        assert_eq!(
            eval(
                "Object.defineProperty(globalThis, 'typedLoopReadOnlyProbe', { value: 2, writable: false, configurable: true });\
                 function run(n) { var total = typedLoopReadOnlyProbe = 0;\
                   for (var i = 0; i < n; i++) {\
                     total = total + typedLoopReadOnlyProbe;\
                     typedLoopReadOnlyProbe = typedLoopReadOnlyProbe + 1;\
                   }\
                   return total + ':' + typedLoopReadOnlyProbe; }\
                 run(3);"
            ),
            Ok(Value::String("6:2".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_deoptimize_instead_of_guessing() {
        // A non-numeric operand mid-loop must fall back to the interpreter and
        // produce the interpreter's answer, including string concatenation.
        assert_eq!(
            eval(
                "function run() { var s = 0, flip = 3;\
                   for (var i = 0; i < 6; i++) { if (i === 3) flip = 'x'; s = s + flip; }\
                   return String(s); }\
                 run();"
            ),
            Ok(Value::String("9xxx".to_owned().into()))
        );
        // A hole and an out-of-range index both leave the fast path.
        assert_eq!(
            eval(
                "function run() { var table = [1, , 3], total = 0;\
                   for (var i = 0; i < 4; i++) { total = total + table[i]; }\
                   return String(total); }\
                 run();"
            ),
            Ok(Value::String("NaN".to_owned().into()))
        );
        // A receiver that stops being an array declines on the next entry.
        assert_eq!(
            eval(
                "function run() { var table = [1, 2, 3], total = 0;\
                   for (var i = 0; i < 3; i++) { total = total + table[i]; }\
                   table = { 0: 10, 1: 20, 2: 30 };\
                   for (var j = 0; j < 3; j++) { total = total + table[j]; }\
                   return total; }\
                 run();"
            ),
            Ok(Value::Number(66.0))
        );
        // A captured counter keeps the observable path, so the closure sees
        // every value the interpreter would produce.
        assert_eq!(
            eval(
                "function run() { var seen = [], s = 0;\
                   for (var i = 0; i < 3; i++) { s = s + i; seen.push(function () { return i; }); }\
                   return s + ':' + seen[0]() + ':' + seen.length; }\
                 run();"
            ),
            Ok(Value::String("3:3:3".to_owned().into()))
        );
    }
    /// A region whose body already changed something observable cannot be
    /// replayed from the loop header, so a guard that fails halfway has to
    /// resume at the exact instruction it stopped on.
    #[test]
    fn typed_loops_resume_where_they_stopped() {
        // The element write happens before the operand that leaves the fast
        // path, so replaying the iteration would double it.
        assert_eq!(
            eval(
                "function run() { var table = [0, 0, 0, 0], step = 2;\
                   for (var i = 0; i < 4; i++) {\
                     table[i] = table[i] + 1;\
                     if (i === 2) step = 'x';\
                     table[i] = table[i] + step;\
                   }\
                   return table.join(','); }\
                 run();"
            ),
            Ok(Value::String("3,3,1x,1x".to_owned().into()))
        );
        // Same for a named property write followed by a non-numeric operand.
        assert_eq!(
            eval(
                "function run() { var point = { total: 0, step: 1 }, log = 0;\
                   for (var i = 0; i < 5; i++) {\
                     point.total = point.total + 1;\
                     if (i === 3) point.step = 'x';\
                     log = log + 1;\
                     point.total = point.total + point.step;\
                   }\
                   return String(point.total) + ':' + log; }\
                 run();"
            ),
            Ok(Value::String("7x1x:5".to_owned().into()))
        );
    }

    /// The shapes the bytecode compiler fuses into single instructions — the
    /// counted loop's comparison and its increment, an assignment that also
    /// feeds the completion temporaries — are what ordinary loops are made of.
    #[test]
    fn typed_loops_accept_fused_loop_shapes() {
        assert_eq!(
            eval(
                "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += 2; } return s; }\
                 run(40);"
            ),
            Ok(Value::Number(80.0))
        );
        // `continue` jumps backwards into the region and the body leaves the
        // completion bookkeeping behind on the operand stack.
        assert_eq!(
            eval(
                "function run(n) { var s = 0, i = 0;\
                   while (i < n) { i++; if (i % 3 === 0) { continue; } s += i; }\
                   return s; }\
                 run(30);"
            ),
            Ok(Value::Number(300.0))
        );
        // A counted loop that runs backwards, so the fused increment is absent
        // and the update is the ordinary shape.
        assert_eq!(
            eval(
                "function run(n) { var s = 1; for (var i = n; i > 0; i--) { s = s * 1.5 - 0.5; }\
                   return s.toFixed(4); }\
                 run(20);"
            ),
            Ok(Value::String("1.0000".to_owned().into()))
        );
    }

    /// Control flow inside a region: both arms of a branch and a nested loop's
    /// own backedge have to agree with the interpreter.
    #[test]
    fn typed_loops_handle_branches_and_nesting() {
        // The two arms of the conditional leave a different value on the stack
        // at the join.
        assert_eq!(
            eval(
                "function run(n) { var s = 0;\
                   for (var i = 0; i < n; i++) { s += (i % 3 === 0 ? i * 2 : i - 1); }\
                   return s; }\
                 run(30);"
            ),
            Ok(Value::Number(550.0))
        );
        // A nested loop is one region with a backward jump inside it.
        assert_eq!(
            eval(
                "function run(n) { var s = 0;\
                   for (var i = 0; i < n; i++) { for (var j = 0; j < i; j++) { s += j; } }\
                   return s; }\
                 run(20);"
            ),
            Ok(Value::Number(1140.0))
        );
        // A `break` out of the inner loop leaves the region at a point the
        // interpreter has to be able to continue from.
        assert_eq!(
            eval(
                "function run(n) { var s = 0;\
                   for (var i = 0; i < n; i++) {\
                     for (var j = 0; j < 8; j++) { if (j > i) break; s += 1; }\
                   }\
                   return s; }\
                 run(12);"
            ),
            Ok(Value::Number(68.0))
        );
    }

    /// Property reads, `Math` intrinsics, and constants no register file can
    /// hold, all inside admitted regions.
    #[test]
    fn typed_loops_read_properties_and_call_math() {
        // An own field, an inherited method's value, and `Math.sqrt`.
        assert_eq!(
            eval(
                "function Point(x) { this.x = x; }\
                 Point.prototype.bias = 3;\
                 function run(n) { var p = new Point(4), s = 0;\
                   for (var i = 0; i < n; i++) { s += Math.sqrt(p.x) + p.bias; }\
                   return s; }\
                 run(10);"
            ),
            Ok(Value::Number(50.0))
        );
        // A detached numeric intrinsic is an ordinary `Call`, not a
        // receiver-preserving `CallResolved`. It must nevertheless use the
        // same guarded native operation once the frame-local callee is known.
        let unbound_source = "function run(n) { var floor = Math.floor, s = 0; for (var i = 0; i < n; i++) { s = s + floor(i / 2); } return s; }";
        assert_eq!(
            super::compile_all(&nested_function(unbound_source)).len(),
            1,
            "{unbound_source}"
        );
        assert_eq!(
            eval(&format!("{unbound_source} run(8);")),
            Ok(Value::Number(12.0))
        );
        // A generic Function local now compiles: the region records a helper
        // site and loop entry tries to flatten whatever the slot holds. This
        // callee writes a global, so nothing can be flattened and the program
        // declines *before* the loop runs -- one failed preparation per frame,
        // not a deoptimization per iteration. The counter is what proves the
        // interpreter still performed every call.
        let fallback_source = "var typedLoopCallCount = 0; function run(n, callbackValue) { var numeric = callbackValue, total = 0; for (var i = 0; i < n; i++) { total = total + numeric(i); } return total + ':' + typedLoopCallCount; } function callback(value) { typedLoopCallCount++; return value; }";
        assert_eq!(
            super::compile_all(&nested_function(fallback_source)).len(),
            1,
            "{fallback_source}"
        );
        let overwritten_alias_source = "function run(n, callbackValue) { var numeric = Math.floor; numeric = callbackValue; var total = 0; for (var i = 0; i < n; i++) { total = total + numeric(i); } return total; }";
        assert_eq!(
            super::compile_all(&nested_function(overwritten_alias_source)).len(),
            1,
            "{overwritten_alias_source}"
        );
        assert_eq!(
            eval(&format!("{fallback_source} run(4, callback);")),
            Ok(Value::String("6:4".to_owned().into()))
        );
        assert_eq!(
            eval(&format!(
                "{overwritten_alias_source} run(6, function (v) {{ return v + 1; }});"
            )),
            Ok(Value::Number(21.0))
        );
        // A string constant inside the region is held boxed and still compares
        // the way the interpreter does.
        assert_eq!(
            eval(
                "function run(n) { var s = 0;\
                   for (var i = 0; i < n; i++) { s += (typeof i === 'number' ? 1 : 0); }\
                   return s; }\
                 run(12);"
            ),
            Ok(Value::Number(12.0))
        );
        // An element that is an object, read through a frame-slot array: the
        // receiver has to survive into the operand stack if the read leaves the
        // fast path, so it cannot live in the unboxed register file.
        assert_eq!(
            eval(
                "function run(n) { var rows = [{ x: 5 }, { x: 6 }], s = 0;\
                   for (var i = 0; i < n; i++) { s += rows[i % 2].x; }\
                   return s; }\
                 run(9);"
            ),
            Ok(Value::Number(49.0))
        );
        // Writing a field the region also reads keeps the object's own value.
        assert_eq!(
            eval(
                "function run(n) { var box = { total: 0 };\
                   for (var i = 0; i < n; i++) { box.total = box.total + i; }\
                   return box.total; }\
                 run(15);"
            ),
            Ok(Value::Number(105.0))
        );
    }

    /// A loop whose body calls a function stays in the register program when
    /// the callee is one a closed-form leaf evaluator can answer. The call was
    /// already frameless before this; what changes is that its receiver,
    /// callee, arguments, and result no longer round-trip through the operand
    /// stack, which is what lets the surrounding loop run natively at all.
    #[test]
    fn closed_form_leaf_calls_keep_their_loop_in_registers() {
        // Compiling is not enough to assert: a region whose call it cannot
        // execute still produces a program, and then deoptimizes at the call on
        // its first iteration. What has to hold is that the program contains
        // the operation that answers the call in registers.
        fn calls_in_registers(source: &str) -> bool {
            super::compile_all(&nested_function(source))
                .iter()
                .any(|program| {
                    program
                        .ops
                        .iter()
                        .any(|op| matches!(op, super::TypedOp::CallClosedFormLeaf { .. }))
                })
        }

        assert!(
            calls_in_registers(
                "function run(n, pool) { var t = 0;\
                   for (var i = 0; i < n; i++) { t += pool[i & 63].advance(i); }\
                   return t; }"
            ),
            "a prototype-dispatched method call should run in registers"
        );
        assert!(
            calls_in_registers(
                "function run(n, cs) { var t = 0;\
                   for (var i = 0; i < n; i++) { t += cs[i & 3](i); }\
                   return t; }"
            ),
            "a computed-callee call should run in registers"
        );
        // A hoisted `Math` receiver keeps the unboxed intrinsic operation
        // instead, so its result feeds arithmetic without a boxing round trip.
        assert!(
            !calls_in_registers(
                "function run(n) { var t = 0;\
                   for (var i = 0; i < n; i++) { t += Math.sqrt(i); }\
                   return t; }"
            ),
            "a Math call should stay on the intrinsic operation"
        );
    }

    /// The call operation runs user code, so each way its assumptions can stop
    /// holding has to hand the loop back with the same answer the interpreter
    /// gives. Every expected value here was cross-checked against QuickJS-NG.
    #[test]
    fn closed_form_leaf_calls_match_interpreted_results() {
        // `this` is the element, not the pool the element came from.
        assert_eq!(
            eval(
                "function Stepper(s) { this.step = s; }\
                 Stepper.prototype.advance = function (v) { return v + this.step; };\
                 function run(n) { var pool = [new Stepper(1), new Stepper(10)], t = 0;\
                   for (var i = 0; i < n; i++) { t += pool[i % 2].advance(i); }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(100.0))
        );
        // A computed callee takes its container as the receiver.
        assert_eq!(
            eval(
                "function run(n) { var a = [function (v) { return v + (this.length || 0); }], t = 0;\
                   for (var i = 0; i < n; i++) { t += a[0](i); }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(55.0))
        );
        // The callee is replaced mid-loop by a body the evaluators decline.
        assert_eq!(
            eval(
                "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
                   for (var i = 0; i < n; i++) {\
                     if (i === 4) { o.f = function (v) { return v + this.k; }; o.k = 100; }\
                     t += o.f(i);\
                   }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(649.0))
        );
        // ... by a native, ...
        assert_eq!(
            eval(
                "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
                   for (var i = 0; i < n; i++) { if (i === 4) { o.f = Math.abs; } t += o.f(-i); }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(37.0))
        );
        // ... and by something not callable at all.
        assert_eq!(
            eval(
                "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
                   for (var i = 0; i < n; i++) { if (i === 4) { o.f = null; }\
                     try { t += o.f(i); } catch (e) { t += 1000; } }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(6010.0))
        );
        // A leaf body that throws must throw, not be answered in registers.
        assert_eq!(
            eval(
                "function run(n) { var o = { f: function (v) { return v.q; } }, t = 0;\
                   for (var i = 0; i < n; i++) {\
                     try { t += o.f(i === 5 ? null : { q: 1 }); } catch (e) { t += 500; }\
                   }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(509.0))
        );
        // A class constructor called without `new` must throw rather than be
        // answered from registers.
        assert_eq!(
            eval(
                "class C { constructor(v) { this.v = v; } }\
                 function run(n) { var t = 0;\
                   for (var i = 0; i < n; i++) { try { t += C(i); } catch (e) { t += 7; } }\
                   return t; }\
                 run(6);"
            ),
            Ok(Value::Number(42.0))
        );
        // An accessor that answers with a closed-form function still works.
        assert_eq!(
            eval(
                "function run(n) { var o = {}, g = function (v) { return v + 5; };\
                   Object.defineProperty(o, 'f', { get: function () { return g; } });\
                   var t = 0; for (var i = 0; i < n; i++) { t += o.f(i); }\
                   return t; }\
                 run(10);"
            ),
            Ok(Value::Number(95.0))
        );
        // A hoisted `Math` receiver keeps the unboxed intrinsic operation.
        assert_eq!(
            eval(
                "function run(n) { var t = 0;\
                   for (var i = 0; i < n; i++) { t += Math.sqrt(i) + Math.min(i, 3); }\
                   return t; }\
                 run(10).toFixed(6);"
            ),
            Ok(Value::String("43.306001".to_owned().into()))
        );
    }

    /// A region is admitted on what it can execute, not on how much of its work
    /// is boxed. Each source here crossed the old "more than a third of the
    /// operations are boxed" rejection and so declined on every backedge, which
    /// left its arithmetic, induction variable, and branches on the generic
    /// dispatcher too.
    #[test]
    fn property_heavy_regions_are_admitted() {
        // An object held in a local across the property reads. One element read
        // plus one move plus one named read is already past the old ratio.
        let via_local = nested_function(
            "function run(n, pool) { var total = 0;\
               for (var i = 0; i < n; i++) { var row = pool[i & 63]; total += row.step; }\
               return total; }",
        );
        assert!(
            !super::compile_all(&via_local).is_empty(),
            "a region that holds an element in a local should be admitted"
        );

        // Two element reads in one iteration, without any local.
        let two_reads = nested_function(
            "function run(n, pool) { var total = 0;\
               for (var i = 0; i < n; i++) { total += pool[i & 63].step + pool[i & 63].carry; }\
               return total; }",
        );
        assert!(
            !super::compile_all(&two_reads).is_empty(),
            "a region with two element reads should be admitted"
        );

        // The generic-path sentinel's shape: an element read into a local, then
        // three named reads off it.
        let three_fields = nested_function(
            "function run(n, pool) { var total = 0;\
               for (var i = 0; i < n; i++) {\
                 var row = pool[i & 63];\
                 total += i + row.step + row.carry + row.rest;\
               }\
               return total; }",
        );
        assert!(
            !super::compile_all(&three_fields).is_empty(),
            "a region reading three fields per iteration should be admitted"
        );
    }

    /// Admission is a performance decision, so each newly admitted shape has to
    /// answer exactly what the interpreter answers -- including when the
    /// assumptions its operations guard stop holding partway through the loop.
    #[test]
    fn admitted_property_heavy_regions_match_interpreted_results() {
        // Receivers deliberately built with three different storage layouts, so
        // no single cached slot covers the site.
        assert_eq!(
            eval(
                "function run(n) { var pool = [];\
                   for (var k = 0; k < 3; k++) {\
                     if (k === 0) pool.push({ step: 1, carry: 2, rest: 3, left: k });\
                     else if (k === 1) pool.push({ left: k, carry: 2, step: 1, rest: 3 });\
                     else pool.push({ left: k, rest: 3, extra: k, carry: 2, step: 1 });\
                   }\
                   var total = 0;\
                   for (var i = 0; i < n; i++) {\
                     var row = pool[i % 3];\
                     total += i + row.step + row.carry + row.rest;\
                   }\
                   return total; }\
                 run(30);"
            ),
            Ok(Value::Number(615.0))
        );
        // A getter installed before the loop: the named read has to leave the
        // register program and resume at the exact instruction it stopped on.
        assert_eq!(
            eval(
                "function run(n) { var row = { carry: 10 };\
                   Object.defineProperty(row, 'step', { get: function () { return 2; } });\
                   var pool = [row], total = 0;\
                   for (var i = 0; i < n; i++) { var r = pool[0]; total += r.step + r.carry; }\
                   return total; }\
                 run(7);"
            ),
            Ok(Value::Number(84.0))
        );
        // The receiver's shape changes partway through, so the operation's
        // remembered storage slot stops being valid mid-loop.
        assert_eq!(
            eval(
                "function run(n) { var pool = [{ step: 1, carry: 2 }, { step: 3, carry: 4 }];\
                   var total = 0;\
                   for (var i = 0; i < n; i++) {\
                     var row = pool[i % 2];\
                     total += row.step + row.carry;\
                     if (i === 3) { pool[0] = { carry: 20, step: 10 }; }\
                   }\
                   return total; }\
                 run(8);"
            ),
            Ok(Value::Number(94.0))
        );
        // An element that is not an object at all: the read has to deoptimize
        // rather than answer from the boxed register file.
        assert_eq!(
            eval(
                "function run(n) { var pool = [{ step: 1, carry: 2 }, { step: 3, carry: 4 }];\
                   var total = 0;\
                   for (var i = 0; i < n; i++) {\
                     if (i === 4) { pool[0] = 'text'; }\
                     var row = pool[i % 2];\
                     total += row.step === undefined ? 100 : row.step + row.carry;\
                   }\
                   return total; }\
                 run(8);"
            ),
            Ok(Value::Number(234.0))
        );
    }

    /// The search loop is the shape both `Leave` and `BoxedEquality` exist for:
    /// it leaves through its result rather than its header test, and it compares
    /// by identity. Before either operation the whole region declined.
    const SEARCH: &str = "function run(list, target) {\
           for (var i = 0; i < list.length; i++) {\
             if (list[i].pos == target) { return i; }\
           }\
           return -1; }";

    #[test]
    fn a_search_loop_compiles_to_an_identity_test_and_an_early_leave() {
        let bytecode = nested_function(SEARCH);
        let programs = super::compile_all(&bytecode);
        let program = programs.first().expect("search loop should be admitted");
        assert!(
            program
                .ops
                .iter()
                .any(|op| matches!(op, super::TypedOp::BoxedEquality { .. })),
            "{:#?}",
            program.ops
        );
        assert!(
            program
                .ops
                .iter()
                .any(|op| matches!(op, super::TypedOp::Leave { .. })),
            "{:#?}",
            program.ops
        );
    }

    #[test]
    fn a_search_loop_returns_the_interpreted_answer() {
        let script = format!(
            "{SEARCH} var a = {{}}; var b = {{}}; \
             run([{{ pos: a }}, {{ pos: b }}, {{ pos: a }}], b);"
        );
        assert_eq!(eval(&script), Ok(Value::Number(1.0)));
        let missing = format!("{SEARCH} run([{{ pos: {{}} }}], {{}});");
        assert_eq!(eval(&missing), Ok(Value::Number(-1.0)));
    }

    #[test]
    fn a_comparison_that_would_coerce_still_runs_its_hook() {
        // `number == object` is `ToPrimitive` on the object, which is user code.
        // The tier must hand the instruction back rather than answer it, so the
        // `valueOf` hook has to run once per iteration.
        // Every name the loop body touches is a local or a parameter, so the
        // region really is admitted and the assertion really does exercise the
        // guard; the only global write lives inside the hook.
        let script = "function run(list, probe) { var hits = 0;\
               for (var i = 0; i < list.length; i++) {\
                 if (list[i].pos == probe) { hits = hits + 1; } }\
               return hits; }\
             var calls = 0;\
             var probe = { valueOf: function () { calls = calls + 1; return 2; } };\
             run([{ pos: 1 }, { pos: 2 }, { pos: 3 }, { pos: 2 }], probe) * 100 + calls;";
        assert_eq!(eval(script), Ok(Value::Number(204.0)));
    }

    /// The helper graph exists for this shape: a numeric loop whose body calls
    /// ordinary functions, which used to abort the whole region. This is the
    /// `imaging-darkroom` pixel loop reduced to its call structure -- three
    /// levels of ordinary call, an intrinsic, a captured number and a captured
    /// function.
    const DARKROOM_HELPERS: &str = "function FastLog2(x) { return Math.log(x) / Math.LN2; }\
         var LOG2_HALF = FastLog2(0.5);\
         function FastBias(b, x) { return Math.pow(x, FastLog2(b) / LOG2_HALF); }\
         function FastGain(g, x) { return (x < 0.5)\
             ? FastBias(1.0 - g, 2.0 * x) * 0.5\
             : 1.0 - FastBias(1.0 - g, 2.0 - 2.0 * x) * 0.5; }\
         function Clamp(x) { return (x < 0.0) ? 0.0 : ((x > 1.0) ? 1.0 : x); }\
         function pixel(x, contrast) { return FastGain(contrast, Clamp(x)); }\
         function run(data, n, contrast) { var total = 0;\
           for (var i = 0; i < n; i++) { total = total + pixel(data[i], contrast); }\
           return total; }";

    fn named_function(source: &str, name: &str) -> crate::bytecode::Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = crate::bytecode::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                super::super::ir::Op::NewFunction {
                    name: actual,
                    bytecode,
                    ..
                } if actual.as_deref() == Some(name) => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("named function should be nested in the script")
    }

    #[test]
    fn a_nested_helper_graph_answers_exactly_as_the_interpreter_does() {
        // The loop is admitted and runs its calls through the flattened graph;
        // the straight-line sum has no loop at all, so it can only be
        // interpreted. The two must agree bit for bit.
        assert_eq!(
            super::compile_all(&named_function(DARKROOM_HELPERS, "run")).len(),
            1
        );
        let data = "[-0.5, 0.25, 0.75, 1.5, 0.5]";
        let looped = format!("{DARKROOM_HELPERS} run({data}, 5, 0.4);");
        let straight = format!(
            "{DARKROOM_HELPERS} var d = {data};\
             pixel(d[0], 0.4) + pixel(d[1], 0.4) + pixel(d[2], 0.4)\
               + pixel(d[3], 0.4) + pixel(d[4], 0.4);"
        );
        let Ok(Value::Number(interpreted)) = eval(&straight) else {
            panic!("straight-line sum should evaluate to a number");
        };
        assert!(interpreted.is_finite(), "{interpreted}");
        assert_eq!(eval(&looped), Ok(Value::Number(interpreted)));
    }

    #[test]
    fn a_flattened_helper_reproduces_its_own_branches() {
        let script = "function Clamp(x) { return (x < 0.0) ? 0.0 : ((x > 1.0) ? 1.0 : x); }\
             function run(data, n) { var total = 0;\
               for (var i = 0; i < n; i++) { total = total + Clamp(data[i]); }\
               return total; }\
             run([-1, 0.25, 2, 0.5], 4) * 100;";
        assert_eq!(eval(script), Ok(Value::Number(175.0)));
    }

    #[test]
    fn a_helper_replaced_mid_loop_stops_using_the_prepared_body() {
        // The prepared body is `twice`; the identity check at every call is
        // what makes the switch to `thrice` produce the interpreted answer
        // instead of the stale one.
        let script = "function twice(x) { return x * 2; }\
             function thrice(x) { return x * 3; }\
             function run(a, b, n) { var f = a, total = 0;\
               for (var i = 0; i < n; i++) { total = total + f(i); if (i === 1) { f = b; } }\
               return total; }\
             run(twice, thrice, 4);";
        assert_eq!(eval(script), Ok(Value::Number(17.0)));
    }

    #[test]
    fn a_helper_that_is_not_pure_arithmetic_declines_without_losing_its_effect() {
        // The body writes a global, which no flattened graph can express, so
        // preparation fails and the loop stays interpreted. The counter proves
        // every call still ran.
        let script = "var calls = 0;\
             function step(x) { calls = calls + 1; return x + 1; }\
             function run(n) { var total = 0;\
               for (var i = 0; i < n; i++) { total = total + step(i); }\
               return total; }\
             run(5) * 100 + calls;";
        assert_eq!(eval(script), Ok(Value::Number(1505.0)));
    }

    #[test]
    fn strict_equality_over_boxed_operands_matches_the_interpreter() {
        let script = "function run(list, needle) { var hits = 0;\
               for (var i = 0; i < list.length; i++) {\
                 if (list[i] === needle) { hits = hits + 1; } }\
               return hits; }\
             run(['a', 'b', 'a', 'c'], 'a') * 10 + run([1, 2, 1], '1');";
        assert_eq!(eval(script), Ok(Value::Number(20.0)));
    }
}

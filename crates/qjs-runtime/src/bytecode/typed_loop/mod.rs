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
//! Admission is conservative and checked three times. At compile time the
//! region's opcodes must all be in the whitelist, its stack behaviour must be
//! statically consistent, and its operations must be mostly scalar: where the
//! work is the property protocol instead, the interpreter's own inline caches
//! are already as good, and running such a region natively measured neutral to
//! slower. At entry every slot the program reads must hold a representable
//! value, every slot it writes must be an authoritative frame slot, and every
//! receiver must be a dense array. Mid-run, each operation that cannot be
//! completed without observable behaviour hands the loop back.
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

mod compile;
mod execute;

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
    /// Leaves the loop: the condition value goes back on the operand stack,
    /// because the instruction at the loop's exit pops it.
    Exit {
        cond: u16,
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
}

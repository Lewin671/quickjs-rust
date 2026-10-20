"""Semantic engine roles shared by layout selection and audit."""
from typing import Mapping

# The engine crate as it appears in a decoded symbol; `audit` uses it to tell
# order-file entries naming this engine from standard-library ones.
RUNTIME_CRATE = "qjs_runtime"

_BYTECODE = f"{RUNTIME_CRATE}::bytecode"
_EXECUTE = f"{_BYTECODE}::typed_loop::execute"
_WIDE_FRAME = f"{_BYTECODE}::compact_fn::wide::loop_frame::WideLoopFrame"
_VM = f"{_BYTECODE}::vm::Vm"

# Historical wide dispatch spellings: older compilers inline execute into
# run's closure or into try_run_typed_loop. An explicit execute declaration
# is preferred below; a closure nested inside execute is only a helper and
# cannot replace that declaration as the dispatch target.
WIDE_EXECUTOR = f"{_EXECUTE}::run::<{_WIDE_FRAME}>"
WIDE_EXECUTOR_FALLBACK = f"{_EXECUTE}::try_run_typed_loop::<{_WIDE_FRAME}>"
# A deliberately outlined dispatch has its own declaration. Its deopt
# closures are helpers, whereas legacy run closures can own the whole loop.
WIDE_DISPATCH = f"{_EXECUTE}::execute::<{_WIDE_FRAME}>"
# The same dispatch loop instantiated for the interpreter's own top-level
# loops, pinned ahead of its wide twin.
INTERPRETER_EXECUTOR = f"{_EXECUTE}::run::<{_VM}>"
INTERPRETER_EXECUTOR_FALLBACK = f"{_EXECUTE}::try_run_typed_loop::<{_VM}>"
INTERPRETER_DISPATCH = f"{_EXECUTE}::execute::<{_VM}>"
DIRECT_EXECUTOR_TARGETS = frozenset((WIDE_DISPATCH, INTERPRETER_DISPATCH))

# The executor's hot callees, in their pinned order. Each owns a fixed-size
# slot so that a callee growing does not move the ones after it. The first
# entry is the executor's fallback spelling: whichever of the two the compiler
# used for the loop, the other one -- when the binary has it -- is a callee.
CALLEES = (
    WIDE_EXECUTOR_FALLBACK,
    f"{_EXECUTE}::get_named_object",
    f"<{RUNTIME_CRATE}::value::array::ArrayRef>::direct_dense_index_value",
    f"{_EXECUTE}::call_leaf",
    f"{_BYTECODE}::vm_numeric_leaf::try_eval_numeric_leaf",
    f"<{RUNTIME_CRATE}::value::array::ArrayData>::has_property_at_index",
    f"{_BYTECODE}::vm_numeric_leaf::direct_number_binary",
    f"{_EXECUTE}::ordinary_data_property",
    f"<{RUNTIME_CRATE}::value::object::ObjectRef>::own_data_property_read",
    f"<{RUNTIME_CRATE}::value::object::ObjectRef>::write_existing_own_data_property",
    f"{_EXECUTE}::boxed_truthiness",
    f"{_EXECUTE}::boxed_equality",
    f"{_EXECUTE}::get_named",
    # Numeric call-tree and helper-graph drivers have independent slots.
    f"{_BYTECODE}::compact_fn::numeric_plan::run",
    f"<{_BYTECODE}::typed_loop::helper_graph::numeric::NumProgram>::run",
    f"<{_BYTECODE}::named_property_cache::NamedPropertyCache>::probe",
    f"{_BYTECODE}::compact_fn::wide::activation::run",
    f"<{RUNTIME_CRATE}::value::object::ObjectRef>::shared_data_slot_number",
    f"<{RUNTIME_CRATE}::value::object::ObjectRef>::shared_data_slot_write_number",
)
# A callee's own alignment is independent of the executor's current window.
CALLEE_OFFSETS = {f"{_EXECUTE}::call_leaf": 0x580}

# When the linker emits more than one body of a pinned function -- a duplicate
# it suffixed, or a second `{closure}` -- no symbol table says which body a
# caller reaches, so nothing here picks one: the target is reported as
# ambiguous and left out of the head. Name the exact spelling to pin here when
# one of them is the body that runs, which is a decision only a measurement
# can make. Empty by default, so the ambiguity is always reported instead of
# resolved by a guess.
SELECTED_SPELLINGS: Mapping[str, str] = {}

# After the budgeted callees: defined once per codegen unit, in a number of
# copies that changes with unrelated code, so nothing pinned may follow them.
TAIL_CALLEES = (
    f"<{RUNTIME_CRATE}::value::Value as core::clone::Clone>::clone",
    f"core::ptr::drop_in_place::<{RUNTIME_CRATE}::value::Value>",
)

# One role per row of the head. Each executor role accepts an alternative
# spelling of the same loop (preferred one first); every callee and tail
# callee is a slot of its own.
EXECUTOR_ROLES = ("wide executor", "interpreter executor")
ROLE_TARGETS = (
    ("wide executor", (WIDE_DISPATCH, WIDE_EXECUTOR, WIDE_EXECUTOR_FALLBACK)),
    ("interpreter executor", (INTERPRETER_DISPATCH, INTERPRETER_EXECUTOR, INTERPRETER_EXECUTOR_FALLBACK)),
    *((("callee"), (name,)) for name in CALLEES),
    *((("tail callee"), (name,)) for name in TAIL_CALLEES),
)
ROLE_ORDER = EXECUTOR_ROLES + ("callee", "tail callee")


"""Symbols, sizes and fake tool output shared by the layout tool tests.

Every engine symbol literal here is a real spelling: the current ones are
copied from the recorded `nm` of the base revision (job
`20261016T110634Z-property-machine-code-dump`), and the `_OLD` ones from the
committed `hot-functions-x86_64.order`, which an earlier build generated.
Inventing a spelling would test the decoder against itself rather than against
what rustc emits. The standard-library filler names are synthetic: only their
crate, uniqueness and 16-byte-aligned sizes matter to the code under test, and
they are generated with length-correct `v0` identifiers.

The fake binary is one section. `rows` lays the functions out consecutively
from `START`, and `tool_patch` reports `__text` as starting there with a size
that covers every function the fixture defines, so the last function has an
extent like any other. `__mh_execute_header` is emitted *before* the section:
it is not a function of `__text`, and a layout may not place anything by it.
"""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Sequence
from unittest.mock import patch

from tools.benchmark import layout_pin

START = 0x100000000
# The Mach-O header symbol sits at the image base, before `__text`.
IMAGE_BASE = START - 0x1000

# The wide tier's dispatch loop, as a closure around `run<WideLoopFrame>`: the
# spelling the sampled profiles name at the base revision.
WIDE_CLOSURE = ("__RNCINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute3run"
                "NtNtNtNtB8_10compact_fn4wide10loop_frame13WideLoopFrameEs_0Ba_")
# The interpreter's own instantiation of the same loop.
INTERPRETER_CLOSURE = ("__RNCINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute"
                       "3runNtNtB8_2vm2VmEs_0Ba_")
INTERPRETER_FALLBACK = ("__RINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute"
                        "18try_run_typed_loopNtNtB6_2vm2VmEB8_")
# Not an executor: `Vm`'s own activation loop, which no target may conflate
# with the typed-loop instantiations above.
VM_RUN_CURRENT_ACTIVATION = ("__RNvMs5_NtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode2vm"
                             "NtB5_2Vm22run_current_activation")
CALL_LEAF = "__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute9call_leaf"
GET_NAMED_OBJECT = ("__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute"
                    "16get_named_object")
GET_NAMED = "__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute9get_named"
BOXED_TRUTHINESS = ("__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute"
                    "16boxed_truthiness")
ORDINARY_DATA_PROPERTY = ("__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute"
                          "22ordinary_data_property")
TRY_EVAL_NUMERIC_LEAF = ("__RNvNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode15vm_numeric_leaf"
                         "21try_eval_numeric_leaf")
DIRECT_NUMBER_BINARY = ("__RNvNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode15vm_numeric_leaf"
                        "20direct_number_binary")
HAS_PROPERTY_AT_INDEX = ("__RNvMs3_NtNtCs93CNxA9ZQ5c_11qjs_runtime5value5arrayNtB5_9ArrayData"
                         "21has_property_at_index")
OWN_DATA_PROPERTY_READ = ("__RNvMNtNtNtCs93CNxA9ZQ5c_11qjs_runtime5value6object10slot_reads"
                          "NtB4_9ObjectRef22own_data_property_read")
WRITE_EXISTING_OWN_DATA_PROPERTY = ("__RNvMs6_NtNtCs93CNxA9ZQ5c_11qjs_runtime5value6object"
                                    "NtB5_9ObjectRef32write_existing_own_data_property")
NUMERIC_PLAN_RUN = ("__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10compact_fn12numeric_plan"
                    "3run")
NUM_PROGRAM_RUN = ("__RNvMNtNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop12helper_graph"
                   "7numericNtB2_10NumProgram3run")
DIRECT_DENSE_INDEX_VALUE = ("__RNvMs4_NtNtCs93CNxA9ZQ5c_11qjs_runtime5value5arrayNtB5_8ArrayRef"
                            "24direct_dense_index_value")
NAMED_PROPERTY_CACHE_PROBE = ("__RNvMNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode20named_property_cache"
                              "NtB2_18NamedPropertyCache5probe")
DROP_IN_PLACE = ("__RINvNtCs3jiI88Gf6x3_4core3ptr13drop_in_placeNtNtCs93CNxA9ZQ5c_11qjs_runtime"
                 "5value5ValueEBK_")
# The linker appends this suffix to a duplicate of the same function, at an
# address of its own; nothing in a symbol table says which body runs.
DROP_IN_PLACE_DUPLICATE = DROP_IN_PLACE + ".3946"
CLONE = ("__RNvXs1_NtCs93CNxA9ZQ5c_11qjs_runtime5valueNtB5_5Value"
         "NtNtCs3jiI88Gf6x3_4core5clone5Clone5clone")

# The same functions as an earlier build spelled them: another crate
# disambiguator, another closure index, another set of backreference offsets.
WIDE_PLAIN_OLD = ("__RINvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute3run"
                  "NtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_")
WIDE_FALLBACK_OLD = ("__RINvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute"
                     "18try_run_typed_loopNtNtNtNtB6_10compact_fn4wide10loop_frame"
                     "13WideLoopFrameEB8_")
INTERPRETER_PLAIN_OLD = ("__RINvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute3run"
                         "NtNtB6_2vm2VmEB8_")
CALL_LEAF_OLD = "__RNvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute9call_leaf"
# A pinned callee the current engine does not have under any spelling: it was
# inlined away, so a rewrite drops its line and reports it.
BOXED_EQUALITY_OLD = ("__RNvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute"
                      "14boxed_equality")
# A stale engine name that is not a target of any kind: a rewrite cannot tell
# where it belongs, so it has to refuse rather than drop it silently.
REMOVED_BY_LTO_OLD = ("__RNvNtNtNtCscvRFTxyzId2_11qjs_runtime8bytecode10typed_loop7execute"
                      "14removed_by_lto")


def _std_name(index: int) -> str:
    name = f"f{index}"
    return f"__RNvNtCsg55jX0GwzBC_3std2io{len(name)}{name}"


def _std_name_of(name: str) -> str:
    return f"__RNvNtCsg55jX0GwzBC_3std2io{len(name)}{name}"


# Standard-library filler candidates: concrete functions in `std`, emitted
# once, 16-byte aligned. A page-sized one, a few large ones and many
# independent 16-byte ones, so a subset solver can reach every offset and slot
# padding a test asks for without reusing a spelling: a pin needs several
# kilobytes of them between the executor's slot and its callees' slots.
STD = (
    (_std_name_of("page_worth"), 0x1000),
    (_std_name_of("half_page"), 0x800),
    (_std_name_of("quarter_page"), 0x400),
    (_std_name_of("eighth_page"), 0x200),
    (_std_name_of("sixteenth_page"), 0x100),
    *((_std_name(index), 0x10) for index in range(1200)),
)

# The Mach-O header is a `T` symbol before the section; it is not a function of
# `__text`, and the distance to `_main` is not a size to lay anything out by.
HEADER_NAME = "__mh_execute_header"
MAIN_NAME = "_main"
MAIN = (MAIN_NAME, 0x200)
EH_PERSONALITY = ("_rust_eh_personality", 0x40)

# Everything the engine fixture defines, in section order. `boxed_equality` is
# deliberately absent: it is a pinned callee the current engine does not have.
# The executors are sized so their offsets modulo 4 KiB differ, as they do in a
# real link, and `_main` (whose size no assertion needs) comes last.
ENGINE = (
    *STD,
    (WIDE_CLOSURE, 0x2100),
    (INTERPRETER_CLOSURE, 0x1000),
    (INTERPRETER_FALLBACK, 0x400),
    (CALL_LEAF, 0x300),
    (GET_NAMED_OBJECT, 0x200),
    (GET_NAMED, 0x180),
    (BOXED_TRUTHINESS, 0x100),
    (ORDINARY_DATA_PROPERTY, 0x400),
    (TRY_EVAL_NUMERIC_LEAF, 0x1900),
    (DIRECT_NUMBER_BINARY, 0xD00),
    (HAS_PROPERTY_AT_INDEX, 0x240),
    (OWN_DATA_PROPERTY_READ, 0x8C0),
    (WRITE_EXISTING_OWN_DATA_PROPERTY, 0xA20),
    (NUMERIC_PLAN_RUN, 0x2500),
    (NUM_PROGRAM_RUN, 0x2300),
    (DIRECT_DENSE_INDEX_VALUE, 0x120),
    (NAMED_PROPERTY_CACHE_PROBE, 0x1430),
    (DROP_IN_PLACE, 0xF0),
    (DROP_IN_PLACE_DUPLICATE, 0xF0),
    (CLONE, 0x100),
    (VM_RUN_CURRENT_ACTIVATION, 0x100),
    (EH_PERSONALITY),
    MAIN,
)
# The order file an earlier pin produced: the same engine names in an older
# build's spelling, plus one pinned callee the binary no longer has.
ENGINE_ORDER = (WIDE_PLAIN_OLD, INTERPRETER_PLAIN_OLD, CALL_LEAF_OLD, WIDE_FALLBACK_OLD,
                BOXED_EQUALITY_OLD)

_EXTERNAL_KINDS = {MAIN_NAME: "T", "_rust_eh_personality": "T", HEADER_NAME: "T"}


def rows(entries: Sequence[tuple[str, int]], start: int = START,
         header: bool = True) -> list[tuple[int, str, str]]:
    """(address, nm type, name) rows for a fake binary.

    The functions are laid out consecutively from `start`; the last entry has
    no following address, so its extent comes from the section size. With
    `header`, the Mach-O header symbol is emitted before the section.
    """
    built, address = [], start
    for name, size in entries:
        built.append((address, _EXTERNAL_KINDS.get(name, "t"), name))
        address += size
    if header:
        built.insert(0, (IMAGE_BASE, "T", HEADER_NAME))
    return built


def section_size(entries: Sequence[tuple[str, int]]) -> int:
    """The `__text` size that covers every function `entries` defines."""
    return sum(size for _, size in entries)


def nm_text(built: Sequence[tuple[int, str, str]]) -> str:
    """`nm` output for those rows, with non-text rows that must be ignored."""
    lines = ["                 U _CCRandomGenerateBytes",
             "0000000000000000 b _some_bss",
             "                 U __NSGetArgc"]
    lines += [f"{address:016x} {kind} {name}" for address, kind, name in built]
    return "\n".join(lines) + "\n"


def otool_text(start: int = START, size: int = 0x1000) -> str:
    """`otool -l` output naming the `__text` section's address and size."""
    return ("Load command 1\n"
            "      cmd LC_SEGMENT_64\n"
            "  sectname __text\n"
            "   segname __TEXT\n"
            f"      addr 0x{start:x}\n"
            f"      size 0x{size:x}\n")


def tool_patch(entries: Sequence[tuple[str, int]] = ENGINE, start: int = START):
    """Patches `layout_pin.subprocess.run` with the fake `otool` and `nm`."""
    listing = nm_text(rows(entries, start))
    section = otool_text(start, section_size(entries))

    def run(argv, **_kwargs):
        if argv[0] == "otool":
            return subprocess.CompletedProcess(argv, 0, stdout=section, stderr="")
        if argv[0] == "nm":
            return subprocess.CompletedProcess(argv, 0, stdout=listing, stderr="")
        raise AssertionError(f"unexpected command {argv}")

    return patch("tools.benchmark.layout_pin.subprocess.run", side_effect=run)


def read_symbols(entries: Sequence[tuple[str, int]] = ENGINE,
                 start: int = START) -> layout_pin.TextSymbols:
    """The fixture binary's `__text` symbols, read through the real parser."""
    with tool_patch(entries, start):
        return layout_pin.text_symbols(Path("qjs"))


def write_order(path: Path, lines: Sequence[str], header: Sequence[str] = ()) -> Path:
    """An order file whose body is `lines`, after any `header` comment lines."""
    comments = list(header) or ["# Hot functions first, hottest first. Generated by "
                                "`python3 -m tools.benchmark.order_file`; regenerate after "
                                "large changes."]
    path.write_text("".join(f"{line}\n" for line in comments)
                    + "".join(f"{line}\n" for line in lines), encoding="utf-8")
    return path

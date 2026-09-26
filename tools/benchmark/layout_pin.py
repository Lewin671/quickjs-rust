"""Pin the typed-loop executor's address in the hot-code layout.

The order file (`order_file.py`) keeps hot functions in a fixed order, but
their addresses still move whenever a function listed before them changes
size, and one listed near the top -- the VM, the wide driver -- changes with
almost every edit. The typed-loop executor
(`try_run_typed_loop<WideLoopFrame>`) and its callees are the part of the
layout that swings most: with byte-identical code, `capturing_closure_call`
and `ai-astar` run 18-25% more cycles in one placement than in another.
Measured 2026-09-24 by moving only the executor: the fast placements are a
narrow window of its start address modulo 4 KiB, with its callees in the
order `CALLEES` lists right after it; the slow ones are everywhere else. The
window moves when the executor's own code changes, so `DEFAULT_OFFSET` is
re-scanned with it. Some versions of the executor are sensitive at 16-byte
granularity (one in eight offsets fast); others -- the one pinned now -- are
within 2% at every offset. Scan with several typed-loop cases, not two: a
position fast for the call sentinels and ai-astar was 5% slow for
imaging-desaturate.

This rewrites the head of an order file to: standard-library functions whose
total size moves the interpreter's instantiation of the executor
(`run<Vm>`) to its offset, more filler, the wide executor at its offset
and its callees, then the rest of the list unchanged. The filler functions are
precompiled into the standard library, so their sizes do not change with
this repository's code, and nothing listed before the executor does either:
its address is then fixed until the toolchain or the executor itself
changes. Re-scan the offset (`--offset`, with the canaries in
docs/performance-workflow.md) after editing the executor.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ORDER = ROOT / "crates/qjs-cli/hot-functions.order"
DEFAULT_OFFSET = 0x0
# The same executor instantiated for the interpreter's own loops -- a
# script's top-level `for` -- is pinned too, ahead of its wide twin:
# it is most of access-fannkuch and math-partial-sums, and floating in the
# unordered tail it moved with every edit (partial-sums +4.7% from one).
VM_EXECUTOR = re.compile(r"typed_loop7execute3runNtNtB6_2vm2Vm")
DEFAULT_VM_OFFSET = 0xc40
PAGE = 0x1000
ALIGN = 16
# The executor's dispatch loop: `run<WideLoopFrame>` when the compiler keeps
# it out of line (it has since 2026-09-25; the order file's ranking predates
# that and never listed it), otherwise `try_run_typed_loop<WideLoopFrame>`,
# into which it is then inlined. Searched in the binary, first match wins.
EXECUTORS = (
    re.compile(r"typed_loop7execute3run.*WideLoopFrame"),
    re.compile(r"typed_loop7execute18try_run_typed_loop.*WideLoopFrame"),
)
# Suffix patterns of the executor's hot callees, in their pinned order.
CALLEES = (
    "typed_loop7execute18try_run_typed_loopNtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_",
    "typed_loop7execute16get_named_object",
    "ArrayRef24direct_dense_index_value",
    "typed_loop7execute9call_leaf",
    "vm_numeric_leaf21try_eval_numeric_leaf",
    "ArrayData21has_property_at_index",
    "vm_numeric_leaf20direct_number_binary",
    "typed_loop7execute22ordinary_data_property",
    "slot_readsNtB4_9ObjectRef22own_data_property_read",
    "ObjectRef32write_existing_own_data_property",
    "typed_loop7execute16boxed_truthiness",
)
# After the budgeted callees: defined once per codegen unit, in a number of
# copies that changes with unrelated code, so nothing pinned may follow them.
TAIL_CALLEES = (
    "5valueNtB5_5ValueNtNtCsl8K0bEFm1U0_4core5clone5Clone5clone",
    "core3ptr13drop_in_placeNtNtCs9nYd1Hk1rek_11qjs_runtime5value5ValueEBK_",
)
# The executor and each callee above own a fixed-size slot, the rest of it
# standard-library filler, so a function that grows inside its slot moves
# nothing after it. Without slots, get_named_object growing 176 bytes cost
# heterogeneous_property_read 4% and an unpinned property helper moving cost
# string_key_map_churn 22%, both at identical instruction counts. Slot sizes
# persist in the order file (`# budget`) and are only re-derived, with
# SLOT_HEADROOM to spare, for a function that outgrew its slot -- which moves
# everything after it: re-scan then.
SLOT_GRANULE = 0x100
SLOT_HEADROOM = 0x100
_BUDGET = re.compile(r"^# budget (0x[0-9a-f]+) (\S+)$")
_STD = "Csg55jX0GwzBC_3std"


def text_symbols(binary: Path) -> tuple[int, dict[str, int], dict[str, int]]:
    """The `__text` start, and each text symbol's size and definition count."""
    sections = subprocess.run(["otool", "-l", str(binary)], capture_output=True, text=True,
                              check=True).stdout
    match = re.search(r"sectname __text\s+segname __TEXT\s+addr (0x[0-9a-f]+)", sections)
    if not match:
        raise ValueError(f"{binary}: no __TEXT,__text section")
    start = int(match.group(1), 16)
    listing = subprocess.run(["nm", "-n", str(binary)], capture_output=True, text=True,
                             check=True).stdout
    rows = []
    for line in listing.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[1] in {"t", "T"}:
            rows.append((int(parts[0], 16), parts[2]))
    sizes: dict[str, int] = {}
    counts: dict[str, int] = {}
    for (address, name), (following, _) in zip(rows, rows[1:]):
        counts[name] = counts.get(name, 0) + 1
        sizes.setdefault(name, following - address)
    return start, sizes, counts


def filler(sizes: dict[str, int], counts: dict[str, int], length: int,
           exclude: set[str]) -> list[str]:
    """Concrete standard-library functions whose sizes sum to `length`: no
    symbol naming this workspace, whose instantiation could change size."""
    candidates = sorted(
        ((size, name) for name, size in sizes.items()
         if counts.get(name) == 1 and name.startswith("__RNv") and _STD in name
         and "qjs" not in name and size > 0 and size % ALIGN == 0 and name not in exclude),
        reverse=True,
    )
    units = length // ALIGN
    reachable: dict[int, list[str]] = {0: []}
    for size, name in candidates:
        step = size // ALIGN
        for total, picked in list(reachable.items()):
            if total + step <= units and total + step not in reachable:
                reachable[total + step] = picked + [name]
        if units in reachable:
            return reachable[units]
    raise ValueError(f"no standard-library filler sums to {length:#x} bytes")


def pin(lines: list[str], start: int, sizes: dict[str, int], counts: dict[str, int],
        offset: int, vm_offset: int | None = None,
        budgets: dict[str, int] | None = None) -> list[str]:
    """`lines` (symbols, no comments) with the executor pinned at `offset`
    and, given `vm_offset`, its interpreter instantiation at that one."""
    head = pin_head(lines, start, sizes, counts, offset, vm_offset, budgets)
    return head + [line for line in lines if line not in head]


def slot_budget(size: int, previous: int | None) -> int:
    """A pinned function's slot: the recorded one while the function fits."""
    if previous is not None and size <= previous:
        return previous
    return -(-size // SLOT_GRANULE) * SLOT_GRANULE + SLOT_HEADROOM


def pin_head(lines: list[str], start: int, sizes: dict[str, int], counts: dict[str, int],
             offset: int, vm_offset: int | None = None,
             budgets: dict[str, int] | None = None) -> list[str]:
    """The symbols [`pin`] puts ahead of the rest of `lines`. Given
    `budgets` (updated in place), the executor and each budgeted callee are
    followed by filler up to their slot size."""
    executor = None
    for pattern in EXECUTORS:
        executor = next((name for name in sorted(sizes) if pattern.search(name)), None)
        if executor is not None:
            break
    if executor is None:
        raise ValueError("the binary has no typed-loop executor symbol")
    callees = []
    known = sorted(set(lines) | set(sizes))
    for suffix in CALLEES:
        callees += [name for name in known
                    if name.endswith(suffix) and name not in callees and name != executor]
    tail = []
    for suffix in TAIL_CALLEES:
        tail += [name for name in known
                 if name.endswith(suffix) and name not in callees + tail and name != executor]
    listed = set(lines)
    head: list[str] = []
    position = start
    vm_executor = next((name for name in sorted(sizes) if VM_EXECUTOR.search(name)), None)
    if vm_offset is not None and vm_executor is not None and vm_executor != executor:
        # The interpreter's twin goes first, so nothing whose size moves with
        # ordinary edits -- the callees below, some defined once per codegen
        # unit (`Value::clone`) in a number of copies that changes with
        # unrelated code -- is ahead of either executor. Sizes are distances
        # to the next symbol, all 16-byte aligned, so they do not depend on
        # where the current binary put each function.
        head += filler(sizes, counts, (vm_offset - position) % PAGE, listed | set(head))
        position += (vm_offset - position) % PAGE
        head.append(vm_executor)
        position += sizes.get(vm_executor, 0)
    head += filler(sizes, counts, (offset - position) % PAGE, listed | set(head))
    for name in [executor] + callees:
        head.append(name)
        if budgets is None or name not in sizes:
            continue
        budgets[name] = slot_budget(sizes[name], budgets.get(name))
        head += filler(sizes, counts, budgets[name] - sizes[name], listed | set(head))
    return head + tail


_PIN_HEADER = re.compile(r"^# pinned: .* (?:filler|head) (\d+)")


def pin_order_file(order: Path, binary: Path, offset: int = DEFAULT_OFFSET,
                   vm_offset: int | None = DEFAULT_VM_OFFSET) -> None:
    """Rewrites `order` with the executors pinned, replacing the head of any
    previous pin (its length is recorded in the header; everything in it is
    re-derived from the binary)."""
    text = order.read_text(encoding="utf-8").splitlines()
    previous = 0
    for line in text:
        if match := _PIN_HEADER.match(line):
            previous = int(match.group(1))
    budgets = {match.group(2): int(match.group(1), 16)
               for line in text if (match := _BUDGET.match(line))}
    header = [line for line in text if line.startswith("#")
              and not _PIN_HEADER.match(line) and not _BUDGET.match(line)]
    symbols = [line for line in text if line and not line.startswith("#")][previous:]
    start, sizes, counts = text_symbols(binary)
    head = pin_head(symbols, start, sizes, counts, offset, vm_offset, budgets)
    pinned = head + [line for line in symbols if line not in head]
    header += [f"# budget {size:#x} {name}" for name, size in budgets.items()
               if name in head]
    vm = f", interpreter's at {vm_offset:#x}" if vm_offset is not None else ""
    header.append(f"# pinned: typed-loop executor at {offset:#x}{vm} mod 4 KiB "
                  f"(python3 -m tools.benchmark.layout_pin), head {len(head)}")
    order.write_text("\n".join(header + pinned) + "\n", encoding="utf-8")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.layout_pin",
                                     description=__doc__.split("\n\n")[0])
    parser.add_argument("--binary", type=Path, required=True,
                        help="a release qjs built with the current toolchain")
    parser.add_argument("--order", type=Path, default=DEFAULT_ORDER)
    parser.add_argument("--offset", type=lambda text: int(text, 0), default=DEFAULT_OFFSET,
                        help="executor start modulo 4 KiB (default %(default)#x)")
    parser.add_argument("--vm-offset", type=lambda text: int(text, 0), default=DEFAULT_VM_OFFSET,
                        help="interpreter executor start modulo 4 KiB (default %(default)#x)")
    args = parser.parse_args(argv)
    try:
        pin_order_file(args.order, args.binary.resolve(), args.offset, args.vm_offset)
    except (ValueError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"executor pinned at {args.offset:#x} -> {args.order}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

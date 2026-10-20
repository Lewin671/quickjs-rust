"""Select engine layout roles, pin current spellings and audit raw linker names.

Canonical Rust paths find candidate functions across compiler name changes.
Only actual raw names can certify an order file as current; missing or
ambiguous required executors fail before writing. Slot padding is exact,
including whole pages. Audited addresses are observations, not a guarantee
that a later source edit preserves placement. Procedure:
`docs/performance-workflow.md#re-pinning-the-code-layout`.
"""

from __future__ import annotations

import argparse
import platform
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping, Sequence

from .layout_symbols import SymbolEntry, SymbolIndex, crate_names, parse_symbol
from .layout_tables import LayoutError, OrderFile, TextSymbols, read_order, text_symbols

ROOT = Path(__file__).resolve().parents[2]
ORDER_NAME = ("hot-functions-x86_64.order" if platform.machine() == "x86_64"
              else "hot-functions.order")
DEFAULT_ORDER = ROOT / "crates/qjs-cli" / ORDER_NAME
# Historical ARM64 offsets, kept as the defaults for compatibility. Prefer
# `--preserve-offsets`, which reads where the executors already are.
DEFAULT_OFFSET = 0x400
DEFAULT_VM_OFFSET = 0xc40
PAGE = 0x1000
ALIGN = 16

from .layout_targets import (
    RUNTIME_CRATE,
    WIDE_EXECUTOR,
    WIDE_EXECUTOR_FALLBACK,
    INTERPRETER_EXECUTOR,
    INTERPRETER_EXECUTOR_FALLBACK,
    DIRECT_EXECUTOR_TARGETS,
    CALLEES,
    CALLEE_OFFSETS,
    SELECTED_SPELLINGS,
    TAIL_CALLEES,
    EXECUTOR_ROLES,
    ROLE_TARGETS,
    ROLE_ORDER,
)

# A pinned function's slot: the recorded one while the function fits, else a
# re-derived one with SLOT_HEADROOM to spare -- which moves everything after
# it, so the offset has to be re-scanned then.
SLOT_GRANULE = 0x100
SLOT_HEADROOM = 0x100
_DISAMBIGUATOR = re.compile(r"s[0-9A-Za-z]*_")
_IDENTIFIER_LENGTH = re.compile(r"[1-9][0-9]*_?")


def definition_crate(symbol: str) -> str | None:
    """The crate a concrete `v0` item path is defined in, without its hash.

    Follow path prefixes rather than searching identifier contents for a
    crate-shaped substring, so an identifier that merely looks like a crate
    marker cannot promote a function to a filler candidate. Unsupported
    prefixes conservatively exclude the candidate.
    """
    if not symbol.startswith("__RNv"):
        return None
    cursor = 3
    while cursor < len(symbol):
        tag = symbol[cursor]
        cursor += 1
        if tag == "N":
            if cursor >= len(symbol) or not symbol[cursor].isascii() or not symbol[cursor].isalpha():
                return None
            cursor += 1  # namespace, followed by the parent path
        elif tag in {"M", "X", "C"}:
            if match := _DISAMBIGUATOR.match(symbol, cursor):
                cursor = match.end()
            if tag == "C":
                match = _IDENTIFIER_LENGTH.match(symbol, cursor)
                if match is None:
                    return None
                length = int(match.group().rstrip("_"))
                cursor = match.end()
                return symbol[cursor:cursor + length] if cursor + length <= len(symbol) else None
        elif tag not in {"I", "Y"}:
            return None
    return None


def filler(sizes: Mapping[str, int], counts: Mapping[str, int], length: int,
           exclude: Iterable[str]) -> list[str]:
    """Concrete standard-library functions whose sizes sum to `length`.

    Only functions the linker emits once, defined in the standard library and
    a multiple of ALIGN long: their sizes do not change with this repository's
    code, so placing them here fixes everything after them.
    """
    if length < 0 or length % ALIGN:
        raise LayoutError(f"filler length must be a nonnegative multiple of {ALIGN}")
    if length == 0:
        return []
    excluded = set(exclude)
    candidates = sorted(
        ((size, name) for name, size in sizes.items()
         if counts.get(name) == 1 and definition_crate(name) == "std"
         and "qjs" not in name and size > 0 and size % ALIGN == 0 and name not in excluded),
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
    raise LayoutError(f"no standard-library filler sums to {length:#x} bytes")


def slot_budget(size: int, previous: int | None) -> int:
    """A pinned function's slot: the recorded one while the function fits."""
    if previous is not None and size <= previous:
        return previous
    return -(-size // SLOT_GRANULE) * SLOT_GRANULE + SLOT_HEADROOM


def definitions(entries: Iterable[SymbolEntry], symbols: TextSymbols
                ) -> tuple[tuple[SymbolEntry, ...], tuple[SymbolEntry, ...]]:
    """(one spelling per live address, the spellings not selected).

    Spellings that share an address are one definition -- coalesced aliases --
    and the spelling owning that address's size is the one to name. Spellings
    at *different* addresses are different bodies, so all of them come back
    for the caller to refuse: no symbol table says which body a caller
    reaches. A group with no sized spelling holds only a zero-size alias,
    which is never selected.
    """
    by_address: dict[int, list[SymbolEntry]] = {}
    for entry in entries:
        address = symbols.address_of(entry.text)
        if address is not None:
            by_address.setdefault(address, []).append(entry)
    selected: list[SymbolEntry] = []
    unselected: list[SymbolEntry] = []
    for address in sorted(by_address):
        group = sorted(by_address[address], key=lambda entry: entry.text)
        sized = [entry for entry in group if symbols.size_of(entry.text)]
        if not sized:
            unselected += group
            continue
        selected.append(sized[0])
        unselected += [entry for entry in group if entry is not sized[0]]
    return tuple(selected), tuple(unselected)


@dataclass(frozen=True)
class Finding:
    """What one role matched in a binary and an order file.

    `binary` holds one spelling per live address, with `addresses` and `sizes`
    parallel to it. Two entries therefore mean the role has two bodies and
    `ambiguous` is set: the tool refuses to pick one, because a symbol table
    does not say which of them runs.
    """

    role: str
    target: str
    alternatives: tuple[str, ...]
    binary: tuple[SymbolEntry, ...]
    addresses: tuple[int, ...]
    sizes: tuple[int | None, ...]
    order: tuple[SymbolEntry, ...]
    aliases: tuple[SymbolEntry, ...]
    unpinnable: tuple[SymbolEntry, ...] = ()

    @property
    def in_binary(self) -> bool:
        return bool(self.binary)

    @property
    def listed(self) -> bool:
        return bool(self.order)

    @property
    def ambiguous(self) -> bool:
        """Two live definitions at different addresses: nothing picks one."""
        return len(self.binary) > 1

    @property
    def chosen(self) -> SymbolEntry | None:
        """The spelling to write into an order file, or `None` to write none.

        Never an order-file spelling the binary does not have: writing one
        would look like a pin while the linker ignored it.
        """
        return None if self.ambiguous or not self.binary else self.binary[0]

    @property
    def address(self) -> int | None:
        return self.addresses[0] if self.addresses else None

    @property
    def size(self) -> int | None:
        return self.sizes[0] if self.sizes else None

    @property
    def offset(self) -> int | None:
        """The target's address modulo the 4 KiB page it is pinned within."""
        return None if self.address is None else self.address % PAGE

    def describe(self) -> str:
        """One deterministic line naming the role, the target and the match."""
        if self.address is not None:
            where = f"{self.address:#x} ({self.offset:#x} mod 4 KiB)"
        elif self.listed:
            where = "in the order file only"
        else:
            where = "missing"
        detail = f"{self.role}: {self.target} -> {where}"
        if self.ambiguous:
            detail += "; ambiguous, live at " + ", ".join(
                f"{address:#x}" for address in self.addresses)
        if self.unpinnable and not self.ambiguous:
            detail += ("; defined but unpinnable: " + self.unpinnable[0].text
                       + " shares an address or is the last symbol, so it has no size")
        if self.alternatives:
            detail += f"; also present: {', '.join(self.alternatives)}"
        if self.aliases:
            detail += f"; {len(self.aliases)} alias spelling(s)"
        return detail


@dataclass(frozen=True)
class StaleEntry:
    """An order-file entry the binary does not have under that spelling.

    A canonical match is only the replacement to write, never a reason to call
    the entry live: the linker resolves raw spellings.
    """

    name: str
    source: str  # "symbol" | "budget"
    reason: str
    canonical: str | None
    replacement: str | None
    engine: bool

    def describe(self) -> str:
        detail = f"stale {self.source}: {self.name} ({self.reason})"
        if self.replacement is not None:
            detail += f"; the binary spells it {self.replacement}"
        return detail


def _slot(findings: Sequence[Finding], role: str) -> Finding:
    group = [finding for finding in findings if finding.role == role]
    if len(group) != 1:
        raise LayoutError(f"expected one finding for {role!r}, found {len(group)}")
    return group[0]


def _narrow(target: str, binary: tuple[SymbolEntry, ...], aliases: tuple[SymbolEntry, ...]
            ) -> tuple[tuple[SymbolEntry, ...], tuple[SymbolEntry, ...]]:
    """Apply an explicit `SELECTED_SPELLINGS` choice to one target's bodies.

    `binary` holds one spelling per live address, so narrowing it to a named
    spelling is how a caller says which of two bodies it means. A configured
    spelling that is not one of them is an error, not a silent fallback.
    """
    selected = SELECTED_SPELLINGS.get(target)
    if selected is None or not binary:
        return binary, aliases
    kept = tuple(entry for entry in binary if entry.text == selected)
    if not kept:
        raise LayoutError(
            f"SELECTED_SPELLINGS names {selected} for {target}, which the binary does not "
            "define; its live spellings are " + ", ".join(entry.text for entry in binary))
    return kept, aliases + tuple(entry for entry in binary if entry is not kept[0])


def findings_for(symbols: TextSymbols, listed: Sequence[str]) -> tuple[Finding, ...]:
    """One finding per role, in `ROLE_TARGETS` order.

    A role with alternative spellings takes the first with a live definition
    in the binary, else the first the order file names; the others are
    reported in `alternatives`.
    """
    binaries = SymbolIndex(symbols.names)
    order = SymbolIndex(listed)
    def matches(index, target):
        entries = index.match(target)
        # A closure nested inside an outlined execute is a deopt helper,
        # not that declaration's dispatch body. Historical run wrappers
        # remain valid alternatives when no direct execute was emitted.
        return tuple(entry for entry in entries if not entry.symbol.wrapped) \
            if target in DIRECT_EXECUTOR_TARGETS else entries
    findings = []
    for role, targets in ROLE_TARGETS:
        matched = [(target, matches(binaries, target), matches(order, target)) for target in targets]
        chosen = (next((row for row in matched if row[1]), None)
                  or next((row for row in matched if row[2]), None) or matched[0])
        binary, aliases = definitions(chosen[1], symbols)
        binary, aliases = _narrow(chosen[0], binary, aliases)
        findings.append(Finding(
            role=role,
            target=chosen[0],
            alternatives=tuple(row[0] for row in matched
                               if row is not chosen and (row[1] or row[2])),
            binary=binary,
            addresses=tuple(symbols.address_of(entry.text) for entry in binary),
            sizes=tuple(symbols.size_of(entry.text) for entry in binary),
            order=chosen[2],
            aliases=aliases,
            unpinnable=tuple(entry for entry in chosen[1]
                             if symbols.address_of(entry.text) is not None
                             and symbols.size_of(entry.text) is None),
        ))
    return tuple(findings)


def _staleness(order: OrderFile, symbols: TextSymbols,
               binaries: SymbolIndex) -> tuple[StaleEntry, ...]:
    """Every order-file entry the binary does not have under that spelling.

    The head a previous pin wrote is included, and so are the slot budgets:
    both name symbols, and both go stale when the engine moves.
    """
    raw = set(symbols.names)
    found: list[StaleEntry] = []
    for source, names in (("symbol", order.symbols), ("budget", tuple(order.budgets))):
        for name in names:
            if name in raw:
                size, recorded = symbols.size_of(name), order.budgets.get(name)
                if source == "budget" and recorded is not None and size is not None \
                        and size > recorded:
                    found.append(StaleEntry(name, "budget",
                                            f"slot {recorded:#x} cannot hold {size:#x} bytes",
                                            None, None, False))
                continue
            symbol = parse_symbol(name)
            engine = symbol is not None and RUNTIME_CRATE in crate_names(symbol.path)
            replacement, reason = None, "not in the binary"
            if symbol is not None:
                live, _ = definitions(binaries.match(symbol.canonical), symbols)
                if live:
                    replacement = live[0].text
                    if len(live) > 1:
                        reason += f", and {len(live)} bodies elsewhere match it"
            found.append(StaleEntry(name, source, reason,
                                    None if symbol is None else symbol.canonical,
                                    replacement, engine))
    return tuple(sorted(found, key=lambda entry: (not entry.engine, entry.source, entry.name)))


def _duplicates(order: OrderFile) -> tuple[str, ...]:
    """Symbol lines the file lists more than once: ambiguous file content."""
    counted = Counter(order.symbols)
    return tuple(sorted(name for name, count in counted.items() if count > 1))


@dataclass(frozen=True)
class LayoutReport:
    """What a binary and an order file matched, role by role."""

    binary: Path
    order: Path | None
    text_start: int
    binary_symbols: int
    order_symbols: int
    findings: tuple[Finding, ...]
    stale: tuple[StaleEntry, ...] = ()
    duplicates: tuple[str, ...] = ()

    def slot(self, role: str) -> Finding:
        return _slot(self.findings, role)

    def ordered(self) -> tuple[Finding, ...]:
        """The findings, executor roles first."""
        return tuple(finding for role in ROLE_ORDER for finding in self.findings
                     if finding.role == role)

    def stale_engine(self) -> tuple[StaleEntry, ...]:
        return tuple(entry for entry in self.stale if entry.engine)

    def missing_targets(self) -> tuple[Finding, ...]:
        """Roles the binary has no live definition for."""
        return tuple(finding for finding in self.ordered() if not finding.in_binary)

    def ambiguous_targets(self) -> tuple[Finding, ...]:
        return tuple(finding for finding in self.ordered() if finding.ambiguous)

    def executor_problems(self) -> tuple[str, ...]:
        """The findings that make pinning impossible: an executor is unusable."""
        return tuple(finding.describe() for finding in self.ordered()
                     if finding.role in EXECUTOR_ROLES
                     and (not finding.in_binary or finding.ambiguous))

    def strict_problems(self) -> tuple[str, ...]:
        """What `--strict` fails on: the executors, and anything stale.

        A callee the binary no longer has, or has twice, is reported but not
        fatal: the compiler inlines and renames them, the head simply leaves
        that slot out, and no order file can keep naming a function that is
        gone. Failing on that would reject a current file for a helper that
        legitimately disappeared.
        """
        found = list(self.executor_problems())
        found += [entry.describe() for entry in self.stale]
        found += [f"order file: lists {name} more than once" for name in self.duplicates]
        return tuple(found)

    def problems(self) -> tuple[str, ...]:
        """Everything a reader should know: strict problems and absent roles."""
        found = list(self.strict_problems())
        found += [finding.describe() for finding in self.missing_targets()
                  if finding.role not in EXECUTOR_ROLES]
        found += [finding.describe() for finding in self.ambiguous_targets()
                  if finding.role not in EXECUTOR_ROLES]
        return found

    def summary(self) -> str:
        """One deterministic line: where the executors are and what is stale."""
        parts = []
        for label, role in (("executor", "wide executor"),
                            ("interpreter", "interpreter executor")):
            finding = self.slot(role)
            where = "missing" if finding.address is None else f"{finding.offset:#x}"
            parts.append(f"{label} {where}")
        callees = [finding for finding in self.ordered() if finding.role == "callee"]
        parts.append(f"callees {sum(1 for finding in callees if finding.in_binary)}"
                     f"/{len(callees)} in the binary")
        parts.append(f"stale order entries {len(self.stale)}")
        return ", ".join(parts)

    def ok(self) -> bool:
        return not self.strict_problems()


def _analyse(binary: Path, order: Path | None, order_file: OrderFile | None,
             symbols: TextSymbols) -> LayoutReport:
    binaries = SymbolIndex(symbols.names)
    listed: Sequence[str] = () if order_file is None else order_file.names
    stale = () if order_file is None else _staleness(order_file, symbols, binaries)
    return LayoutReport(
        binary=binary,
        order=order,
        text_start=symbols.start,
        binary_symbols=len(symbols.names),
        order_symbols=0 if order_file is None else len(order_file.symbols),
        findings=findings_for(symbols, listed),
        stale=stale,
        duplicates=() if order_file is None else _duplicates(order_file),
    )


def audit(binary: Path, order: Path | None = None) -> LayoutReport:
    """Report every role's match in `binary`, and in `order` when given.

    A missing or ambiguous target is a finding, not an error, so a caller
    decides how to fail closed (`LayoutReport.strict_problems`, `.ok`). Only
    an unreadable binary or order file raises.
    """
    symbols = text_symbols(binary)
    return _analyse(binary, order, None if order is None else read_order(order), symbols)


def _require(findings: Sequence[Finding], check_interpreter: bool) -> None:
    """Raise unless each executor the caller needs has exactly one live body."""
    for role in EXECUTOR_ROLES:
        if role == "interpreter executor" and not check_interpreter:
            continue
        finding = _slot(findings, role)
        if not finding.in_binary:
            if finding.unpinnable:
                raise LayoutError(
                    f"the binary defines the {role} only as {finding.unpinnable[0].text}, "
                    "which is an alias or has no size, so it is unsafe "
                    "to pin")
            detail = (f"the order file names {finding.order[0].text}, which the linker "
                      "ignores" if finding.listed else "no spelling of it exists")
            raise LayoutError(f"the binary has no {role} symbol ({finding.target}): {detail}")
        if finding.chosen is None:
            raise LayoutError(
                f"the binary has {len(finding.binary)} live {role} definitions, so which body "
                "runs is unknown: " + ", ".join(
                    f"{entry.text} at {address:#x}"
                    for entry, address in zip(finding.binary, finding.addresses))
                + f"; name one in SELECTED_SPELLINGS[{finding.target!r}] to pin it")


def _head(lines: Sequence[str], symbols: TextSymbols, findings: Sequence[Finding],
          offset: int, vm_offset: int | None, budgets: dict[str, int] | None) -> list[str]:
    """The symbols to place ahead of the rest of `lines`.

    Given `budgets` (updated in place), the executor and each budgeted callee
    are followed by filler up to their slot size, so a function that grows
    inside its slot moves nothing after it. A callee the binary does not have,
    or has two bodies of, is left out and reported instead of being written as
    a name the linker would ignore.
    """
    executor = _slot(findings, "wide executor").chosen
    if executor is None:
        raise LayoutError("the binary has no wide executor symbol to pin")

    def selected(role: str, target: str) -> SymbolEntry | None:
        for finding in findings:
            if finding.role == role and finding.target == target:
                return finding.chosen
        raise LayoutError(f"no target is defined for {role} {target}")

    callees = [entry for entry in
               (selected("callee", name) for name in CALLEES)
               if entry is not None and entry.text != executor.text]
    tail = [entry for entry in (selected("tail callee", name) for name in TAIL_CALLEES)
            if entry is not None]
    # Filler may come from anywhere in the standard library, including a
    # function the order file already lists: the rewrite moves that line into
    # the head and drops it from its old place, so the file never lists a name
    # twice. Only what is already in the head, or is a target, is unavailable.
    # Excluding the body instead would leave nothing to fill with when the
    # file names every function, as a natural order does.
    reserved = {entry.text for finding in findings for entry in finding.binary}
    head: list[str] = []
    position = symbols.start

    def fill_to(target: int) -> None:
        """Fill with fixed-size standard-library functions up to `target`."""
        nonlocal position
        length = (target - position) % PAGE
        head.extend(filler(symbols.sizes, symbols.counts, length, set(head) | reserved))
        position += length

    interpreter = None if vm_offset is None else _slot(findings, "interpreter executor").chosen
    if vm_offset is not None and interpreter is not None:
        # The interpreter's twin goes first, so nothing whose size moves with
        # ordinary edits -- the callees below, some defined once per codegen
        # unit (`Value::clone`) in a number of copies that changes with
        # unrelated code -- is ahead of either executor.
        fill_to(vm_offset)
        head.append(interpreter.text)
        position += symbols.sizes.get(interpreter.text, 0)
    fill_to(offset)
    for entry in [executor, *callees]:
        name = entry.text
        desired = CALLEE_OFFSETS.get(entry.symbol.canonical)
        if desired is not None and budgets is not None:
            fill_to(desired)
        head.append(name)
        size = symbols.sizes.get(name, 0)
        position += size
        if budgets is None or name not in symbols.sizes:
            continue
        budgets[name] = slot_budget(size, budgets.get(name))
        padding = budgets[name] - size
        head.extend(filler(symbols.sizes, symbols.counts, padding, set(head) | reserved))
        position += padding
    head.extend(entry.text for entry in tail)
    return head


def pin_head(lines: Sequence[str], symbols: TextSymbols, offset: int = DEFAULT_OFFSET,
             vm_offset: int | None = DEFAULT_VM_OFFSET,
             budgets: dict[str, int] | None = None) -> list[str]:
    """[`pin`]'s head: the targets and their filler, ahead of the rest."""
    for name, value in (("offset", offset), ("vm_offset", vm_offset)):
        if value is not None and (value < 0 or value >= PAGE or value % ALIGN):
            raise LayoutError(f"{name} must be {ALIGN}-byte aligned inside one page")
    findings = findings_for(symbols, lines)
    _require(findings, vm_offset is not None)
    return _head(lines, symbols, findings, offset, vm_offset, budgets)


def pin(lines: Sequence[str], symbols: TextSymbols, offset: int = DEFAULT_OFFSET,
        vm_offset: int | None = DEFAULT_VM_OFFSET,
        budgets: dict[str, int] | None = None) -> list[str]:
    """`lines` (symbols, no comments) with the executor pinned at `offset`
    and, given `vm_offset`, its interpreter instantiation at that one."""
    head = pin_head(lines, symbols, offset, vm_offset, budgets)
    return head + [line for line in lines if line not in head]


def pin_order_file(order: Path, binary: Path, offset: int = DEFAULT_OFFSET,
                   vm_offset: int | None = DEFAULT_VM_OFFSET) -> LayoutReport:
    """Rewrite `order` with the executors pinned; return what matched.

    Raises `LayoutError` instead of writing a file whose required targets it
    could not locate in `binary`. The head is rebuilt from the binary's own
    spellings, so an entry the binary does not have is dropped rather than
    rewritten into a file that would look pinned while pinning nothing.
    """
    order_file = read_order(order)
    symbols = text_symbols(binary)
    report = _analyse(binary, order, order_file, symbols)
    _require(report.findings, vm_offset is not None)
    current = set(symbols.names)
    index = SymbolIndex(symbols.names)
    optional = set(CALLEES) | set(TAIL_CALLEES)

    def rebind(name: str) -> str | None:
        if name in current:
            return name
        decoded = parse_symbol(name)
        if decoded is None:
            raise LayoutError(f"cannot resolve stale order entry {name}")
        live, _ = definitions(index.match(decoded.canonical), symbols)
        if len(live) == 1:
            return live[0].text
        if len(live) > 1:
            raise LayoutError(f"ambiguous stale order entry {name}: "
                              + ", ".join(entry.text for entry in live))
        if decoded.canonical in optional:
            return None
        raise LayoutError(f"cannot resolve stale order entry {name}")

    body = tuple(dict.fromkeys(value for name in order_file.body
                              if (value := rebind(name)) is not None))
    budgets = {}
    for old, budget in order_file.budgets.items():
        name = rebind(old)
        if name is None:
            continue
        if name in budgets and budgets[name] != budget:
            raise LayoutError(f"conflicting recorded budgets for {name}")
        if symbols.size_of(name) is None:
            raise LayoutError(f"recorded budget names an alias without an extent: {name}")
        budgets[name] = budget
    head = _head(body, symbols, report.findings, offset, vm_offset, budgets)
    pinned = head + [line for line in body if line not in head]
    header = list(order_file.header)
    header += [f"# budget {size:#x} {name}" for name, size in budgets.items() if name in head]
    vm = f", interpreter's at {vm_offset:#x}" if vm_offset is not None else ""
    header.append(f"# pinned: typed-loop executor at {offset:#x}{vm} mod 4 KiB "
                  f"(python3 -m tools.benchmark.layout_pin), head {len(head)}")
    order.write_text("\n".join(header + pinned) + "\n", encoding="utf-8")
    return report


def executor_offsets(binary: Path) -> tuple[int, int | None]:
    """The executors' offsets modulo 4 KiB as the binary already has them.

    `--preserve-offsets` uses this so a re-pin keeps the placement the binary
    was built with, instead of a default that belongs to another architecture.
    """
    symbols = text_symbols(binary)
    findings = findings_for(symbols, ())
    _require(findings, False)
    wide = _slot(findings, "wide executor")
    if not wide.in_binary or wide.offset is None:
        raise LayoutError(f"the binary has no wide executor to read an offset from "
                          f"({wide.target})")
    interpreter = _slot(findings, "interpreter executor")
    if interpreter.ambiguous:
        _require(findings, True)
    return wide.offset, (interpreter.offset if interpreter.in_binary else None)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.layout_pin",
                                     description=__doc__.split("\n\n")[0])
    parser.add_argument("--binary", type=Path, required=True,
                        help="a release qjs built with the current toolchain")
    parser.add_argument("--order", type=Path, default=DEFAULT_ORDER)
    parser.add_argument("--offset", type=lambda text: int(text, 0), default=None,
                        help="executor start modulo 4 KiB (default %#x, or the executor's "
                             "current address with --preserve-offsets)" % DEFAULT_OFFSET)
    parser.add_argument("--vm-offset", type=lambda text: int(text, 0), default=None,
                        help="interpreter executor start modulo 4 KiB (default %#x, or its "
                             "current address with --preserve-offsets)" % DEFAULT_VM_OFFSET)
    parser.add_argument("--preserve-offsets", action="store_true",
                        help="pin at the executors' own current offsets instead of the "
                             "default constants")
    args = parser.parse_args(argv)
    offset, vm_offset = args.offset, args.vm_offset
    try:
        binary = args.binary.resolve()
        if args.preserve_offsets:
            if offset is not None or vm_offset is not None:
                raise LayoutError("--preserve-offsets reads both offsets from the binary; "
                                  "do not pass --offset or --vm-offset with it")
            offset, vm_offset = executor_offsets(binary)
        else:
            offset = DEFAULT_OFFSET if offset is None else offset
            vm_offset = DEFAULT_VM_OFFSET if vm_offset is None else vm_offset
        report = pin_order_file(args.order, binary, offset, vm_offset)
    except (LayoutError, subprocess.CalledProcessError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    for problem in report.strict_problems():
        print(f"warning: {problem}", file=sys.stderr)
    vm = "disabled" if vm_offset is None else f"{vm_offset:#x}"
    print(f"executor pinned at {offset:#x}, interpreter at {vm}: "
          f"{report.summary()}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

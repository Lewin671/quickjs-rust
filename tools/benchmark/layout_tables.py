"""Mach-O text extents and complete linker-order input, without selection policy."""
from __future__ import annotations

import re
import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

_BUDGET = re.compile(r"^# budget (0x[0-9a-f]+) (\S+)$")
_PIN_HEADER = re.compile(r"^# pinned: .* (?:filler|head) (\d+)")
TEXT_TYPES = frozenset({"t", "T"})
NON_FUNCTION_SYMBOLS = frozenset({"__mh_execute_header"})


class LayoutError(ValueError):
    """The layout cannot be pinned or audited as asked."""


@dataclass(frozen=True)
class TextSymbols:
    """The `__text` functions of one binary.

    `sizes` maps a symbol to the distance to the next address, which is its
    extent through the next address or section end. Same-address aliases are
    excluded because nm cannot prove which spelling owns the atom. `counts` counts the spellings that share one name.
    """

    start: int
    names: tuple[str, ...]
    addresses: Mapping[str, int]
    sizes: Mapping[str, int]
    counts: Mapping[str, int]

    def size_of(self, text: str) -> int | None:
        """The function extent, or None for a same-address/duplicate-name alias."""
        return self.sizes.get(text, 0) or None

    def address_of(self, text: str) -> int | None:
        """The symbol's address, or `None` when the binary does not define it."""
        return self.addresses.get(text)


def text_symbols(binary: Path) -> TextSymbols:
    """Read only __TEXT,__text; aliases are not safe slot/filler extents."""
    sections = subprocess.run(["otool", "-l", str(binary)], capture_output=True,
                              text=True, check=True).stdout
    match = re.search(r"sectname __text\s+segname __TEXT\s+addr (0x[0-9a-f]+)"
                      r"\s+size (0x[0-9a-f]+)", sections)
    if not match:
        raise LayoutError(f"{binary}: no __TEXT,__text section with bounds")
    start, length = (int(value, 16) for value in match.groups())
    end = start + length
    listing = subprocess.run(["nm", "-n", str(binary)], capture_output=True,
                             text=True, check=True).stdout
    rows = []
    for line in listing.splitlines():
        parts = line.split()
        if len(parts) != 3 or parts[1] not in TEXT_TYPES:
            continue
        try:
            address = int(parts[0], 16)
        except ValueError:
            continue
        if start <= address < end and parts[2] not in NON_FUNCTION_SYMBOLS:
            rows.append((address, parts[2]))
    rows.sort()
    addresses = {}
    counts = Counter(name for _, name in rows)
    at_address = Counter(address for address, _ in rows)
    following = dict(zip(sorted(at_address), [*sorted(at_address)[1:], end]))
    sizes = {}
    for address, name in rows:
        addresses.setdefault(name, address)
        # nm does not identify which same-address name owns the atom.
        if counts[name] == 1 and at_address[address] == 1:
            sizes[name] = following[address] - address
    return TextSymbols(start, tuple(dict.fromkeys(name for _, name in rows)),
                       addresses, sizes, dict(counts))


@dataclass(frozen=True)
class OrderFile:
    """An order file as written, for both reading and rewriting.

    `symbols` is every symbol line in file order, the pinned head included: an
    audit has to see the whole file, while a rewrite replaces only the head
    and keeps the body. `budgets` are the recorded slots, which name symbols
    too and go stale like any other entry.
    """

    header: tuple[str, ...]
    symbols: tuple[str, ...]
    budgets: Mapping[str, int]
    head_length: int

    @property
    def head(self) -> tuple[str, ...]:
        return self.symbols[:self.head_length]

    @property
    def body(self) -> tuple[str, ...]:
        """The symbols after a previous pin's head: what a rewrite keeps."""
        return self.symbols[self.head_length:]

    @property
    def names(self) -> tuple[str, ...]:
        """Every symbol the file names, budget names included."""
        extra = tuple(name for name in self.budgets if name not in self.symbols)
        return self.symbols + extra


def read_order(order: Path) -> OrderFile:
    """Read the complete order file: head, budgets and body."""
    text = [line.strip() for line in order.read_text(encoding="utf-8").splitlines()]
    head_length = 0
    for line in text:
        if match := _PIN_HEADER.match(line):
            head_length = int(match.group(1))
    budgets = {match.group(2): int(match.group(1), 16)
               for line in text if (match := _BUDGET.match(line))}
    header = tuple(line for line in text if line.startswith("#")
                   and not _PIN_HEADER.match(line) and not _BUDGET.match(line))
    symbols = tuple(line for line in text if line and not line.startswith("#"))
    if head_length > len(symbols):
        raise LayoutError("recorded pin head is longer than the order file")
    return OrderFile(header, symbols, budgets, head_length)



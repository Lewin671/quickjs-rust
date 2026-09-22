"""Function-size differences between two executables.

An unrelated edit can change inlining or unrolling of a hot function; diffing
symbol sizes is the first check before attributing a regression to a change's
own logic (docs/performance-knowledge.md, "Codegen").
"""

from __future__ import annotations

import re
import subprocess
from collections import defaultdict
from pathlib import Path

TEXT_TYPES = {"t", "T"}
# Legacy Rust mangling appends a per-build hash that must not split one function in two.
_RUST_HASH = re.compile(r"::h[0-9a-f]{16}$")


class SymbolError(RuntimeError):
    pass


def parse_nm(output: str) -> dict[str, int]:
    """Sizes of text symbols from address-sorted `nm -n -C` output.

    A symbol's size is the distance to the next defined address. The Rust
    hash suffix is dropped and symbols that then share a name (monomorphized
    copies) are summed, so the result is comparable across builds.
    """
    entries: list[tuple[int, str, str]] = []
    for line in output.splitlines():
        parts = line.split(" ", 2)
        if len(parts) != 3:
            continue
        address, kind, name = parts
        try:
            entries.append((int(address, 16), kind, name))
        except ValueError:
            continue
    sizes: dict[str, int] = defaultdict(int)
    for (address, kind, name), (next_address, _, _) in zip(entries, entries[1:]):
        if kind in TEXT_TYPES and next_address > address:
            sizes[_RUST_HASH.sub("", name)] += next_address - address
    return dict(sizes)


def symbol_sizes(binary: Path) -> dict[str, int]:
    try:
        completed = subprocess.run(
            ["nm", "-n", "-C", "--defined-only", str(binary)],
            capture_output=True, text=True, timeout=120, check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SymbolError(f"nm failed: {error}") from error
    if completed.returncode != 0:
        raise SymbolError(f"nm exited {completed.returncode}: {completed.stderr.strip()[:200]}")
    sizes = parse_nm(completed.stdout)
    if not sizes:
        raise SymbolError(f"no text symbols in {binary} (stripped?)")
    return sizes


def size_diff(base: Path, candidate: Path) -> list[tuple[str, int, int]]:
    """(name, base size, candidate size) for every changed symbol, largest change first."""
    before, after = symbol_sizes(base), symbol_sizes(candidate)
    changed = [
        (name, before.get(name, 0), after.get(name, 0))
        for name in before.keys() | after.keys()
        if before.get(name, 0) != after.get(name, 0)
    ]
    changed.sort(key=lambda row: (-abs(row[2] - row[1]), row[0]))
    return changed


def render(changes: list[tuple[str, int, int]], limit: int) -> str:
    if not changes:
        return "symbol sizes: no function changed size"
    lines = [
        f"symbol sizes: {len(changes)} function(s) changed; largest {min(limit, len(changes))}:",
        "| base bytes | candidate bytes | delta | function |",
        "| ---: | ---: | ---: | --- |",
    ]
    for name, before, after in changes[:limit]:
        shown = name if len(name) <= 110 else name[:107] + "..."
        lines.append(f"| {before} | {after} | {after - before:+d} | `{shown}` |")
    return "\n".join(lines)

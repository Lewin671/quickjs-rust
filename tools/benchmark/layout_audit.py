"""Audit a binary and an order file against the pinned layout targets.

`layout_pin.py` owns the targets and the pinning; this module owns the
report. It answers the question a pin cannot answer about itself: do the
functions the order file names still exist in this binary under the spelling
the file gives, where are they, and which entries are stale?

Staleness is judged on the raw spelling, because a raw spelling is what the
linker resolves. An entry whose function still exists under another spelling
is stale all the same -- the report names the replacement to write, and
never counts a canonical match as a live entry. That distinction is the whole
failure mode: at the base revision the committed x86_64 file named 521 engine
symbols of which 0 matched the binary, yet the program linked and looked
pinned, because the linker silently ignores a name it cannot resolve.

Exit status: 0 when the executors were found, 1 when they were not, and with
`--strict` also 1 when any target is ambiguous or any entry -- symbol, budget
or a slot too small for its function -- is stale. `--strict` is the mode to
use when the file's currency has to be asserted rather than reported.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Sequence

from .layout_pin import DEFAULT_ORDER, LayoutError, LayoutReport, audit

# How many stale entries the text report lists by default; the count is always
# reported, and `--json` always carries every one of them.
DEFAULT_LIMIT = 10


def render(report: LayoutReport, limit: int = DEFAULT_LIMIT) -> str:
    """The report as deterministic text: one line per role, then staleness."""
    lines = [f"binary {report.binary}: {report.binary_symbols} text symbols, "
             f"__text at {report.text_start:#x}"]
    if report.order is not None:
        line = f"order  {report.order}: {report.order_symbols} entries"
        if report.stale:
            line += (f", {len(report.stale)} stale"
                     f" ({len(report.stale_engine())} naming the engine)")
        lines.append(line)
    for finding in report.ordered():
        lines.append("  " + finding.describe())
        for entry in finding.aliases:
            lines.append(f"      alias spelling: {entry.text}")
    if report.stale:
        shown = report.stale[:limit]
        lines.append(f"  stale entries ({len(report.stale)}"
                     + (f", first {len(shown)})" if len(shown) < len(report.stale) else ")"))
        lines.extend(f"    {entry.describe()}" for entry in shown)
    return "\n".join(lines)


def as_json(report: LayoutReport) -> dict:
    """The report as JSON data, with every stale entry listed."""
    return {
        "binary": str(report.binary),
        "order": None if report.order is None else str(report.order),
        "text_start": report.text_start,
        "binary_symbols": report.binary_symbols,
        "order_symbols": report.order_symbols,
        "findings": [
            {
                "role": finding.role,
                "target": finding.target,
                "alternatives": list(finding.alternatives),
                "in_binary": finding.in_binary,
                "listed": finding.listed,
                "ambiguous": finding.ambiguous,
                "address": finding.address,
                "offset": finding.offset,
                "size": finding.size,
                "addresses": list(finding.addresses),
                "binary": [entry.text for entry in finding.binary],
                "order": [entry.text for entry in finding.order],
                "aliases": [entry.text for entry in finding.aliases],
            }
            for finding in report.ordered()
        ],
        "stale": [
            {
                "name": entry.name,
                "source": entry.source,
                "reason": entry.reason,
                "engine": entry.engine,
                "canonical": entry.canonical,
                "replacement": entry.replacement,
            }
            for entry in report.stale
        ],
        "problems": list(report.problems()),
        "executor_problems": list(report.executor_problems()),
        "strict_problems": list(report.strict_problems()),
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.layout_audit",
                                     description=__doc__.split("\n\n")[0])
    parser.add_argument("--binary", type=Path, required=True,
                        help="the qjs build to audit")
    parser.add_argument("--order", type=Path, default=DEFAULT_ORDER,
                        help="the order file to audit against it (default: the host's)")
    parser.add_argument("--json", action="store_true", help="report as JSON data")
    parser.add_argument("--strict", action="store_true",
                        help="fail when a required executor is ambiguous or any entry, budget or "
                             "slot size is stale")
    parser.add_argument("--limit", type=int, default=DEFAULT_LIMIT,
                        help="stale entries to list in the text report (default %(default)s)")
    args = parser.parse_args(argv)
    try:
        report = audit(args.binary.resolve(), args.order)
    except (LayoutError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(as_json(report), indent=2))
    else:
        print(render(report, args.limit))
    problems = report.strict_problems() if args.strict else report.executor_problems()
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    return 1 if problems else 0

if __name__ == "__main__":
    raise SystemExit(main())

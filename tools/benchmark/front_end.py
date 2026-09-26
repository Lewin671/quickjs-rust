"""Parse-and-compile cost of the external corpus, against a reference engine.

Every external case runs its whole process, so its time includes lexing,
parsing and compiling the bundle, which this engine did 2-3x slower than
QuickJS-NG (2026-09-26; 1-8% of many cases' totals). This measures that
front end alone: each bundle is wrapped in a function that is never called,
so the engine parses and compiles all of it and runs nothing, and the cost
of an empty script (process start) is subtracted. Hardware counters, as in
`tools.benchmark.screen`; diagnostic only, never decision evidence.

Usage:
  python3 -m tools.benchmark.front_end --binary B [--reference NG] [--base A]
         [--case external/<suite>/<case> ...] [--pairs N]

With `--reference` the table gives the engine's front-end cost over the
reference's; with `--base` (another build of this engine), candidate over
base. The `share` column is the front-end excess over the reference as a
fraction of the case's whole run on `--binary`: what a front end as fast as
the reference would save.
"""

from __future__ import annotations

import argparse
import math
import statistics
import sys
import tempfile
from pathlib import Path
from typing import Sequence

from .external_preview import ExternalPreviewError
from .measure_lock import measurement_lock
from .screen import (
    DEFAULT_CACHE,
    CaseSpec,
    CounterTool,
    ScreenError,
    counter_tool,
    resolve_cases,
    sample,
)

EMPTY_SCRIPT = "1;\n"


def _flags(binary: Path, reference: Path | None) -> list[str]:
    # QuickJS-NG treats a file as a module unless told otherwise.
    return ["--script"] if reference is not None and binary == reference else ["--raw"]


def _cost(tool: CounterTool, binary: Path, flags: list[str], script: Path, pairs: int,
          timeout: float) -> tuple[float, float]:
    """Median instructions and cycles of running `script`."""
    taken = [sample(tool, [str(binary), *flags, str(script)], timeout) for _ in range(pairs)]
    return (statistics.median(s.instructions for s in taken),
            statistics.median(s.cycles for s in taken))


def _wrap(bundle: Path, work: Path) -> Path:
    wrapped = work / f"front-end--{bundle.name}"
    source = bundle.read_text(encoding="utf-8")
    wrapped.write_text("function __qjsFrontEndOnly__() {\n" + source + "\n}\n1;\n", encoding="utf-8")
    return wrapped


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.front_end",
                                     description=__doc__.split("\n\n")[0])
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--reference", type=Path,
                        help="a reference engine (QuickJS-NG, run with --script)")
    parser.add_argument("--base", type=Path, help="another build of this engine")
    parser.add_argument("--case", action="append", default=[],
                        help="external/<suite>/<case>; repeatable (default: every external case)")
    parser.add_argument("--pairs", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if (args.reference is None) == (args.base is None):
        print("error: give exactly one of --reference and --base", file=sys.stderr)
        return 2
    binary = args.binary.resolve()
    other = (args.reference or args.base).resolve()
    reference = other if args.reference is not None else None
    try:
        tool = counter_tool()
        with measurement_lock("front-end"), tempfile.TemporaryDirectory(prefix="qjs-front-end-") as work:
            work_dir = Path(work)
            cases: list[CaseSpec] = resolve_cases(args.case or ["external"], args.cache_root, work_dir)
            empty = work_dir / "empty.js"
            empty.write_text(EMPTY_SCRIPT, encoding="utf-8")
            start = {role: _cost(tool, exe, _flags(exe, reference), empty, args.pairs, args.timeout)
                     for role, exe in (("binary", binary), ("other", other))}
            rows = []
            for case in cases:
                if case.kind != "whole":
                    continue
                wrapped = _wrap(case.workload, work_dir)
                front = {}
                for role, exe in (("binary", binary), ("other", other)):
                    instructions, cycles = _cost(tool, exe, _flags(exe, reference), wrapped,
                                                 args.pairs, args.timeout)
                    front[role] = (instructions - start[role][0], cycles - start[role][1])
                _, whole_cycles = _cost(tool, binary, ["--raw"], case.workload, 1, args.timeout)
                rows.append((case.id, front, whole_cycles))
    except (ScreenError, ExternalPreviewError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    label = "reference" if reference is not None else "base"
    print(f"{'case':56s} {'Mcyc':>8s} {label + ' Mcyc':>15s} {'ratio':>6s} {'share':>6s}")
    logs = []
    for case_id, front, whole_cycles in rows:
        mine, theirs = front["binary"][1], front["other"][1]
        ratio = mine / theirs if theirs > 0 and mine > 0 else math.nan
        share = (mine - theirs) / whole_cycles if whole_cycles > 0 else math.nan
        if ratio == ratio:
            logs.append(math.log(ratio))
        print(f"{case_id:56s} {mine / 1e6:8.1f} {theirs / 1e6:15.1f} {ratio:6.2f} {share:6.1%}")
    if logs:
        print(f"geometric mean front-end cycles ratio: {math.exp(sum(logs) / len(logs)):.3f} "
              f"over {len(logs)} cases (diagnostic only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

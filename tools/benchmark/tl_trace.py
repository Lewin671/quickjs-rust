"""Summarize `QJS_TL_TRACE=1` output from a perf-counters build.

Prints the two histograms described in docs/benchmarking.md ("Execution
counters"): deoptimization sites by frequency, and interpreted typed-loop
regions weighted by backward edges joined with their last give-up or decline
reason. Trace output can be large, so it is streamed rather than captured.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Iterable, Sequence

from .screen import ScreenError, _argv, resolve_cases, DEFAULT_CACHE

_DEOPT_IP = re.compile(r" ip \d+ ")
_DEOPT_BC = re.compile(r" bc .*$")


def summarize(lines: Iterable[str], limit: int = 15) -> str:
    deopts: Counter[str] = Counter()
    edges: Counter[str] = Counter()
    reasons: dict[str, str] = {}
    for line in lines:
        line = line.rstrip("\n")
        if line.startswith("TLDEOPT"):
            deopts[_DEOPT_BC.sub("", _DEOPT_IP.sub(" ", line))] += 1
            continue
        fields = line.split()
        if len(fields) < 3:
            continue
        if fields[0] == "TLEDGE":
            edges[fields[2]] += 1
        elif fields[0] in {"TLGIVEUP", "TLDECLINE"}:
            reasons[fields[2]] = line
    out = [f"TLDEOPT sites ({sum(deopts.values())} deopts):"]
    out += [f"  {count:>10}  {site}" for site, count in deopts.most_common(limit)] or ["  none"]
    out.append(f"TLEDGE interpreted regions ({sum(edges.values())} edges):")
    out += [
        f"  {count:>10}  {region}  {reasons.get(region, '(no give-up/decline line)')}"
        for region, count in edges.most_common(limit)
    ] or ["  none"]
    return "\n".join(out)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python3 -m tools.benchmark.tl_trace",
        description="run one screen case on a perf-counters build with QJS_TL_TRACE=1 and summarize",
    )
    parser.add_argument("--binary", type=Path, required=True, help="a perf-counters qjs build")
    parser.add_argument("--case", required=True, help="one screen case spec, e.g. external/<suite>/<case>")
    parser.add_argument("--iterations", type=int, default=10000, help="N for internal cases")
    parser.add_argument("--limit", type=int, default=15)
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE)
    args = parser.parse_args(argv)
    try:
        with tempfile.TemporaryDirectory(prefix="qjs-trace-") as work:
            cases = resolve_cases([args.case], args.cache_root, Path(work))
            if len(cases) != 1:
                raise ScreenError("--case must name exactly one case")
            command = _argv(args.binary.resolve(), cases[0], args.iterations)
            process = subprocess.Popen(
                command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
                env={**os.environ, "QJS_TL_TRACE": "1"}, errors="replace",
            )
            assert process.stderr is not None
            summary = summarize(process.stderr, args.limit)
            status = process.wait()
    except (ScreenError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"trace: {json.dumps(command)} exited {status}")
    print(summary)
    return 0 if status == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

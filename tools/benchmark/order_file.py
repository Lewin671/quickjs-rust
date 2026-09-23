"""Generate the linker order file that pins the hot code layout.

Unrelated edits move hot functions to different addresses, and on this host
that alone shifts some cases' cycles by several percent with identical
instruction counts (docs/performance-knowledge.md, "Codegen"). Listing the
hot functions first, in a fixed order, keeps their layout stable across
edits. `crates/qjs-cli/build.rs` passes the file to the macOS linker.

The ranking samples every selected case with macOS `sample`, weights each
case equally, and lists every sampled function, hottest first. The workspace builds with v0 symbol mangling
(`.cargo/config.toml`), whose names carry no signature hash, so the list keeps
naming a function whose signature changes; precompiled standard-library
symbols are matched by their legacy hash suffix.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
import time
from collections import Counter
from pathlib import Path
from typing import Sequence

from .screen import (
    DEFAULT_CACHE, ScreenError, _argv, calibrate, counter_tool, resolve_cases, sample,
)

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / "crates/qjs-cli/hot-functions.order"
_HASH = re.compile(r"h[0-9a-f]{16}")
_PROFILE_LINE = r"^[\s+!:|]*(\d+)\s+(.*?)\s+\(in {image}\)"
_MANGLED_HASH = re.compile(r"17(h[0-9a-f]{16})E$")


def symbol_keys(binary: Path) -> dict[str, str]:
    """Profile key -> linker symbol for the binary's text symbols: v0
    symbols by their own name, legacy ones by their hash suffix."""
    output = subprocess.run(["nm", str(binary)], capture_output=True, text=True, check=True).stdout
    mapping: dict[str, str] = {}
    for line in output.splitlines():
        parts = line.split()
        if len(parts) != 3 or parts[1] not in {"t", "T"}:
            continue
        symbol = parts[2]
        if symbol.startswith("__R"):
            mapping[symbol] = symbol
        elif match := _MANGLED_HASH.search(symbol):
            mapping[match.group(1)] = symbol
    return mapping


def profile_key(name: str) -> str | None:
    """The key a profiled function is matched by: a v0 symbol as the linker
    spells it (`sample` prints it raw, without the leading underscore), or
    the hash suffix of a legacy-mangled standard-library symbol."""
    if name.startswith("_R"):
        return "_" + name.split()[0]
    found = _HASH.findall(name)
    return found[-1] if found else None


def parse_profile(text: str, image: str = "qjs") -> Counter[str]:
    """Samples per function key in the executable image `image`, summed over
    every call-graph occurrence."""
    pattern = re.compile(_PROFILE_LINE.format(image=re.escape(image)))
    weights: Counter[str] = Counter()
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        key = profile_key(match.group(2))
        if key:
            weights[key] += int(match.group(1))
    return weights


def profile_process(argv: list[str], seconds: float) -> Counter[str]:
    image = Path(argv[0]).name
    with tempfile.TemporaryDirectory(prefix="qjs-order-") as work:
        report = Path(work) / "profile.txt"
        process = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.05)
        subprocess.run(
            ["sample", str(process.pid), str(max(1, round(seconds))), "1", "-mayDie",
             "-file", str(report)],
            capture_output=True, check=False,
        )
        process.wait()
        return (
            parse_profile(report.read_text(errors="replace"), image) if report.exists() else Counter()
        )


def rank(profiles: Sequence[Counter[str]], coverage: float) -> list[str]:
    """Keys ordered by equal-case-weighted share, up to `coverage`."""
    total: Counter[str] = Counter()
    for profile in profiles:
        size = sum(profile.values())
        if size:
            for key, value in profile.items():
                total[key] += value / size
    grand = sum(total.values())
    ordered, covered = [], 0.0
    for key, value in total.most_common():
        if grand and covered / grand >= coverage:
            break
        ordered.append(key)
        covered += value
    return ordered


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.order_file", description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--case", action="append", default=[],
                        help="screen case spec; repeatable (default: external, sentinel, broad)")
    parser.add_argument("--seconds", type=float, default=1.0, help="sampling window per case")
    # Inclusive sums weight outer frames once per stack, so a share cut-off
    # would keep dispatchers and drop the leaves they call; keep everything
    # sampled unless asked otherwise.
    parser.add_argument("--coverage", type=float, default=1.0)
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)
    binary = args.binary.resolve()
    try:
        tool = counter_tool()
        mapping = symbol_keys(binary)
        profiles = []
        with tempfile.TemporaryDirectory(prefix="qjs-order-cases-") as work:
            for case in resolve_cases(args.case or ["external", "sentinel", "broad"],
                                      args.cache_root, Path(work)):
                iterations = None
                if case.kind == "internal":
                    iterations = calibrate(lambda n: sample(tool, _argv(binary, case, n), 120.0))
                print(f"profiling {case.id} ...", file=sys.stderr, flush=True)
                profiles.append(profile_process(_argv(binary, case, iterations), args.seconds))
    except (ScreenError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    ordered = [mapping[key] for key in rank(profiles, args.coverage) if key in mapping]
    # An empty list would silently unpin the whole layout.
    if not ordered:
        print("error: no profiled function matched the binary's symbols", file=sys.stderr)
        return 1
    args.output.write_text(
        "# Hot functions first, hottest first. Generated by "
        "`python3 -m tools.benchmark.order_file`; regenerate after large changes.\n"
        + "".join(f"{symbol}\n" for symbol in ordered),
        encoding="utf-8",
    )
    print(f"{len(ordered)} symbols -> {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

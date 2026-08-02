#!/usr/bin/env python3
"""Amplified, strictly alternating A/B over the cached external corpus.

Why this exists
---------------

The obvious way to compare two engines on SunSpider and Kraken -- run each
file once per binary and take a ratio -- does not work here. Process start is
5.2 ms for QuickJS-NG and 5.4 ms for this engine, while most SunSpider cases
do only 12-35 ms of *work*. Two consequences, both observed:

* Startup dilutes every short case toward 1.0, so the corpus geometric mean
  understates the engine gap. Measured 2026-08-02, the same 26 cases read
  1.60 diluted and 1.61 amplified -- but per case the difference is large:
  `access-nbody` reads 1.92 diluted and 3.06 amplified.
* Scheduling noise then dominates what is left. `access-nbody` measured 1.92
  and 3.69 against the *same two binaries* an hour apart.

So this runner repeats each case's source **text** until one process runs for
at least `--target` seconds. Repeating the text keeps every copy at top level,
which matters: an engine selects its execution tier partly on scope, so
wrapping the body in a function would measure something else. Linearity was
verified on `access-nbody`, `crypto-aes`, `string-base64`,
`access-binary-trees`, `string-tagcloud` and `crypto-md5` -- per-copy time is
constant across 1, 4 and 16 copies.

Reading the output
------------------

A ratio whose `[min, max]` spans 1.0 is not evidence of anything. Three
repetitions is enough to rank cases; take a suspected regression to `--reps 11`
before believing it.

Usage:
  scripts/external-corpus-ab.py BASE CAND [--reps N] [--target SECONDS]
                                [--label L] [--only SUBSTRING]

`BASE` and `CAND` are engine binaries. Cases the corpus cache does not hold, or
that either binary fails, are reported as skipped rather than scored.
"""

import argparse
import glob
import math
import os
import statistics
import subprocess
import tempfile
import time

ROOT = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "target",
    "benchmarks",
    "external-cache",
)
# Beyond this the amplified source is large enough to distort parse time.
MAX_COPIES = 64
# Both engines start in about this long; used only to size the repetition
# count, never subtracted from a reported number.
STARTUP_SECONDS = 0.0053


def cases(only):
    found = []
    for path in sorted(glob.glob(os.path.join(ROOT, "sunspider-1.0", "*.js"))):
        found.append((f"ss/{os.path.basename(path)[:-3]}", None, path))
    kraken = os.path.join(ROOT, "kraken-1.1")
    for path in sorted(glob.glob(os.path.join(kraken, "*.js"))):
        name = os.path.basename(path)[:-3]
        if name.endswith("-data"):
            continue
        data = os.path.join(kraken, f"{name}-data.js")
        found.append((f"kr/{name}", data if os.path.exists(data) else None, path))
    if only:
        found = [case for case in found if only in case[0]]
    return found


def build(data, body, copies):
    """Writes the data file once, then the body `copies` times at top level."""
    handle = tempfile.NamedTemporaryFile("w", suffix=".js", delete=False)
    if data:
        with open(data, encoding="utf-8", errors="replace") as source:
            handle.write(source.read())
        handle.write("\n")
    with open(body, encoding="utf-8", errors="replace") as source:
        text = source.read()
    for _ in range(copies):
        handle.write(text)
        handle.write("\n")
    handle.close()
    return handle.name


def run(binary, path):
    start = time.perf_counter()
    result = subprocess.run(
        [binary, path],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=900,
        check=False,
    )
    return (time.perf_counter() - start) if result.returncode == 0 else None


def copies_for(base, cand, data, body, target):
    probe = build(data, body, 1)
    try:
        once = run(base, probe)
        other = run(cand, probe)
    finally:
        os.unlink(probe)
    if once is None or other is None:
        return None
    work = max(once - STARTUP_SECONDS, 0.001)
    return max(1, min(MAX_COPIES, math.ceil(target / work)))


def measure(base, cand, path, reps):
    ratios, base_times, cand_times = [], [], []
    for rep in range(reps):
        order = (base, cand) if rep % 2 == 0 else (cand, base)
        timing = {}
        for binary in order:
            value = run(binary, path)
            if value is None or value <= 0:
                return None
            timing[binary] = value
        ratios.append(timing[cand] / timing[base])
        base_times.append(timing[base])
        cand_times.append(timing[cand])
    return ratios, base_times, cand_times


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("base")
    parser.add_argument("cand")
    parser.add_argument("--reps", type=int, default=5)
    parser.add_argument("--target", type=float, default=2.0)
    parser.add_argument("--label", default="cand/base")
    parser.add_argument("--only", default=None)
    args = parser.parse_args()

    if not os.path.isdir(ROOT):
        raise SystemExit(
            f"no external corpus cache at {ROOT}; run scripts/benchmark.sh once to populate it"
        )

    print(f"{'case':34s} {args.label:>9s}   [min, max]     copies  base_s  cand_s")
    ratios, skipped = [], []
    for name, data, body in cases(args.only):
        copies = copies_for(args.base, args.cand, data, body, args.target)
        if copies is None:
            skipped.append(name)
            continue
        path = build(data, body, copies)
        try:
            measured = measure(args.base, args.cand, path, args.reps)
        finally:
            os.unlink(path)
        if measured is None:
            skipped.append(name)
            continue
        case_ratios, base_times, cand_times = measured
        median = statistics.median(case_ratios)
        ratios.append(median)
        print(
            f"{name:34s} {median:9.4f}   [{min(case_ratios):.4f}, {max(case_ratios):.4f}] "
            f"{copies:6d} {statistics.median(base_times):7.3f} "
            f"{statistics.median(cand_times):7.3f}"
        )
    if ratios:
        geomean = math.exp(sum(math.log(ratio) for ratio in ratios) / len(ratios))
        print(f"\n{'GEOMEAN (' + str(len(ratios)) + ' cases)':34s} {geomean:9.4f}")
        print(f"worst three: {[round(value, 4) for value in sorted(ratios)[-3:]]}")
    if skipped:
        print(f"skipped (nonzero exit or timeout): {', '.join(skipped)}")


if __name__ == "__main__":
    main()

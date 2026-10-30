#!/usr/bin/env python3
"""Probe CLI recursion depth and semantic output, never benchmark timing.

The host stack is unchanged. A probe cap is not an engine limit. Child core dumps
are disabled; engine behavior and host stack limits are unmodified.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess


CASES = {
    "forEach": (
        "function walk(d){let s=1;[d].forEach(function(k){if(k>0)s+=walk(k-1);});return s;} walk(DEPTH);",
        lambda depth: str(depth + 1),
    ),
    "map": (
        "function walk(d){let s=1;[d].map(function(k){if(k>0)s+=walk(k-1);return k;});return s;} walk(DEPTH);",
        lambda depth: str(depth + 1),
    ),
    "reduce": (
        "function walk(d){return [d].reduce(function(s,k){return s+(k>0?walk(k-1):0);},1);} walk(DEPTH);",
        lambda depth: str(depth + 1),
    ),
    "call": ("function f(n){return n?1+f.call(null,n-1):0;} f(DEPTH);", str),
    "apply": ("function f(n){return n?1+f.apply(null,[n-1]):0;} f(DEPTH);", str),
    "reflect_apply": (
        "function f(n){return n?1+Reflect.apply(f,null,[n-1]):0;} f(DEPTH);",
        str,
    ),
    "yield_star": (
        "function* walk(n){if(n)return yield* walk(n-1);return 42;} walk(DEPTH).next().value;",
        lambda depth: "42",
    ),
    "plain_in_callback": (
        "function f(n){return n?1+f(n-1):0;} var result;[0].forEach(function(){result=f(DEPTH);});result;",
        str,
    ),
}


def no_core():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def probe(binary, program, expected, depth):
    source = program.replace("DEPTH", str(depth))
    env = {key: value for key, value in os.environ.items() if not key.startswith("QJS_")}
    try:
        run = subprocess.run(
            [str(binary), "--raw", "-e", source],
            text=True,
            capture_output=True,
            timeout=30,
            preexec_fn=no_core,
            env=env,
        )
    except subprocess.TimeoutExpired:
        return {"depth": depth, "ok": False, "kind": "timeout"}
    ok = run.returncode == 0 and run.stdout.strip() == expected(depth)
    if ok:
        kind = "pass"
    elif "overflowed its stack" in run.stderr:
        kind = "stack-abort"
    elif "RangeError" in run.stderr:
        kind = "RangeError"
    else:
        kind = "other-error"
    return {
        "depth": depth,
        "ok": ok,
        "kind": kind,
        "exit": run.returncode,
        "stdout": run.stdout,
        "stderr": run.stderr[:500],
    }


def measure(binary, case, cap):
    source, expected = CASES[case]
    trace = []

    def test(depth):
        result = probe(binary, source, expected, depth)
        trace.append(result)
        return result["ok"]

    assert test(0), (case, trace)
    low, high = 0, 16
    while high <= cap and test(high):
        low, high = high, high * 2
    if low == cap:
        return {"largest_pass": low, "ceiling": False, "trace": trace}
    high = min(high, cap)
    if high == cap and low < high and test(high):
        return {"largest_pass": high, "ceiling": False, "trace": trace}
    while high - low > 1:
        middle = (low + high) // 2
        if test(middle):
            low = middle
        else:
            high = middle
    failure = next(result for result in reversed(trace) if not result["ok"])
    return {
        "largest_pass": low,
        "first_failure": high,
        "failure_kind": failure["kind"],
        "ceiling": True,
        "trace": trace,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--cap", type=int, default=4096)
    parser.add_argument("--cases", nargs="*", choices=list(CASES), default=list(CASES))
    args = parser.parse_args()
    if args.cap <= 0:
        parser.error("--cap must be positive")
    results = {
        "label": args.label,
        "features": "default",
        "host_stack": "ordinary CLI main thread; no stack override",
        "measure": "depth/exit/semantic-output only; no timing",
        "profiles": {},
    }
    for profile in ["debug", "release"]:
        binary = Path(args.root) / profile / "qjs"
        rows = {}
        results["profiles"][profile] = {
            "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "cases": rows,
        }
        for case in args.cases:
            row = measure(binary, case, args.cap)
            rows[case] = row
            Path(args.output).write_text(json.dumps(results, indent=2) + "\n")
            print(args.label, profile, case, row["largest_pass"],
                  row.get("failure_kind", "cap"), flush=True)


if __name__ == "__main__":
    main()

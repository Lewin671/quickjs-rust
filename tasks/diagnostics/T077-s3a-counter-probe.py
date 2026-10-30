#!/usr/bin/env python3
"""Reproduce S3a review counters on development inputs only; no timing.

Run at repository root with PYTHONPATH=. and a perf-counters CLI. This validates
both complete-map repeats, stdout/checksums and the native/non-native census.
"""

import hashlib
import json
import os
from pathlib import Path
import subprocess

from tools.benchmark.screen import DEFAULT_CACHE, resolve_cases


def main():
    work = Path("target/t077-s3a-review/programs")
    work.mkdir(parents=True, exist_ok=True)
    cases = resolve_cases(["development"], DEFAULT_CACHE, work)
    baseline = json.loads(Path("tasks/diagnostics/T077-s3-counts.json").read_text())
    assert [case.id for case in cases] == [row["id"] for row in baseline]
    env = {key: value for key, value in os.environ.items() if not key.startswith("QJS_")}
    rows = []
    for case, old in zip(cases, baseline):
        assert hashlib.sha256(case.workload.read_bytes()).hexdigest() == old["sha256"]
        previous = None
        for repeat in range(2):
            run = subprocess.run(
                ["target/t077-s3a/counter-build/release/qjs", "--raw", str(case.workload)],
                capture_output=True, text=True, env=env, timeout=600,
            )
            assert run.returncode == 0 and run.stdout == case.expected_stdout, (case.id, run.stderr)
            counters = {}
            for line in run.stderr.split("QJS_PERF_COUNTERS\n", 1)[1].splitlines():
                fields = line.split()
                if len(fields) == 2 and fields[1].isdigit():
                    counters[fields[0]] = int(fields[1])
            if previous is not None:
                assert counters == previous, (case.id, repeat)
            previous = counters
        assert counters["native_direct_entry"] == counters["native_calls"] == old["counters"]["native_calls"], case.id
        for name in ["argument_vec_native_pop", "argument_vec_tier_fallback",
                     "argument_vec_slice_fallback", "native_family_probes", "native_family_declines"]:
            assert counters[name] == 0, (case.id, name)
        borrowed = sum(counters[name] for name in [
            "native_stack_call_borrow", "native_stack_construct_borrow",
            "non_native_stack_call_borrow", "non_native_stack_construct_borrow",
        ])
        assert old["counters"]["argument_vec_pop"] - counters["argument_vec_pop"] == borrowed
        rows.append({
            "id": case.id, "sha256": old["sha256"],
            "expected_stdout": case.expected_stdout, "repeats_equal": True,
            "counters": counters,
        })
        print(case.id, "two stdout/checksum/full maps and transport identities match", flush=True)
    Path("target/t077-s3a-review/reproduced-counts.json").write_text(
        json.dumps(rows, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()

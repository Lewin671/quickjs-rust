# Public workload correctness

`tools.benchmark.public_correctness` is a semantic gate over every broad,
sentinel, external and development workload, using the inventories and contracts
listed in [benchmarking](benchmarking.md). It never resolves or runs held-out
programs and produces no counters, calibration, timing metrics or performance
verdict. It can run locally without an iMac timing reservation.

Build the same candidate in release and with optimized verification checks, then
run from the repository root:

```sh
cargo build --release -p qjs-cli
CARGO_TARGET_DIR=target/public-verify cargo build --release -p qjs-cli \
  --config profile.release.debug-assertions=true \
  --config profile.release.overflow-checks=true
python3 -m tools.benchmark.public_correctness \
  --binary target/release/qjs --verify-binary target/public-verify/release/qjs \
  --output-dir target/public-check
```

The verification binary must enable debug assertions and overflow checks.
Optimization keeps full public programs within their process deadlines without
disabling verification. Default and IC-off use release; IC-verify uses that
assertion-enabled binary. Inherited QJS
flags are cleared, the selected mode's flag is set, and TZ is UTC. Internal
cases run at their manifest's initial iteration count and twice that count,
capped at its maximum; their complete result line is checked against the
declared identity, operation count and closed-form checksum. Whole external and
development programs run intact against their existing expected stdout, including
development checksums. Existing resolvers hash-check fetched source artifacts.

Use a fresh output directory per attempt. `request.json` records all case IDs,
mode flags, producer/source/binary/input hashes before execution. `cases.jsonl`
retains every invocation, expected/actual stdout and process failure. Checks
continue after failures so `summary.json` accounts for the entire corpus in all
three modes. Any wrong output, process error or timeout makes the command fail.
Input or binary mutation also fails the check. Failed attempts are never
overwritten or selected away. Source bundles can be regenerated through the
pinned resolvers; raw process output and all conclusions stay in the report.

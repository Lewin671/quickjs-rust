# S2 frozen-source correctness qualification

Lead post-round4 decision treats the five-pair external 1.0016 as within triage
noise and runs formal 30-block/held-out/RSS acceptance independently on the iMac.
This owner qualifies candidate fc3deef7, runtime dc710e4b + order 4098ee67.
No runtime/build/harness source changes, origin push, merge or iMac job submission.

## Reproduction and corpus routing

The requested TEST262_DIR is
`/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262`,
pin `4249661388e5d3f92a85186213da140a6481490f`, 53,572 upstream JavaScript files.
The tracked scan/burndown scripts unconditionally assign their worktree corpus,
so the environment alone cannot route them. Temporary copies under
`target/t077-s2-freeze/routed-harness/` change only ROOT_DIR (back to this frozen
worktree), TEST262_DIR (honor the requested environment), and the front-end's
BASELINE path (the routed copy). No selection, timeout, expectation or execution
semantics change. [Hashes and exact path-only diffs](T077-s2-freeze-routing.json).

Build an isolated release CLI with `cargo build -p qjs-cli --release --features
agents --target-dir target/t077-s2-freeze/agent-cli-build`, pin it, hash it and
prove --agent with `__quickjsRustAgentStart(""); "agent-ready";`.
[Pin, toolchain, agent smoke and passing one-case preflight](T077-s2-freeze-pin.json).
Run both routed front-ends with --exact --all, QJS_AGENTS=1,
QJS_CLI_PROFILE=release, TEST262_CASE_TIMEOUT_SECONDS=20,
TEST262_BASELINE_JOBS=4, the same pinned QJS_CLI_BIN and shared TEST262_DIR.
IC-off additionally sets QJS_DISABLE_IC=1; cached unsets it and QJS_VERIFY_IC.
Reports are fresh exact-cached and exact-off directories in target/t077-s2-freeze.

Burndown rows are appended only after complete scans. The ledger permits one
row per commit: cached names fc3deef7; IC-off names documentation-only
preflight descendant 4a6cbab8, with exact engine/CLI/harness input identity proved by
git diff. Both execute the same fc3deef7 pin. No incomplete scan is a result.

## Complete results

Both commands completed successfully (exit 0) on 2026-10-31. Each unfiltered,
all-case report covers 53,572 upstream files, 42,672 configured/runnable cases
and 10,900 unchanged NG configuration skips (10,621 feature, 279 fixture).

| Mode | Rust pass | Fail | Timeout | Not run | Actionable gaps | Burndown commit |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Cached | 42,672 | 0 | 0 | 0 | 0 | fc3deef7 |
| IC-off | 42,672 | 0 | 0 | 0 | 0 | 4a6cbab8 |

QuickJS-NG records 42,602 passes and 70 failures in each run; Rust passes all
70 reference-only failures. This matches the previous configured pass standing,
with no lost passes or new exclusions. [Cached summary](T077-s2-freeze-cached-summary.json),
[IC-off summary](T077-s2-freeze-off-summary.json), [result/hash/ledger receipt](T077-s2-freeze-results.json).

The two complete 53,572-row case-result JSONL files are byte-identical, SHA
`2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
The fresh release CLI's unchanged SHA is
`f8dbeab935c0376b12f0a26cc954911226ba9d5c6329fca6aa9b771e2dd9aeab`.
The shared corpus remains clean and at its recorded pin after cleanup.

Both complete burndown rows were appended with the path-routed original
recorder. Cached records fc3deef7. IC-off records 4a6cbab8 only because the
ledger allows one row per commit: `git diff --quiet fc3deef7 4a6cbab8 -- crates
scripts Cargo.toml Cargo.lock .cargo rust-toolchain.toml` returns 0. Both modes
execute the same pinned fc3deef7 binary and the same frozen harness bodies;
only IC policy differs. The worktree-to-fc3deef7 input comparison also returns 0.
No runtime, CLI, build, tracked harness, corpus pointer or exclusion changed.

The owner submitted no iMac jobs, origin push or merge. Lead formal 30-block,
held-out and RSS acceptance remains independent and is not claimed here.

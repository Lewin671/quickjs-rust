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
Run both routed front-ends with --exact --all, QJS_AGENTS=1,
QJS_CLI_PROFILE=release, TEST262_CASE_TIMEOUT_SECONDS=20,
TEST262_BASELINE_JOBS=4, the same pinned QJS_CLI_BIN and shared TEST262_DIR.
IC-off additionally sets QJS_DISABLE_IC=1; cached unsets it and QJS_VERIFY_IC.
Reports are fresh exact-cached and exact-off directories in target/t077-s2-freeze.

Burndown rows are appended only after complete scans. The ledger permits one
row per commit: cached names fc3deef7; IC-off names this documentation-only
preflight descendant, with exact engine/CLI/harness input identity proved by
git diff. Both execute the same fc3deef7 pin. No incomplete scan is a result.

## Status

Preflight records no scan outcome. Complete results, case hashes and ledger
provenance will replace this block after both modes finish. Lead acceptance
measurements remain independent; this owner submits no iMac jobs.

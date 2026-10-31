# S2 exact checkpoints and corrected CLI agent pin

Qualified 061e7f28 complete cached/off scans pass 42,672 configured cases each,
zero failures/timeouts/gaps, 53,572 total records and 10,900 unchanged skips.
Both case JSONL hashes are 2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a.
Release CLI SHA ff1847b15a326818b252dbba5610430609635effec3b120b61dc4b1a4a13c349;
its `--agent` smoke test succeeds. Cached ledger row is 061e7f28; off row uses
documentation-only descendant 3bc77a2d (identical runtime/CLI/harness inputs).
These complete scans have appended immutable burndown records.

The later 01ee5ee6 complete cached/off scans each record 42,611 passes and 61
failures, zero timeouts, with the same configured inventory. Every failure is
`TypeError: $262.agent is not available in this context`. These are invalid CLI
qualification runs, not evidence of a runtime pass loss: pin SHA
5efd3b8e5ea1e9fed8b2983ebc7b7d67c9a605301494c5446f71d8dcf3b88072 enables
`qjs-runtime/agents` but omits the separate CLI `agents` feature. Direct
`--agent` smoke reproduces the missing execution context. Both completed failed
scans are retained in burndown at 01ee5ee6 and equivalent doc descendant d8f41ae6,
with explicit invalid-cli source labels. No exclusions or records were edited.

Correct builds use a dedicated output directory, so another Cargo invocation
cannot replace the shared target binary between build and pin:

```sh
cargo build -p qjs-cli --release --features agents --target-dir <dedicated-dir>
<pinned-cli> --raw --agent -e '__quickjsRustAgentStart(""); "agent-ready";'
```

A correct e1bad6bd CLI passes that smoke and all [61 formerly failing configured
paths](T077-s2-r2-agent-rechecks.json). The focused Rust-only Atomics prefix
also executed 101 waitAsync cases outside the unchanged QuickJS-NG-configured
inventory; those remain failures and are not part of the qualified 42,672.
This partial recheck is not a complete scan or burndown record. Final source
still requires fresh complete cached/off scans with the correct pin.

Debug IC comparison requires a debug image: setting QJS_VERIFY_IC on a release
binary alone does not execute debug assertions. Final verified fixture/fuzz
commands explicitly select the debug pin and record its hash.

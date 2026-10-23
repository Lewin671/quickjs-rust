# T071 phase attribution

`phase_probe.rs` uses only public engine APIs. Copy it into an **isolated**
revision checkout as `crates/qjs-runtime/examples/t071_phases.rs`, then build:

```sh
cargo build --release -p qjs-runtime --example t071_phases
```

Use separate checkout and build directories for base and candidate. On macOS:

```sh
python3 tasks/diagnostics/t071-flat/phases.py \
  --base-bin /path/to/base/target/release/examples/t071_phases \
  --candidate-bin /path/to/candidate/target/release/examples/t071_phases \
  --case immutable=/path/to/prepared/immutable.js \
  --case graphlib=/path/to/prepared/graphlib.js --pairs 3 \
  --output /path/to/new-counts.jsonl
```

Inputs are the prepared programs from the recorded quick-triage job; upstream
bundles must remain outside Git. The child blocks at READY/PARSED/COMPILED/
EXECUTED; the parent reads its cumulative instructions through the existing
macOS counter reader. Live blocked reads work although that helper is named
`read_zombie`. Phase subtraction includes barrier overhead and the execution
phase includes realm initialization. It is instruction attribution, not a
qualified performance timing or acceptance run. The output must be a new file.

`.../t071_phases dump /path/to/source.js` prints the full unexecuted Debug IR.
The ordinary compiler regression uses FNV-1a 64 of those UTF-8 bytes, including
the trailing newline. The dump covers nested bytecode, locals, captures,
parameter positions and frame plans. Hashes are tied to `6e899d24`; intentional
IR changes require reviewing and explicitly updating the snapshots.

The owning [T071 record](../../archive/T071-lexical-context-review.md) records source
revisions, phase medians, checksums and interpretation limits.

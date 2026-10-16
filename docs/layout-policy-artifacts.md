# Layout policy and panel artifacts

`benchmarks/policies/<host>-layout-vN.json` is a measured, hash-checked
`quickjs-layout-policy` schema-1 artifact. A schema-3 unit binds its exact
repository-relative `policy_path` and `policy_sha256`; policy changes require
a new version. `host` records machine/OS/CPU/memory/toolchain/linker, Cargo
configuration, canonical recipe and manifest hashes; `host_node` binds the
specific machine. Analysis verifies the recorded host; collectors also
verify the live host. An unqualified policy is never usable.

`cases` maps all 76 selection cases to `wall_band` B_i and
`instruction_band` I_i; `held_out` maps all nine acceptance programs to their
A/A wall bands. `lanes` holds broad, sentinel, external, equal-case portfolio
and held-out ceilings. Qualification retains 30-block intervals and source
hashes. B_i covers A2's two observed layout estimators and global A/A median
bias; I_i covers its rounded A/A instruction envelope. Held-out bands use
rounded reciprocal 95% A/A interval endpoints. These empirical envelopes do
not substitute for confidence intervals. The rule itself is owned by T022.

`layout_gate` freezes `mechanism_class` (`cpu-work` or `instruction-blind`),
`memory_wall_targets` (a subset of targets), `panel_required`, `panel_pairs`
(5), `held_out_pairs` (30) and `formal_blocks` (30 or 60). Instruction-blind
units require nonempty memory targets and `panel_required: true`.

`layout_variants` writes a schema-1 `quickjs-layout-variants` identity, six
immutable executable paths and nm/disassembly/normalized-body audits. The
fixed NOP sizes are natural/16/64/240/512/1008; compiler alignment may round
one shift by less than 32 bytes. Every original text symbol must keep its
order, normalized instruction body and a single uniform displacement.
Relocated branches/RIP references resolve to the same symbols/section
offsets; only the last function's trailing section filler may differ.
Any drift refuses timing. Only x86_64 macOS release builds are supported.

`layout_panel` writes `quickjs-layout-panel` schema 1: source revisions,
all binary hashes/audits, exact paths, host, preflight and raw case hashes.
`panels` contains natural only or exactly six ordered matching-layout screens;
held-out has one natural screen of 30 pairs. Raw case packets retain every
sample, separate calibration, per-role costs and paired ratios. Replay
recomputes all ratios, validates complete inventories and binds natural
binary hashes/revisions to the sealed formal bundle. It refuses missing,
short/nonpositive increments, altered paths, checksums, samples or hashes.
`panel.json` stays immutable; completion adds `complete.json`.

Decision schema 5 carries `layout_verdict`, raw-replayed formal statistics
and policy/panel/held-out SHA-256 bindings. Case effects are paired-ratio
medians; lanes/portfolio are equal-case geomeans. Their 95% intervals jointly
resample block IDs with 20,000 draws, seed 0, and require >=30 blocks and
<=3% relative half-width. Promotion checks held-out upper intervals against
its program/lane bands as well as point estimates. Legacy decisions retain
their own schema and statistics. Shared formal snapshots use padded role
names (ten characters) and exclusive creation; measurement protocols are
throughput v10, sentinels v3 and resources v3.


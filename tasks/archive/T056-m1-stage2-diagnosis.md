# T056 M1.2: diagnosis of the rejected layout-authority shape

- Status: closed (diagnosis complete). The rejected shape stays closed. No runtime change is
  retained from this work.
- Implementation: rejected candidate `873c9bc5` on `agent/t056-m1-stage2/codex`. The
  ablations are diagnosis-only commits on `agent/t056-m12-diagnosis/claude`, which is never
  merged.
- Verified at: candidate `873c9bc5` against stage 1 `ab9c29b6` and plan base `351004f3`,
  2026-10-20. The iMac jobs are listed below.
- Evidence: this record. Raw job directories are in the workspace under
  `outputs/imac-bench/<job>/`, and scripts and inputs are in `tmp/t056-m12-diagnosis/`.
  Both are local artifacts, not retained in Git.
- Unresolved: the new shape in [property layouts](../../docs/design/property-layouts.md)
  depends on T063 canonical string storage and needs a fresh queue, profiles and a frozen plan.
- Next action: none here. [T056](../T056-interpreter-core-v2.md) owns the next plan.

## Rejection being explained

The sole stage-2 screen was job `20261018T055933Z-t056-m1s2-screen1`:
- frozen plan `interpreter-core-v2-atoms-layouts`, stage 2/3, candidate `873c9bc5` against
  `351004f3`;
- final v4 `reject`: 42 of 86 cases fail, and the external, development, sentinel and
  portfolio lanes exceed their ceilings on all six layouts;
- not DISTURBED.

The full record is [T056-m1-stage2-evidence](T056-m1-stage2-evidence.md) (imported unchanged
from that branch at `c6c06bca`). The slowest cases rose in instructions as much as in wall
time:
- json-parse-financial 5.65x;
- raytrace-public-class-fields 3.77x;
- access-binary-trees 3.32x;
- date-format-xparb 2.32x;
- cdjs 1.57x;
- hash-map 1.36x.

## Reproduction and mechanisms

Plain release builds, instructions retired from `time -l`, non-timing. iMac x86_64 job
`20261020T062939Z-t056-m12-instr-diagnosis` reproduces the screen's instruction medians:

| Workload | s2/s1 here | Screen |
| --- | ---: | ---: |
| json-parse-financial | 5.643 | 5.6513 |
| raytrace-public-class-fields | 3.766 | 3.7667 |
| access-binary-trees | 3.327 | 3.3218 |
| date-format-xparb | 2.321 | 2.3245 |
| cdjs | 1.567 | 1.5652 |
| hash-map | 1.357 | 1.3556 |

Stage 1 has no per-object layout:
- `Small` stores compare text linearly and append with `Vec::push`;
- `Shaped` stores share `ObjectLiteralShape`;
- atoms serve only bytecode linking.

Stage 2 keeps that payload work and adds a second, value-republished layout. The diagnostic
`perf-counters` additions (commit `29bf491c`) attribute the added work as follows:

| Mechanism | Source | Counter evidence |
| --- | --- | --- |
| Weak shape cache and weak hints. Intermediate shapes die, and every object re-hashes (SipHash), allocates and re-indexes them. | `property/layout.rs` `ShapeCache::intern`, `make_index`, `publish` | class_new: 2 misses per object (the 2 intermediate shapes), 16 hint hits in 600,770 transitions |
| Every append clones and compares the full field vector. | `PropertyLayout::{append,reconfigure,remove,change_encoding}`, `Shape::matches` | raytrace: 5.5M fields cloned, 22.0M compared |
| A private `AtomDomain` per builder object, then a full relink. | `PropertyStorage::{empty,relink,publish_with_order}` | json_parse: 20,653 domains and 20,002 relinks for 20k objects |
| A private domain per function named table. | `NamedPropertyMap::default` | closures: 200,653 domains for 200k functions |
| Two hashes per text probe. A churned key re-allocates its atom. | `PropertyLayout::lookup`, `AtomDomain::key_identity` | key_churn: 50,645 atoms for 64 keys |

Sampling job `20261020T063654Z-t056-m12-sample-diagnosis` (`--timing`, not DISTURBED) put
26–28% of stage-2 self time in `ShapeCache::intern`, SipHash, index building, `lookup` and
`intern_key`; stage 1 spends 0% there. Raytrace malloc/free self time rose from 12% to 41%.

## Ablations (diagnosis only)

These steps are cumulative on `29bf491c`, after reverting the first-round ablations A and B
(`e415a9d7`, `e8b4de97`). Each step passed the 2,320 runtime tests and a differential check
against QuickJS-NG covering:
- class-field initializers that delete or observe the instance;
- base constructors that return another object;
- equal keys with different attributes;
- delete then re-add, more than 32 properties, freeze/seal, JSON duplicates;
- function and array named properties, getters, prototype chains, churn, literal mutation and
  `eval`.

Steps (v) and (vii) are exceptions (see below). Steps 5 and 8 also passed the NG fixture comparison (249/249) and these
both-engine Test262 slices, with 0 cases where QuickJS-NG passes and we do not:
- class elements (1531 statement and 1426 expression cases);
- `Object.defineProperty` (1131), `freeze`, `keys`, `getOwnPropertyNames`, `assign`;
- `delete`, `JSON.parse`, object expressions (1170).

| Step | Commit | Change |
| --- | --- | --- |
| (i) | `dd81dbb2` | Edge-labelled append transitions; strong per-encoding roots |
| (i-b) | `54e42646` | Builders intern into the prototype's domain; no relink |
| (ii) | `c83e15c0` | Named tables create their layout at the first install |
| (iii) | `c746b08c` | Text lookup without hashing for shapes of up to 12 fields |
| (iii-b) | `5feb3f90` | Large indexes keyed by atom text; one hash per probe |
| (iv) | `6f2d5885` | Per-domain literal shape slot; bulk edits reconfigure only changed slots |
| (v) | `9c47a521` | Appends do not count as dictionary edits; dictionary at more than 128 fields |
| (vi) | `785610ad` | Edge hits compare key text before interning |
| (vii) | `d6c1fdfa` | Parents own their edges strongly; children hold no parent |
| (viii) | `afb88151` | Payload re-encodings are edges |

x86 s2/s1 instruction ratios. Jobs `20261020T072701Z-t056-m12-steps-instr` and
`20261020T075843Z-t056-m12-steps-6-8-instr`; binaries are hashed in each job's
`binaries.sha256`.

| Workload | s2 | (i) | (i-b) | (ii) | (iii-b) | (iv) | (v) | (viii) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| json-parse-financial | 5.639 | 4.594 | 1.334 | 1.335 | 1.282 | 1.288 | 1.289 | 1.274 |
| raytrace | 3.772 | 1.475 | 1.470 | 1.470 | 1.374 | 1.388 | 1.387 | 1.328 |
| access-binary-trees | 3.327 | 1.517 | 1.516 | 1.513 | 1.364 | 1.380 | 1.386 | 1.380 |
| date-format-xparb | 2.321 | 2.148 | 1.611 | 1.611 | 1.536 | 1.546 | 1.551 | 1.158 |
| cdjs | 1.566 | 1.152 | 1.144 | 1.144 | 1.113 | 1.083 | 1.083 | 1.080 |
| hash-map | 1.357 | 1.084 | 1.073 | 1.073 | 1.046 | 1.062 | 1.061 | 1.060 |
| micro class_new | 2.437 | 1.329 | 1.326 | 1.328 | 1.219 | 1.226 | 1.225 | 1.222 |
| micro json_parse | 7.861 | 6.030 | 1.338 | 1.337 | 1.334 | 1.339 | 1.341 | 1.301 |
| micro closures | 1.324 | 1.340 | 1.336 | 1.052 | 1.028 | 1.030 | 1.030 | 1.032 |
| micro literal_objects | 1.355 | 1.308 | 1.307 | 1.309 | 1.271 | 1.039 | 1.039 | 1.042 |
| micro grow_object | 1.802 | 1.592 | 1.591 | 1.591 | 1.600 | 1.602 | 3.066 | 1.126 |
| micro key_churn | 1.093 | 1.092 | 1.090 | 1.090 | 1.063 | 1.066 | 1.066 | 1.022 |
| micro dynamic_read | 1.087 | 1.086 | 1.083 | 1.084 | 1.046 | 1.052 | 1.048 | 1.054 |
| micro static_read (control) | 1.018 | 1.013 | 1.013 | 1.013 | 1.014 | 1.014 | 1.017 | 1.017 |

Findings:
- **Closures under ablation A.** The first-round ablation A made closures worse
  (1.327→1.388). A added a `Vec` of roots to every private per-function domain. Step (i), which
  keeps roots in a fixed array, holds closures at 1.340, and step (ii) brings them to 1.05.
- **grow_object at step (v).** Step (v) exposed that a re-encoded `Small`→`Dynamic` shape sat
  outside the edge tree, so all later appends missed. Step (viii) fixes this.
- **Chain death.** Step (vii) shows that weak child edges let chains die with the last object.
  Strong parent edges fix date-format-xparb (1.55→1.16).
- **Dictionary test at step (v).** Step (v) changes the policy that 64 appends make a
  dictionary. `dictionary_edits_preserve_reserved_buffers_and_insertion_order` therefore reaches
  the dictionary through 32 reconfigurations instead; its assertions are unchanged.
- **Retention test at step (vii).** The one test failure at (vii),
  `dictionary_order_tombstones_stay_bounded_under_fixed_live_churn` (live atoms 2, expected 1),
  is the bounded retention that strong edges imply. Its new expectation is in the design.
- **Remaining 1.2–1.4.** Sampling job `20261020T080259Z-t056-m12-step8-sample` (`--timing`, not
  DISTURBED) attributes it to:
  - `PropertyLayout::lookup` (5.8–7.9% of self time);
  - `canonical_property_text` (3.7–5.0%);
  - `append` (2.7–6.1%);
  - `insert_new` and `with_prototype_slot`.
  An add canonicalizes three times and probes twice. A measurement-only build without those
  scans (not committed, and wrong for halves-encoded keys) saves another 7–10%: class_new 0.919,
  raytrace 0.904 and binary-trees 0.908 relative to step (viii), on local arm64. T063's
  canonical storage makes the scan unnecessary for typed keys. One probe per add removes the
  second lookup.
- **Not tried.** `ObjectData` slimming (144 bytes against stage 1's 112) is designed in the note
  above but was not measured.

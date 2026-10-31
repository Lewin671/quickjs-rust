# T077 S2 follow-up round 2: late iteration history

- Status: closed diagnostic record; lead round 3 freezes 40814de6 and ends A/order iteration.
- Implementation: source presence and outcomes below are revision-bound; current qualification is in the [active record](../diagnostics/T077-s2fu-round2.md).
- Verified at: the named revisions/jobs below; these are not claims about later HEAD.
- Evidence: committed diagnostic receipts and job names below.
- Unresolved: final gates belong to the active record.
- Next action: follow the lead's freeze and qualification order; no further optimization here.

## Retained-screen decision and next guard stage

Job `20261031T151127Z-t077-s2fu-r2-retained-public86`, f05baa01: 430 valid,
zero invalid/heavy pairs. Wall/instructions: development 0.998221/0.997611;
external 1.002905/1.001250; broad 0.987326/0.998252;
sentinel 0.978639/0.996009. External remains open. The corrected native
collector accepts the first undisturbed retained-source replay; results are
under the native-retained receipt pending final source selection.

Next classify shared monomorphic own-data entries in an unused existing flags
bit at admission. Their metadata is immutable, so a direct identity guard
suffices, as in SharedOwnPoly; keep private/versioned entries on the original
guard and keep shared-own masks branch-free. Entry/CacheData/receiver sizes
must not grow. A dictionary-conversion test must retire the shared witness,
then prove private-epoch invalidation after delete/re-add. No cached JS values,
prototype snapshots or binding-model changes are introduced.

The witness fast gate passes 2,525 runtime tests, 65 selected Test262 cases and
clippy. Shared generic object guards retain the cold-seed fallback; only
monomorphic shared own-data hits omit epochs entirely. Versioned own hits keep
the original guard in a separate adapter; no private-dictionary or global
binding guard is weakened. The task-index budget issue from the archive
checkpoint is repaired in this commit and the fast gate is rerun successfully.

The shared-witness focused replay has no clear overall gain (date-tofte
1.004163, binary trees 1.008911, JSON parse 1.005448, diff 0.996534).
Collect measured operation/allocation counters on retained 4c6fc5c6 for
access-nsieve, binary trees, date-tofte and JSON parse before any further hot
adapter changes. The instruction-only suite was missing the largest residual
(nsieve 1.016722 in the retained full screen); add explicit case selection to
the existing diagnostic counter collector. Counters remain non-timing evidence.

Use a complete public86 screen of the shared witness as well as the focused
probes: the explicit target is aggregate per-lane neutrality, and the ten
probes do not cover the whole external lane. Apply only its generated default
order from `20261031T160025Z-t077-s2fu-r2-shared-witness`. The retained-source
operation-counter job is serialized ahead of that full screen.

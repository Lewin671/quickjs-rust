# Unapplied S3b hostname qualification proposal

**Approval pending. This patch has not been applied. No valid public86 timing
result exists.** Engine candidate remains `ca5ab643`, baseline main `3cc12c6f`.

The [frozen qualification audit](../T077-s3b-host-qualification/README.md) records
an exact match of all hardware/OS/toolchain/recipe/configuration/manifests fields,
but expected `iMac.local` versus actual `iMac.lan` on the same specified runner.
A later read-only SSH hostname check still returns `iMac.lan`.

The [proposed patch](proposal.patch) accepts only that exact expected/actual pair
in the loader's hostname guard, retaining the entire identity equality check.
It does not edit a policy, threshold, inventory, recipe or requested endpoint.
Quick triage continues recording the actual `platform.node()` in its sealed
request. Formal/held-out/RSS remain lead-owned, with their existing independent
evidence validation; this proposal does not authorize or bypass those gates.

[Offline checks](check-proposal.py) execute the proposal in a separate module
namespace against generated unit-policy fixtures, never replacing the live
module or running benchmarks. All **18 checks** pass: original name, exact alias,
unrelated names, reverse mapping rejection and each of thirteen identity fields
independently mismatched. Accepted policy values remain identical. The original
source is asserted unchanged. [Results](offline-results.json) and
[hashes](metadata.json) are retained.

Reproduce from the repository root:

```sh
PYTHONPATH="$PWD" python3 tasks/diagnostics/T077-s3b-host-alias-proposal/check-proposal.py
```

The asynchronous question asks whether to approve this logged alias-only guard
change or supply another qualified policy. Posting that question is not approval;
elapsed time is not approval. Continue independent complete exact checks while
waiting. No origin push or merge into main.

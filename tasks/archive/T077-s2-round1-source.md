# T077 S2 round-1 frozen source

Source checkpoint, 2026-10-30; performance standing is diagnostic only.
Engine source: `e1d8a6a382412bf62fbf7db1f7726c909f446e6a`.

- Runtime tree: `4be4522f2de4f89b86e47b506402d2f3aaf3f163`.
- CLI tree: `32059389a6725a6f8448ea57aebd8de699df254b`.
- Test262: `4249661388e5d3f92a85186213da140a6481490f`.
- QuickJS-NG: `f7830186043e4488f2998759d60a514faf07cbc9`.
- arm64 release agents CLI SHA-256: `c5c306cdf29331858398fb8347a1326af929fbd512855bd829bf4df8a53443cf`.
- arm64 release counter CLI SHA-256: `ba86661abf6ab8c23b518e488f06db75453b1c8beb93998f38e460c58b862f4e`.
- arm64 debug verifier CLI SHA-256: `c3a53f2bece0b37a012d8cad7c5579e4d482610af30e30d86accfe15b925cb5d`.

The full gate and 266 verify-mode NG fixtures passed on this source. Final exact
and IC-off scans are pending at this checkpoint. The IC-off scan uses this
checkpoint's documentation-only descendant for the one-record-per-commit ledger;
its runtime/build/harness inputs must match the source above. No acceptance or
complete-scan result is claimed here. Final results belong in the round-1 receipt.

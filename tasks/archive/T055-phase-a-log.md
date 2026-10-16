# T055: Phase A results and blocker (2026-10-16)

Frozen plan commit: 82a21c6219977b63659f30131e5098d102bf02a1; the design
above is unchanged. All runtime/build inputs start at the recorded source.
No production change, origin push, merge or phase B implementation occurred.

Jobs (workspace outputs/imac-bench/<job>/):

- 20261016T150418Z-layout-noise-matrix-build: exit 1 before timing; the first
  normalizer mistook integer immediates/embedded-data disassembly for relocated
  addresses. Its first identity file is retained. Corrected address recognition
  does not change recipes, padding, ordering or statistical rules.
- 20261016T150643Z-layout-noise-full-gate: full check at 82a21c62 passed, 170 s.
- 20261016T150830Z-layout-noise-matrix-audit-fixed: exit 0, 734 s. All 12
  binaries built; each recipe's 4205 original functions survive, and each of
  its five variants has exactly the declared uniform displacement, original
  order/sizes and relocation-normalized instructions. Zero real static drift.
  R0v0/R1v0 each pass 5170 subset cases and all 247 NG fixtures, UTC/C locale.
- 20261016T151909Z-layout-noise-loaded-metadata: exit 0; all six corresponding
  R0/R1 entire text sections are byte-identical at identical addresses.
- 20261016T152218Z-layout-noise-matrix-timing: cancelled (130) after three
  workers hit the frozen minimum-increment guard. No complete case/matrix.

The iMac is i7-10700 @ 2.90GHz, macOS 12.6 (21G115). R0v0 is the canonical
6081812-byte binary, SHA-256
926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
R1v0 has the same size, SHA-256
543884ecf407d1a831e052bcd919f236a0a66f4c67b2baa229be76a16f20781e.
Both natural text sections are 4916832 bytes, SHA-256
6c2e7c07cb9ccd2f4a4b2c2c68358a2c844afee215b9ce6c51bd8d15228fc85b.
All six padded pairs also have identical text; full hashes/sizes are in the
workspace report. Cargo's runtime fingerprint confirms all three R1 flags.
Thus R1 is a machine-code no-op here; the JCC hardware hypothesis remains
untested, and no mitigation is adopted.

| Case | Round (zero-based) | Variant | N-to-2N wall ms |
| --- | ---: | --- | ---: |
| broad/plain_function_call | 1 | R0v4 | -56.458329 |
| broad/captured_write | 4 | R0v2 | -190.853849 |
| broad/method_call | 6 | R0v1 | -162.518805 |

Every offending execution followed a successful 80-84 ms tiny-launch warmup.
For plain_function_call, N wall/user time is 633.976/290 ms and 2N is
577.518/560 ms: extra non-user time corrupts the wall difference. Its specific
system/validation/wait source is not resolved. Worker exceptions were delayed
by executor shutdown waiting for the surviving worker; the queue job was
cancelled after raw-data diagnosis and no child process remained.

Retained partial rounds: plain_function_call 2, method_call 7, captured_write 5,
captured_read 15, each below 60; 0/76 cases complete. All original samples and
checksums remain; none were removed, pooled, or used for a spread/CI/verdict.
The cancelled queue job has no final disturbance classification (observed
intruder samples total at most 82% CPU, below its 200% threshold).

Per-case spreads, all lane geomeans, precision and the predeclared noise-
reduction reading are unavailable. Complete artifact/table inventory and
blocking evidence: workspace tmp/layout-noise-control/REPORT-phase-a.md and
outputs/layout-noise-control/blocked-result.json. These are supplementary;
the conclusions, numbers and revisions are recorded here.

The brief forbids changing the frozen design after seeing results. Obtaining
a valid band now requires a newly frozen sampling/path-warmup protocol and
prompt worker-error termination, first checked by A/A. That decision belongs
to the lead; no replacement timing, additional recipe or favorable rerun was
silently substituted. Phase A remains blocked, rather than claimed complete.

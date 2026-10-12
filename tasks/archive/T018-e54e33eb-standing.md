# T018: Formal standing at e54e33eb

- Status: historical-campaign. Superseded standing, not current priority.
- Implementation: engine at `e54e33eb`; later integration state is in T037.
- Verified at: `e54e33eb`, 2026-10-11, 30-block comparison against `d1313d52`.
- Evidence: original standing below; raw bundle and queue are local artifacts,
  not retained. Their hashes bind the recorded conclusions.
- Unresolved: original audio-dft precision was insufficient for promotion.
- Next action: use current T018 standing and its fresh queue.

## Standing at e54e33eb

Candidate `e54e33eb9371c5cb4627bd8e590a648085a7fa26`, base `d1313d52`,
NG `f7830186`. Same-host diagnostic, not fixed-hardware qualification.
Command: `./scripts/perf-compare.sh --base d1313d52 --candidate e54e33eb
--blocks 30 --output-dir target/comparison/brief01-e54e33eb-20261011`.
Bundle `summary.json` SHA-256:
`febc1b2f4904f113eca4ca4850630012e4e08ded0f85829ace3c91c9f48b6955`.
Queue `target/comparison/brief01-e54e33eb-20261011-queue.json` SHA-256:
`d064c7e9a06d2e7d2d998a26bc1d0febb0dc7e8177ccb7508d0d0f7108442e8c`.
Profile index: `target/profiles/brief01-e54e33eb/evidence-index.json`.

| Lane | Wall candidate / NG | Cycles candidate / NG |
| --- | --- | --- |
| Broad (25) | 0.9701 | 0.9690 |
| Sentinel (6) | 0.8583 | 0.8588 |
| JetStream JS subset (5) | 0.9864 | 0.9852 |
| Kraken (14) | 0.6047 | 0.6013 |
| SunSpider (26) | 1.0224 | 1.0206 |

External >1.10 wall/NG, in queue order: `bitops-bitwise-and` 4.2509,
`date-format-tofte` 1.3974, `hash-map` 1.3581, `cdjs` 1.3488,
`3d-raytrace` 1.3340, `access-binary-trees` 1.3058,
`string-validate-input` 1.2447, `crypto-md5` 1.2326,
`raytrace-public-class-fields` 1.1877, `crypto-aes` 1.1091.
Sentinel wall/NG: recursive 0.7455, prototype 0.9930, polymorphic 0.8188,
capturing 0.7179, heterogeneous 0.999928, churn 0.9190. None >1.0;
heterogeneous CI [0.99697, 1.00241] does not establish stable <=1.0.

Decision readiness: **inconclusive**, not promotion evidence. The only
imprecise case is audio-dft: candidate/base cycles CI [0.96136, 1.02474],
relative half-width 4.25% >3%. The complete queue is usable for selection.

Linearity diagnosis: old d1313d52 normalized wall probes 1.168/1.245 had
instructions 0.9985/0.9995 and cycles 1.0168/1.0038. Fresh probes of that
same base executable give 0.9894/0.9950, HEAD 0.9972/0.9988; all 75 probe
groups pass. Host timing interference is supported, its specific source
unproven. Neither whole-work elimination nor N/2N tier switching explains
it: many_locals_call has 1003/2003 call attempts at N=1000/2000;
property_read has 1000/2000 backedges and 12290/24290 interpreter ops.
The latter uses virtual object slots and declines typed entry; it is not
an ordinary property-cache control. A complete quiet-host remeasurement
resolved the failure; no case or threshold change is warranted.


# T018: Case standing at af4d0d3c

- Status: historical-campaign. Frozen case-level snapshot of the named revision.
- Implementation: both batch-1 leaves present, neither promotion retained.
- Verified at: `af4d0d3c`, 2026-10-12, complete locked 30-block comparison.
- Evidence: [T018](../T018-broad-performance.md#standing-at-af4d0d3c) owns
  bundle/queue identity and hashes; complete selected case lists below.
- Unresolved: audio-dft precision; unit decisions and reasons live in T037.
- Next action: follow the active task's follow-up, not this frozen snapshot.

Cases over the standing ceilings in either metric: external >1.10,
sentinel >1.0, broad >1.5. Lower is faster; these are the complete lists.

| Case | Wall / NG | Cycles / NG |
| --- | ---: | ---: |
| `external/jetstream3-js-subset/cdjs` | 1.355981 | 1.354902 |
| `external/jetstream3-js-subset/hash-map` | 1.365174 | 1.361951 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 1.116547 | 1.121438 |
| `external/sunspider-1.0/3d-raytrace` | 1.331227 | 1.349071 |
| `external/sunspider-1.0/access-binary-trees` | 1.306625 | 1.332032 |
| `external/sunspider-1.0/access-nbody` | 1.096721 | 1.107219 |
| `external/sunspider-1.0/crypto-aes` | 1.115539 | 1.114732 |
| `external/sunspider-1.0/crypto-md5` | 1.234213 | 1.260205 |
| `external/sunspider-1.0/date-format-tofte` | 1.422100 | 1.435574 |
| `external/sunspider-1.0/date-format-xparb` | 1.128172 | 1.139950 |
| `external/sunspider-1.0/string-validate-input` | 1.231028 | 1.249889 |
| `sentinel/heterogeneous_property_read` | 1.007156 | 1.005879 |
| `sentinel/prototype_method_call` | 1.012212 | 1.008629 |
| `broad/array_read` | 2.627011 | 2.597424 |
| `broad/captured_write` | 1.795043 | 1.798010 |
| `broad/dynamic_method_call` | 2.425130 | 2.406204 |
| `broad/property_dynamic_read` | 1.594014 | 1.602920 |
| `broad/property_read` | 4.481405 | 4.455892 |
| `broad/property_write` | 3.039413 | 3.032958 |


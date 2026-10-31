# T077 S2 final creation-proof counter replay

Executable source: `fc64629a975431cd94cc22df2930bf49ad9593a8`, staged tree `d7cc512a089b65d7d8252d619f597daa9549f819`.
Agents correctness CLI SHA-256: `5181de100e8c5b838c1fe0ada28e620b2c9713e646d21080f0762f407ccfa717`.
Final perf-counters CLI SHA-256: `c41a7e6e091b65c67fd74c9d2ce8feb9bd32a263406e21a5c746b3fc8ed2deab`.
Development manifest SHA-256: `2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.

The counter CLI was built from the working tree committed unchanged as this
source; the collector captured predecessor HEAD `884d3ba2` while the commit
hook was still running. The recorded staged tree exactly matches `fc64629a`.
Each of the same ten pinned development programs was replayed once; every
checksum matches. No timings, external, selection or held-out inputs collected.

Every per-program family IC event in the [first S2 receipt](T077-s2-counters.md)
is unchanged in this replay (all fields compared, including zeros). Keep that
receipt frozen: it establishes the earlier candidate, and this replay establishes
the same family-event table for the final source. Instrumentation limitations
there apply here too. Source bytecode dispatch reads, atoms, interning, transitions
and field-copy counts are unchanged; creation proofs remove additional generic
prototype-chain probes on the deeper immutable constructors.

| Counter | S1 base b2daec46 | Final S2 | Ratio |
| --- | ---: | ---: | ---: |
| named_property_reads | 54,553,596 | 54,553,592 | 1.0000 |
| property_text_probes | 71,290,034 | 53,798,888 | 0.7546 |
| property_id_probes | 76,273,957 | 55,954,184 | 0.7336 |
| property_key_interns | 1,037,872 | 1,160,194 | 1.1179 |
| property_atoms_created | 1,035,713 | 1,049,956 | 1.0138 |
| property_shape_field_copies | 856,221 | 977,767 | 1.1420 |
| property_transition_hits | 4,364,214 | 4,615,283 | 1.0575 |
| property_transition_misses | 44,253 | 46,773 | 1.0569 |

Only these per-program authority counters changed relative to `fc5da912`:

| Program | Counter | First S2 | Final S2 |
| --- | --- | ---: | ---: |
| immutable/persistent | property_text_probes | 5,510,678 | 3,059,056 |
| immutable/persistent | property_id_probes | 4,596,747 | 2,145,125 |

This is work-count evidence, not acceptance or a speed claim. The final source
still requires iMac triage against `dd0905b2` and lead-owned formal comparisons.
No iMac timing job was submitted because its temporary-ref push conflicts with
the unclarified no-push instruction.

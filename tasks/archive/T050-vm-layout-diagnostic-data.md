# T050: Complete layout diagnostic tables

Supplement to [the frozen study record](T050-vm-layout-causality.md). All values
are medians over 15 complete balanced blocks; no valid run was filtered or
repeated to obtain favorable data. Ratios above one mean increased wall cost.
This is diagnostic evidence, not a performance-unit promotion.

## All ordinary-VM factorial cases

| Case | Move VM, base reader | Move VM, candidate reader | Reader at 0x020 | Reader at 0x270 | A/A |
| --- | --- | --- | --- | --- | --- |
| broad/branch_arithmetic | 1.009886 | 0.996299 | 1.001748 | 0.988614 | 1.001707 |
| broad/dynamic_method_call | 0.998289 | 1.016631 | 1.001796 | 1.019421 | 1.007415 |
| broad/global_read | 1.002005 | 0.986418 | 0.996124 | 0.999678 | 0.992803 |
| broad/local_read | 1.002535 | 0.994903 | 1.006089 | 1.003721 | 1.003838 |
| broad/property_dynamic_read | 1.026281 | 0.995092 | 1.008603 | 1.002853 | 1.025075 |
| broad/property_read | 0.996244 | 1.005118 | 1.006746 | 1.011039 | 0.999299 |
| external/jetstream3-js-subset/cdjs | 1.000077 | 0.998911 | 0.997461 | 0.997243 | 1.002808 |
| external/jetstream3-js-subset/hash-map | 1.003369 | 1.000163 | 0.991720 | 0.988201 | 1.000370 |
| external/sunspider-1.0/3d-raytrace | 0.999190 | 1.008031 | 0.988917 | 0.988282 | 0.997771 |
| sentinel/capturing_closure_call | 0.996801 | 0.999350 | 0.997939 | 1.009873 | 0.994792 |
| sentinel/heterogeneous_property_read | 0.994195 | 1.009506 | 1.009863 | 1.017689 | 0.992453 |
| sentinel/polymorphic_call_site | 1.000933 | 1.000556 | 1.003264 | 0.992268 | 0.993857 |
| sentinel/prototype_method_call | 1.003328 | 1.003486 | 0.994821 | 0.998388 | 1.002182 |
| sentinel/recursive_call_tree | 1.007034 | 0.995803 | 1.011040 | 1.003795 | 0.985767 |
| sentinel/string_key_map_churn | 0.999143 | 0.998703 | 0.996836 | 0.986679 | 0.999003 |

The branch A/A individual ratios span 0.49-2.05 here. These are retained,
not removed. Consequently a small median alone cannot establish neutrality.

## All exact-original serial cases

| Case | Original candidate/base | A/A |
| --- | --- | --- |
| broad/branch_arithmetic | 1.085164 | 1.005284 |
| broad/global_read | 1.125122 | 0.996177 |
| broad/local_read | 1.080216 | 0.999767 |
| broad/property_dynamic_read | 1.039326 | 1.000598 |
| external/jetstream3-js-subset/hash-map | 0.988060 | 1.003056 |

The dynamic-read median ratio is 1.039326, paired-median bootstrap 95%
interval [1.017248, 1.050438]; its A/A median is 1.000598 with interval
[0.957302, 1.018500]. Its cause was not individually isolated. The three
large typed-loop regressions were explained; this is not a claim about all
T049 differences. This remaining attribution does not authorize reopening
the exhausted optimization unit.

## All isolated sampled typed-loop cases

| Case | Only 0x7a0/0x5c0 | A/A |
| --- | --- | --- |
| broad/branch_arithmetic | 1.083914 | 1.002534 |
| broad/global_read | 1.129073 | 0.999176 |
| broad/local_read | 1.082748 | 1.001270 |
| external/jetstream3-js-subset/hash-map | 1.001710 | 0.999427 |

The study record owns binary hashes and job identities. Serial runs use the
same warmed literal path, retain absolute N/2N samples, and verify every
output. Their paired-median intervals are unadjusted for multiple testing.

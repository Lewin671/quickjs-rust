# S2 packed-header RSS at f380089d

Non-timing resource job `20261030T223438Z-t077-s2-r2-packed-x86-rss`,
source f380089d versus dcee3c56; three pairs/input, identical stdout. These
are resource diagnostics, not 15-pair resource or formal timing acceptance.
[Raw samples](T077-s2-r2-packed-rss-f380.json) contain input and binary hashes.
[Reproducer](T077-s2-retained-receivers.py) preserves the exact original inputs;
`fn_mem_nocall` omits the method call but still calls `f()` directly.

x86_64 verified agents/perf-counters runtime tests: 2,522 pass. Hot function
header is 304 bytes; with Rc counters its request fits the allocator's 320-byte
class, versus the former 320+16 ->384-byte class. Array hot/cold remains 56/104.

| input | base median bytes | candidate median bytes | ratio |
| --- | ---: | ---: | ---: |
| arr_mem | 42008576 | 42332160 | 1.007703 |
| arr_mem_nocall | 41971712 | 42307584 | 1.008002 |
| fn_mem | 120180736 | 110809088 | 0.922020 |
| fn_mem_nocall | 110481408 | 110800896 | 1.002892 |

The prior cdda4bd4 function no-method-call ratio was 1.179772. The class-name
packing repairs that allocation-class cost. Later immutable-IC source has its
own resource verification pending; this checkpoint does not claim its standing.

# S3c round 3: native depth repair

Source is the runtime content of the commit containing this receipt; source.json
pins every runtime source byte and binary hashes are in depth/native-frames.
Parent C2 is cd020e8e; comparison milestone is 8322fc98. These are depth and
instruction measurements on the ordinary host stack, without a CLI policy change.

The debug drivers outline operand temporaries and entry preparation; release
inlines those helpers. General dispatch moves callback-free operand work into
similar helpers. Context borrows native-site metadata constructed at invocation,
reducing the facade from 64 to 48 bytes without changing ownership or lifetime.
No per-call box, reference directory or snapshot was added.

| Native driver | S3b debug | C2 debug | Repair debug | C2 release | Repair release |
| --- | ---: | ---: | ---: | ---: | ---: |
| wide run/root | 15,408 | 19,952 | 8,816 | 1,264 | 1,104 |
| numeric run_frames | 5,504 | 7,664 | 4,016 | inlined | inlined |

Prologue bytes include pre-indexed saves plus explicit SP subtraction. The
numeric driver is inlined in release; no standalone symbol exists in these builds.
The linked JSON retains prologues rather than inferring a zero-sized native frame.

| Depth | S3b debug | Repair debug | S3b release | Repair release |
| --- | ---: | ---: | ---: | ---: |
| forEach | 183 | 216 | 935 | 974 |
| map | 182 | 214 | 927 | 965 |
| reduce | 236 | 245 | 1,026 | 1,079 |
| call | 437 | 668 | 3,448 | 4,096 cap |
| apply | 366 | 516 | 2,335 | 2,643 |
| Reflect.apply | 367 | 517 | 2,491 | 2,845 |
| yield-star | 152 | 211 | 820 | 1,269 |
| plain in callback | 4,096 cap | 4,096 cap | 4,096 cap | 4,096 cap |

The existing call-cost collector measures N/2N instruction deltas (one million
additional calls), preserving outputs/counters. C2 -> repair instructions/call:
upw0 932.088 -> 932.082; upw1 1267.084 -> 1267.159; upw4 2242.577 -> 2242.184.
The largest increase is 0.075/call (0.006%), within process/count noise; no
systematic entry cost remains. This is the depth prerequisite, not the C3 payoff.

See [depth](depth.json), [native frames](native-frames.json),
[call cost](call-cost.json), and [source manifest](source.json).

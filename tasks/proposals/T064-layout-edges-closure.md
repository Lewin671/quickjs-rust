# Draft closure: interpreter-core-v2-layout-edges

**Proposed status: closed-superseded. Not effective yet.** Enact only when the
lead commits the approved K→A→B successor as a frozen performance unit.
This draft records neither a new screen nor a `decide` result.

Original frozen plan:
`tasks/performance-units/interpreter-core-v2-layout-edges.json`, base
`f58c696e8379261ed726f0cea1051ed6d34f431a`, SHA-256
`d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284`.
Do not edit that JSON or any historical decision/raw evidence.

Reason: its fixed post-T063 base and M1-only runtime isolation cannot include
current-main runtime changes and prerequisite K. Its final performance standing
remains unresolved. Supersession is a contract transition, not a fabricated
`decide=inconclusive`, stage abort or promotion.

The valid failed screen remains spent: candidate
`0a9af81bebcef80e9239954f8db191a71f10316e`, iMac job
`20261020T210106Z-t056-m1a-chain-selection-k6` (exit 3, complete K=6 reject).
Its `out/binding.json` SHA-256 is
`399dadf214549ae0b5a99cf30422a9ca5ffb0514fdd758a81d9ca58b00e1818f`;
`out/verdict.json` SHA-256 is
`bf3e23bbd59dd2c7c1e7539fc579eeeb7c182d0e7b4c0a0a0fa6c6e8bfd87ae8`.
One of the original two shared A+B failures remains. Carry that one reserve,
not a new pair, into the successor. Later static/instruction/mini diagnostics
spent no additional acceptance attempt; `5dd3a810` remains unpromoted and
untimed under the frozen acceptance contract.

At enactment, update the old plan's index row to closed-superseded, link the
frozen successor and this closure, and update T056/T064 resume blocks. The
successor keeps current M as its fixed base, K at most one failure and A+B one
shared failure, aggregate at most two with no borrowing. A valid K failure
stops isolation and sends the lead to M2. Until that enactment the old plan
has not been closed by this proposal.

Authority: [T064 transition and ledger](../T064-kernel-isolation.md#frozen-plan-transition-and-attempt-ledger-proposal-for-review).

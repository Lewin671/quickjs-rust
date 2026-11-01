# Current S3b hostname qualification blocker

Read-only job `20261101T012446Z-t077-s3b-repaired-host-identity`, candidate
`ca5ab643`, exits 0. [Identity](identity.json) records its UTC and CST check
times: policy node **iMac.local**, actual node **iMac.lan**, **zero differences**
in the full hardware/OS/toolchain/recipe/configuration/manifest dictionary.
The remote command also asserts the unchanged v5 policy SHA-256 before auditing.

[Command](command.sh), [log](log), [status](status) and [hashes](metadata.json)
retain provenance. No benchmark was executed and no timing was collected. The
live qualification guard remains unchanged and would still reject this node.

The [alias-only proposal](../T077-s3b-host-alias-proposal/README.md) remains
unapplied, with explicit approval pending; all 18 offline proposal checks pass.
No policy bands, inventories, recipes or actual reported hostname were changed.
There is still no valid public86 result for candidate `ca5ab643` vs main `3cc12c6f`.
This is the remaining qualification decision, not a performance or correctness
waiver. Lead owns formal/held-out/RSS. No origin push or merge into main.

# Round-three binding repairs

## R3: self-name scope and positional captures

Parent `4853bd7f`. Before repair, the strict/plain/generator/callback discriminator
returned `global:global:global:3:0:global:3`; required
`param:param:param:3:3:global:0`. Dynamic crossing through a retained self-name
scope found no enclosing lease and selected a global before the already-received
cell. Reads and saved assignment references now fall back to that positional
cell after live crossing fails, ahead of module/global homonyms. Import policy
is retained only when the cell is actually the import's cell.

Validation: focused discriminator and all 68 direct-eval tests pass; exact lead
reproducer matches NG (`A param|B let|C global=0|D 1 global=g|E local`). Staged fast
gate passes 2,673 runtime tests and 65 selected Test262 cases. No full or exact
standing is claimed yet. Standalone R2 commit remains unchanged.

# T000: Task Title

- Status: active | blocked | closed-landed | closed-rejected |
  closed-superseded | historical-campaign.
- Implementation: present | partial | reverted | none. State this
  independently of whether the evidence was accepted.
- Verified at: commit, date, and the checks that were run at that commit.
- Evidence: a path in the repository, or "local artifact, not retained".
- Unresolved: open issues, or "none".
- Next action: the next justified step, or "none".

Keep this block first and current; update it at every handoff or completed
unit. Write "not recorded" where a field is unknown. Results apply only to
the revision they name. When the status becomes closed or historical, move
the file to `tasks/archive/` with `git mv`. Keep an active file under 200
lines by moving its finished log to `tasks/archive/<Tnnn>-log.md`.

## Goal

The intended outcome and the observable success criteria.

For a structural change, name the target architecture, its invariants and
the migration stages, or link the design note that does. For performance
work, follow `docs/performance-workflow.md` and
`tasks/T022-performance-priority-controller.md`; link the frozen
`tasks/performance-units/<unit>.json` plan instead of restating its targets,
budgets or metrics.

## Scope

- Allowed paths:
- Forbidden paths:
- Owner boundary:
- Base sha, branch and worktree (parallel work only):

## Acceptance

- [ ]

## Verification

```sh
./scripts/check.sh
```

Add narrower commands when useful.

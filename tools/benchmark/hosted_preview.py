"""Executable admission and durable-evidence helpers for hosted previews."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


HOSTED_BASE_REF = "main"
HOSTED_PUSH_REF = "refs/heads/main"
BASE_MODE = "base_owned_harness"
PUSH_MODE = "main_push_head_owned_harness"
MANUAL_MODE = "manual_main_head_owned_harness"
PR_INTEGRITY_SCOPE = "cooperative_same_repository_pull_request"
PUSH_INTEGRITY_SCOPE = "trusted_main_push"
MANUAL_INTEGRITY_SCOPE = "trusted_manual_main_dispatch"
_REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\Z")
_REVISION = re.compile(r"[0-9a-f]{40}\Z")


class HostedPreviewError(ValueError):
    """Hosted preview transition or evidence request is invalid."""


@dataclass(frozen=True)
class Admission:
    run: bool
    mode: str | None
    reason: str
    event_name: str
    integrity_scope: str


def _repository(value: str, label: str) -> None:
    if not _REPOSITORY.fullmatch(value):
        raise HostedPreviewError(f"{label}: expected owner/name")


def _revision(value: str, label: str) -> None:
    if not _REVISION.fullmatch(value):
        raise HostedPreviewError(f"{label}: expected full lowercase revision")


def _ref(value: str, label: str) -> None:
    if not value or value.strip() != value or any(character.isspace() for character in value):
        raise HostedPreviewError(f"{label}: expected a non-empty ref without whitespace")


def decide_pr_admission(
    event_name: str,
    head_repository: str,
    base_repository: str,
    base_sha: str,
    base_ref: str,
    pr_number: int,
    head_ref: str,
) -> Admission:
    """Return the long-term base-owned decision for one PR event."""
    for value, label in (
        (head_repository, "head repository"), (base_repository, "base repository")
    ):
        _repository(value, label)
    _revision(base_sha, "base SHA")
    if type(pr_number) is not int or pr_number < 1:
        raise HostedPreviewError("PR number: expected positive integer")
    for value, label in ((base_ref, "base ref"), (head_ref, "head ref")):
        _ref(value, label)
    if event_name != "pull_request_target":
        raise HostedPreviewError("event name: expected pull_request_target")
    scope = PR_INTEGRITY_SCOPE
    if base_ref != HOSTED_BASE_REF:
        return Admission(False, None, "base_branch_unsupported", event_name, scope)
    if head_repository != base_repository:
        return Admission(False, None, "fork_preview_unsupported", event_name, scope)
    return Admission(True, BASE_MODE, "base_owned_long_term_path", event_name, scope)


def decide_push_admission(
    event_name: str,
    repository: str,
    event_repository: str,
    ref: str,
    before_sha: str,
    after_sha: str,
    workflow_sha: str,
) -> Admission:
    """Return the fail-closed head-owned decision for one main push."""
    _repository(repository, "workflow repository")
    _repository(event_repository, "event repository")
    _ref(ref, "push ref")
    for value, label in (
        (before_sha, "before SHA"),
        (after_sha, "after SHA"),
        (workflow_sha, "workflow SHA"),
    ):
        _revision(value, label)
    if event_name != "push":
        raise HostedPreviewError("event name: expected push")
    scope = PUSH_INTEGRITY_SCOPE
    if ref != HOSTED_PUSH_REF:
        return Admission(False, None, "push_ref_unsupported", event_name, scope)
    if repository != event_repository:
        return Admission(False, None, "push_repository_mismatch", event_name, scope)
    if before_sha == "0" * 40:
        return Admission(False, None, "zero_before_sha", event_name, scope)
    if after_sha == "0" * 40:
        return Admission(False, None, "zero_after_sha", event_name, scope)
    if after_sha != workflow_sha:
        return Admission(False, None, "workflow_sha_mismatch", event_name, scope)
    if before_sha == after_sha:
        return Admission(False, None, "unchanged_push_sha", event_name, scope)
    return Admission(True, PUSH_MODE, "trusted_main_push", event_name, scope)


def decide_dispatch_admission(
    event_name: str,
    repository: str,
    event_repository: str,
    ref: str,
    revision: str,
    base_revision: str,
    workflow_sha: str,
) -> Admission:
    """Return the fail-closed head-owned decision for one manual fixed-base run."""
    _repository(repository, "workflow repository")
    _repository(event_repository, "event repository")
    _ref(ref, "dispatch ref")
    _revision(revision, "dispatch revision")
    _revision(base_revision, "dispatch base revision")
    _revision(workflow_sha, "workflow SHA")
    if event_name != "workflow_dispatch":
        raise HostedPreviewError("event name: expected workflow_dispatch")
    scope = MANUAL_INTEGRITY_SCOPE
    if ref != HOSTED_PUSH_REF:
        return Admission(False, None, "dispatch_ref_unsupported", event_name, scope)
    if repository != event_repository:
        return Admission(False, None, "dispatch_repository_mismatch", event_name, scope)
    if revision == "0" * 40:
        return Admission(False, None, "zero_dispatch_revision", event_name, scope)
    if base_revision == "0" * 40:
        return Admission(False, None, "zero_dispatch_base_revision", event_name, scope)
    if revision != workflow_sha:
        return Admission(False, None, "workflow_sha_mismatch", event_name, scope)
    return Admission(True, MANUAL_MODE, "trusted_manual_main_dispatch", event_name, scope)


def _write_replace(path: Path, content: bytes) -> None:
    path = path.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=path.parent, prefix=f".{path.name}.", delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()


# The broad lane is the longest by far, and nearly all of its time is
# per-case setup (calibration, warmup and the linearity probes), so its cases
# are divided between this many runners. Each shard measures its cases with
# all three engines on its own runner under the unchanged protocol: every
# case ratio is still a same-host comparison. What a shard cannot supply is
# an interval for the lane's overall ratio, whose bootstrap resamples blocks
# jointly across cases measured on one host.
BROAD_SHARDS = 2
BROAD_STAGES = tuple(f"broad-{shard}" for shard in range(1, BROAD_SHARDS + 1))
STAGES = ("build", *BROAD_STAGES, "external", "sentinel")
LANE_LABELS = {
    **{stage: f"broad lane shard {stage[len('broad-'):]}" for stage in BROAD_STAGES},
    "broad": "broad lane", "external": "external lane", "sentinel": "sentinel lane",
}
# The evidence each measuring stage leaves for the publisher, which must agree
# with the build job's record on the executables it measured.
STAGE_EVIDENCE = {
    **{stage: f"{stage}-summary.json" for stage in BROAD_STAGES},
    "external": "external-report.json",
    "sentinel": "sentinel-summary.json",
}
REQUIRED_LANES = ("broad", "external")


def _load(path: Path) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def _stage(output: Path, stage: str) -> dict[str, str]:
    """A stage's recorded state, or `missing` when it left no record."""
    status = _load(output / f"{stage}-status.json")
    if status is None or status.get("stage") != stage:
        return {
            "state": "missing", "phase": "not_started",
            "message": "no status was recorded", "run_attempt": "None",
        }
    return {
        "state": str(status.get("state")),
        "phase": str(status.get("phase")),
        "message": str(status.get("message")),
        "run_attempt": str(status.get("run_attempt")),
    }


def _lane_binaries(output: Path, lane: str, evidence: dict[str, Any]) -> dict[str, Any] | None:
    """The executable hashes a lane says it measured, by role."""
    if lane in BROAD_STAGES:
        engines = evidence.get("engines")
        if not isinstance(engines, dict):
            return None
        return {
            role: engine.get("binary_sha256") if isinstance(engine, dict) else None
            for role, engine in engines.items()
        }
    if lane == "external":
        return evidence.get("binary_sha256")
    receipts = {
        role: _load(output / f"sentinel-{role}-receipt.json")
        for role in ("candidate", "base", "quickjs-ng")
    }
    return {role: (receipt or {}).get("binary_sha256") for role, receipt in receipts.items()}


# What reading a field that is absent or of the wrong type raises. Evidence
# that parses can still be malformed, and must cost only its own lane.
MALFORMED = (KeyError, TypeError, ValueError, AttributeError, IndexError, ArithmeticError)


def _predates(lane_attempt: str, build_attempt: str) -> bool:
    """Whether a lane's record was produced before the build it must follow.

    Stage artifacts are named by stage so that "Re-run failed jobs" can reuse
    the jobs it does not repeat. The price is that a lane which fails before
    it can upload leaves its previous attempt's artifact in place. Rerunning
    everything repeats the build, so a lane record older than the build
    record is exactly that leftover; a partial rerun leaves the build's
    attempt alone, and every lane record is then at least as new.
    """
    if lane_attempt.isdigit() and build_attempt.isdigit():
        return int(lane_attempt) < int(build_attempt)
    return lane_attempt != build_attempt


def _renders(lane: str, evidence: dict[str, Any]) -> bool:
    """Whether a lane's evidence has the structure the summary reads.

    The renderer is the consumer, so it is also the check: evidence that
    parses but lacks a field would otherwise raise while composing and take
    every healthy lane's publication down with it.
    """
    from .preview_summary import render_preview

    lanes: dict[str, Any] = {"broad": None}
    lanes["broad" if lane.startswith("broad") else lane] = evidence
    try:
        render_preview(**lanes)
    except MALFORMED:
        return False
    return True


def collect(
    output: Path, build_succeeded: bool = True,
) -> tuple[dict[str, dict[str, str]], dict[str, Any], dict[str, str]]:
    """Read every stage's record and admit only evidence that is whole.

    Returns the stage records, the admitted evidence by lane, and for each
    lane without admitted evidence the reason a reader should be given. A
    lane is admitted only when its stage reached `success` no earlier than
    the build did, its evidence parses and has the structure the summary
    reads, and it measured exactly the executables the build job recorded. `build_succeeded` is the build
    job's own conclusion in this attempt; a build record cannot outvote it.
    """
    stages = {stage: _stage(output, stage) for stage in STAGES}
    if not build_succeeded and stages["build"]["state"] == "success":
        stages["build"] = {
            **stages["build"], "state": "superseded", "phase": "build_job",
            "message": "the build job did not succeed in this attempt",
        }
    identity = _load(output / "build-identity.json") or {}
    built = identity.get("binaries") if stages["build"]["state"] == "success" else None
    evidence: dict[str, Any] = {}
    notes: dict[str, str] = {}
    for lane, name in STAGE_EVIDENCE.items():
        record = stages[lane]
        label = LANE_LABELS[lane]
        if not isinstance(built, dict):
            build = stages["build"]
            notes[lane] = (
                f"the build stage did not produce the three executables "
                f"(state {build['state']}, phase {build['phase']})."
            )
        elif record["state"] == "missing":
            notes[lane] = f"the {label} left no record: its job did not start or uploaded nothing."
        elif _predates(record["run_attempt"], stages["build"]["run_attempt"]):
            notes[lane] = (
                f"the {label} left no record in this attempt: what it uploaded "
                "predates this run's build."
            )
        elif record["state"] == "incomplete":
            notes[lane] = record["message"]
        elif record["state"] != "success":
            notes[lane] = (
                f"the {label} did not finish (state {record['state']}, "
                f"phase {record['phase']})."
            )
        else:
            loaded = _load(output / name)
            if loaded is None:
                notes[lane] = f"the {label} reported success but its evidence is unreadable."
            elif _lane_binaries(output, lane, loaded) != built:
                notes[lane] = (
                    f"the {label} did not measure the executables the build stage recorded."
                )
            elif not _renders(lane, loaded):
                notes[lane] = f"the {label} reported success but its evidence is malformed."
            else:
                evidence[lane] = loaded
    # The broad lane exists only when every shard was admitted and they fit
    # together; a partial portfolio is never published as the lane.
    shards = [evidence.pop(stage) for stage in BROAD_STAGES if stage in evidence]
    absent = [notes.pop(stage) for stage in BROAD_STAGES if stage in notes]
    if absent:
        notes["broad"] = " ".join(dict.fromkeys(absent))
    else:
        try:
            merged = merge_broad_shards(shards)
        except HostedPreviewError as error:
            notes["broad"] = f"the broad lane's shards do not fit together: {error}."
        except MALFORMED:
            notes["broad"] = "the broad lane's shards reported success but are malformed."
        else:
            if _renders("broad", merged):
                evidence["broad"] = merged
            else:
                notes["broad"] = "the broad lane's shards reported success but are malformed."
    return stages, evidence, notes


def merge_broad_shards(shards: list[dict[str, Any]]) -> dict[str, Any]:
    """Join the shards' machine summaries into the broad lane's.

    Each shard validated its own cases; this checks only that the shards
    belong together -- same harness, profile and executables, and exactly
    the frozen portfolio between them, each case once. The overall ratio is
    the geometric mean of the case ratios, which is what the analysis
    computes for an unsharded lane; it carries no interval here.
    """
    from .preview import HOSTED_CASES

    numbers = sorted(str(shard.get("shard")) for shard in shards)
    if numbers != [str(number) for number in range(1, BROAD_SHARDS + 1)]:
        raise HostedPreviewError("expected each shard exactly once")
    first = shards[0]
    shared = ("profile_id", "harness", "integrity_scope", "engines", "requested_blocks")
    for shard in shards[1:]:
        if any(shard.get(key) != first.get(key) for key in shared):
            raise HostedPreviewError("the shards disagree about what they measured")
    merged = {
        key: value for key, value in first.items() if key not in {"shard", "comparisons"}
    }
    merged["shards"] = BROAD_SHARDS
    merged["valid_blocks"] = min(shard["valid_blocks"] for shard in shards)
    if any(shard["health"] == "invalid" or not shard["comparisons"] for shard in shards):
        # One shard failing its linearity diagnostic leaves the lane without
        # a direction, exactly as it would unsharded.
        merged.update({"health": "invalid", "linearity": "fail", "comparisons": {}})
        return merged
    merged["comparisons"] = {}
    for label in ("candidate vs base", "candidate vs QuickJS-NG"):
        cases = {
            case["id"]: case for shard in shards for case in shard["comparisons"][label]["cases"]
        }
        counted = sum(len(shard["comparisons"][label]["cases"]) for shard in shards)
        if counted != len(HOSTED_CASES) or set(cases) != set(HOSTED_CASES):
            raise HostedPreviewError("the shards do not cover the frozen portfolio exactly once")
        ratios = [cases[case_id]["ratio"] for case_id in HOSTED_CASES]
        merged["comparisons"][label] = {
            "label": label,
            "ratio": math.exp(sum(math.log(ratio) for ratio in ratios) / len(ratios)),
            "cases": [cases[case_id] for case_id in HOSTED_CASES],
        }
    return merged


def publish(output_dir: Path, step_summary: Path, build_succeeded: bool = True) -> bool:
    """Compose and publish the summary; report whether the preview is whole.

    Publication never depends on success: the summary and status are written
    and appended to the step summary first, from whichever lanes produced
    admitted evidence, and only then does the caller fail the job for a
    missing required lane. Rendering uses this checkout's code on validated
    JSON; nothing a lane rendered is published.
    """
    from .preview_summary import escape_markdown, render_preview

    output = output_dir.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    stages, evidence, notes = collect(output, build_succeeded)
    complete = all(lane in evidence for lane in REQUIRED_LANES)
    banner = None
    if not complete:
        banner = (
            "**No complete performance conclusion was produced.** "
            + " ".join(
                f"The {LANE_LABELS[lane]} is required and has no admitted evidence."
                for lane in REQUIRED_LANES if lane not in evidence
            )
        )
    markdown = render_preview(
        evidence.get("broad"), evidence.get("external"), evidence.get("sentinel"),
        {lane: escape_markdown(note) for lane, note in notes.items()}, banner=banner,
    )
    payload = {
        "schema_version": 2,
        "state": "success" if complete else "failed",
        "classification": (
            "informational_non_gating_not_fixed_hardware_claim" if complete
            else "no_performance_conclusion"
        ),
        "stages": stages,
        "lanes_with_evidence": sorted(evidence),
    }
    if "broad" in evidence:
        # The composed lane, for readers of the evidence; the shards' own
        # summaries stay beside it.
        _write_replace(output / "summary.json", _json_bytes(evidence["broad"]))
    # The run's evidence came from several jobs. This index binds what was
    # published to the exact bytes it was published from.
    payload["evidence_sha256"] = {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(output.iterdir())
        if path.is_file() and path.name not in {"status.json", "summary.md"}
    }
    _write_replace(output / "status.json", _json_bytes(payload))
    _write_replace(output / "summary.md", markdown.encode())
    target = step_summary.expanduser().resolve()
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("ab") as handle:
        handle.write(markdown.encode())
    return complete


def _admit_pr(args: argparse.Namespace) -> None:
    admission = decide_pr_admission(
        args.event_name, args.head_repository, args.base_repository, args.base_sha,
        args.base_ref, args.pr_number, args.head_ref,
    )
    if args.require_mode is not None and (
        not admission.run or admission.mode != args.require_mode
    ):
        raise HostedPreviewError(
            f"event is not admitted as {args.require_mode}: {admission.reason}"
        )
    print(json.dumps(asdict(admission), sort_keys=True, separators=(",", ":")))


def _admit_push(args: argparse.Namespace) -> None:
    admission = decide_push_admission(
        args.event_name, args.repository, args.event_repository, args.ref,
        args.before_sha, args.after_sha, args.workflow_sha,
    )
    if args.require_mode is not None and (
        not admission.run or admission.mode != args.require_mode
    ):
        raise HostedPreviewError(
            f"event is not admitted as {args.require_mode}: {admission.reason}"
        )
    print(json.dumps(asdict(admission), sort_keys=True, separators=(",", ":")))


def _admit_dispatch(args: argparse.Namespace) -> None:
    admission = decide_dispatch_admission(
        args.event_name, args.repository, args.event_repository, args.ref,
        args.revision, args.base_revision, args.workflow_sha,
    )
    if args.require_mode is not None and (
        not admission.run or admission.mode != args.require_mode
    ):
        raise HostedPreviewError(
            f"event is not admitted as {args.require_mode}: {admission.reason}"
        )
    print(json.dumps(asdict(admission), sort_keys=True, separators=(",", ":")))


def _publish(args: argparse.Namespace) -> None:
    if not publish(args.output_dir, args.step_summary, args.build_result == "success"):
        raise HostedPreviewError("required preview evidence is missing; see the summary")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    admit = commands.add_parser("admit")
    admit.add_argument(
        "--event-name", choices=("pull_request_target",), required=True
    )
    admit.add_argument("--head-repository", required=True)
    admit.add_argument("--base-repository", required=True)
    admit.add_argument("--base-sha", required=True)
    admit.add_argument("--base-ref", required=True)
    admit.add_argument("--pr-number", type=int, required=True)
    admit.add_argument("--head-ref", required=True)
    admit.add_argument("--require-mode", choices=(BASE_MODE,))
    admit.set_defaults(function=_admit_pr)
    admit_push = commands.add_parser("admit-push")
    admit_push.add_argument("--event-name", required=True)
    admit_push.add_argument("--repository", required=True)
    admit_push.add_argument("--event-repository", required=True)
    admit_push.add_argument("--ref", required=True)
    admit_push.add_argument("--before-sha", required=True)
    admit_push.add_argument("--after-sha", required=True)
    admit_push.add_argument("--workflow-sha", required=True)
    admit_push.add_argument("--require-mode", choices=(PUSH_MODE,))
    admit_push.set_defaults(function=_admit_push)
    admit_dispatch = commands.add_parser("admit-dispatch")
    admit_dispatch.add_argument("--event-name", required=True)
    admit_dispatch.add_argument("--repository", required=True)
    admit_dispatch.add_argument("--event-repository", required=True)
    admit_dispatch.add_argument("--ref", required=True)
    admit_dispatch.add_argument("--revision", required=True)
    admit_dispatch.add_argument("--base-revision", required=True)
    admit_dispatch.add_argument("--workflow-sha", required=True)
    admit_dispatch.add_argument("--require-mode", choices=(MANUAL_MODE,))
    admit_dispatch.set_defaults(function=_admit_dispatch)
    publisher = commands.add_parser("publish")
    publisher.add_argument("--output-dir", type=Path, required=True)
    publisher.add_argument("--step-summary", type=Path, required=True)
    publisher.add_argument("--build-result", choices=("success", "failure"), default="success")
    publisher.set_defaults(function=_publish)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        args.function(args)
        return 0
    except (HostedPreviewError, OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

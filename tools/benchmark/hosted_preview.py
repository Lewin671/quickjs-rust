"""Executable admission and durable-evidence helpers for hosted previews."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


BOOTSTRAP_BASE_SHA = "d8ac450f92b4a773250310d5f91835cd47d39a98"
BOOTSTRAP_PR_NUMBER = 126
BOOTSTRAP_HEAD_REF = "agent/performance-benchmark-system/root"
HOSTED_BASE_REF = "main"
BASE_MODE = "base_owned_harness"
BOOTSTRAP_MODE = "bootstrap_same_repo_candidate_harness"
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


def decide_admission(
    event_name: str,
    head_repository: str,
    base_repository: str,
    base_sha: str,
    base_ref: str,
    pr_number: int,
    head_ref: str,
) -> Admission:
    """Return the frozen transition decision for one PR event."""
    for value, label in (
        (head_repository, "head repository"), (base_repository, "base repository")
    ):
        if not _REPOSITORY.fullmatch(value):
            raise HostedPreviewError(f"{label}: expected owner/name")
    if not _REVISION.fullmatch(base_sha):
        raise HostedPreviewError("base SHA: expected full lowercase revision")
    if type(pr_number) is not int or pr_number < 1:
        raise HostedPreviewError("PR number: expected positive integer")
    for value, label in ((base_ref, "base ref"), (head_ref, "head ref")):
        if not value or value.strip() != value or any(character.isspace() for character in value):
            raise HostedPreviewError(f"{label}: expected a non-empty ref without whitespace")
    scope = "cooperative_same_repository_pull_request"
    if base_ref != HOSTED_BASE_REF:
        return Admission(False, None, "base_branch_unsupported", event_name, scope)
    if head_repository != base_repository:
        return Admission(False, None, "fork_preview_unsupported", event_name, scope)
    if event_name == "pull_request_target":
        return Admission(True, BASE_MODE, "base_owned_long_term_path", event_name, scope)
    if event_name == "pull_request":
        if base_sha != BOOTSTRAP_BASE_SHA:
            return Admission(False, None, "bootstrap_base_sha_mismatch", event_name, scope)
        if pr_number != BOOTSTRAP_PR_NUMBER:
            return Admission(False, None, "bootstrap_pr_number_mismatch", event_name, scope)
        if head_ref != BOOTSTRAP_HEAD_REF:
            return Admission(False, None, "bootstrap_head_ref_mismatch", event_name, scope)
        return Admission(True, BOOTSTRAP_MODE, "exact_transition_bootstrap", event_name, scope)
    raise HostedPreviewError("event name: expected pull_request or pull_request_target")


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


def publish_or_fallback(
    output_dir: Path, step_summary: Path, message: str, job_status: str
) -> None:
    """Publish an existing summary or create a consistent pre-orchestrator failure."""
    output = output_dir.expanduser().resolve()
    summary = output / "summary.md"
    status = output / "status.json"
    output.mkdir(parents=True, exist_ok=True)
    phase = "pre_orchestrator"
    preserve_failed = False
    if status.is_file():
        try:
            existing = json.loads(status.read_text(encoding="utf-8"))
            if isinstance(existing, dict):
                existing_phase = existing.get("phase")
                if isinstance(existing_phase, str) and existing_phase:
                    phase = existing_phase
                preserve_failed = existing.get("state") == "failed"
        except (OSError, UnicodeError, json.JSONDecodeError):
            pass
    needs_fallback = not summary.is_file() or not status.is_file()
    if job_status != "success" and not preserve_failed:
        needs_fallback = True
    if needs_fallback:
        payload = {
            "schema_version": 1,
            "state": "failed",
            "phase": phase,
            "classification": "no_performance_conclusion",
            "job_status": job_status,
            "message": message,
        }
        markdown = "\n".join([
            "## Performance Preview Failed", "",
            "> **No performance conclusion was produced.**",
            f"> {message}", "",
            f"- Failure phase: `{phase}`", "",
        ])
        _write_replace(status, _json_bytes(payload))
        _write_replace(summary, markdown.encode())
    try:
        rendered = summary.read_bytes()
    except OSError as error:
        raise HostedPreviewError(f"cannot read durable summary: {error}") from error
    target = step_summary.expanduser().resolve()
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("ab") as handle:
        handle.write(rendered)


def _admit(args: argparse.Namespace) -> None:
    admission = decide_admission(
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


def _publish(args: argparse.Namespace) -> None:
    publish_or_fallback(
        args.output_dir, args.step_summary, args.message, args.job_status
    )


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    admit = commands.add_parser("admit")
    admit.add_argument(
        "--event-name", choices=("pull_request", "pull_request_target"), required=True
    )
    admit.add_argument("--head-repository", required=True)
    admit.add_argument("--base-repository", required=True)
    admit.add_argument("--base-sha", required=True)
    admit.add_argument("--base-ref", required=True)
    admit.add_argument("--pr-number", type=int, required=True)
    admit.add_argument("--head-ref", required=True)
    admit.add_argument("--require-mode", choices=(BASE_MODE, BOOTSTRAP_MODE))
    admit.set_defaults(function=_admit)
    publish = commands.add_parser("publish")
    publish.add_argument("--output-dir", type=Path, required=True)
    publish.add_argument("--step-summary", type=Path, required=True)
    publish.add_argument(
        "--job-status", choices=("success", "failure", "cancelled"), default="failure"
    )
    publish.add_argument(
        "--message", default="setup failed before the benchmark orchestrator completed"
    )
    publish.set_defaults(function=_publish)
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

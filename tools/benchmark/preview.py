"""Build provenance and GitHub summary helpers for hosted performance previews."""

from __future__ import annotations

import argparse
import html
import json
import math
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from .hosted_preview import (
    BASE_MODE,
    PR_INTEGRITY_SCOPE,
    PUSH_INTEGRITY_SCOPE,
    PUSH_MODE,
)
from .schema import ManifestError, load_manifest, sha256_file


class PreviewError(ValueError):
    """Hosted preview inputs cannot produce truthful informational evidence."""


RUST_BUILD_FLAGS = (
    "--locked", "--release", "-p", "qjs-cli",
    "--config=profile.release.opt-level=3",
    "--config=profile.release.debug=false",
    "--config=profile.release.debug-assertions=false",
    "--config=profile.release.overflow-checks=false",
    "--config=profile.release.lto=false",
    "--config=profile.release.codegen-units=16",
    "--config=profile.release.panic=\"unwind\"",
    "--config=profile.release.incremental=false",
    "--config=profile.release.strip=\"none\"",
)
HARNESS_MODES = (BASE_MODE, PUSH_MODE)
HOSTED_CASES = (
    "plain_function_call", "method_call", "captured_read", "captured_write",
    "many_locals_call", "property_read", "array_read", "function_call_two_args",
    "function_call_reordered", "top_level_function_call", "dynamic_method_call",
    "local_read", "global_read", "property_dynamic_read", "property_write",
    "array_dynamic_read", "array_write", "empty_loop", "branch_arithmetic",
    "math_abs", "array_index_of", "string_slice", "object_allocation",
    "array_allocation", "closure_allocation_call",
)


def _integrity_scope(harness_mode: str) -> str:
    return {
        BASE_MODE: PR_INTEGRITY_SCOPE,
        PUSH_MODE: PUSH_INTEGRITY_SCOPE,
    }[harness_mode]


_GITHUB_CLONE_URL = re.compile(
    r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\.git\Z"
)
_SAFE_PROFILE = re.compile(r"[a-z0-9._-]+\Z")
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def _reject_constant(value: str) -> None:
    raise PreviewError(f"preview JSON contains non-standard numeric constant {value}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PreviewError(f"preview JSON contains duplicate key {key!r}")
        result[key] = value
    return result


def _read_object(path: Path, where: str) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_bytes(),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PreviewError(f"cannot read {where} {path}: {error}") from error
    if not isinstance(value, dict):
        raise PreviewError(f"{where}: expected an object")
    return value


def _string(value: Any, where: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value:
        raise PreviewError(f"{where}: expected a non-empty trimmed string")
    return value


def _revision(value: Any, where: str) -> str:
    text = _string(value, where)
    if len(text) != 40 or any(character not in "0123456789abcdef" for character in text):
        raise PreviewError(f"{where}: expected a full lowercase git SHA")
    return text


def _github_url(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _GITHUB_CLONE_URL.fullmatch(text):
        raise PreviewError(f"{where}: expected canonical HTTPS GitHub clone URL")
    return text


def _sha256(value: Any, where: str) -> str:
    text = _string(value, where)
    if not _SHA256.fullmatch(text):
        raise PreviewError(f"{where}: expected lowercase SHA-256")
    return text


def _positive_number(value: Any, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise PreviewError(f"{where}: expected a finite positive number")
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise PreviewError(f"{where}: expected a finite positive number")
    return number


def _write_new(path: Path, content: bytes) -> None:
    path = path.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise PreviewError(f"refusing to overwrite existing output: {path}")
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=path.parent, prefix=f".{path.name}.", delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError as error:
            raise PreviewError(f"refusing to overwrite existing output: {path}") from error
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


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
    try:
        return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()
    except (TypeError, ValueError) as error:
        raise PreviewError(f"preview output cannot be encoded: {error}") from error


def _recipe(
    identity: str,
    toolchain: str,
    target: str,
    flags: list[str],
    *,
    lto: str,
    strip: str,
    allocator: str,
    host_features: str,
) -> dict[str, Any]:
    return {
        "engine_identity": identity,
        "build_mode": "release",
        "toolchain": toolchain,
        "target": target,
        "features": [],
        "flags": flags,
        "lto": lto,
        "strip": strip,
        "allocator": allocator,
        "host_features": host_features,
    }


def _receipt(
    *,
    identity: str,
    source_repo: str,
    revision: str,
    profile_id: str,
    recipe: dict[str, Any],
    binary: Path,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "engine_identity": identity,
        "source": {
            "repo": _github_url(source_repo, f"{identity} source repo"),
            "revision": _revision(revision, f"{identity} source revision"),
            "dirty": False,
        },
        "profile_id": profile_id,
        "build": {
            key: recipe[key]
            for key in (
                "build_mode", "toolchain", "target", "features", "flags",
                "lto", "strip", "allocator", "host_features",
            )
        },
        "binary_sha256": sha256_file(binary.expanduser().resolve()),
    }


def prepare(args: argparse.Namespace) -> None:
    template_path = args.template.expanduser().resolve()
    template = load_manifest(template_path)
    data = _read_object(template_path, "measurement manifest")
    if tuple(case.id for case in template.cases) != HOSTED_CASES:
        raise PreviewError("hosted preview requires the complete frozen broad portfolio")
    profile_id = _string(args.profile_id, "profile id")
    platform = _string(args.platform, "platform")
    rust_flags = list(RUST_BUILD_FLAGS)
    quickjs_flags = [
        f"CC={_string(args.quickjs_cc, 'QuickJS-NG CC')}",
        "BUILD_TYPE=Release",
        "all",
    ]
    rust_recipe = _recipe(
        "qjs-rust", _string(args.rust_toolchain, "Rust toolchain"),
        _string(args.rust_target, "Rust target"), rust_flags,
        lto="off-forced-by-cargo-config",
        strip="none-forced-by-cargo-config",
        allocator="source-controlled-not-independently-verified",
        host_features="generic-cpu-forced-by-cargo-encoded-rustflags",
    )
    quickjs_recipe = _recipe(
        template.reference_identity,
        _string(args.quickjs_toolchain, "QuickJS-NG toolchain"),
        _string(args.quickjs_target, "QuickJS-NG target"), quickjs_flags,
        lto="cmake-release-default-not-independently-overridden",
        strip="cmake-release-default-not-independently-overridden",
        allocator="source-controlled-not-independently-verified",
        host_features="cc-default-no-architecture-flags",
    )
    data["profile"] = {"id": profile_id, "platform": platform}
    data["build_recipes"] = [rust_recipe, quickjs_recipe]

    manifest_output = args.manifest_output.expanduser().resolve()
    if manifest_output.parent.name != "benchmarks":
        raise PreviewError("dynamic manifest must be written directly under a benchmarks directory")
    _write_new(manifest_output, _json_bytes(data))
    # The normal claim-grade loader validates hashes, paths, frozen protocol,
    # portfolio, and recipe shape. Hosted preview never gets a relaxed parser.
    dynamic = load_manifest(manifest_output)
    if dynamic.protocol_sha256 != template.protocol_sha256:
        raise PreviewError("dynamic manifest changed the measurement protocol")

    outputs = (
        (
            args.candidate_receipt,
            _receipt(
                identity="qjs-rust", source_repo=args.candidate_repo,
                revision=args.candidate_revision, profile_id=profile_id,
                recipe=rust_recipe, binary=args.candidate_binary,
            ),
        ),
        (
            args.base_receipt,
            _receipt(
                identity="qjs-rust", source_repo=args.base_repo,
                revision=args.base_revision, profile_id=profile_id,
                recipe=rust_recipe, binary=args.base_binary,
            ),
        ),
        (
            args.quickjs_receipt,
            _receipt(
                identity=template.reference_identity,
                source_repo=template.reference_repo,
                revision=template.reference_revision,
                profile_id=profile_id, recipe=quickjs_recipe,
                binary=args.quickjs_binary,
            ),
        ),
    )
    for path, receipt in outputs:
        _write_new(path, _json_bytes(receipt))


def reference(args: argparse.Namespace) -> None:
    manifest = load_manifest(args.manifest)
    print(manifest.reference_repo if args.field == "repo" else manifest.reference_revision)


def verify_source(args: argparse.Namespace) -> None:
    source = args.source.expanduser().resolve()
    expected = _revision(args.revision, "source revision")
    try:
        head = subprocess.run(
            ["git", "-C", str(source), "rev-parse", "HEAD"],
            capture_output=True, text=True, timeout=10, check=False,
        )
        status = subprocess.run(
            ["git", "-C", str(source), "status", "--porcelain", "--untracked-files=normal"],
            capture_output=True, text=True, timeout=10, check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise PreviewError(f"cannot inspect source repository {source}: {error}") from error
    if head.returncode != 0 or head.stdout.strip() != expected:
        raise PreviewError(
            f"source repository {source} does not match expected revision {expected}"
        )
    if status.returncode != 0:
        raise PreviewError(
            f"cannot inspect source cleanliness for {source}: {status.stderr.strip()}"
        )
    if status.stdout:
        raise PreviewError(
            f"source repository {source} is dirty after build: {status.stdout.strip()}"
        )


def escape_markdown(value: str) -> str:
    """Escape dynamic text for a GitHub Markdown table cell."""
    escaped = html.escape(value, quote=False).replace("\\", "\\\\")
    for character in "`*_{}[]()#+-.!|>":
        escaped = escaped.replace(character, f"\\{character}")
    return escaped.replace("\r", " ").replace("\n", "<br>")


def _comparison(report: dict[str, Any], key: str, label: str) -> dict[str, Any]:
    comparisons = report.get("comparisons")
    if not isinstance(comparisons, dict) or not isinstance(comparisons.get(key), dict):
        raise PreviewError(f"report is missing required {label} comparison")
    overall = comparisons[key].get("overall")
    if not isinstance(overall, dict):
        raise PreviewError(f"report is missing required {label} overall result")
    ratio = _positive_number(overall.get("ratio"), f"{label} ratio")
    interval = overall.get("confidence_interval")
    if not isinstance(interval, dict):
        raise PreviewError(f"report is missing required {label} confidence interval")
    lower = _positive_number(interval.get("lower"), f"{label} CI lower")
    upper = _positive_number(interval.get("upper"), f"{label} CI upper")
    if not lower <= ratio <= upper:
        raise PreviewError(f"{label} confidence interval does not contain the ratio")
    if ratio > 1:
        direction = "higher ns/op"
        percent = (ratio - 1) * 100
    elif ratio < 1:
        direction = "lower ns/op"
        percent = (1 - ratio) * 100
    else:
        direction = "equal ns/op"
        percent = 0.0
    return {
        "label": label,
        "ratio": ratio,
        "ci_lower": lower,
        "ci_upper": upper,
        "direction": direction,
        "percent": percent,
    }


def _engine_provenance(run: dict[str, Any]) -> dict[str, dict[str, str]]:
    engines = run.get("engines")
    if not isinstance(engines, list) or [item.get("role") for item in engines if isinstance(item, dict)] != [
        "candidate", "base", "quickjs-ng",
    ]:
        raise PreviewError("report is missing ordered three-engine provenance")
    result = {}
    for item in engines:
        receipt = item.get("receipt")
        source = receipt.get("source") if isinstance(receipt, dict) else None
        if not isinstance(source, dict):
            raise PreviewError("report engine is missing verified receipt source")
        role = item["role"]
        result[role] = {
            "source_revision": _revision(source.get("revision"), f"{role} source revision"),
            "binary_sha256": _sha256(item.get("binary_sha256"), f"{role} binary SHA-256"),
        }
    return result


def summarize(
    report: dict[str, Any], *, harness_mode: str, harness_revision: str,
) -> tuple[str, dict[str, Any]]:
    if report.get("schema_id") != "quickjs-benchmark-report" or report.get("schema_version") != 3:
        raise PreviewError("report has an unsupported schema identity")
    if report.get("claim_eligible") is not False:
        raise PreviewError("hosted preview report must remain non-claim evidence")
    analysis = report.get("analysis_contract")
    bootstrap = analysis.get("bootstrap") if isinstance(analysis, dict) else None
    if not isinstance(bootstrap, dict) or bootstrap.get("confidence") != 0.95:
        raise PreviewError("hosted preview summary requires the frozen 95% confidence policy")
    run = report.get("run")
    coverage = report.get("coverage")
    health = report.get("health")
    if not isinstance(run, dict) or not isinstance(coverage, dict) or not isinstance(health, dict):
        raise PreviewError("report is missing run, coverage, or health")
    if coverage.get("comparison_input_complete") is not True:
        raise PreviewError("report comparison input is incomplete")
    if (
        coverage.get("roles") != 3
        or coverage.get("cases") != len(HOSTED_CASES)
        or coverage.get("blocks") != 3
    ):
        raise PreviewError("hosted preview requires three roles, the broad portfolio, and three blocks")
    blocks = health.get("blocks")
    if not isinstance(blocks, dict) or type(blocks.get("valid")) is not int:
        raise PreviewError("report is missing valid-block health")
    valid_blocks = blocks["valid"]
    if valid_blocks != 3 or blocks.get("invalid") != 0 or blocks.get("status") != "non_claim":
        raise PreviewError("hosted preview requires exactly 3/3 valid non-claim blocks")
    status = _string(health.get("status"), "health status")
    if status != "inconclusive":
        raise PreviewError("hosted preview overall health must be inconclusive")
    linearity = health.get("linearity")
    if not isinstance(linearity, dict) or linearity.get("status") != "pass":
        raise PreviewError("hosted preview linearity health must pass")
    profile = run.get("profile")
    if not isinstance(profile, dict):
        raise PreviewError("report is missing profile identity")
    profile_id = _string(profile.get("id"), "profile id")
    if not _SAFE_PROFILE.fullmatch(profile_id):
        raise PreviewError("profile id contains unsafe Markdown characters")
    if harness_mode not in HARNESS_MODES:
        raise PreviewError("unknown harness ownership mode")
    harness_revision = _revision(harness_revision, "harness revision")
    engines = _engine_provenance(run)
    results = [
        _comparison(report, "candidate_vs_base", "candidate vs base"),
        _comparison(report, "candidate_vs_quickjs_ng", "candidate vs QuickJS-NG"),
    ]
    lines = [
        "## Performance Preview",
        "",
        "> **Informational only — non-gating — not a fixed-hardware claim.**",
        "> GitHub-hosted runners are variable. A slowdown never fails this job; missing or invalid evidence does.",
        "",
        "Ratio = candidate wall ns/op ÷ comparator wall ns/op. Above 1.0 is higher ns/op; below 1.0 is lower ns/op.",
        "",
        "| Comparison | Overall ratio | 95% CI | Direction |",
        "| --- | ---: | ---: | --- |",
    ]
    for result in results:
        lines.append(
            f"| {result['label']} | {result['ratio']:.4f}× | "
            f"[{result['ci_lower']:.4f}×, {result['ci_upper']:.4f}×] | "
            f"{result['percent']:.2f}% {result['direction']} |"
        )
    lines.extend([
        "",
        f"- Health: {status}; block health: non_claim; linearity: pass",
        f"- Valid blocks: `{valid_blocks}/3`",
        f"- Harness ownership mode: `{harness_mode}` at `{harness_revision}`",
        f"- Integrity scope: `{_integrity_scope(harness_mode)}`",
        "- Security boundary: candidate build/execution is not sandboxed; artifacts do not resist a malicious candidate",
        f"- Profile: `{profile_id}`",
        f"- Portfolio: `{len(HOSTED_CASES)}/{len(HOSTED_CASES)} cases`, roles: `candidate/base/quickjs-ng`",
        f"- Candidate: `{engines['candidate']['source_revision']}` / `{engines['candidate']['binary_sha256']}`",
        f"- Base: `{engines['base']['source_revision']}` / `{engines['base']['binary_sha256']}`",
        f"- QuickJS-NG: `{engines['quickjs-ng']['source_revision']}` / `{engines['quickjs-ng']['binary_sha256']}`",
        "",
    ])
    machine = {
        "schema_version": 1,
        "state": "success",
        "classification": "informational_non_gating_not_fixed_hardware_claim",
        "ratio_semantics": "candidate_over_comparator_wall_ns_per_operation",
        "profile_id": profile_id,
        "health": status,
        "valid_blocks": valid_blocks,
        "requested_blocks": 3,
        "harness": {"mode": harness_mode, "revision": harness_revision},
        "integrity_scope": _integrity_scope(harness_mode),
        "malicious_candidate_resistant": False,
        "engines": engines,
        "comparisons": {result["label"]: result for result in results},
    }
    return "\n".join(lines), machine


def summary(args: argparse.Namespace) -> None:
    markdown, machine = summarize(
        _read_object(args.report, "benchmark report"),
        harness_mode=args.harness_mode,
        harness_revision=args.harness_revision,
    )
    _write_replace(args.markdown, markdown.encode("utf-8"))
    _write_replace(args.json_output, _json_bytes(machine))
    _write_replace(args.status_output, _json_bytes(machine))


def status(args: argparse.Namespace) -> None:
    mode = _string(args.harness_mode, "harness mode")
    if mode not in HARNESS_MODES:
        raise PreviewError("unknown harness ownership mode")
    payload = {
        "schema_version": 1,
        "state": args.state,
        "phase": _string(args.phase, "failure phase"),
        "classification": "no_performance_conclusion",
        "harness": {
            "mode": mode,
            "revision": _revision(args.harness_revision, "harness revision"),
        },
        "sources": {
            "candidate": _revision(args.candidate_revision, "candidate revision"),
            "base": _revision(args.base_revision, "base revision"),
            "quickjs-ng": _revision(args.reference_revision, "reference revision"),
        },
        "message": _string(args.message, "status message"),
    }
    heading = "Pending / Failed" if args.state == "pending" else "Failed"
    markdown = "\n".join([
        f"## Performance Preview {heading}", "",
        "> **No performance conclusion was produced.**",
        f"> {escape_markdown(args.message)}", "",
        f"- Failure phase: `{escape_markdown(payload['phase'])}`",
        f"- Harness ownership mode: `{mode}` at `{payload['harness']['revision']}`",
        f"- Integrity scope: `{_integrity_scope(mode)}`", "",
    ])
    output = args.output_dir.expanduser().resolve()
    _write_replace(output / "status.json", _json_bytes(payload))
    _write_replace(output / "summary.md", markdown.encode())


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="orchestrate hosted performance preview evidence")
    commands = parser.add_subparsers(dest="command", required=True)
    inspect = commands.add_parser("reference")
    inspect.add_argument("--manifest", type=Path, required=True)
    inspect.add_argument("--field", choices=("repo", "revision"), required=True)
    inspect.set_defaults(function=reference)

    verify = commands.add_parser("verify-source")
    verify.add_argument("--source", type=Path, required=True)
    verify.add_argument("--revision", required=True)
    verify.set_defaults(function=verify_source)

    build = commands.add_parser("prepare")
    for name in (
        "template", "manifest-output", "candidate-binary", "base-binary",
        "quickjs-binary", "candidate-receipt", "base-receipt", "quickjs-receipt",
    ):
        build.add_argument(f"--{name}", type=Path, required=True)
    for name in (
        "candidate-repo", "candidate-revision", "base-repo", "base-revision",
        "profile-id", "platform", "rust-toolchain", "rust-target",
        "quickjs-toolchain", "quickjs-target", "quickjs-cc",
    ):
        build.add_argument(f"--{name}", required=True)
    build.set_defaults(function=prepare)

    render = commands.add_parser("summary")
    render.add_argument("--report", type=Path, required=True)
    render.add_argument("--markdown", type=Path, required=True)
    render.add_argument("--json-output", type=Path, required=True)
    render.add_argument("--status-output", type=Path, required=True)
    render.add_argument("--harness-mode", choices=HARNESS_MODES, required=True)
    render.add_argument("--harness-revision", required=True)
    render.set_defaults(function=summary)

    state = commands.add_parser("status")
    state.add_argument("--state", choices=("pending", "failed"), required=True)
    state.add_argument("--phase", required=True)
    state.add_argument("--output-dir", type=Path, required=True)
    state.add_argument("--harness-mode", choices=HARNESS_MODES, required=True)
    state.add_argument("--harness-revision", required=True)
    state.add_argument("--candidate-revision", required=True)
    state.add_argument("--base-revision", required=True)
    state.add_argument("--reference-revision", required=True)
    state.add_argument("--message", required=True)
    state.set_defaults(function=status)
    return parser


def main() -> int:
    try:
        args = _parser().parse_args()
        args.function(args)
        return 0
    except (PreviewError, ManifestError, OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

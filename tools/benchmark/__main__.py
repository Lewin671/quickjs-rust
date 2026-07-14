"""Command-line interface for the repository benchmark runner."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import replace
from datetime import datetime, timezone
from pathlib import Path

from .adapters import AdapterError, load_engine
from .planning import measurement_plan
from .runner import BenchmarkRun, JsonlWriter
from .receipts import ReceiptError, load_receipt
from .schema import ManifestError, load_manifest


def _parser(root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="run versioned quickjs-rust black-box benchmarks")
    parser.add_argument("--manifest", type=Path, default=root / "benchmarks/manifest.json")
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--candidate-receipt", type=Path)
    parser.add_argument("--candidate-adapter", choices=("qjs-rust-raw", "qjs-file"), default="qjs-rust-raw")
    parser.add_argument("--candidate-identity", default="qjs-rust")
    parser.add_argument("--base", type=Path)
    parser.add_argument("--base-receipt", type=Path)
    parser.add_argument("--base-adapter", choices=("qjs-rust-raw", "qjs-file"), default="qjs-rust-raw")
    parser.add_argument("--base-identity", default="qjs-rust")
    parser.add_argument("--quickjs-ng", type=Path)
    parser.add_argument("--quickjs-ng-receipt", type=Path)
    parser.add_argument("--quickjs-ng-adapter", choices=("qjs-rust-raw", "qjs-file"), default="qjs-file")
    parser.add_argument("--quickjs-ng-identity", default="quickjs-ng")
    parser.add_argument("--case", action="append", default=[], help="exact case id; repeatable")
    parser.add_argument("--filter", help="substring filter applied after --case")
    parser.add_argument("--blocks", type=int, default=30)
    parser.add_argument("--seed", type=int, default=20250713)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--dry-run", action="store_true", help="validate and print the plan without binaries")
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    parser = _parser(root)
    args = parser.parse_args()
    if args.blocks < 1:
        parser.error("--blocks must be >= 1")
    try:
        manifest = load_manifest(args.manifest)
        selected = list(manifest.cases)
        if args.case:
            requested = set(args.case)
            unknown = requested - {case.id for case in selected}
            if unknown:
                raise ManifestError(f"unknown selected cases: {sorted(unknown)}")
            selected = [case for case in selected if case.id in requested]
        if args.filter:
            selected = [case for case in selected if args.filter in case.id or args.filter in case.family]
        if not selected:
            raise ManifestError("case selection is empty")

        role_args = [
            (
                "candidate", args.candidate_adapter, args.candidate_identity,
                args.candidate, args.candidate_receipt,
            ),
            ("base", args.base_adapter, args.base_identity, args.base, args.base_receipt),
            (
                "quickjs-ng", args.quickjs_ng_adapter, args.quickjs_ng_identity,
                args.quickjs_ng, args.quickjs_ng_receipt,
            ),
        ]
        for role, _adapter, identity, binary, receipt in role_args:
            if identity not in manifest.build_recipes:
                raise ManifestError(f"{role}: unknown engine identity {identity!r}")
            if role == "quickjs-ng" and identity != manifest.reference_identity:
                raise ManifestError(
                    f"quickjs-ng role identity must be {manifest.reference_identity!r}"
                )
            if receipt is not None and binary is None:
                raise ReceiptError(f"--{role}-receipt requires --{role}")
        present_roles = [
            role for role, _adapter, _identity, binary, _receipt in role_args if binary is not None
        ]
        if args.dry_run:
            roles = present_roles or ["candidate", "base", "quickjs-ng"]
            plan = measurement_plan(roles, [case.id for case in selected], args.blocks, args.seed)
            print(json.dumps({
                "blocks": args.blocks,
                "cases": [case.id for case in selected],
                "claim_eligible": False,
                "comparison_input_complete": False,
                "engine_declarations": [
                    {"role": role, "adapter_id": adapter, "engine_identity": identity}
                    for role, adapter, identity, binary, _receipt in role_args
                    if binary is not None or not present_roles
                ],
                "manifest_sha256": manifest.sha256,
                "lane_id": manifest.lane_id,
                "portfolio_complete": len(selected) == len(manifest.cases),
                "profile_id": manifest.profile.id,
                "protocol_id": manifest.protocol_id,
                "protocol_sha256": manifest.protocol_sha256,
                "roles": roles,
                "samples": [item.__dict__ for item in plan],
                "schema_version": 4,
                "seed": args.seed,
                "series_id": manifest.series_id,
            }, sort_keys=True, indent=2))
            return 0
        if not present_roles:
            parser.error("provide at least one engine binary (or use --dry-run)")
        engines = []
        for role, adapter_id, identity, binary, receipt_path in role_args:
            if binary is None:
                continue
            engine = load_engine(role, adapter_id, identity, binary)
            if receipt_path is not None:
                pinned_reference = None
                if role == "quickjs-ng":
                    pinned_reference = (
                        manifest.reference_identity,
                        manifest.reference_repo,
                        manifest.reference_revision,
                    )
                receipt = load_receipt(
                    receipt_path,
                    expected_binary_sha256=engine.binary_sha256,
                    expected_engine_identity=identity,
                    expected_profile_id=manifest.profile.id,
                    expected_recipe=manifest.build_recipes[identity],
                    pinned_reference=pinned_reference,
                )
                engine = replace(engine, receipt=receipt)
            engines.append(engine)
        output = args.output
        if output is None:
            timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            output = root / "target/benchmarks" / f"run-{timestamp}.jsonl"
        output = output.expanduser().resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open("x", encoding="utf-8") as handle:
            success = BenchmarkRun(
                manifest, engines, selected, args.blocks, args.seed, JsonlWriter(handle), root
            ).execute()
        print(output)
        return 0 if success else 1
    except (ManifestError, ReceiptError, AdapterError, OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

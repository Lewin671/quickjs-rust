"""Command line for one resource measurement lane per evidence file."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import replace
from pathlib import Path

from .adapters import AdapterError, load_engine
from .planning import measurement_plan
from .receipts import ReceiptError, load_receipt
from .resource_process import ResourceProcessError
from .resource_runner import ResourceJsonlWriter, ResourceRun, ResourceRunError
from .resource_schema import ResourceManifestError, load_resource_manifest


ALIASES = {
    "fresh": "fresh_process_latency/wall_ns_per_process",
    "rss": "peak_rss/bytes",
    "size": "binary_size/bytes",
}


def _parser(root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="run one versioned QuickJS resource lane")
    parser.add_argument("--manifest", type=Path, default=root / "benchmarks/resources.json")
    parser.add_argument("--lane", choices=tuple(ALIASES), required=True)
    for role, default_adapter, default_identity in (
        ("candidate", "qjs-rust-raw", "qjs-rust"),
        ("base", "qjs-rust-raw", "qjs-rust"),
        ("quickjs-ng", "qjs-file", "quickjs-ng"),
    ):
        parser.add_argument(f"--{role}", dest=role.replace("-", "_"), type=Path)
        parser.add_argument(f"--{role}-receipt", dest=f"{role.replace('-', '_')}_receipt", type=Path)
        parser.add_argument(
            f"--{role}-adapter", dest=f"{role.replace('-', '_')}_adapter",
            choices=("qjs-rust-raw", "qjs-file"), default=default_adapter,
        )
        parser.add_argument(
            f"--{role}-identity", dest=f"{role.replace('-', '_')}_identity",
            default=default_identity,
        )
    parser.add_argument("--blocks", type=int)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    return parser


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    parser = _parser(root)
    args = parser.parse_args()
    try:
        manifest = load_resource_manifest(args.manifest)
        lane = manifest.lanes[ALIASES[args.lane]]
        blocks = args.blocks if args.blocks is not None else lane.initial_blocks
        seed = args.seed if args.seed is not None else lane.seed
        if lane.kind == "dynamic" and blocks not in {lane.initial_blocks, lane.max_blocks}:
            raise ResourceManifestError("dynamic resource runs require exactly 30 or 60 blocks")
        if lane.kind == "dynamic" and seed != lane.seed:
            raise ResourceManifestError("dynamic resource runs require the manifest seed")
        if lane.kind == "static" and blocks != 1:
            raise ResourceManifestError("binary size requires exactly one sample per role")
        declarations = []
        for role in ("candidate", "base", "quickjs-ng"):
            key = role.replace("-", "_")
            declarations.append((
                role, getattr(args, f"{key}_adapter"), getattr(args, f"{key}_identity"),
                getattr(args, key), getattr(args, f"{key}_receipt"),
            ))
        for role, _adapter, identity, binary, receipt in declarations:
            if identity not in manifest.build_recipes:
                raise ResourceManifestError(f"{role}: unknown engine identity {identity!r}")
            expected_identity = (
                manifest.reference_identity if role == "quickjs-ng" else "qjs-rust"
            )
            if identity != expected_identity:
                raise ResourceManifestError(
                    f"{role}: identity must be {expected_identity!r}"
                )
            if receipt is not None and binary is None:
                raise ReceiptError(f"--{role}-receipt requires --{role}")
        present = [item for item in declarations if item[3] is not None]
        if args.dry_run:
            roles = [item[0] for item in present] or ["candidate", "base", "quickjs-ng"]
            if lane.kind == "dynamic":
                assert lane.case is not None
                plan = [item.__dict__ for item in measurement_plan(
                    roles, [lane.case.id], blocks, seed
                )]
            else:
                plan = [
                    {"block": None, "order": order, "role": role, "case_id": None}
                    for order, role in enumerate(roles)
                ]
            print(json.dumps({
                "blocks": blocks, "claim_eligible": False, "lane_id": lane.id,
                "manifest_sha256": manifest.sha256, "plan": plan,
                "protocol_id": manifest.protocol_id,
                "protocol_sha256": manifest.protocol_sha256,
                "schema_version": 1, "seed": seed if lane.kind == "dynamic" else None,
            }, sort_keys=True, indent=2))
            return 0
        if not present:
            parser.error("provide at least one engine binary (or use --dry-run)")
        if args.output is None:
            parser.error("--output is required for evidence runs")
        engines = []
        for role, adapter, identity, binary, receipt_path in present:
            engine = load_engine(role, adapter, identity, binary)
            if receipt_path is not None:
                receipt = load_receipt(
                    receipt_path,
                    expected_binary_sha256=engine.binary_sha256,
                    expected_engine_identity=identity,
                    expected_profile_id=manifest.profile.id,
                    expected_recipe=manifest.build_recipes[identity],
                    pinned_reference=(
                        manifest.reference_identity, manifest.reference_repo,
                        manifest.reference_revision,
                    ) if role == "quickjs-ng" else None,
                )
                engine = replace(engine, receipt=receipt)
            engines.append(engine)
        output = args.output.expanduser().resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open("x", encoding="utf-8") as handle:
            success = ResourceRun(
                manifest, lane, engines, blocks, seed, ResourceJsonlWriter(handle), root
            ).execute()
        print(output)
        return 0 if success else 1
    except (
        AdapterError, OSError, ReceiptError, ResourceManifestError,
        ResourceProcessError, ResourceRunError, ValueError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

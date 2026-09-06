"""Complete synthetic evidence for decision tests; never a timing claim."""
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BROAD = json.loads((ROOT / "benchmarks/manifest.json").read_text())
EXTERNAL = json.loads((ROOT / "benchmarks/external-preview.json").read_text())
SENTINEL = json.loads((ROOT / "benchmarks/generic-sentinels-manifest.json").read_text())
HOST = {"node": "test-host", "system": "test", "machine": "test"}


def effect(ratio):
    return {"ratio": ratio, "confidence_interval": {"lower": ratio * 0.999, "upper": ratio * 1.001},
            "valid_blocks": 30, "relative_half_width": 0.002, "status": "healthy"}


def summary(candidate, base):
    return {"state": "success", "engines": {
        role: {"source_revision": revision, "binary_sha256": revision[0] * 64}
        for index, (role, revision) in enumerate([
            ("candidate", candidate), ("base", base),
            ("quickjs-ng", BROAD["reference_engine"]["revision"])])}}


def internal(engines, count=25, lane="broad"):
    inventory = (BROAD if lane == "broad" else SENTINEL)["cases"]
    if lane == "broad":
        inventory = inventory[:count]
    return {"schema_id": "quickjs-benchmark-report", "schema_version": 3,
            "run": {"host": HOST, "engines": [
                {"role": role, "binary_sha256": engine["binary_sha256"],
                 "receipt": {"source": {"revision": engine["source_revision"], "dirty": False}}}
                for role, engine in engines.items()]},
            "health": {"input_valid": True, "status": "healthy", "blocks": {"valid": 30, "invalid": 0},
                       "linearity": {"status": "pass"}},
            "coverage": {"comparison_input_complete": True, "physical_plan_complete": True,
                         "roles": 3, "cases": len(inventory)},
            "comparisons": {
                "candidate_vs_base": {"cases": {c["id"]: effect(0.99) for c in inventory}},
                "candidate_vs_quickjs_ng": {"cases": {
                    c["id"]: {**effect(0.8 if c["id"] == "plain_function_call" else 0.3),
                              "family": c["family"]} for c in inventory}}}}


def external(engines, target_base=0.9, complete=True):
    suites = []
    for suite in sorted(EXTERNAL["suites"], key=lambda s: s["id"] != "sunspider-1.0"):
        cases = []
        for c in suite["cases"]:
            ratio, ng = 0.99, 0.3
            if suite["id"] == "sunspider-1.0":
                if c["id"] == "3d-cube":
                    ratio, ng = target_base, 4.0
                elif c["id"] == "3d-morph":
                    ratio, ng = 1.01, 2.0
            cases.append({"id": c["id"], "capability": dict.fromkeys(engines, "ok"),
                          "candidate_over_base": ratio, "candidate_over_quickjs_ng": ng,
                          "paired_comparisons": {"base": effect(ratio), "quickjs-ng": effect(ng)}})
        if not complete:
            cases[-1]["paired_comparisons"]["base"] = None
            cases[-1]["candidate_over_base"] = None
        suites.append({"id": suite["id"], "cases": cases,
                       "complete_base_comparison": complete, "complete_comparison": complete})
    return {"schema_version": 2, "artifact_type": "quickjs-external-preview-report",
            "host": HOST, "blocks": 30,
            "binary_sha256": {role: engine["binary_sha256"] for role, engine in engines.items()},
            "suites": suites}


def profile(directory, revision):
    asset = directory / "profile.sample"
    asset.write_text("30 stack samples in generic VM\n")
    workload = directory / "profile.js"
    workload.write_text("f();\n")
    receipt = {"schema_version": 1, "artifact_type": "quickjs-profile-receipt",
               "base_sha": revision, "binary_sha256": revision[0] * 64,
               "opportunity_ids": ["external/sunspider-1.0/3d-cube"],
               "tool": "test sampler", "command": ["sample", "123", "10"],
               "profile": {"path": asset.name, "sha256": hashlib.sha256(asset.read_bytes()).hexdigest()},
               "workload": {"path": workload.name, "sha256": hashlib.sha256(workload.read_bytes()).hexdigest()}}
    path = directory / "profile-receipt.json"
    path.write_text(json.dumps(receipt))
    return path.name, hashlib.sha256(path.read_bytes()).hexdigest()


def zero_gap(candidate):
    total = 1234  # Synthetic inventory; decision tests do not need a Test262 checkout.
    return {"schema": 1, "commit": candidate, "total": total, "configured": total,
            "ng_config_skipped": 0,
            "rust": {"pass": total, "fail": 0, "timeout": 0, "not_run": 0},
            "ng": {"pass": total, "fail": 0, "timeout": 0},
            "comparison": dict.fromkeys(["actionable_gap", "ng_pass_rust_fail",
                                        "ng_pass_rust_timeout", "ng_pass_rust_not_run"], 0)}


def isolate_test262_inventory(test_case):
    """Keep unit tests independent of optional reference submodule checkouts."""
    from unittest.mock import patch
    probe = patch("tools.benchmark.performance_evaluation.pinned_test262_count", return_value=1234)
    probe.start()
    test_case.addCleanup(probe.stop)

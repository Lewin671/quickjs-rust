"""What the performance policy freezes.

Pure data: the protocol keys and their manifest/identity shapes, the pinned
workflow digest, and the hosted-preview implementation inventory. It lives
apart from the validation that reads it so the frozen set can be reviewed as
a list of commitments rather than found inside the code that enforces them.

Adding a hosted lane means adding its protocol here and its inputs to the
inventory; omitting either lets a workload change alter hosted evidence while
the audit still passes.
"""

from __future__ import annotations

PROTOCOL_KEYS = (
    "resource_analysis",
    "resource_measurement",
    "sentinel_measurement",
    "throughput_analysis",
    "throughput_measurement",
)
PROTOCOL_SHAPES = {
    "throughput_measurement": (
        "benchmarks/manifest.json", "quickjs-measurement-protocol-v8"
    ),
    "throughput_analysis": (
        "benchmarks/analysis.json", "quickjs-analysis-protocol-v5"
    ),
    # The hosted preview measures a second frozen portfolio, so its manifest
    # and workload must be pinned here too. Without this, a sentinel workload
    # change would alter hosted evidence while passing the audit on the
    # strength of its own self-hash alone.
    "sentinel_measurement": (
        "benchmarks/generic-sentinels-manifest.json",
        "quickjs-generic-sentinel-protocol-v1",
    ),
    "resource_measurement": (
        "benchmarks/resources.json", "quickjs-resource-measurement-protocol-v1"
    ),
    "resource_analysis": (
        "benchmarks/resource-analysis.json", "quickjs-resource-analysis-protocol-v1"
    ),
}
EXPECTED_WORKFLOW_SHA256 = "bb319acebba6297f4b99e4982d442cac4a0ca3d3c5f018e7f038ea4e87f084e7"
PREVIEW_ORCHESTRATOR = "scripts/performance-preview.sh"
PREVIEW_ROLES = ("candidate", "base", "quickjs-ng")
PREVIEW_IMPLEMENTATION_FILES = (
    ".cargo/config.toml",
    ".github/actions/setup-rust/action.yml",
    ".github/workflows/performance-smoke.yml",
    "benchmarks/external-corpora.json",
    "benchmarks/external-preview.json",
    "scripts/external-corpus-audit.sh",
    "scripts/external-performance-preview.sh",
    "scripts/performance-policy-audit.sh",
    "scripts/performance-preview.sh",
    "tools/benchmark/build_cache.py",
    "tools/benchmark/build_cache_identity.py",
    "tools/benchmark/external_corpora.py",
    "tools/benchmark/external_preview.py",
    "tools/benchmark/external_preview_markdown.py",
    "tools/benchmark/hosted_preview.py",
    "tools/benchmark/performance_policy.py",
    "tools/benchmark/performance_policy_frozen.py",
    "tools/benchmark/preview.py",
    "tools/benchmark/preview_sentinel.py",
)

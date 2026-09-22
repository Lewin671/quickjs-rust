"""Counter-based candidate/base screen for local performance iteration.

A screen answers "is this change worth a formal measurement?" on a shared
machine. Hardware counters (instructions retired, cycles) barely move under
background load, while wall time does, so the screen reports counter ratios
first and wall time only as context. Its output is diagnostic: it is not
decision evidence and `performance-decision.sh decide` never reads it.

Internal workloads are measured by increment: each pair runs N and 2N
iterations and keeps the difference, which removes process startup and
parsing. External bundles and plain scripts are whole-process measurements.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import re
import statistics
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Sequence

from .external_preview import (
    _EXPECTED_STDOUT_BY_ROLE,
    ExternalPreviewError,
    _bundle_source,
    fetch_corpora,
    load_manifest,
)
from . import symbols
from .process import ProcessResult, run_process

ROOT = Path(__file__).resolve().parents[2]
# `user_ns` is sampled for calibration only; `/usr/bin/time` reports it at
# 10 ms resolution, too coarse for a ratio.
METRICS = ("instructions", "cycles", "wall_ns")
INTERNAL_WORKLOADS = {
    "sentinel": ROOT / "benchmarks/workloads/generic-sentinels.js",
    "broad": ROOT / "benchmarks/workloads/broad-micro.js",
}
INTERNAL_MANIFESTS = {
    "sentinel": ROOT / "benchmarks/generic-sentinels-manifest.json",
    "broad": ROOT / "benchmarks/manifest.json",
}
EXTERNAL_MANIFEST = ROOT / "benchmarks/external-preview.json"
DEFAULT_CACHE = ROOT / "target/benchmarks/external-cache"
RESULT_PREFIX = "QJS_BENCH_RESULT "
CALIBRATION_TARGET_NS = 200_000_000
CALIBRATION_MAX_ITERATIONS = 1 << 26


class ScreenError(RuntimeError):
    """A screen that cannot produce trustworthy counter ratios."""


class CountersUnavailable(ScreenError):
    """The host cannot report the hardware counters the screen relies on."""


@dataclass(frozen=True)
class Sample:
    instructions: int
    cycles: int
    user_ns: int
    wall_ns: int
    stdout: str

    def metric(self, name: str) -> int:
        return getattr(self, name)


@dataclass(frozen=True)
class CaseSpec:
    id: str
    kind: str  # "internal" or "whole"
    workload: Path
    case_id: str | None = None
    expected_stdout: str | None = None


@dataclass
class CaseResult:
    id: str
    kind: str
    iterations: int | None
    ratios: dict[str, list[float]] = field(default_factory=dict)

    def median(self, metric: str) -> float:
        return statistics.median(self.ratios[metric])

    def spread(self, metric: str) -> tuple[float, float]:
        values = self.ratios[metric]
        return min(values), max(values)


# Counter collection ------------------------------------------------------

_MAC_LINE = re.compile(r"^\s*(\d+)\s+(instructions retired|cycles elapsed)\s*$")
_MAC_TIMES = re.compile(r"^\s*([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys\s*$")


def parse_macos_time(stderr: str) -> dict[str, int]:
    """Parse `/usr/bin/time -l` output appended to a process's stderr."""
    values: dict[str, int] = {}
    for line in stderr.splitlines():
        counter = _MAC_LINE.match(line)
        if counter:
            key = "instructions" if counter.group(2).startswith("instructions") else "cycles"
            values[key] = int(counter.group(1))
            continue
        times = _MAC_TIMES.match(line)
        if times:
            values["user_ns"] = round(float(times.group(2)) * 1e9)
    if "instructions" not in values or "cycles" not in values:
        raise CountersUnavailable("/usr/bin/time -l reported no instruction or cycle counters")
    if values["instructions"] == 0 or values["cycles"] == 0:
        raise CountersUnavailable("/usr/bin/time -l reported zero hardware counters")
    return values


def parse_perf_stat(stderr: str) -> dict[str, int]:
    """Parse `perf stat -x,` CSV lines for instructions, cycles and task-clock."""
    values: dict[str, int] = {}
    for line in stderr.splitlines():
        fields = line.split(",")
        if len(fields) < 3:
            continue
        raw, _unit, event = fields[0], fields[1], fields[2]
        name = event.split(":")[0]
        if name not in {"instructions", "cycles", "task-clock"}:
            continue
        if raw.startswith("<"):
            raise CountersUnavailable(f"perf stat reported {name} as {raw}")
        if name == "task-clock":
            values["user_ns"] = round(float(raw) * 1e6)  # msec -> ns
        else:
            values[name] = int(float(raw))
    if "instructions" not in values or "cycles" not in values:
        raise CountersUnavailable("perf stat reported no instruction or cycle counters")
    return values


@dataclass(frozen=True)
class CounterTool:
    prefix: tuple[str, ...]
    parse: Callable[[str], dict[str, int]]


def counter_tool(system: str | None = None) -> CounterTool:
    system = system or platform.system()
    if system == "Darwin":
        return CounterTool(("/usr/bin/time", "-l"), parse_macos_time)
    if system == "Linux":
        return CounterTool(
            ("perf", "stat", "-x,", "-e", "instructions:u,cycles:u,task-clock", "--"),
            parse_perf_stat,
        )
    raise CountersUnavailable(f"no hardware counter tool is known for {system}")


def sample(tool: CounterTool, argv: Sequence[str], timeout: float,
           runner: Callable[[list[str], float], ProcessResult] = run_process) -> Sample:
    result = runner([*tool.prefix, *argv], timeout)
    if result.timed_out:
        raise ScreenError(f"timed out: {' '.join(argv)}")
    if result.exit_code != 0:
        raise ScreenError(f"exit status {result.exit_code}: {' '.join(argv)}\n{result.stderr[-2000:]}")
    values = tool.parse(result.stderr)
    return Sample(
        instructions=values["instructions"],
        cycles=values["cycles"],
        user_ns=values.get("user_ns", 0),
        wall_ns=result.duration_ns,
        stdout=result.stdout,
    )


# Cases -------------------------------------------------------------------

def _internal_case_ids(lane: str) -> list[str]:
    with INTERNAL_MANIFESTS[lane].open(encoding="utf-8") as handle:
        return [case["id"] for case in json.load(handle)["cases"]]


def resolve_cases(specs: Sequence[str], cache_root: Path, work_dir: Path) -> list[CaseSpec]:
    """Expand `sentinel`, `broad`, `sentinel/<id>`, `broad/<id>`,
    `external/<suite>/<case>` and `file:<path>` into runnable cases."""
    cases: list[CaseSpec] = []
    external_manifest = None
    for spec in specs:
        if spec in INTERNAL_WORKLOADS:
            cases.extend(
                CaseSpec(f"{spec}/{case_id}", "internal", INTERNAL_WORKLOADS[spec], case_id)
                for case_id in _internal_case_ids(spec)
            )
        elif spec.split("/", 1)[0] in INTERNAL_WORKLOADS and "/" in spec:
            lane, case_id = spec.split("/", 1)
            if case_id not in _internal_case_ids(lane):
                raise ScreenError(f"unknown {lane} case {case_id}")
            cases.append(CaseSpec(spec, "internal", INTERNAL_WORKLOADS[lane], case_id))
        elif spec.startswith("external/"):
            parts = spec.split("/")
            if len(parts) != 3:
                raise ScreenError(f"external cases are external/<suite>/<case>: {spec}")
            if external_manifest is None:
                external_manifest = load_manifest(EXTERNAL_MANIFEST)
                fetch_corpora(external_manifest, cache_root)
            suite = next((s for s in external_manifest.suites if s.id == parts[1]), None)
            case = next((c for c in suite.cases if c.id == parts[2]), None) if suite else None
            if case is None:
                raise ScreenError(f"unknown external case {spec}")
            bundle = work_dir / f"{parts[1]}--{parts[2]}.js"
            bundle.write_text(_bundle_source(suite, case, cache_root.resolve()), encoding="utf-8")
            cases.append(CaseSpec(spec, "whole", bundle,
                                  expected_stdout=_EXPECTED_STDOUT_BY_ROLE["candidate"]))
        elif spec.startswith("file:"):
            path = Path(spec[len("file:"):]).resolve()
            if not path.is_file():
                raise ScreenError(f"no such script {path}")
            cases.append(CaseSpec(spec, "whole", path))
        else:
            raise ScreenError(f"unrecognized case {spec!r}")
    return cases


def _argv(binary: Path, case: CaseSpec, iterations: int | None) -> list[str]:
    argv = [str(binary), "--raw", str(case.workload)]
    if case.kind == "internal":
        argv.extend([str(case.case_id), str(iterations)])
    return argv


def _check_output(case: CaseSpec, stdout: str) -> str:
    """Return the part of stdout both engines must agree on."""
    if case.kind == "internal":
        lines = [line for line in stdout.splitlines() if line.startswith(RESULT_PREFIX)]
        if len(lines) != 1:
            raise ScreenError(f"{case.id}: expected one {RESULT_PREFIX.strip()} line")
        result = json.loads(lines[0][len(RESULT_PREFIX):])
        return json.dumps({"operations": result["operations"], "checksum": result["checksum"]})
    if case.expected_stdout is not None and stdout != case.expected_stdout:
        raise ScreenError(f"{case.id}: stdout did not match the external sentinel contract")
    return stdout


# Measurement -------------------------------------------------------------

def calibrate(measure: Callable[[int], Sample], start: int = 1024) -> int:
    """Double iterations until one run's user time reaches the target."""
    iterations = start
    while iterations < CALIBRATION_MAX_ITERATIONS:
        taken = measure(iterations)
        elapsed = taken.user_ns or taken.wall_ns
        if elapsed >= CALIBRATION_TARGET_NS:
            return iterations
        if elapsed <= 0:
            iterations *= 8
            continue
        scale = CALIBRATION_TARGET_NS / elapsed
        iterations = int(iterations * min(8.0, max(2.0, scale * 1.1)))
    return CALIBRATION_MAX_ITERATIONS


def increment(small: Sample, large: Sample, metric: str) -> int:
    return large.metric(metric) - small.metric(metric)


def screen_case(case: CaseSpec, candidate: Path, base: Path, pairs: int, tool: CounterTool,
                timeout: float, iterations: int | None = None,
                runner: Callable[[list[str], float], ProcessResult] = run_process) -> CaseResult:
    def run(binary: Path, count: int | None) -> Sample:
        return sample(tool, _argv(binary, case, count), timeout, runner)

    if case.kind == "internal" and iterations is None:
        iterations = calibrate(lambda count: run(base, count))
    result = CaseResult(case.id, case.kind, iterations, {metric: [] for metric in METRICS})
    for pair in range(pairs):
        # Alternate which role goes first so slow drift cancels across pairs.
        roles = [("base", base), ("candidate", candidate)]
        if pair % 2:
            roles.reverse()
        taken: dict[str, dict[str, Sample]] = {}
        for role, binary in roles:
            if case.kind == "internal":
                assert iterations is not None
                taken[role] = {"small": run(binary, iterations), "large": run(binary, iterations * 2)}
            else:
                taken[role] = {"whole": run(binary, None)}
        if case.kind == "whole":
            if _check_output(case, taken["candidate"]["whole"].stdout) != _check_output(
                    case, taken["base"]["whole"].stdout):
                raise ScreenError(f"{case.id}: candidate and base printed different output")
        else:
            for size in ("small", "large"):
                if _check_output(case, taken["candidate"][size].stdout) != _check_output(
                        case, taken["base"][size].stdout):
                    raise ScreenError(f"{case.id}: candidate and base disagree on the result checksum")
        for metric in METRICS:
            if case.kind == "internal":
                base_cost = increment(taken["base"]["small"], taken["base"]["large"], metric)
                candidate_cost = increment(taken["candidate"]["small"], taken["candidate"]["large"], metric)
            else:
                base_cost = taken["base"]["whole"].metric(metric)
                candidate_cost = taken["candidate"]["whole"].metric(metric)
            if base_cost > 0 and candidate_cost > 0:
                result.ratios[metric].append(candidate_cost / base_cost)
    for metric in ("instructions", "cycles"):
        if not result.ratios[metric]:
            raise ScreenError(f"{case.id}: no positive {metric} increment; raise iterations")
    return result


def geomean(values: Sequence[float]) -> float:
    return math.exp(sum(math.log(value) for value in values) / len(values))


def check_load(max_load: float, require_quiet: bool, settle_seconds: float = 0.0,
               sleep: Callable[[float], None] | None = None,
               loadavg: Callable[[], tuple[float, float, float]] | None = None) -> float:
    """Wait up to `settle_seconds` for the 1-minute load to drop below
    `max_load` (a build that just finished keeps it high for a while). A
    still-busy host is reported, not refused, because the counter ratios the
    screen judges tolerate load; `require_quiet` refuses instead."""
    import time

    sleep = sleep or time.sleep
    loadavg = loadavg or os.getloadavg
    waited = 0.0
    load = loadavg()[0]
    while load > max_load and waited < settle_seconds:
        if waited == 0.0:
            print(f"waiting for load {load:.2f} to settle below {max_load:.2f} ...", file=sys.stderr, flush=True)
        sleep(5.0)
        waited += 5.0
        load = loadavg()[0]
    if load > max_load and require_quiet:
        raise ScreenError(f"1-minute load average {load:.2f} exceeds {max_load:.2f} and --require-quiet was given")
    return load


# Plan gate ---------------------------------------------------------------

@dataclass(frozen=True)
class PlanGate:
    unit_id: str
    base_sha: str
    target_ids: tuple[str, ...]
    control_ids: tuple[str, ...]
    target_max: float
    control_max: float

    def case_specs(self) -> list[str]:
        """Targets, controls, then every sentinel, without duplicates."""
        specs = [*self.target_ids, *self.control_ids]
        specs += [f"sentinel/{case_id}" for case_id in _internal_case_ids("sentinel")]
        return list(dict.fromkeys(specs))


def load_plan_gate(path: Path) -> PlanGate:
    with path.open(encoding="utf-8") as handle:
        unit = json.load(handle)
    try:
        gate = unit["fast_gate"]
        return PlanGate(
            unit_id=unit["unit_id"],
            base_sha=unit["base_sha"],
            target_ids=tuple(gate["target_ids"]),
            control_ids=tuple(gate["control_ids"]),
            target_max=float(gate["target_max_candidate_over_base"]),
            control_max=float(gate["control_max_candidate_over_base"]),
        )
    except (KeyError, TypeError, ValueError) as error:
        raise ScreenError(f"{path}: not a performance unit plan with a fast_gate ({error})") from error


def gate_verdict(results: Sequence[CaseResult], gate: PlanGate) -> tuple[bool, list[str]]:
    """Apply the screen gate from docs/performance-workflow.md to cycles."""
    by_id = {result.id: result for result in results}
    failures: list[str] = []
    for case_id in gate.target_ids:
        result = by_id[case_id]
        median = result.median("cycles")
        if median > gate.target_max:
            failures.append(f"target {case_id}: median cycles {median:.4f} > {gate.target_max}")
        if result.spread("cycles")[1] >= 1.0:
            failures.append(f"target {case_id}: a pair did not improve (max {result.spread('cycles')[1]:.4f})")
    for case_id in gate.case_specs():
        if case_id in gate.target_ids:
            continue
        median = by_id[case_id].median("cycles")
        if median > gate.control_max:
            failures.append(f"control {case_id}: median cycles {median:.4f} > {gate.control_max}")
    return not failures, failures


# Report and CLI ----------------------------------------------------------

def render(results: Sequence[CaseResult], pairs: int, aa: bool) -> str:
    title = "A/A control (same binary as both roles)" if aa else "candidate / base"
    lines = [
        f"Screen: {title}, {pairs} pair(s); medians with [min, max] across pairs.",
        "Counters decide whether to run a formal measurement; this is not decision evidence.",
        "",
        "| case | iterations | instructions | cycles | wall (context) |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]

    def cell(result: CaseResult, metric: str) -> str:
        if not result.ratios[metric]:
            return "n/a"
        low, high = result.spread(metric)
        return f"{result.median(metric):.4f} [{low:.4f}, {high:.4f}]"

    for result in results:
        iterations = str(result.iterations) if result.iterations is not None else "whole process"
        lines.append(
            f"| {result.id} | {iterations} | {cell(result, 'instructions')} | "
            f"{cell(result, 'cycles')} | {cell(result, 'wall_ns')} |"
        )
    lines.append("")
    for metric in METRICS:
        medians = [r.median(metric) for r in results if r.ratios[metric]]
        if medians:
            lines.append(f"geomean {metric}: {geomean(medians):.4f}")
    return "\n".join(lines)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python3 -m tools.benchmark.screen",
        description="counter-based candidate/base screen; diagnostic only, never decision evidence",
    )
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--base", type=Path, help="omit together with --aa to screen the candidate against itself")
    parser.add_argument("--aa", action="store_true", help="use the candidate as both roles to measure the noise floor")
    parser.add_argument(
        "--case", action="append", default=[],
        help="sentinel | broad | sentinel/<id> | broad/<id> | external/<suite>/<case> | file:<path>; "
             "repeatable (default: sentinel)",
    )
    parser.add_argument("--pairs", type=int, default=5)
    parser.add_argument("--iterations", type=int, help="fixed N for internal cases (default: calibrate to ~200 ms)")
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--max-load", type=float, default=(os.cpu_count() or 2) / 2)
    parser.add_argument("--settle", type=float, default=30.0,
                        help="seconds to wait for a high load average to drop before starting")
    parser.add_argument("--require-quiet", action="store_true",
                        help="refuse to run while the load average exceeds --max-load")
    parser.add_argument("--json", type=Path, help="also write per-pair ratios as JSON")
    parser.add_argument("--plan", type=Path,
                        help="unit plan: screen its fast_gate targets/controls plus the sentinels and "
                             "print a pass/fail verdict (exit 3 on fail)")
    parser.add_argument("--symbols", type=int, metavar="N", default=0,
                        help="also list the N largest function-size changes between the executables")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if args.aa:
        if args.base is not None:
            print("error: --aa uses the candidate as both roles; drop --base", file=sys.stderr)
            return 2
        args.base = args.candidate
    elif args.base is None:
        print("error: --base is required unless --aa is given", file=sys.stderr)
        return 2
    if args.pairs < 1:
        print("error: --pairs must be at least 1", file=sys.stderr)
        return 2
    candidate, base = args.candidate.resolve(), args.base.resolve()
    for binary in (candidate, base):
        if not binary.is_file() or not os.access(binary, os.X_OK):
            print(f"error: not an executable: {binary}", file=sys.stderr)
            return 2
    try:
        load_before = check_load(args.max_load, args.require_quiet, args.settle)
        tool = counter_tool()
        with tempfile.TemporaryDirectory(prefix="qjs-screen-") as work:
            gate = load_plan_gate(args.plan) if args.plan else None
            specs = list(dict.fromkeys([*(gate.case_specs() if gate else []), *args.case]))
            cases = resolve_cases(specs or ["sentinel"], args.cache_root, Path(work))
            results = []
            for case in cases:
                print(f"screening {case.id} ...", file=sys.stderr, flush=True)
                results.append(screen_case(case, candidate, base, args.pairs, tool, args.timeout, args.iterations))
        load_after = os.getloadavg()[0]
    except (ScreenError, ExternalPreviewError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(render(results, args.pairs, args.aa))
    print(f"load average: {load_before:.2f} before, {load_after:.2f} after")
    if max(load_before, load_after) > args.max_load:
        print(f"warning: load exceeded {args.max_load:.2f}; the wall column is unreliable, "
              "counter ratios are still usable")
    verdict = None
    if gate is not None:
        passed, failures = gate_verdict(results, gate)
        verdict = {"unit_id": gate.unit_id, "passed": passed, "failures": failures}
        print(f"\nscreen gate for {gate.unit_id}: {'PASS' if passed else 'FAIL'}")
        for failure in failures:
            print(f"  - {failure}")
    if args.symbols:
        try:
            print("\n" + symbols.render(symbols.size_diff(base, candidate), args.symbols))
        except symbols.SymbolError as error:
            print(f"\nsymbol sizes unavailable: {error}")
    if args.json:
        args.json.write_text(json.dumps({
            "artifact_type": "quickjs-screen",
            "decision_evidence": False,
            "candidate": str(candidate),
            "base": str(base),
            "aa": args.aa,
            "pairs": args.pairs,
            "load_average": {"before": load_before, "after": load_after},
            "cases": [{"id": r.id, "kind": r.kind, "iterations": r.iterations, "ratios": r.ratios} for r in results],
            "gate": verdict,
        }, indent=2) + "\n", encoding="utf-8")
    return 3 if verdict is not None and not verdict["passed"] else 0


if __name__ == "__main__":
    raise SystemExit(main())

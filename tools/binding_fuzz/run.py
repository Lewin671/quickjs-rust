"""Six-mode seeded correctness comparison; no benchmark timing or acceptance waiver.

Adapted from reviewer A's drive.py. Raw failures are retained, never substituted
with another mode's output. Common base/NG differences remain visible as PRE;
CHANGED requires review even when it may reflect a partial semantic repair.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
from collections import Counter

from .generate import Gen
from .generate_this import G

PROFILES = ("binding", "binding-simple", "receiver")
STRESS = {"QJS_VERIFY_BINDINGS": "1", "QJS_FORCE_EVAL_EXPOSURE": "1",
          "QJS_POISON_RELEASED_SCOPES": "1", "QJS_VERIFY_IC": "1"}
BLOCKERS = {"INCONS", "REGR", "CHANGED", "PANIC", "TIMEOUT", "CRASH", "REFERENCE_FAILURE"}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def program(profile, seed):
    if profile == "receiver":
        return G(seed).program()
    if profile in ("binding", "binding-simple"):
        return Gen(seed, simple_parameters=profile == "binding-simple").program()
    raise ValueError(profile)


def normalize(stdout, stderr, rust):
    lines = stdout.splitlines()
    if rust and lines and lines[-1] == "Undefined":
        lines.pop()  # CLI completion, not a generator observation.
    for line in stderr.splitlines():
        if not line.strip() or line.lstrip().startswith("at "):
            continue
        payload = re.sub(r"^error: (?:throw statement executed: )?", "", line)
        error = re.match(r"^(\w*Error)\b", payload)
        if error:
            lines.append("UNCAUGHT " + error[1])
        else:
            lines.append("STDERR " + payload)
    return lines


def execute(binary, source, extra, timeout, ng=False):
    env = {k: v for k, v in os.environ.items() if not k.startswith("QJS_")}
    env.update(TZ="UTC", **extra)
    command = [str(binary), *(["--std", "--script"] if ng else []), str(source)]
    try:
        run = subprocess.run(command, env=env, capture_output=True, text=True,
                             errors="replace", timeout=timeout)
        stdout, stderr, status = run.stdout, run.stderr, run.returncode
        timed_out = False
    except subprocess.TimeoutExpired as error:
        def decoded(value):
            return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
        stdout, stderr, status = decoded(error.stdout), decoded(error.stderr), None
        timed_out = True
    normalized = normalize(stdout, stderr, rust=not ng)
    abrupt = any(line.startswith("UNCAUGHT ") for line in normalized)
    outcome = "ok" if status == 0 else "error" if abrupt else f"exit:{status}"
    return {"exit": status, "stdout": stdout, "stderr": stderr,
            "normalized": normalized, "outcome": outcome, "timeout": timed_out,
            "panic": "panicked at" in stderr or "assertion failed" in stderr,
            "crash": status is not None and status < 0,
            "reference_failure": ng and "InternalError:" in stderr}


def observations(result):
    output = {"$outcome": result["outcome"]}
    for index, line in enumerate(result["normalized"]):
        match = re.match(r"^([^:]{1,60}): (.*)$", line)
        key, value = (match[1], match[2]) if match else (f"#{index}", line)
        if line.startswith("UNCAUGHT "):
            key = "$uncaught"
        while key in output:
            key += "'"
        output[key] = value
    output["$order"] = list(output)[1:]
    return output


def classify(modes):
    candidates = [key for key in modes if key not in ("ng", "base")]
    tags, differences = set(), []
    for mode, result in modes.items():
        for field, tag in (("panic", "PANIC"), ("timeout", "TIMEOUT"),
                           ("crash", "CRASH"), ("reference_failure", "REFERENCE_FAILURE")):
            if result[field]:
                tags.add(tag)
                differences.append({"kind": tag, "mode": mode})
    values = {key: observations(value) for key, value in modes.items()}
    if any(values[key] != values["default"] for key in candidates):
        tags.add("INCONS")
    keys = sorted(set().union(*(set(value) for value in values.values())))
    for key in keys:
        row = {mode: value.get(key, "<missing>") for mode, value in values.items()}
        bad = any(row[mode] != row["ng"] for mode in candidates)
        if not bad and row["base"] == row["ng"]:
            continue
        if not bad:
            tag = "FIXED"
        elif row["base"] == row["ng"]:
            tag = "REGR"
        elif row["default"] != row["base"]:
            tag = "CHANGED"
        else:
            tag = "PRE"
        tags.add(tag)
        differences.append({"kind": tag, "observation": key, "values": row})
    return sorted(tags), differences


def probe(profile, seed, args, output):
    text = program(profile, seed)
    source = output / "work" / f"{profile}-{seed}.js"
    source.write_text(text)
    modes = {}
    for name, binary, flags in (
        ("ng", args.ng, {}), ("default", args.binary, {}),
        ("off", args.binary, {"QJS_DISABLE_IC": "1"}),
        ("verify", args.verify_binary, {"QJS_VERIFY_IC": "1"}),
        ("stress", args.verify_binary, STRESS), ("base", args.base_binary, {}),
    ):
        modes[name] = execute(binary, source, flags, args.timeout, ng=name == "ng")
    tags, differences = classify(modes)
    row = {"profile": profile, "seed": seed,
           "source_sha256": hashlib.sha256(text.encode()).hexdigest(),
           "tags": tags, "differences": differences, "modes": modes}
    if set(tags) - {"FIXED"}:
        finding = output / "findings" / f"{profile}-{seed}.js"
        finding.write_text(text)
    source.unlink()
    return row


def source_inputs():
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "crates", "Cargo.toml", "Cargo.lock",
         "rust-toolchain.toml", ".cargo"], text=True).split("\0")
    return {name: digest(name) for name in names if name and Path(name).is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ("binary", "verify-binary", "base-binary", "ng"):
        parser.add_argument("--" + flag, type=lambda value: Path(value).resolve(), required=True)
    parser.add_argument("--base-revision", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--profiles", nargs="+", choices=PROFILES, default=PROFILES)
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--count", type=int, default=2000, help="Seeds per profile")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--timeout", type=float, default=20)
    args = parser.parse_args()
    if min(args.count, args.jobs, args.timeout) <= 0 or args.start < 0:
        parser.error("count, jobs, timeout must be positive; start must be nonnegative")
    output = args.output_dir.resolve()
    output.mkdir(parents=True)  # Refuse overwriting a receipt.
    (output / "work").mkdir()
    (output / "findings").mkdir()
    binaries = {key: {"path": str(value), "sha256": digest(value)} for key, value in
                vars(args).items() if key in ("binary", "verify_binary", "base_binary", "ng")}
    inputs = source_inputs()
    producers = {str(path): digest(path) for path in sorted(Path(__file__).parent.glob("*.py"))}
    request = {"parent_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
               "base_revision": args.base_revision, "binaries": binaries,
               "source_inputs": inputs, "producers": producers,
               "profiles": args.profiles, "start": args.start, "count": args.count,
               "jobs": args.jobs, "timeout": args.timeout, "modes": ["ng", "default", "off", "verify", "stress", "base"],
               "stress": STRESS, "started_unix": time.time()}
    (output / "request.json").write_text(json.dumps(request, indent=2) + "\n")
    counts, completed, total = Counter(), 0, args.count * len(args.profiles)
    with (output / "cases.jsonl").open("w") as rows:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = [pool.submit(probe, profile, seed, args, output)
                       for profile in args.profiles for seed in range(args.start, args.start + args.count)]
            for future in concurrent.futures.as_completed(futures):
                row = future.result()  # Harness errors abort, never become passing cases.
                rows.write(json.dumps(row) + "\n")
                counts.update(row["tags"] or ["MATCH"])
                completed += 1
                if completed % 50 == 0 or completed == total:
                    rows.flush()
                    print(f"{completed}/{total} {dict(counts)}", flush=True)
    stable = inputs == source_inputs() and all(digest(info["path"]) == info["sha256"]
                                               for info in binaries.values())
    stable = stable and all(digest(path) == value for path, value in producers.items())
    summary = {"completed": completed, "requested": total, "counts": dict(counts),
               "stable_inputs": stable, "no_new_regressions_or_mode_failures":
               stable and not any(counts[tag] for tag in BLOCKERS),
               "all_engines_agree": not any(counts[tag] for tag in counts if tag != "MATCH"),
               "finished_unix": time.time()}
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary), flush=True)
    return 0 if summary["no_new_regressions_or_mode_failures"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

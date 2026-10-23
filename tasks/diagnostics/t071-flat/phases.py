"""macOS instruction attribution across blocking parse/compile/eval barriers.

This diagnostic deliberately reads a live blocked child using the existing
counter reader. It does not use wall time or alter production engine code.
"""

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from tools.benchmark.counters import read_zombie


def sample(binary, source):
    with subprocess.Popen(
        [str(binary), "phases", str(source)], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    ) as child:
        counts, output = [], []
        try:
            for marker in ("READY", "PARSED", "COMPILED", "EXECUTED"):
                while True:
                    line = child.stdout.readline()
                    if not line:
                        raise RuntimeError(child.stderr.read())
                    if line.strip() == "__T071_" + marker:
                        break
                    output.append(line.rstrip())
                counter = read_zombie(child.pid)
                if counter is None:
                    raise RuntimeError("hardware instruction counters unavailable")
                counts.append(counter[0])
                child.stdin.write("next\n")
                child.stdin.flush()
            child.wait()
            if child.returncode:
                raise RuntimeError(child.stderr.read())
        finally:
            if child.poll() is None:
                child.kill()
        return dict(parse=counts[1] - counts[0], compile=counts[2] - counts[1],
                    execution=counts[3] - counts[2], output=output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-bin", required=True, type=Path)
    parser.add_argument("--candidate-bin", required=True, type=Path)
    parser.add_argument("--case", required=True, action="append", metavar="LABEL=JS")
    parser.add_argument("--pairs", type=int, default=3)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.pairs < 1:
        parser.error("--pairs must be positive")
    binaries = dict(base=args.base_bin.resolve(), candidate=args.candidate_bin.resolve())
    rows = []
    with args.output.open("x") as output:
        for case in args.case:
            label, source = case.split("=", 1)
            for pair in range(args.pairs):
                order = ("base", "candidate") if pair % 2 == 0 else ("candidate", "base")
                for revision in order:
                    row = dict(case=label, pair=pair, revision=revision,
                               **sample(binaries[revision], Path(source).resolve()))
                    rows.append(row)
                    output.write(json.dumps(row) + "\n")
                    output.flush()
                    print(json.dumps(row), flush=True)
            selected = [row for row in rows if row["case"] == label]
            if any(row["output"] != selected[0]["output"] for row in selected):
                raise RuntimeError("case output differs between samples")
            summary = {}
            for phase in ("parse", "compile", "execution"):
                medians = [statistics.median(row[phase] for row in selected
                           if row["revision"] == revision) for revision in binaries]
                summary[phase] = dict(base=medians[0], candidate=medians[1],
                                      ratio=medians[1] / medians[0])
            print("SUMMARY", label, json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()

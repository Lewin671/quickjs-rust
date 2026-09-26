"""Write every external case's bundle to a directory, exactly as a run
executes it, as `<suite>--<case>.js` -- for `sample` profiles, trace runs and
other diagnostic work. Diagnostic only; formal runs build their own.

Usage:
  python3 -m tools.benchmark.bundles [--output-dir target/bundles]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Sequence

from .external_preview import ExternalPreviewError, _bundle_source, fetch_corpora, load_manifest
from .screen import DEFAULT_CACHE, EXTERNAL_MANIFEST


def write_bundles(manifest_path: Path, cache_root: Path, output_dir: Path) -> int:
    manifest = load_manifest(manifest_path)
    fetch_corpora(manifest, cache_root)
    output_dir.mkdir(parents=True, exist_ok=True)
    written = 0
    for suite in manifest.suites:
        for case in suite.cases:
            bundle = output_dir / f"{suite.id}--{case.id}.js"
            bundle.write_text(_bundle_source(suite, case, cache_root.resolve()), encoding="utf-8")
            written += 1
    return written


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python3 -m tools.benchmark.bundles",
                                     description=__doc__.split("\n\n")[0])
    parser.add_argument("--output-dir", type=Path, default=Path("target/bundles"))
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--manifest", type=Path, default=EXTERNAL_MANIFEST)
    args = parser.parse_args(argv)
    try:
        written = write_bundles(args.manifest, args.cache_root, args.output_dir)
    except (ExternalPreviewError, ValueError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps({"bundles": written, "output_dir": str(args.output_dir)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

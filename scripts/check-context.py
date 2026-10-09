#!/usr/bin/env python3
"""Mechanical checks that keep the agent-facing context from drifting.

Prose rules go stale; these do not. The checks are deliberately few:

* `CLAUDE.md` is a symlink to `AGENTS.md`.
* Each entry document stays within its line budget, so current guidance
  cannot quietly turn back into a log.
* Relative Markdown links, images and `#anchors` in current documents
  resolve. Inline links and reference definitions are read the way
  CommonMark reads them; fenced code blocks and code spans are skipped.
  Not covered: indented code blocks (a link inside one is still checked),
  raw HTML links, and a `[text][label]` whose label has no definition.
* Every active task opens with the resume block from `tasks/TEMPLATE.md`.
* `tasks/README.md` lists every task file, and
  `tasks/performance-units/README.md` lists every frozen plan.

`tasks/archive/` holds historical records. It is exempt from budgets and
from the link check, because a record keeps the links it had when written.

Usage: scripts/check-context.py [--root DIR]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# Ceilings in physical lines. A document that needs more is answering more
# than one question; split it or move history to tasks/archive/.
BUDGETS = {
    "AGENTS.md": 270,
    "README.md": 140,
    "tasks/README.md": 120,
    "tasks/TEMPLATE.md": 60,
    "tasks/performance-units/README.md": 180,
    "scripts/README.md": 120,
    "docs/architecture.md": 200,
    "docs/harness.md": 260,
    "docs/conformance/README.md": 80,
    "docs/performance-workflow.md": 260,
    "docs/performance-knowledge.md": 130,
    "docs/benchmarking.md": 480,
}
DESIGN_BUDGET = 160
ACTIVE_TASK_BUDGET = 220

RESUME_FIELDS = (
    "Status",
    "Implementation",
    "Verified at",
    "Evidence",
    "Unresolved",
    "Next action",
)
STATUSES = (
    "active",
    "blocked",
    "closed-landed",
    "closed-rejected",
    "closed-superseded",
    "historical-campaign",
)
RESUME_WINDOW = 30

# A reference definition: [label]: target "title". Backticks in a label are
# literal, and `[^note]:` is a footnote, not a link.
REFERENCE = re.compile(r"^ {0,3}\[(?!\^)(?:[^\]\\\n]|\\.)+\]:(.*)$")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
FENCE = re.compile(r"^\s*(```|~~~)")
TASK_NAME = re.compile(r"^T\d{3}-.+\.md$")


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def slug(heading: str) -> str:
    """GitHub's anchor for a heading: lowercase, punctuation dropped."""
    text = re.sub(r"`([^`]*)`", r"\1", heading)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def prose_lines(path: Path) -> list[str]:
    """The file's lines with fenced code blocks blanked out."""
    lines = []
    fenced = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            fenced = not fenced
            lines.append("")
            continue
        lines.append("" if fenced else line)
    return lines


def unescape(text: str) -> str:
    return re.sub(r"\\(.)", r"\1", text)


def inline_destination(line: str, start: int) -> tuple[str | None, int]:
    """The destination of a link whose `(` ends just before `start`.

    Follows CommonMark: `<...>` may hold spaces; a bare destination ends at
    whitespace or at the parenthesis that closes the link, and parentheses
    inside it nest. Returns the destination, or None when this is not a link,
    and the index to continue scanning from.
    """
    pos = start
    while pos < len(line) and line[pos] in " \t\n":
        pos += 1
    if pos < len(line) and line[pos] == "<":
        end = line.find(">", pos + 1)
        if end < 0 or "\n" in line[pos:end]:
            return None, start
        destination, pos = line[pos + 1 : end], end + 1
    else:
        begin, depth = pos, 0
        while pos < len(line) and not line[pos].isspace():
            if line[pos] == "\\" and pos + 1 < len(line):
                pos += 2
                continue
            if line[pos] == "(":
                depth += 1
            elif line[pos] == ")":
                if depth == 0:
                    break
                depth -= 1
            pos += 1
        if depth:
            return None, start
        destination = unescape(line[begin:pos])
    # Anything between the destination and the closing parenthesis is a title.
    close = line.find(")", pos)
    if close < 0:
        return None, start
    return destination, close + 1


def inline_targets(line: str) -> list[tuple[int, str]]:
    """Inline link and image destinations in one paragraph, with offsets."""
    found = []
    pos = 0
    while pos < len(line):
        char = line[pos]
        if char == "\\":
            pos += 2
        elif char == "`":
            # A code span runs to the next backtick run of the same length.
            # Without one the backticks are literal text.
            end = pos
            while end < len(line) and line[end] == "`":
                end += 1
            fence = line[pos:end]
            close = re.search(rf"(?<!`){fence}(?!`)", line[end:])
            pos = end + close.end() if close else end
        elif line.startswith("](", pos):
            destination, pos_after = inline_destination(line, pos + 2)
            if destination is None:
                pos += 1
            else:
                found.append((pos, destination))
                pos = pos_after
        else:
            pos += 1
    return found


def definition_target(rest: str) -> str | None:
    """The destination that follows a reference label, or None for none."""
    rest = rest.strip()
    if not rest:
        return None
    if rest.startswith("<"):
        end = rest.find(">")
        return rest[1:end] if end > 0 else None
    return unescape(rest.split()[0])


def link_targets(lines: list[str]) -> list[tuple[int, str]]:
    """Every link destination in `lines`, with its 1-based line number."""
    found = []
    # Inline links are scanned a paragraph at a time, because a code span or
    # a link's text may wrap across lines.
    start = 0
    for index in range(len(lines) + 1):
        if index < len(lines) and lines[index].strip():
            continue
        paragraph = "\n".join(lines[start:index])
        for offset, target in inline_targets(paragraph):
            found.append((start + 1 + paragraph.count("\n", 0, offset), target))
        start = index + 1
    for index, line in enumerate(lines):
        definition = REFERENCE.match(line)
        if not definition:
            continue
        rest = definition.group(1)
        if not rest.strip() and index + 1 < len(lines):
            # The destination may sit on the line after the label.
            rest = lines[index + 1]
        target = definition_target(rest)
        if target is not None:
            found.append((index + 1, target))
    return sorted(found)


def anchors(path: Path) -> set[str]:
    found: set[str] = set()
    counts: dict[str, int] = {}
    for line in prose_lines(path):
        match = HEADING.match(line)
        if not match:
            continue
        base = slug(match.group(2))
        seen = counts.get(base, 0)
        counts[base] = seen + 1
        found.add(base if seen == 0 else f"{base}-{seen}")
    return found


def current_documents(root: Path) -> list[Path]:
    documents = [root / "AGENTS.md", root / "README.md", root / "scripts/README.md"]
    documents += sorted((root / "docs").rglob("*.md"))
    documents += sorted(
        path
        for path in (root / "tasks").rglob("*.md")
        if "archive" not in path.relative_to(root).parts
    )
    return [path for path in documents if path.is_file()]


def check_symlink(root: Path) -> list[str]:
    claude = root / "CLAUDE.md"
    if not claude.is_symlink() or claude.resolve() != (root / "AGENTS.md").resolve():
        return ["CLAUDE.md must be a symlink to AGENTS.md"]
    return []


def check_budgets(root: Path) -> list[str]:
    errors = []
    limits = {root / name: limit for name, limit in BUDGETS.items()}
    for path in sorted((root / "docs/design").glob("*.md")):
        limits[path] = DESIGN_BUDGET
    for path in sorted((root / "tasks").glob("T*.md")):
        if TASK_NAME.match(path.name):
            limits[path] = ACTIVE_TASK_BUDGET
    for path, limit in limits.items():
        if not path.is_file():
            continue
        lines = line_count(path)
        if lines > limit:
            errors.append(
                f"{path.relative_to(root)}: {lines} lines, budget {limit}; "
                "split the document or move history to tasks/archive/"
            )
    return errors


def check_links(root: Path) -> list[str]:
    errors = []
    anchor_cache: dict[Path, set[str]] = {}
    for document in current_documents(root):
        for number, target in link_targets(prose_lines(document)):
            if not target or re.match(r"^[a-z][a-z0-9+.\-]*:", target):
                continue
            file_part, _, fragment = target.partition("#")
            destination = (
                document if not file_part else (document.parent / file_part)
            ).resolve()
            where = f"{document.relative_to(root)}:{number}"
            if not destination.exists():
                errors.append(f"{where}: link target does not exist: {target}")
                continue
            if not fragment or destination.suffix != ".md":
                continue
            if destination not in anchor_cache:
                anchor_cache[destination] = anchors(destination)
            if fragment.lower() not in anchor_cache[destination]:
                errors.append(f"{where}: no heading for anchor: {target}")
    return errors


def check_task_resume_blocks(root: Path) -> list[str]:
    errors = []
    for path in sorted((root / "tasks").glob("T*.md")):
        if not TASK_NAME.match(path.name):
            continue
        head = path.read_text(encoding="utf-8").splitlines()[:RESUME_WINDOW]
        values: dict[str, str] = {}
        current = None
        for line in head:
            match = re.match(r"^[\s\-*|]*\**([A-Za-z ]+?)\**\s*[:|]\s*(.*)$", line)
            if match and match.group(1).strip() in RESUME_FIELDS:
                current = match.group(1).strip()
                if current in values:
                    current = None
                else:
                    values[current] = match.group(2)
            elif current and line.startswith((" ", "\t")) and line.strip():
                # An indented continuation line belongs to the field above.
                values[current] += " " + line.strip()
            else:
                current = None
        name = path.relative_to(root)
        missing = [field for field in RESUME_FIELDS if field not in values]
        if missing:
            errors.append(
                f"{name}: resume block is missing {', '.join(missing)} "
                f"in its first {RESUME_WINDOW} lines (see tasks/TEMPLATE.md)"
            )
            continue
        empty = [f for f in RESUME_FIELDS if not values[f].strip("`* |.")]
        if empty:
            errors.append(
                f"{name}: resume block leaves {', '.join(empty)} empty; "
                'write the value or "not recorded"'
            )
            continue
        status = values["Status"].strip("`* |").split()[0:1]
        if not status or status[0].rstrip(".,;") not in STATUSES:
            errors.append(
                f"{name}: Status must start with one of {', '.join(STATUSES)}"
            )
    return errors


def names(text: str, name: str) -> bool:
    """Whether `text` contains `name` as a whole file or plan name."""
    pattern = rf"(?<![\w.\-]){re.escape(name)}(?![\w\-]|\.\w)"
    return re.search(pattern, text) is not None


def check_indexes(root: Path) -> list[str]:
    errors = []
    index = root / "tasks/README.md"
    if index.is_file():
        text = index.read_text(encoding="utf-8")
        tasks = sorted((root / "tasks").glob("T*.md"))
        tasks += sorted((root / "tasks/archive").glob("T*.md"))
        for path in tasks:
            if TASK_NAME.match(path.name) and not names(text, path.name):
                errors.append(f"tasks/README.md does not list {path.relative_to(root)}")
    units = root / "tasks/performance-units"
    unit_index = units / "README.md"
    if unit_index.is_file():
        text = unit_index.read_text(encoding="utf-8")
        for path in sorted(units.glob("*.json")):
            if not (names(text, path.stem) or names(text, path.name)):
                errors.append(
                    f"tasks/performance-units/README.md does not list {path.name}"
                )
    return errors


CHECKS = (
    check_symlink,
    check_budgets,
    check_links,
    check_task_resume_blocks,
    check_indexes,
)


def run(root: Path) -> list[str]:
    errors: list[str] = []
    for check in CHECKS:
        errors.extend(check(root))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    errors = run(args.root.resolve())
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print("check-context: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())

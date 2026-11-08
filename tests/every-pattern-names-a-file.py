"""Check that every path pattern a tool reads still names a file.

    every-pattern-names-a-file MUST_RUN RUFF PATHS

MUST_RUN is tools/merge-net/must-run.txt, whose lines each name an area, the
path patterns that are it, and after `=>` the VM tests it runs. RUFF is
ruff.toml, whose per-file ignores are keyed by path patterns. PATHS lists every
file the repository holds, a path a line, relative to its root.

Neither tool says when a pattern matches nothing. After a file moves, the net
would quietly stop running a VM test before a merge, and ruff would quietly
read a file at its strictest or not at all, so a pattern that names no file
fails here, saying which and where. Every failure is printed before the check
exits 1.
"""

import sys
import tomllib
from fnmatch import fnmatchcase
from pathlib import Path, PurePosixPath


def must_run_patterns(must_run: Path) -> list[str]:
    """Give the path patterns of each of must-run.txt's areas."""
    patterns = []
    for line in must_run.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        area, _, rest = line.partition(":")
        paths, arrow, _ = rest.partition("=>")
        if not arrow:
            sys.exit(f"{must_run}: the {area} line has no '=>' before its VM tests.")
        patterns.extend(paths.split())
    return patterns


def ruff_patterns(ruff: Path) -> list[str]:
    """Give the path patterns ruff.toml's per-file ignores are keyed by."""
    settings = tomllib.loads(ruff.read_text())
    return list(settings.get("lint", {}).get("per-file-ignores", {}))


def names_one(pattern: str, paths: list[str], *, by_name: bool) -> bool:
    """Whether a pattern matches any path, or any file's name where a tool reads it so."""
    return any(
        fnmatchcase(path, pattern)
        or (by_name and fnmatchcase(PurePosixPath(path).name, pattern))
        for path in paths
    )


def faults(must_run: list[str], ruff: list[str], paths: list[str]) -> list[str]:
    """Say each pattern that names no file, a sentence each."""
    said = [
        f"tools/merge-net/must-run.txt names {pattern}, which matches no file: "
        "point it at where the area's files are now, or the net stops running its VM tests."
        for pattern in must_run
        if not names_one(pattern, paths, by_name=False)
    ]
    # ruff reads a pattern with no slash against each file's name alone.
    said.extend(
        f"ruff.toml ignores rules for {pattern}, which matches no file: "
        "point it at where the file is now, or take it off."
        for pattern in ruff
        if not names_one(pattern, paths, by_name="/" not in pattern)
    )
    return said


def main() -> None:
    """Read each tool's patterns and the repository's files, and say each fault."""
    must_run, ruff, listing = (Path(argument) for argument in sys.argv[1:4])
    paths = listing.read_text().splitlines()
    said = faults(must_run_patterns(must_run), ruff_patterns(ruff), paths)
    for fault in said:
        print(fault, file=sys.stderr)
    if said:
        sys.exit(1)


if __name__ == "__main__":
    main()

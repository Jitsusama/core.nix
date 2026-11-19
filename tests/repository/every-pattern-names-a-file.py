"""Check that every path pattern a tool reads still names a file.

    every-pattern-names-a-file MUST_RUN RUFF UNREAD PATHS CHECKS LEFT_OUT

MUST_RUN is tools/merge-net/must-run.txt, whose lines each name an area, the
path patterns that are it, and after `=>` the VM tests it runs. RUFF is
ruff.toml, whose per-file ignores are keyed by path patterns. UNREAD lists the
patterns treefmt.nix tells the linters to leave unread, a pattern a line.
PATHS lists every file the repository holds, a path a line, relative to its
root. CHECKS lists the flake's checks, a name a line, which a must-run.txt
pattern opening `check:` names rather than a file. LEFT_OUT lists what
tests/repository's scripts and checks tell ripgrep to leave out, a line each
holding the file, the flag and the pattern after its `!`, a tab apart, or the
file, no flag and a line holding a glob it couldn't read. Each pattern is
read against the repository's root, which each script searches from, so one
searching from somewhere else goes unseen.

None of the tools says when a pattern matches nothing. After a file moves, the
net would quietly stop running a VM test before a merge, ruff would quietly
read a file at its strictest or not at all, the next file to take a name
treefmt leaves unread would go unlinted, and a script would read the file it
meant to leave out, so a pattern that names no file fails here, saying which
and where. Every failure is printed before the check exits 1.
"""

import glob
import re
import sys
import tomllib
from fnmatch import fnmatchcase
from pathlib import Path, PurePosixPath

# An area's path opening so names a check that moved, not a file.
CHECK = "check:"
# A pattern holding one names no one file, since the shell, or Nix in a .nix
# file, fills it in, so it's passed over and goes unchecked.
SHELL_VARIABLE = "$"
# ripgrep's flag that reads a glob with case folded. A --glob after
# --glob-case-insensitive folds too, and is read here as written, so it can
# only fail where ripgrep would have found the file.
FOLDED = "--iglob"


def must_run_patterns(must_run: Path) -> list[str]:
    """Give the path patterns of each of must-run.txt's areas, a check's with its `check:`."""
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
        fnmatchcase(path, pattern) or (by_name and fnmatchcase(PurePosixPath(path).name, pattern))
        for path in paths
    )


def left_out_patterns(left_out: Path) -> list[tuple[str, str, str]]:
    """Give each pattern a file tells ripgrep to leave out, with the file and the flag."""
    lines = [line.split("\t", 2) for line in left_out.read_text().splitlines() if line]
    return [(source, flag, pattern) for source, flag, pattern in lines if pattern]


def ripgrep_names(pattern: str, files: set[str], folders: set[str], *, folded: bool) -> bool:
    """Whether a ripgrep glob matches a file or a folder, read as gitignore reads one."""
    # One ending in a slash matches only a folder. One with no other slash
    # matches a name at any depth, and one with a slash matches the path from
    # where the search starts, its * stopping at each slash and its ** taking
    # any number of folders, none included.
    places = folders if pattern.endswith("/") else files | folders
    pattern = pattern.rstrip("/")
    pattern = pattern.removeprefix("/") if "/" in pattern else f"**/{pattern}"
    translated = glob.translate(
        pattern.replace("[^", "[!"), recursive=True, include_hidden=True, seps="/"
    )
    matches = re.compile(translated, re.IGNORECASE if folded else 0).match
    return any(matches(place) for place in places)


def faults(
    must_run: list[str], ruff: list[str], unread: list[str], paths: list[str], checks: list[str]
) -> list[str]:
    """Say each pattern that names no file, or no check, a sentence each."""
    said = [
        f"tools/merge-net/must-run.txt names {pattern}, which matches no file: "
        "point it at where the area's files are now, or the net stops running its VM tests."
        for pattern in must_run
        if not pattern.startswith(CHECK) and not names_one(pattern, paths, by_name=False)
    ]
    said.extend(
        f"tools/merge-net/must-run.txt names {pattern}, which matches no check: "
        "point it at the check's name now, or the net stops running its VM tests."
        for pattern in must_run
        if pattern.startswith(CHECK)
        and not names_one(pattern.removeprefix(CHECK), checks, by_name=False)
    )
    # ruff reads a pattern with no slash against each file's name alone.
    said.extend(
        f"ruff.toml ignores rules for {pattern}, which matches no file: "
        "point it at where the file is now, or take it off."
        for pattern in ruff
        if not names_one(pattern, paths, by_name="/" not in pattern)
    )
    # treefmt reads every pattern against the whole path, its * reaching
    # across folders, as fnmatch's does.
    said.extend(
        f"treefmt.nix leaves {pattern} unread, which matches no file: "
        "take it off, or the next file to take its name goes unlinted."
        for pattern in unread
        if not names_one(pattern, paths, by_name=False)
    )
    return said


def left_out_faults(left_out: list[tuple[str, str, str]], paths: list[str]) -> list[str]:
    """Say each pattern a file leaves out that names no file, or can't be read, a sentence each."""
    files = set(paths)
    folders = {str(folder) for path in paths for folder in PurePosixPath(path).parents}
    folders.discard(".")
    said = []
    for source, flag, pattern in left_out:
        if flag and SHELL_VARIABLE in pattern:
            continue
        if not flag:
            said.append(
                f"{source} holds a ripgrep glob this check can't read, in {pattern}: "
                "write each pattern given after the flag out in full."
            )
        elif "{" in pattern:
            said.append(
                f"{source} tells ripgrep to leave out {pattern}, whose braces this check can't "
                "read: give each choice a glob of its own."
            )
        elif not ripgrep_names(pattern, files, folders, folded=flag == FOLDED):
            said.append(
                f"{source} tells ripgrep to leave out {pattern}, which matches no file: "
                "point it at where the file is now, or take it off."
            )
    return said


def main() -> None:
    """Read each tool's patterns and the repository's files, and say each fault."""
    must_run, ruff, unread, listing, named, left_out = (
        Path(argument) for argument in sys.argv[1:7]
    )
    paths = listing.read_text().splitlines()
    checks = named.read_text().splitlines()
    patterns = [line for line in unread.read_text().splitlines() if line]
    said = faults(must_run_patterns(must_run), ruff_patterns(ruff), patterns, paths, checks)
    said.extend(left_out_faults(left_out_patterns(left_out), paths))
    for fault in said:
        print(fault, file=sys.stderr)
    if said:
        sys.exit(1)


if __name__ == "__main__":
    main()

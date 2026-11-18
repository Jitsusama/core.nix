"""Check that every folder's scripts find what they import, handed to mypy alone.

    every-script-finds-what-it-imports TREE [--include GLOB ...]
        [--exclude GLOB ...] -- MYPY [ARGUMENT ...]

`nix fmt` hands mypy only the scripts that changed, and mypy finds a module
beside a script it's given, in the folder it runs from, or on its path, and
nowhere else. So a script importing from another folder passes while that
folder's scripts are handed over with it, as when every script is, and fails
the first time it changes alone. Core's own `nix fmt` broke that way for
every session at once, when the shell's cases moved away from the modules
they import. This runs MYPY, with each ARGUMENT, as the repository's treefmt
runs it, from TREE's root, once for each folder holding a file it hands
mypy: one matching an --include GLOB and no --exclude GLOB, matched against
its path from TREE's root as treefmt matches it. A GLOB is read as fnmatch
reads it, where `*` crosses folders as in treefmt's; a brace isn't read as
a choice, and no pattern of core's has one. Each run is handed that
folder's files and no others, and the rule fails naming each folder whose
files mypy failed, with how it ended and what it said, and fails a tree
with none. mypy's exit says too little to tell a missing import from a
syntax error or options it can't read, so the rule names no cause; it
leaves that to what mypy said. Every run shares one cache, so what one
folder's run reads once the next needn't read again; a module is found
before its cache is read, so the cache finds none a run couldn't.
"""

import argparse
import fnmatch
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath


def folders(tree: Path, include: list[str], exclude: list[str]) -> dict[str, list[str]]:
    """Each folder of the tree holding files mypy is handed, with those files."""
    held: dict[str, list[str]] = {}
    for path in sorted(tree.rglob("*")):
        name = path.relative_to(tree).as_posix()
        if (
            path.is_file()
            and any(fnmatch.fnmatchcase(name, glob) for glob in include)
            and not any(fnmatch.fnmatchcase(name, glob) for glob in exclude)
        ):
            held.setdefault(PurePosixPath(name).parent.as_posix(), []).append(name)
    return held


def named(held: list[str]) -> str:
    """Name the folders given as a reader finds them, the tree's root by that name."""
    return ", ".join("the tree's root" if folder == "." else folder for folder in held)


def main() -> int:
    """Hand mypy each folder's files alone, and say which it failed."""
    parser = argparse.ArgumentParser()
    parser.add_argument("tree", type=Path)
    parser.add_argument("--include", action="append", default=[])
    parser.add_argument("--exclude", action="append", default=[])
    parser.add_argument("mypy", nargs="+")
    given = parser.parse_args()
    held = folders(given.tree, given.include, given.exclude)
    if not held:
        print(f"{given.tree} holds no file mypy is handed, so nothing was judged.", file=sys.stderr)
        return 1
    failed: list[str] = []
    with tempfile.TemporaryDirectory() as cache:
        for folder, scripts in held.items():
            ran = subprocess.run(
                [*given.mypy, "--cache-dir", cache, "--", *scripts],
                cwd=given.tree,
                capture_output=True,
                text=True,
                check=False,
            )
            if ran.returncode == 0:
                continue
            failed.append(folder)
            ended = (
                f"was killed by signal {-ran.returncode}"
                if ran.returncode < 0
                else f"exited {ran.returncode}"
            )
            print(f"{named([folder])}: mypy, handed only its scripts, {ended}:", file=sys.stderr)
            print((ran.stdout + ran.stderr).strip() or "(it said nothing)", file=sys.stderr)
    if failed:
        print(
            "mypy, handed a folder's scripts alone as nix fmt hands a script that changed, "
            f"failed those of {named(failed)}. Where it can't find a module a script imports "
            "from another folder, name that on its path (jitsusama.mypy.paths).",
            file=sys.stderr,
        )
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

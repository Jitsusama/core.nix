"""Check that every per-file exception in core's ruff config still hides a finding.

    every-ruff-exception-hides-a-finding TREE CONFIG

A per-file exception switches a rule off where the code once needed it to be,
and nothing says so when the code moves on: five of core's stayed in ruff.toml
for days after the last of their findings was gone, counted as lints switched
off and ready to wave through the next one. This reads CONFIG, a ruff.toml in
TREE, and runs ruff over TREE with the config as it is, then once with each
code, a rule or a prefix of rules, taken off each of its per-file-ignores
entries. One whose removal raises no finding the config as it is didn't hides
none, and fails, named with its pattern and code. The findings themselves are
compared, not their count, since taking an exception off can also end one: a
noqa the exception left with nothing to match is unused (RUF100) until it goes,
so a count could fall by that as it rose by a finding. Ruff decides which
files each pattern covers, what each code selects and which rules the config
runs at all, as treefmt's ruff does, so nothing here copies its rules. Two
exceptions that hide the same findings each raise nothing when taken off
alone, so each that fails stays off while the rest are judged, and one whose
removal then raises findings stays: taking off all that fail leaves every
finding hidden that was.

Each run replaces lint.per-file-ignores, so a config keeping exceptions where
this doesn't read them is refused before ruff runs, each place said, rather
than passing on what it skipped, as is one code named twice for one pattern,
since both copies go together and the one that hides nothing would pass. Ruff
failing to check fails too, and no run fixes anything, so each reads the same
tree.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

# A finding, as where it is and what it is: its file, its code, and its row
# and column.
type Finding = tuple[str, str, int, int]

# Where ruff takes per-file exceptions from that this doesn't read, and where
# to move each instead.
UNREAD = {
    ("extend",): "fold the config it names in",
    ("per-file-ignores",): "move it to lint.per-file-ignores",
    ("extend-per-file-ignores",): "move it to lint.per-file-ignores",
    ("lint", "extend-per-file-ignores"): "move it to lint.per-file-ignores",
}


class RuffFailedError(Exception):
    """Ruff couldn't read the tree or the config, saying why."""

    def __init__(self, ran: subprocess.CompletedProcess[str]) -> None:
        """Say how ruff exited, and what it said, if anything."""
        super().__init__(f"exit {ran.returncode}: {ran.stderr.strip()}")


def setting(read: dict[str, Any], *path: str) -> object:
    """Give what the config sets at this path, or None where nothing is set."""
    found: Any = read
    for key in path:
        found = found.get(key) if isinstance(found, dict) else None
    return found


def exceptions(read: dict[str, Any]) -> dict[str, list[str]]:
    """Give the config's lint.per-file-ignores, the exceptions this judges."""
    found = setting(read, "lint", "per-file-ignores")
    return found if isinstance(found, dict) else {}


def refused(read: dict[str, Any]) -> list[str]:
    """Say each place the config keeps exceptions that this can't judge."""
    said = [
        f"{'.'.join(path)} isn't read: {instead}."
        for path, instead in UNREAD.items()
        if setting(read, *path)
    ]
    for pattern, codes in exceptions(read).items():
        said.extend(
            f"lint.per-file-ignores names {code} twice for {pattern}: keep one."
            for code in sorted({code for code in codes if codes.count(code) > 1})
        )
    return said


def findings(tree: Path, config: Path, ignores: dict[str, list[str]]) -> set[Finding]:
    """Give what the ruff on the PATH finds over the tree, with these exceptions."""
    # JSON's quoting of a string is TOML's for every character but a few
    # control characters, which no pattern or code holds; one that did would
    # make ruff refuse the override, failing the check rather than passing it.
    table = ", ".join(
        f"{json.dumps(pattern, ensure_ascii=False)} = {json.dumps(codes)}"
        for pattern, codes in ignores.items()
    )
    ran = subprocess.run(
        [
            "ruff",
            "check",
            "--no-cache",
            "--exit-zero",
            "--no-fix",
            "--no-unsafe-fixes",
            "--output-format",
            "json",
            "--config",
            str(config.resolve()),
            "--config",
            f"lint.per-file-ignores = {{{table}}}",
            ".",
        ],
        cwd=tree,
        capture_output=True,
        text=True,
        check=False,
    )
    if ran.returncode != 0:
        raise RuffFailedError(ran)
    return {
        (found["filename"], found["code"], found["location"]["row"], found["location"]["column"])
        for found in json.loads(ran.stdout)
    }


def without(ignores: dict[str, list[str]], taken: set[tuple[str, str]]) -> dict[str, list[str]]:
    """Give the exceptions with each of these codes taken off its pattern."""
    return {
        pattern: [code for code in codes if (pattern, code) not in taken]
        for pattern, codes in ignores.items()
    }


def stale(tree: Path, config: Path, ignores: dict[str, list[str]]) -> list[tuple[str, str]]:
    """Give each exception that can go, as no finding it hides is left unhidden without it."""
    held = findings(tree, config, ignores)
    # Each that fails stays off while the rest are judged, so of two hiding
    # the same findings, a prefix and its rule among them, one stays.
    taken: set[tuple[str, str]] = set()
    for entry in ((pattern, code) for pattern, codes in ignores.items() for code in codes):
        if findings(tree, config, without(ignores, taken | {entry})) <= held:
            taken.add(entry)
    return [entry for entry in ((p, c) for p, cs in ignores.items() for c in cs) if entry in taken]


def main() -> None:
    """Say each per-file exception whose code raises nothing more once it's gone."""
    try:
        tree, config = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-ruff-exception-hides-a-finding TREE CONFIG")
    try:
        read = tomllib.loads(config.read_text())
    except tomllib.TOMLDecodeError as unread:
        sys.exit(f"{config.name} isn't TOML: {unread}")
    if said := refused(read):
        for reason in said:
            print(f"{config.name}: {reason}", file=sys.stderr)
        sys.exit(f"{config.name} is refused, so none of its exceptions is judged.")
    ignores = exceptions(read)
    try:
        idle = stale(tree, config, ignores)
    except RuffFailedError as failed:
        sys.exit(f"ruff couldn't check {tree} with {config.name}, {failed}")
    for pattern, code in idle:
        print(
            f"{config.name}: {code} in {pattern} hides no finding another exception doesn't:"
            " take it off.",
            file=sys.stderr,
        )
    if idle:
        print(
            f"Repositories on core lint with {config.name} too, unread here: check theirs"
            " before taking one off.",
            file=sys.stderr,
        )
    total = sum(len(codes) for codes in ignores.values())
    print(f"{total - len(idle)} of {total} per-file exceptions hide a finding.")
    sys.exit(1 if idle else 0)


if __name__ == "__main__":
    main()

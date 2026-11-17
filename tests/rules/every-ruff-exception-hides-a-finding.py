"""Check that every per-file exception in core's ruff config still hides a finding.

    every-ruff-exception-hides-a-finding TREE CONFIG

A per-file exception switches a rule off where the code once needed it to be,
and nothing says so when the code moves on: five of core's stayed in ruff.toml
for days after the last of their findings was gone, counted as lints switched
off and ready to wave through the next one. This reads CONFIG, a ruff.toml in
TREE, and runs ruff over TREE with the config as it is, then once with each
code, a rule or a prefix of rules, taken off each of its per-file-ignores
entries. Taking an exception off can only raise findings, so one that raises no
more hides none, and fails, named with its pattern and code. Ruff decides which
files each pattern covers, what each code selects and which rules the config
runs at all, as treefmt's ruff does, so nothing here copies its rules. Two
exceptions that hide the same findings each raise nothing when taken off
alone, so each that fails stays off while the rest are judged, and one whose
removal then raises findings stays: taking off all that fail leaves every
finding hidden that was.

Each run replaces lint.per-file-ignores, so a config keeping exceptions where
this doesn't read them is refused before ruff runs, each place said, rather
than passing on what it skipped; ruff failing to check fails too.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

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


def refused(read: dict[str, Any]) -> list[str]:
    """Say each place the config keeps per-file exceptions that this doesn't read."""
    said = []
    for path, instead in UNREAD.items():
        setting: Any = read
        for key in path:
            setting = setting.get(key) if isinstance(setting, dict) else None
        if setting:
            said.append(f"{'.'.join(path)} isn't read: {instead}.")
    return said


def findings(tree: Path, config: Path, ignores: dict[str, list[str]]) -> int:
    """Count what the ruff on the PATH finds over the tree, with these exceptions."""
    table = ", ".join(
        f"{json.dumps(pattern)} = {json.dumps(codes)}" for pattern, codes in ignores.items()
    )
    ran = subprocess.run(
        [
            "ruff",
            "check",
            "--no-cache",
            "--exit-zero",
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
    return len(json.loads(ran.stdout))


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
        if findings(tree, config, without(ignores, taken | {entry})) == held:
            taken.add(entry)
    return [entry for entry in ((p, c) for p, cs in ignores.items() for c in cs) if entry in taken]


def main() -> None:
    """Say each per-file exception whose code raises nothing more once it's gone."""
    try:
        tree, config = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-ruff-exception-hides-a-finding TREE CONFIG")
    read = tomllib.loads(config.read_text())
    if said := refused(read):
        for reason in said:
            print(f"{config.name}: {reason}", file=sys.stderr)
        sys.exit(f"{config.name} is refused, so none of its exceptions is judged.")
    ignores: dict[str, list[str]] = read.get("lint", {}).get("per-file-ignores", {})
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
    total = sum(len(codes) for codes in ignores.values())
    print(f"{total - len(idle)} of {total} per-file exceptions hide a finding.")
    sys.exit(1 if idle else 0)


if __name__ == "__main__":
    main()

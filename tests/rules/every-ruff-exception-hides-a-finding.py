"""Check that every per-file exception in core's ruff config still hides a finding.

    every-ruff-exception-hides-a-finding TREE CONFIG

A per-file exception switches a rule off where the code once needed it to be,
and nothing says so when the code moves on: five of core's stayed in ruff.toml
for days after the last of their findings was gone, counted as lints switched
off and ready to wave through the next one. This reads CONFIG, a ruff.toml in
TREE, and for each code each of its per-file-ignores entries names, a rule or a
prefix of rules, runs ruff over TREE with that code alone, once with the config
as it is and once with that code taken off that one entry. Ruff decides which
files each pattern covers and what a code selects, as treefmt's ruff does. An
exception whose removal raises no more findings hides none, and fails, named
with its pattern and code. Two that hide the same findings each raise nothing
when taken off alone, so the failing ones are taken off in turn, each on top of
those before, and one whose removal then raises findings stays: taking off all
that fail leaves every finding hidden that was. A config keeping exceptions in
a table this doesn't read, or extending another config, fails too, rather than
passing on what it skipped, and so does ruff failing to check. Every failure is
printed before the check exits 1. The config's own ignore list isn't judged,
since its rules are off everywhere, most because they contradict another rule
or the formatter; it stays ignored in every run, as --select alone would
switch them back on, so an exception for one of them hides nothing.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

# The tables ruff takes per-file exceptions from that this doesn't read.
UNREAD = (
    ("extend-per-file-ignores",),
    ("per-file-ignores",),
    ("lint", "extend-per-file-ignores"),
)


class RuffFailedError(Exception):
    """Ruff couldn't read the tree or the config, saying why."""


class Rules:
    """A config's per-file exceptions, and the rules it ignores everywhere."""

    def __init__(self, read: dict[str, Any]) -> None:
        """Take them from a config as read."""
        lint = read.get("lint", {})
        self.ignores: dict[str, list[str]] = lint.get("per-file-ignores", {})
        self.ignored: list[str] = [*read.get("ignore", []), *lint.get("ignore", [])]


def findings(
    tree: Path, config: Path, code: str, rules: Rules, ignores: dict[str, list[str]]
) -> int:
    """Count what the ruff on the PATH finds of one code over the tree, with these exceptions."""
    table = ", ".join(
        f"{json.dumps(pattern)} = {json.dumps(codes)}" for pattern, codes in ignores.items()
    )
    # --select replaces the config's selection, its ignore list with it, so
    # that list is given again beside it.
    ignored = ["--ignore", ",".join(rules.ignored)] if rules.ignored else []
    ran = subprocess.run(
        [
            "ruff",
            "check",
            "--no-cache",
            "--exit-zero",
            "--output-format",
            "json",
            "--config",
            str(config),
            "--config",
            f"lint.per-file-ignores = {{{table}}}",
            "--select",
            code,
            *ignored,
            ".",
        ],
        cwd=tree,
        capture_output=True,
        text=True,
        check=False,
    )
    if ran.returncode != 0:
        raise RuffFailedError(ran.stderr.strip())
    # Every finding counts, since --select keeps only what the code selects,
    # a prefix's rules and a redirected code's new name among them.
    return len(json.loads(ran.stdout))


def without(ignores: dict[str, list[str]], taken: set[tuple[str, str]]) -> dict[str, list[str]]:
    """Give the exceptions with each of these codes taken off its pattern."""
    return {
        pattern: [code for code in codes if (pattern, code) not in taken]
        for pattern, codes in ignores.items()
    }


def unread(read: dict[str, Any]) -> list[str]:
    """Name each table of per-file exceptions the config holds that this doesn't read."""
    named = []
    for path in UNREAD:
        table: Any = read
        for key in path:
            table = table.get(key) if isinstance(table, dict) else None
        if table:
            named.append(".".join(path))
    return named


def stale(tree: Path, config: Path, rules: Rules) -> list[tuple[str, str]]:
    """Give each exception that can go, as no finding it hides is left unhidden without it."""
    ignores = rules.ignores
    # Taken once for every code, so a prefix and one of its rules on one
    # pattern can't both go.
    taken: set[tuple[str, str]] = set()
    for code in sorted({code for codes in ignores.values() for code in codes}):
        held = findings(tree, config, code, rules, without(ignores, taken))
        for entry in ((pattern, code) for pattern, codes in ignores.items() if code in codes):
            if findings(tree, config, code, rules, without(ignores, taken | {entry})) == held:
                taken.add(entry)
    return [entry for entry in ((p, c) for p, cs in ignores.items() for c in cs) if entry in taken]


def main() -> None:
    """Say each per-file exception whose code raises nothing more once it's gone."""
    try:
        tree, config = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-ruff-exception-hides-a-finding TREE CONFIG")
    read = tomllib.loads(config.read_text())
    rules = Rules(read)
    said = [f"{table} isn't read: move it to lint.per-file-ignores." for table in unread(read)]
    if "extend" in read:
        said.append("extend isn't read: fold the config it names in.")
    for failure in said:
        print(f"{config.name}: {failure}", file=sys.stderr)
    try:
        idle = stale(tree, config, rules)
    except RuffFailedError as failed:
        sys.exit(f"ruff couldn't check {tree} with {config.name}:\n{failed}")
    for pattern, code in idle:
        print(
            f"{config.name}: {code} in {pattern} hides no finding another exception doesn't:"
            " take it off.",
            file=sys.stderr,
        )
    total = sum(len(codes) for codes in rules.ignores.values())
    print(f"{total - len(idle)} of {total} per-file exceptions hide a finding.")
    sys.exit(1 if said or idle else 0)


if __name__ == "__main__":
    main()

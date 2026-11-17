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
that fail leaves every finding hidden that was.

Selecting one code replaces the config's selection, so its lint.ignore is given
again beside it, and that alone stands in for the config's selection only when
it selects ALL and ignores no prefix broader than an exception's code. A config
this can't judge so, or that keeps lint settings where this doesn't read them,
is refused, each reason said, before ruff runs; ruff failing to check fails too.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

# Where ruff takes lint settings from that this doesn't read, and where to move
# each instead.
UNREAD = {
    ("extend",): "fold the config it names in",
    ("ignore",): "move it to lint.ignore",
    ("select",): "move it to lint.select",
    ("extend-ignore",): "move it to lint.ignore",
    ("extend-select",): "move it to lint.select",
    ("per-file-ignores",): "move it to lint.per-file-ignores",
    ("extend-per-file-ignores",): "move it to lint.per-file-ignores",
    ("lint", "extend-ignore"): "move it to lint.ignore",
    ("lint", "extend-select"): "move it to lint.select",
    ("lint", "extend-per-file-ignores"): "move it to lint.per-file-ignores",
}


class RuffFailedError(Exception):
    """Ruff couldn't read the tree or the config, saying why."""


class Rules:
    """A config's per-file exceptions, and the rules it ignores everywhere."""

    def __init__(self, read: dict[str, Any]) -> None:
        """Take them from a config as read."""
        lint = read.get("lint", {})
        self.selected: list[str] = lint.get("select", [])
        self.ignores: dict[str, list[str]] = lint.get("per-file-ignores", {})
        self.ignored: list[str] = lint.get("ignore", [])


def refused(read: dict[str, Any], rules: Rules) -> list[str]:
    """Say each reason this can't judge the config's exceptions as ruff would."""
    said = []
    for path, instead in UNREAD.items():
        setting: Any = read
        for key in path:
            setting = setting.get(key) if isinstance(setting, dict) else None
        if setting:
            said.append(f"{'.'.join(path)} isn't read: {instead}.")
    if rules.selected != ["ALL"]:
        said.append("lint.select isn't ALL, the one selection this judges under.")
    codes = sorted({code for codes in rules.ignores.values() for code in codes})
    said += [
        f"lint.ignore's {ignored} is broader than exception code {code}: name its rules."
        for ignored in rules.ignored
        for code in codes
        if code != ignored and code.startswith(ignored)
    ]
    return said


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
    if said := refused(read, rules):
        for reason in said:
            print(f"{config.name}: {reason}", file=sys.stderr)
        sys.exit(f"{config.name} is refused, so none of its exceptions is judged.")
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
    sys.exit(1 if idle else 0)


if __name__ == "__main__":
    main()

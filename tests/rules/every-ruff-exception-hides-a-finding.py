"""Check that every per-file exception in a ruff config still hides a finding.

    every-ruff-exception-hides-a-finding TREE CONFIG

A per-file exception switches a rule off where the code once needed it to be,
and nothing says so when the code moves on: five of core's stayed in ruff.toml
for days after the last of their findings was gone, counted as lints switched
off and ready to wave through the next one. This reads CONFIG, a ruff.toml in TREE,
and for each rule each of its per-file-ignores entries names, runs ruff over
TREE with that rule alone, once with the config as it is and once with that one
rule taken off that one entry. Ruff decides which files each pattern covers, as
treefmt's ruff does. An exception whose removal raises no more findings hides
none, and fails, naming its pattern and rule. Every failure is printed before
the check exits 1. The config's own ignore list isn't read: its rules are off
everywhere, most because they contradict another rule or the formatter.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path


def findings(tree: Path, config: Path, rule: str, ignores: dict[str, list[str]]) -> int:
    """Count what the ruff on the PATH finds of one rule over the tree, with these exceptions."""
    table = ", ".join(
        f"{json.dumps(pattern)} = {json.dumps(rules)}" for pattern, rules in ignores.items()
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
            str(config),
            "--config",
            f"lint.per-file-ignores = {{{table}}}",
            "--select",
            rule,
            ".",
        ],
        cwd=tree,
        capture_output=True,
        text=True,
        check=True,
    )
    return sum(1 for found in json.loads(ran.stdout) if found["code"] == rule)


def without(ignores: dict[str, list[str]], pattern: str, rule: str) -> dict[str, list[str]]:
    """Give the exceptions with one rule taken off one pattern."""
    return {
        each: [other for other in rules if (each, other) != (pattern, rule)]
        for each, rules in ignores.items()
    }


def main() -> None:
    """Say each per-file exception whose rule raises nothing more once it's gone."""
    try:
        tree, config = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-ruff-exception-hides-a-finding TREE CONFIG")
    ignores: dict[str, list[str]] = (
        tomllib.loads(config.read_text()).get("lint", {}).get("per-file-ignores", {})
    )
    held = {
        rule: findings(tree, config, rule, ignores)
        for rule in sorted({rule for rules in ignores.values() for rule in rules})
    }
    said = [
        f"{config.name}: {rule} in {pattern} hides no finding: take it off."
        for pattern, rules in ignores.items()
        for rule in rules
        if findings(tree, config, rule, without(ignores, pattern, rule)) == held[rule]
    ]
    for failure in said:
        print(failure, file=sys.stderr)
    total = sum(len(rules) for rules in ignores.values())
    print(f"{total - len(said)} of {total} per-file exceptions hide a finding.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()

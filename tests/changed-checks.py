"""Name the checks a change touches, and build only those.

A check whose derivation is the same as at the base proves nothing new, since
its result is already what it was. This evaluates every check's derivation at
the base, the merge base with origin's main unless told otherwise, and in the
working tree, and names each check added, changed or removed, and each that
no longer evaluates. With --build it builds those added and changed.

    changed-checks [--base <ref>] [--system <system>] [--build] [--no-vm]

The working tree is read as Nix reads a git checkout, so a new file counts
only once git knows it (git add -N). A change that only some checks read,
such as a module's, touches only those; one every check reads, such as
flake.lock, touches them all.
"""

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

# What a run asked for is refused with, as sysexits.h numbers it.
USAGE = 64

# Each check's derivation path, or null for one that doesn't evaluate, so one
# broken check is named rather than stopping the whole evaluation.
DERIVATIONS = (
    "checks: builtins.mapAttrs (_: check: let tried = builtins.tryEval check.drvPath; "
    "in if tried.success then tried.value else null) checks"
)

type Derivations = Mapping[str, str | None]


@dataclass(frozen=True)
class Comparison:
    """How each check's derivation moved from the base to the working tree."""

    added: list[str]
    changed: list[str]
    removed: list[str]
    broken: list[str]

    def to_build(self, *, vm: bool) -> list[str]:
        """Give the checks worth building, with or without the VM tests."""
        return [name for name in self.added + self.changed if vm or not name.startswith("vm-")]


def compare(before: Derivations, after: Derivations) -> Comparison:
    """Compare each check's derivation at the base with the working tree's.

    A check that stops evaluating is broken, not changed; one that starts to
    evaluate again, or evaluates for the first time, has changed or been added.
    """
    added = sorted(name for name in after.keys() - before.keys() if after[name] is not None)
    removed = sorted(before.keys() - after.keys())
    broken = sorted(name for name in after if after[name] is None and before.get(name) is not None)
    changed = sorted(
        name
        for name in after.keys() & before.keys()
        if after[name] is not None and after[name] != before[name]
    )
    return Comparison(added=added, changed=changed, removed=removed, broken=broken)


def run(*command: str) -> str:
    """Run a program on the path, and give what it printed."""
    # git and nix are the person's own, with their settings, so they're
    # found on the path rather than pinned.
    return subprocess.run(  # noqa: S603
        command,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout.strip()


def derivations(flake: str, system: str) -> dict[str, str | None]:
    """Evaluate each of a flake's checks for a system to its derivation's path."""
    found: dict[str, str | None] = json.loads(
        run("nix", "eval", "--json", f"{flake}#checks.{system}", "--apply", DERIVATIONS)
    )
    return found


def arguments(argv: Sequence[str]) -> argparse.Namespace:
    """Read the command line."""
    parser = argparse.ArgumentParser(
        prog="changed-checks",
        description="Name the checks a change touches, and build only those.",
    )
    parser.add_argument(
        "--base",
        default="refs/remotes/origin/main",
        help="what the change is compared with, from its merge base (origin's main)",
    )
    parser.add_argument("--system", help="the checks' system (this machine's)")
    parser.add_argument("--build", action="store_true", help="build those added and changed")
    parser.add_argument("--no-vm", action="store_true", help="build no VM test")
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    """Name the checks the working tree's change touches, and build them if asked."""
    options = arguments(argv)
    root = run("git", "rev-parse", "--show-toplevel")
    try:
        base = run("git", "-C", root, "merge-base", "HEAD", options.base)
    except subprocess.CalledProcessError:
        print(f"changed-checks: {options.base} shares no history with HEAD", file=sys.stderr)
        return USAGE
    system = options.system or run(
        "nix", "eval", "--raw", "--impure", "--expr", "builtins.currentSystem"
    )
    flake = f"git+file://{root}"
    untracked = run("git", "-C", root, "ls-files", "--others", "--exclude-standard")
    if untracked:
        print("changed-checks: Nix doesn't read files git doesn't know:", file=sys.stderr)
        print(untracked, file=sys.stderr)
    print(f"comparing {system}'s checks at {base[:12]} with the working tree", file=sys.stderr)
    comparison = compare(derivations(f"{flake}?rev={base}", system), derivations(flake, system))
    for kind in ("added", "changed", "removed", "broken"):
        for name in getattr(comparison, kind):
            print(f"{kind:8} {name}")
    if comparison.broken:
        return 1
    targets = comparison.to_build(vm=not options.no_vm)
    if not options.build or not targets:
        return 0
    command = ["nix", "build", "--no-link", "--keep-going", "-L"]
    command += [f"{flake}#checks.{system}.{name}" for name in targets]
    # A failing build is reported by its status, not raised.
    return subprocess.run(command, check=False).returncode  # noqa: S603


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

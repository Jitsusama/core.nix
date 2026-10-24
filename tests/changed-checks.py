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

The checks are evaluated a few dozen at a time, in a nix eval each, rather
than all in one, which held every check at once and reached 24 GB on a
machine many sessions share; each evaluation's memory goes when it ends. A
commit's derivations never change, so the base's are kept under
~/.cache/changed-checks, and only the working tree is evaluated again.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping, Sequence

# What a run asked for is refused with, as sysexits.h numbers it.
USAGE = 64

# How many checks one nix eval takes. Nix's evaluator never gives memory
# back, so one holds everything its checks share and each machine they
# build: a quarter of core's 231 checks held 11.3 GB at their peak and took
# 77 s at a load of 59, and every check at once about 24 GB.
CHUNK = 40
# What a check's name may hold, so a name is never read as Nix code.
NAME = re.compile(r"[A-Za-z0-9_.+-]+")
# Each chosen check's derivation path, or null for one that doesn't evaluate,
# so one broken check is named rather than stopping the whole evaluation.
SELECTING = (
    "checks: builtins.mapAttrs (_: check: let tried = builtins.tryEval check.drvPath; "
    "in if tried.success then tried.value else null) (builtins.intersectAttrs "
    "(builtins.listToAttrs (map (name: {{ inherit name; value = null; }}) [{names}])) checks)"
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


def chunks(names: Sequence[str], size: int) -> list[list[str]]:
    """Split the names, in order, into lists of at most size."""
    return [list(names[start : start + size]) for start in range(0, len(names), size)]


def selecting(names: Sequence[str]) -> str:
    """Give the Nix function taking the checks to these ones' derivations.

    A name that could be read as Nix code rather than a name is refused.
    """
    for name in names:
        if not NAME.fullmatch(name):
            message = f"{name!r} isn't a check's name"
            raise ValueError(message)
    return SELECTING.format(names=" ".join(f'"{name}"' for name in names))


def derivations(flake: str, system: str) -> dict[str, str | None]:
    """Evaluate each of a flake's checks for a system to its derivation's path."""
    checks = f"{flake}#checks.{system}"
    names: list[str] = json.loads(
        run("nix", "eval", "--json", checks, "--apply", "builtins.attrNames")
    )
    found: dict[str, str | None] = {}
    for chunk in chunks(names, CHUNK):
        found |= json.loads(run("nix", "eval", "--json", checks, "--apply", selecting(chunk)))
    return found


def kept(path: Path, evaluate: Callable[[], dict[str, str | None]]) -> dict[str, str | None]:
    """Give the derivations kept at the path, or evaluate and keep them.

    They're written beside the path and moved onto it, so a run stopped
    halfway keeps nothing rather than half.
    """
    if path.exists():
        found: dict[str, str | None] = json.loads(path.read_text())
        return found
    found = evaluate()
    path.parent.mkdir(parents=True, exist_ok=True)
    writing = path.with_suffix(".writing")
    writing.write_text(json.dumps(found))
    writing.replace(path)
    return found


def keeping(base: str, system: str) -> Path:
    """Give where a commit's derivations for a system are kept."""
    cache = os.environ.get("XDG_CACHE_HOME") or str(Path.home() / ".cache")
    return Path(cache) / "changed-checks" / f"{base}-{system}.json"


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
    before = kept(keeping(base, system), lambda: derivations(f"{flake}?rev={base}", system))
    comparison = compare(before, derivations(flake, system))
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

"""Check that every Cargo.lock names each crate's dependencies as its manifest does.

    every-lock-agrees-with-its-manifests TREE WAITING

Nix builds a crate with cargo's `--offline` but not `--locked`, so a lock
missing a dependency a manifest gained is quietly rewritten in the sandbox
and the build passes, while every cargo run outside it rewrites the lock in
the working tree. This reads each Cargo.lock in TREE and, for each crate in it
with no source (one of the repository's own, reached by path), compares the
names the lock gives it with those its Cargo.toml takes, as cargo locks them.
The lock's own crate and its workspace's members lock every dependency, build
and dev dependency, under any target, optional or not. Any other crate locks
its dependencies and build dependencies, but an optional one only when a
feature turns it on, which this doesn't follow: such a crate must lock each
that isn't optional, and may lock those that are.

WAITING is JSON naming the locks, by path, that disagreed before the rule,
while each is refreshed. A lock that disagrees fails, naming each crate and
what's missing or extra; a waiting lock that agrees fails until it comes off
the list, so the list only shrinks. Every failure is printed before the check
exits 1.
"""

import json
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# The tables a manifest names dependencies in, and those cargo locks only for
# the crates it builds tests for.
ALWAYS = ("dependencies", "build-dependencies")
MEMBERS_ONLY = ("dev-dependencies",)


def manifest(folder: Path) -> dict[str, Any]:
    """Read the Cargo.toml in a folder."""
    return tomllib.loads((folder / "Cargo.toml").read_text())


def tables(read: dict[str, Any], names: tuple[str, ...]) -> list[dict[str, Any]]:
    """Give each dependency table of these names, at the top and under each target."""
    found = [read.get(name, {}) for name in names]
    for target in read.get("target", {}).values():
        found.extend(target.get(name, {}) for name in names)
    return found


def taken(read: dict[str, Any], names: tuple[str, ...]) -> dict[str, Any]:
    """Give each dependency these tables take, by the crate's own name, with how it's asked."""
    wanted: dict[str, Any] = {}
    for table in tables(read, names):
        for key, asked in table.items():
            name = asked.get("package", key) if isinstance(asked, dict) else key
            wanted[name] = asked
    return wanted


def members(root: Path, read: dict[str, Any]) -> list[Path]:
    """Give the folders of the lock's own crate and its workspace's members."""
    found = [root] if "package" in read else []
    for pattern in read.get("workspace", {}).get("members", []):
        found.extend(sorted(path for path in root.glob(pattern) if (path / "Cargo.toml").is_file()))
    return found


def optional(asked: object) -> bool:
    """Whether a dependency is locked only when a feature turns it on."""
    return isinstance(asked, dict) and bool(asked.get("optional"))


@dataclass(frozen=True)
class Crate:
    """A crate the lock reaches by path: the names it must lock, and those it may."""

    must: set[str]
    may: set[str]
    # Whether the lock must hold the crate itself, which it needn't when only
    # an optional dependency of a crate outside the workspace reaches it.
    needed: bool


def expected(root: Path) -> dict[str, Crate]:
    """Give each crate the lock reaches by path, with what it should lock for it."""
    read = manifest(root)
    own = {folder.resolve() for folder in members(root, read)}
    # Each folder to read, with whether something reaches it other than by
    # an optional dependency of a crate outside the workspace.
    waiting = [(folder, True) for folder in own]
    crates: dict[str, Crate] = {}
    seen: dict[Path, bool] = {}
    while waiting:
        folder, needed = waiting.pop()
        if seen.get(folder, False) or (folder in seen and not needed):
            continue
        seen[folder] = needed
        crate = manifest(folder)
        member = folder in own
        wanted = taken(crate, ALWAYS + (MEMBERS_ONLY if member else ()))
        must = {name for name, asked in wanted.items() if member or not optional(asked)}
        crates[crate["package"]["name"]] = Crate(must=must, may=set(wanted), needed=needed)
        waiting.extend(
            ((folder / asked["path"]).resolve(), needed and name in must)
            for name, asked in wanted.items()
            if isinstance(asked, dict) and "path" in asked
        )
    return crates


def locked(lock: Path) -> dict[str, set[str]]:
    """Give each crate the lock holds with no source, with the names it locks for it."""
    packages = tomllib.loads(lock.read_text()).get("package", [])
    return {
        package["name"]: {entry.split()[0] for entry in package.get("dependencies", [])}
        for package in packages
        if "source" not in package
    }


def disagreements(lock: Path) -> list[str]:
    """Say each crate whose locked names differ from its manifest's."""
    wanted, held = expected(lock.parent), locked(lock)
    said = []
    for name in sorted(wanted.keys() | held.keys()):
        crate, locks = wanted.get(name), held.get(name)
        if crate is None:
            said.append(f"{name} is locked, though no manifest reaches it by path")
            continue
        if locks is None:
            if crate.needed:
                said.append(f"{name} isn't locked, though a manifest reaches it by path")
            continue
        missing = sorted(crate.must - locks)
        extra = sorted(locks - crate.may)
        if missing or extra:
            parts = [f"missing {', '.join(missing)}"] * bool(missing)
            parts += [f"holding {', '.join(extra)} it no longer takes"] * bool(extra)
            said.append(f"{name}: {'; '.join(parts)}")
    return said


def main() -> None:
    """Say each lock that disagrees with its manifests, and each waiting one that agrees."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-lock-agrees-with-its-manifests TREE WAITING")
    waiting: list[str] = json.loads(waiting_file.read_text())
    locks = sorted(
        lock
        for lock in tree.rglob("Cargo.lock")
        if ".git" not in lock.relative_to(tree).parts and (lock.parent / "Cargo.toml").is_file()
    )
    said: list[str] = []
    for lock in locks:
        path = lock.relative_to(tree).as_posix()
        found = disagreements(lock)
        if found and path not in waiting:
            said.append(
                f"{path} disagrees with its manifests: run `cargo update --offline --workspace` "
                "beside it."
            )
            said.extend(f"  {line}" for line in found)
        elif not found and path in waiting:
            said.append(f"{path} agrees with its manifests now: take it off the waiting list.")
    for failure in said:
        print(failure, file=sys.stderr)
    agreeing = sum(1 for line in said if not line.startswith(" ") and "disagrees" in line)
    print(f"{len(locks) - agreeing} of {len(locks)} locks agree with their manifests.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()

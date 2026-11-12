"""Check that every daemon this repository runs keeps the convention docs/daemons.md sets.

    daemons-agree TREE UNITS WAITING

A daemon here is a program built from one of TREE's crates that answers on a
socket in the user's runtime folder: one systemd holds for it, or one it
names on its command line and binds itself. UNITS is JSON of an
evaluated home's units, as daemons-agree.nix writes it: each socket unit's
paths and the service it starts, and each service's command line. A daemon
agrees when:

- held: systemd holds its socket, so asking while it starts or restarts
  waits on it rather than finding nothing there;
- placed: the socket is `%t/daemons/<name>.sock`, where every client looks;
- named: its socket unit, its service and its program take that name, so
  `systemctl --user status <name>`, `ps` and the socket all say one word;
- fits: the name is at most 15 characters, all `ps -e` shows of a program;
- shown: its command line starts `@<program> <name>`, so `ps -ef` reads its
  name rather than a store path;
- speaks: its crate serves through daemon-serve, the one copy of the
  interface's hello, get, list, watch, unwatch and health.

WAITING is JSON naming, for each daemon that broke the convention before the
rule, the points it breaks while it comes to agree. A daemon breaking a point
it doesn't wait on fails, as does one waiting on a point it now keeps, so the
list only shrinks. Every failure is printed before the check exits 1, and the
last line says how many agree.
"""

import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

POINTS = ("held", "placed", "named", "fits", "shown", "speaks")

# What each point asks of a daemon that breaks it.
FIXES = {
    "held": "give it a socket unit listening there, and take the listener systemd hands it",
    "placed": "listen at %t/daemons/<name>.sock",
    "named": "name its socket unit, service and program for its socket",
    "fits": "give it a name of 15 characters at most",
    "shown": "start its command line @<program> <name>",
    "speaks": "serve through daemon-serve",
}

# What ps shows of a program's name, from the kernel's TASK_COMM_LEN.
SHOWN = 15

# Services that answer in a protocol another program defines, which keep that
# protocol's ways: each with what speaks it.
OTHERS_PROTOCOL = {
    "pi-agent-proxy": "ssh-agent's protocol, which ssh and git speak to it",
}

# Daemons that speak the interface from a program no crate here builds, each
# with why it can't serve through daemon-serve. Every other point holds them.
SPEAKS_ELSEWHERE = {
    "pi-verbs": "the harness's TypeScript, beside the secret scrubber that guards tool output too",
}

# A socket a command line names, in the runtime folder.
SOCKET = re.compile(r"%t/(\S+\.sock)\b")


@dataclass(frozen=True)
class Daemon:
    """One daemon, as its units run it."""

    name: str
    socket: str
    socket_unit: str | None
    service: str
    command: str


def programs(tree: Path) -> dict[str, Path]:
    """Give each program TREE's crates build, with the crate it's built from."""
    found: dict[str, Path] = {}
    for manifest in sorted(tree.rglob("Cargo.toml")):
        relative = manifest.relative_to(tree)
        if ".git" in relative.parts or "target" in relative.parts:
            continue
        crate = tomllib.loads(manifest.read_text())
        package = crate.get("package", {}).get("name")
        source = manifest.parent / "src"
        names = [binary["name"] for binary in crate.get("bin", []) if "name" in binary]
        if package and (source / "main.rs").is_file():
            names.append(package)
        if (source / "bin").is_dir():
            names += [path.stem for path in (source / "bin").glob("*.rs")]
            names += [path.parent.name for path in (source / "bin").glob("*/main.rs")]
        for name in names:
            found.setdefault(name, manifest)
    return found


def serves(manifest: Path) -> bool:
    """Whether a crate serves through daemon-serve."""
    crate = tomllib.loads(manifest.read_text())
    return "daemon-serve" in crate.get("dependencies", {})


def started(command: str) -> tuple[str, str | None]:
    """Give the program a command line runs, and the name it's shown by when it sets one."""
    words = command.split()
    if not words:
        return "", None
    first = words[0].lstrip("-:+!")
    if first.startswith("@"):
        return first[1:], words[1] if len(words) > 1 else None
    return first, None


def daemons(units: dict[str, dict[str, dict[str, Any]]], built: dict[str, Path]) -> list[Daemon]:
    """Give every daemon the units run whose program TREE builds."""
    services: dict[str, str] = {name: unit["command"] for name, unit in units["services"].items()}
    found: list[Daemon] = []
    held: set[str] = set()
    for name, unit in sorted(units["sockets"].items()):
        for path in unit["listens"]:
            if not path.startswith("%t/"):
                continue
            held.add(path)
            command = services.get(unit["service"], "")
            program = Path(started(command)[0]).name
            placed = path.startswith("%t/daemons/") and path.endswith(".sock")
            if program not in built and not placed:
                continue
            stem = Path(path).stem if placed else name
            found.append(Daemon(stem, path, name, unit["service"], command))
    # A daemon systemd starts for a socket of its own names others' sockets on
    # its command line only as their client.
    started_for = {daemon.service for daemon in found}
    for service, command in sorted(services.items()):
        program = Path(started(command)[0]).name
        if program not in built or service in started_for:
            continue
        for socket in SOCKET.findall(command):
            path = f"%t/{socket}"
            if path in held:
                continue
            found.append(Daemon(Path(socket).stem, path, None, service, command))
            break
    return [daemon for daemon in found if daemon.service not in OTHERS_PROTOCOL]


def broken(daemon: Daemon, built: dict[str, Path]) -> list[str]:
    """Give the points of the convention a daemon breaks."""
    program, shown = started(daemon.command)
    manifest = built.get(Path(program).name)
    keeps = {
        "held": daemon.socket_unit is not None,
        "placed": daemon.socket == f"%t/daemons/{daemon.name}.sock",
        "named": {daemon.socket_unit or daemon.name, daemon.service, Path(program).name}
        == {daemon.name},
        "fits": len(daemon.name) <= SHOWN,
        "shown": shown == daemon.name,
        "speaks": daemon.name in SPEAKS_ELSEWHERE or (manifest is not None and serves(manifest)),
    }
    return [point for point in POINTS if not keeps[point]]


def main() -> None:
    """Say each daemon breaking a point it doesn't wait on, and each waiting on one it keeps."""
    try:
        tree, units_file, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: daemons-agree TREE UNITS WAITING")
    built = programs(tree)
    waiting: dict[str, list[str]] = json.loads(waiting_file.read_text())
    found = daemons(json.loads(units_file.read_text()), built)
    said: list[str] = []
    agree = 0
    for daemon in found:
        breaks = broken(daemon, built)
        agree += not breaks
        waits = waiting.get(daemon.name, [])
        said.extend(
            f"{daemon.name} ({daemon.service}.service, {daemon.socket}) breaks {point}, "
            f"which it doesn't wait on: {FIXES[point]}."
            for point in breaks
            if point not in waits
        )
        said.extend(
            f"{daemon.name} waits on {point} but keeps it: take it off the waiting list."
            for point in waits
            if point not in breaks
        )
    names = {daemon.name for daemon in found}
    said.extend(
        f"{name} waits but no daemon goes by that name: take it off the waiting list."
        for name in sorted(waiting)
        if name not in names
    )
    for failure in said:
        print(failure, file=sys.stderr)
    print(f"{agree} of {len(found)} daemons agree.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()

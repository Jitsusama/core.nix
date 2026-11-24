"""Hold Joel's surfaces even, act for act (mycelium-surfaces-stay-even).

Reads the table of acts (acts.toml) and fails, naming each, on:

- an act that leaves a surface out, or names one that isn't a surface;
- a surface that says none, or more than one, of has, owed and waived;
- a file the table names that isn't there, or doesn't hold its words;
- a mycelium command whose --help doesn't say its words;
- a pi tool with no name, an owing with no cell, no due time or no why,
  and a waiver giving no reason;
- an act named twice, or one that doesn't say what it is.

On passing it prints what each surface is still owed, by whom and when, so
the check's log is the list of gaps.

Usage: stays-even.py TABLE ROOT MYCELIUM
"""

import re
import subprocess
import sys
import tomllib
from datetime import datetime
from pathlib import Path
from typing import Any

# The surfaces every act is taken on, as the table names them.
SURFACES = {
    "cli": "the mycelium command",
    "session": "a pi session",
    "map": "the map",
    "launcher": "the launcher",
}
# What a surface can say of an act.
STANDINGS = ("has", "owed", "waived")
# A cell's address, as the mesh gives one.
ADDRESS = re.compile(r"[0-9A-Z]{6}|[0-9A-F]{8}")
# When an owing is due, in local time.
DUE = "%Y-%m-%d %H:%M"


def words_of(has: dict[str, Any]) -> list[str]:
    """The words a has says its place holds, one or several."""
    holds = has.get("holds")
    if isinstance(holds, str):
        return [holds]
    if isinstance(holds, list) and all(isinstance(one, str) for one in holds):
        return holds
    return []


def help_of(mycelium: str, command: str, helps: dict[str, str]) -> str:
    """What `mycelium COMMAND --help` says, asked once for each command."""
    if command not in helps:
        # The program is the pinned mycelium the check gives, and the
        # command a word of the table's, so no shell reads either.
        ran = subprocess.run(  # noqa: S603
            [mycelium, *command.split(), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        helps[command] = ran.stdout if ran.returncode == 0 else ""
    return helps[command]


def has_faults(
    where: str, surface: str, has: dict[str, Any], root: Path, mycelium: str, helps: dict[str, str]
) -> list[str]:
    """What is wrong with where a surface says it has an act."""
    if surface == "session":
        tool = has.get("tool")
        return [] if isinstance(tool, str) and tool != "" else [f"{where} names no pi tool"]
    words = words_of(has)
    if words == [] or "" in words:
        return [f"{where} says no words it holds"]
    if surface == "cli":
        command = has.get("command")
        if not isinstance(command, str) or command == "":
            return [f"{where} names no mycelium command"]
        said = help_of(mycelium, command, helps)
        return [
            f'{where}: mycelium {command} --help doesn\'t say "{one}"'
            for one in words
            if one not in said
        ]
    file = has.get("file")
    if not isinstance(file, str) or file == "":
        return [f"{where} names no file"]
    path = root / file
    if not path.is_file():
        return [f"{where}: {file} isn't there"]
    text = path.read_text(encoding="utf-8")
    return [f'{where}: {file} doesn\'t hold "{one}"' for one in words if one not in text]


def owed_faults(where: str, owed: dict[str, Any]) -> list[str]:
    """What is wrong with what a surface says it is owed."""
    faults = []
    if not isinstance(owed.get("by"), str) or not ADDRESS.fullmatch(owed["by"]):
        faults.append(f"{where} is owed by no cell's address")
    due = owed.get("due")
    try:
        datetime.strptime(str(due), DUE)  # noqa: DTZ007
    except ValueError:
        faults.append(f"{where} is owed with no due time, as 2026-11-24 17:00")
    if not isinstance(owed.get("why"), str) or owed["why"].strip() == "":
        faults.append(f"{where} is owed with no why")
    return faults


def act_faults(act: dict[str, Any], root: Path, mycelium: str, helps: dict[str, str]) -> list[str]:
    """What is wrong with one act's row."""
    name = str(act.get("name", "an act with no name"))
    faults = []
    if not isinstance(act.get("says"), str) or act["says"].strip() == "":
        faults.append(f"{name} doesn't say what it is")
    faults.extend(
        f"{name} names {key}, which isn't a surface"
        for key in act
        if key not in SURFACES and key not in {"name", "says"}
    )
    for surface, called in SURFACES.items():
        where = f"{name} on {called}"
        stands = act.get(surface)
        if not isinstance(stands, dict):
            faults.append(f"{where}: the table leaves it out")
            continue
        said = [standing for standing in STANDINGS if standing in stands]
        if len(said) != 1 or len(stands) != 1:
            faults.append(f"{where} says {' and '.join(sorted(stands)) or 'nothing'}, not one")
            continue
        value = stands[said[0]]
        if said[0] == "has" and isinstance(value, dict):
            faults.extend(has_faults(where, surface, value, root, mycelium, helps))
        elif said[0] == "owed" and isinstance(value, dict):
            faults.extend(owed_faults(where, value))
        elif said[0] == "waived" and isinstance(value, str) and value.strip() != "":
            pass
        else:
            faults.append(f"{where}: its {said[0]} is malformed or gives no reason")
    return faults


def owings(acts: list[dict[str, Any]]) -> list[str]:
    """Each surface still owed an act, soonest due first."""
    owed = [
        (stands["owed"]["due"], stands["owed"]["by"], act["name"], called, stands["owed"]["why"])
        for act in acts
        for surface, called in SURFACES.items()
        if isinstance(stands := act.get(surface), dict) and isinstance(stands.get("owed"), dict)
    ]
    return [
        f"{due}  {by}  {name} on {called}: {why}" for due, by, name, called, why in sorted(owed)
    ]


def main(table: Path, root: Path, mycelium: str) -> int:
    """Check the table, printing each fault, or what is owed when none."""
    acts = tomllib.loads(table.read_text(encoding="utf-8")).get("act", [])
    helps: dict[str, str] = {}
    faults = []
    seen: set[str] = set()
    for act in acts:
        name = act.get("name")
        if name in seen:
            faults.append(f"{name} is named twice")
        seen.add(name)
        faults.extend(act_faults(act, root, mycelium, helps))
    if faults:
        print(f"The surfaces aren't even, by {table.name}:")
        for fault in faults:
            print(f"- {fault}")
        return 1
    print(f"{len(acts)} acts, each on {len(SURFACES)} surfaces; still owed:")
    for owing in owings(acts):
        print(f"- {owing}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 4:  # noqa: PLR2004
        print(__doc__)
        sys.exit(2)
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]))

"""Hold Joel's surfaces even, act for act (mycelium-surfaces-stay-even).

Reads the table of acts (acts.toml) and fails, naming each, on:

- an act that leaves a surface out, or names one that isn't a surface;
- a surface that says none, or more than one, of has, owed and waived;
- a file the table names that isn't there, or doesn't hold its words;
- a mycelium command whose --help doesn't say its words;
- a pi tool with no name, an owing with no cell, no due time or no why,
  and a waiver giving no reason;
- an act named twice or with no name, or one that doesn't say what it is
  or the way to it every surface takes;
- an act a surface lists in its own file (the launcher session row's
  verbs and the map drawer's acts) that the table doesn't name, or names apart with no
  reason, a name the table gives that the file no longer lists or maps
  to no act, and a list its pattern reads nothing of.

A has proves its words are where it says, not that the surface's code
offers the act; the surfaces' own tests prove that. Nor are an act's words
compared across surfaces, since each says it in its own place.

On passing it prints what each surface is still owed, by whom and when, so
the check's log is the list of gaps.

Usage: stays-even.py TABLE ROOT MYCELIUM
"""

import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
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
# When an owing is due, in local time, padded so the text sorts as the time.
DUE = "%Y-%m-%d %H:%M"
DUE_SHAPE = re.compile(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}")
# The longest a command's --help may take, so a hung one names itself.
HELP_SECONDS = 30


@dataclass
class Sources:
    """Where a has is read: core.nix's files, and the locked mycelium's help."""

    root: Path
    mycelium: str
    helps: dict[str, str | None] = field(default_factory=dict)
    failed: dict[str, str] = field(default_factory=dict)

    def help_of(self, command: str) -> str | None:
        """Give what `mycelium COMMAND --help` says, or None where it failed."""
        if command not in self.helps:
            # The program is the locked mycelium the check gives, and the
            # command a word of the table's, so no shell reads either.
            try:
                # S603: the program is the locked mycelium, its words the table's.
                ran = subprocess.run(  # noqa: S603
                    [self.mycelium, *command.split(), "--help"],
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=HELP_SECONDS,
                )
            except subprocess.TimeoutExpired:
                self.helps[command] = None
            else:
                self.helps[command] = ran.stdout if ran.returncode == 0 else None
                if ran.returncode != 0:
                    said = ran.stderr.strip().splitlines() or ["nothing on stderr"]
                    self.failed[command] = f"exited {ran.returncode}: {said[0]}"
        return self.helps[command]

    def why_failed(self, command: str) -> str:
        """Say how `mycelium COMMAND --help` failed, as it was asked."""
        return self.failed.get(command, f"took past {HELP_SECONDS} s")


def words_of(has: dict[str, Any]) -> list[str]:
    """Give the words a has says its place holds, one or several."""
    holds = has.get("holds")
    if isinstance(holds, str):
        return [holds]
    if isinstance(holds, list) and all(isinstance(one, str) for one in holds):
        return holds
    return []


def command_faults(
    where: str, has: dict[str, Any], words: list[str], sources: Sources
) -> list[str]:
    """Find what is wrong with a mycelium command said to have an act."""
    command = has.get("command")
    if not isinstance(command, str) or command == "":
        return [f"{where} names no mycelium command"]
    said = sources.help_of(command)
    if said is None:
        return [f"{where}: mycelium {command} --help failed, {sources.why_failed(command)}"]
    return [
        f'{where}: mycelium {command} --help doesn\'t say "{one}"'
        for one in words
        if one not in said
    ]


def file_faults(where: str, has: dict[str, Any], words: list[str], root: Path) -> list[str]:
    """Find what is wrong with a file of core.nix said to have an act."""
    file = has.get("file")
    if not isinstance(file, str) or file == "":
        return [f"{where} names no file"]
    path = root / file
    if not path.is_file():
        return [f"{where}: {file} isn't there"]
    text = path.read_text(encoding="utf-8")
    return [f'{where}: {file} doesn\'t hold "{one}"' for one in words if one not in text]


def has_faults(where: str, surface: str, has: dict[str, Any], sources: Sources) -> list[str]:
    """Find what is wrong with where a surface says it has an act."""
    if surface == "session":
        tool = has.get("tool")
        return [] if isinstance(tool, str) and tool != "" else [f"{where} names no pi tool"]
    words = words_of(has)
    if words == [] or "" in words:
        return [f"{where} says no words it holds"]
    if surface == "cli":
        return command_faults(where, has, words, sources)
    return file_faults(where, has, words, sources.root)


def owed_faults(where: str, owed: dict[str, Any]) -> list[str]:
    """Find what is wrong with what a surface says it is owed."""
    faults = []
    if not isinstance(owed.get("by"), str) or not ADDRESS.fullmatch(owed["by"]):
        faults.append(f"{where} is owed by no cell's address")
    if not due_of(owed):
        faults.append(f"{where} is owed with no due time, as 2026-11-24 17:00")
    if not isinstance(owed.get("why"), str) or owed["why"].strip() == "":
        faults.append(f"{where} is owed with no why")
    return faults


def due_of(owed: dict[str, Any]) -> datetime | None:
    """Give when an owing is due, or None where it says no time as DUE does."""
    due = owed.get("due")
    if not isinstance(due, str) or not DUE_SHAPE.fullmatch(due):
        return None
    try:
        # DTZ007: a due date is on the person's clock, as the table writes it.
        return datetime.strptime(due, DUE)  # noqa: DTZ007
    except ValueError:
        return None


def act_faults(act: dict[str, Any], sources: Sources) -> list[str]:
    """Find what is wrong with one act's row."""
    named = isinstance(act.get("name"), str) and act["name"] != ""
    name = act["name"] if named else "an act with no name"
    faults = [] if named else [f"{name} is in the table"]
    if not isinstance(act.get("says"), str) or act["says"].strip() == "":
        faults.append(f"{name} doesn't say what it is")
    faults.extend(
        f"{name} names {key}, which isn't a surface"
        for key in act
        if key not in SURFACES and key not in {"name", "says", "through"}
    )
    if not isinstance(act.get("through"), str) or act["through"].strip() == "":
        faults.append(f"{name} doesn't say the way to it every surface takes")
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
            faults.extend(has_faults(where, surface, value, sources))
        elif said[0] == "owed" and isinstance(value, dict):
            faults.extend(owed_faults(where, value))
        elif said[0] == "waived" and isinstance(value, str) and value.strip() != "":
            pass
        else:
            faults.append(f"{where}: its {said[0]} is malformed or gives no reason")
    return faults


def owings(acts: list[dict[str, Any]]) -> list[str]:
    """List each surface still owed an act, soonest due first."""
    owed = [
        (stands["owed"]["due"], stands["owed"], act["name"], called)
        for act in acts
        for surface, called in SURFACES.items()
        if isinstance(stands := act.get(surface), dict) and isinstance(stands.get("owed"), dict)
    ]
    # Every due is zero-padded by now (owed_faults), so its text sorts as
    # its time.
    owed.sort(key=lambda each: (each[0], each[2], each[3]))
    return [
        f"{one['due']}  {one['by']}  {name} on {called}: {one['why']}"
        for _, one, name, called in owed
    ]


def offered_faults(offered: dict[str, Any], names: set[str], root: Path) -> list[str]:
    """Find the acts a surface's own file lists that the table doesn't name.

    Each match of the entry's pattern is one act the surface lists, its
    first group the surface's own word for it. Each must be named as an act
    of the table, or set apart with a reason, so an act a surface gains
    fails until the table takes it; and a word the table gives that the file
    no longer lists fails too, so the mapping can't go stale.
    """
    surface = offered.get("surface")
    if surface not in SURFACES:
        return [f"an offered list names {surface}, which isn't a surface"]
    where = f"what {SURFACES[surface]} lists"
    file, pattern = offered.get("file"), offered.get("pattern")
    if not isinstance(file, str) or not isinstance(pattern, str):
        return [f"{where} names no file and pattern"]
    path = root / file
    if not path.is_file():
        return [f"{where}: {file} isn't there"]
    listed = set(re.findall(pattern, path.read_text(encoding="utf-8"), re.MULTILINE))
    acts, apart = offered.get("acts", {}), offered.get("apart", {})
    faults = [f"{where}: {file} lists no act, as {pattern} reads it"] if not listed else []
    for word in sorted(listed):
        if word in acts:
            if acts[word] not in names:
                faults.append(f"{where}: {word} is said to be {acts[word]}, no act of the table")
        elif not isinstance(apart.get(word), str) or apart[word].strip() == "":
            faults.append(f"{where}: {word} is no act of the table, nor set apart with a reason")
    faults.extend(
        f"{where}: {file} no longer lists {word}"
        for word in sorted((set(acts) | set(apart)) - listed)
    )
    return faults


def main(table: Path, root: Path, mycelium: str) -> int:
    """Check the table, printing each fault, or what is owed when none."""
    read = tomllib.loads(table.read_text(encoding="utf-8"))
    acts = read.get("act", [])
    sources = Sources(root, mycelium)
    faults = []
    seen: set[str] = set()
    for act in acts:
        name = act.get("name")
        if name in seen:
            faults.append(f"{name} is named twice")
        seen.add(name)
        faults.extend(act_faults(act, sources))
    for offered in read.get("offered", []):
        faults.extend(offered_faults(offered, seen, root))
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
    if len(sys.argv) != 4:  # noqa: PLR2004 - the program, the table, the root and mycelium
        print(__doc__)
        sys.exit(2)
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]))

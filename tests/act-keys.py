"""Check that each act's own key is free on the surface it's pressed on.

    act-keys WORDS QUERYKEYS

WORDS is the launcher's vocabulary as the account writes it and QUERYKEYS
modules/quickshell/QueryKeys.js, the table of what each key does in each
mode. An act beside a row's Return takes a key of its own in normal, named
by its words' `key` (VKCF3Z, 2026-11-12), and pressed while its row is
selected. So each such key is one letter, means nothing in normal already,
since normal would take it first, and is given to only one act of a row,
since one key can't do two things there. Each that isn't is printed, and the
check fails. So does the check itself, should its reading of the table stop
finding the keys normal takes.
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

Entry = dict[str, Any]

# Normal's keys beyond its table's: the counts, Space leading, the colon's
# line, the operators' second keys, and those Prompt.qml reads itself
# (VKCF3Z, 2026-11-12). z leads the drawer's folds, as Vim's za does.
OWN = set("jkhlgGydcuopnN/?:-qZwxvz0123456789") | {"Space"}

# A row of the table whose keys normal reads: its own, and every mode's.
ROW = re.compile(r'\{ act: "\w+", mode: "(?:normal|both)"[^}]*?keys: \[(.*?)\] \}', re.DOTALL)
KEY = re.compile(r'"((?:[^"\\]|\\.)+)"')


def taken(table: str) -> set[str]:
    """Give every key normal does something with, as the first key or any after."""
    keys = set(OWN)
    for row in ROW.finditer(table):
        keys.update(KEY.findall(row.group(1)))
    return keys


def unfree(vocabulary: Entry, keys: set[str]) -> list[str]:
    """Say each act's key that isn't one letter, that normal takes, or that one row gives twice."""
    found: list[str] = []
    rows: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for thing in vocabulary.get("things", []):
        key = thing.get("key")
        if key is None:
            continue
        name = thing["id"]
        if not (isinstance(key, str) and len(key) == 1 and key.isprintable() and key != " "):
            found.append(f"{name}'s key {key!r} isn't one letter")
            continue
        if key in keys:
            found.append(f"{name}'s key {key} is normal's own")
        rows[thing.get("in", "")].append((key, name))
    for row, given in rows.items():
        seen: dict[str, str] = {}
        for key, name in given:
            if key in seen:
                found.append(f"{name} and {seen[key]} both take {key} in {row}")
            seen[key] = name
    return found


def refuses(keys: set[str]) -> list[str]:
    """Say what the table's reading misses that it's there to find."""
    # Append, f's letter and the next paragraph are normal's by the table
    # alone, none of them in OWN.
    missed = [key for key in ("a", "f", "}") if key not in keys]
    return [f"normal's {key} isn't read from the table" for key in missed]


def main(argv: list[str]) -> None:
    """Print each act's key that isn't free, and fail if there are any."""
    vocabulary = json.loads(Path(argv[0]).read_text())
    keys = taken(Path(argv[1]).read_text())
    found = refuses(keys) + unfree(vocabulary, keys)
    for said in found:
        print(said)
    if found:
        sys.exit(f"{len(found)} acts' keys aren't free.")


if __name__ == "__main__":
    main(sys.argv[1:])

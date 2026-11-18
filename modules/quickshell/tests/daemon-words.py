"""Check that the words a daemon's tests are given are the launcher's own, every one.

    daemon-words WORDS FIXTURE...

WORDS is the launcher's vocabulary as the account writes it, which each
daemon is handed at run time, and each FIXTURE a slice of it a daemon's tests
hold, so they run without the shell. A daemon's tests pass on its fixture's
words, so a word changed in the vocabulary and not in the fixture passes them
while Joel reads another: Screens' note for turning every screen off said "a
key wakes them" in its fixture for days after the launcher's said more
(2026-11-11). So every value a fixture holds must be the vocabulary's at the
same place, a list of entries with an `id`, as `things` is, matched by its
id; a fixture may hold less, never other. Each that differs fails, printed
with both words. The check first proves it refuses a changed word, a word
the vocabulary hasn't, and an entry it names no more, so a rule that stopped
refusing fails it too.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

type Json = object

# The vocabulary and at least one fixture.
LEAST_ARGUMENTS = 2


def keyed(value: Json) -> Json:
    """Give a value with each list of entries with ids made a table by its id, all the way in."""
    if isinstance(value, dict):
        return {key: keyed(inner) for key, inner in value.items()}
    if (
        isinstance(value, list)
        and value
        and all(isinstance(entry, dict) and "id" in entry for entry in value)
    ):
        return {f"#{entry['id']}": keyed(entry) for entry in value}
    return value


def differences(fixture: Json, words: Json, at: str = "") -> list[str]:
    """Give each place the fixture says other than the vocabulary, with both words."""
    if not isinstance(fixture, dict):
        return (
            [] if fixture == words else [f"{at}: {fixture!r}, where the launcher's says {words!r}"]
        )
    if not isinstance(words, dict):
        return [f"{at}: a table, where the launcher's says {words!r}"]
    found: list[str] = []
    for key, inner in fixture.items():
        if key not in words:
            found.append(f"{at}/{key}: {inner!r}, which the launcher's hasn't")
        else:
            found.extend(differences(inner, words[key], f"{at}/{key}"))
    return found


def disagrees(fixture: Json, words: Json) -> list[str]:
    """Give where a fixture disagrees with the vocabulary, its entries matched by id."""
    return differences(keyed(fixture), keyed(words))


def refuses_what_it_should() -> list[str]:
    """Give each rule that let a drifted fixture through, from fixtures made to drift."""
    words = {"things": [{"id": "displays", "said": {"note": "a key wakes them, hubs stay on"}}]}
    drifted = {
        "a changed word": {"things": [{"id": "displays", "said": {"note": "a key wakes them"}}]},
        "a word it hasn't": {"things": [{"id": "displays", "said": {"other": "x"}}]},
        "an entry gone": {"things": [{"id": "screens", "said": {"note": "x"}}]},
    }
    let_through = [rule for rule, fixture in drifted.items() if not disagrees(fixture, words)]
    if disagrees({"things": [{"id": "displays"}]}, words):
        let_through.append("a fixture holding less")
    return let_through


def main(arguments: list[str]) -> int:
    """Check each fixture against the vocabulary, and say every word that differs."""
    if len(arguments) < LEAST_ARGUMENTS:
        print("usage: daemon-words WORDS FIXTURE...", file=sys.stderr)
        return 64
    let_through = refuses_what_it_should()
    for rule in let_through:
        print(f"daemon-words: the check lets {rule} through", file=sys.stderr)
    words = json.loads(Path(arguments[0]).read_text(encoding="utf-8"))
    failed = bool(let_through)
    for name in arguments[1:]:
        fixture = json.loads(Path(name).read_text(encoding="utf-8"))
        found = disagrees(fixture, words)
        for difference in found:
            print(f"{name}{difference}", file=sys.stderr)
        failed = failed or bool(found)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

"""Hold the key map to the key language (keys-speak-the-language).

Reads the language (language.toml) and the key map every layer writes
(keys.json), and fails, naming each, on:

- a chord a layer binds that the language gives to another act;
- an act a surface's kinds hold that the surface leaves out;
- a chord bound to an act that isn't one of the act's keys.

Every surface, a layer that names its kinds, is held to all three. The
source, Neovim, is held to the first and last for each key it binds
everywhere in normal mode, so a change to keys.nix fails until the language
agrees. A layer that names no kinds yet speaks the language only where a
chord names an act.

Usage: speaks-the-language.py LANGUAGE KEYS SOURCE
"""

import json
import sys
import tomllib
from pathlib import Path
from typing import Any

# A layer of the key map, or one of its chords, as keys.json has them.
Layer = dict[str, Any]
Bind = dict[str, Any]
# Each act's keys in the key map's spelling, its modes and its kinds.
Acts = dict[str, dict[str, set[str]]]

# Neovim's names for keys, as the key map spells them once normalised
# (modules/keys/home.nix, normaliseOne).
NAMED = {
    "space": "space",
    "cr": "enter",
    "enter": "enter",
    "return": "enter",
    "esc": "escape",
    "bs": "backspace",
    "tab": "tab",
    "up": "up",
    "down": "down",
    "left": "left",
    "right": "right",
    "pageup": "pageup",
    "pagedown": "pagedown",
    "home": "home",
    "end": "end",
    "del": "delete",
    "lt": "<",
    "bslash": "\\",
}
HELD = {"d": "super", "c": "ctrl", "a": "alt", "m": "alt", "s": "shift"}
ORDER = ["super", "ctrl", "alt", "shift"]


class UnknownKeyError(ValueError):
    """A key the language names that Neovim has no name for."""

    def __init__(self, name: str) -> None:
        """Name the key."""
        super().__init__(f"no such key as <{name}>")


def key_name(name: str) -> str:
    """Spell a named key, as <Down> names it, as the key map does."""
    lowered = name.lower()
    if lowered in NAMED:
        return NAMED[lowered]
    if lowered[0] == "f" and lowered[1:].isdigit():
        return lowered
    if len(name) == 1:
        return name.lower()
    raise UnknownKeyError(name)


def one_key(token: str) -> str:
    """Spell one key, a character or a <...> name, as the key map does."""
    if not token.startswith("<") or len(token) == 1:
        if token.isalpha() and token.isupper():
            return f"shift+{token.lower()}"
        return token
    inner = token[1:-1]
    parts = inner.split("-")
    # A key of - itself ends in an empty part, as <C-->.
    if inner.endswith("-") and len(parts) > 1:
        parts = [*parts[:-2], "-"]
    *held, key = parts
    modifiers = {HELD[part.lower()] for part in held}
    return "+".join([m for m in ORDER if m in modifiers] + [key_name(key)])


def spelled(notation: str) -> str:
    """Spell keys in Neovim's notation, as gg or <Space>?, as the key map does.

    The key map parts a sequence's keys with a space: g g, space ?.
    """
    keys, rest = [], notation
    while rest:
        end = rest.find(">") if rest.startswith("<") else -1
        if end > 1:
            keys.append(one_key(rest[: end + 1]))
            rest = rest[end + 1 :]
        else:
            keys.append(one_key(rest[0]))
            rest = rest[1:]
    return " ".join(keys)


def acts_of(language: dict[str, Any]) -> Acts:
    """Read each act's keys, spelled as the key map's, its modes and kinds.

    An act is pressed in normal mode unless its modes say otherwise.
    """
    return {
        name: {
            "keys": {spelled(key) for key in act["keys"]},
            "modes": set(act.get("modes", ["normal"])),
            "kinds": set(act["kinds"]),
        }
        for name, act in language["acts"].items()
    }


def owner(acts: Acts, keys: str, modes: set[str]) -> str | None:
    """Find the act the language gives these keys to in any of these modes."""
    for name, act in acts.items():
        if keys in act["keys"] and act["modes"] & modes:
            return name
    return None


def language_faults(acts: Acts) -> list[str]:
    """Find two acts the language gives one key in one mode."""
    faults = []
    seen: dict[tuple[str, str], str] = {}
    for name, act in acts.items():
        for mode in sorted(act["modes"]):
            for keys in sorted(act["keys"]):
                other = seen.setdefault((mode, keys), name)
                if other != name:
                    faults.append(
                        f"the language gives {keys} to both {other} and {name} in {mode} mode"
                    )
    return faults


def bind_faults(layer: str, binds: list[Bind], acts: Acts, modes: set[str]) -> list[str]:
    """Find a chord given to another act, or bound to an act that isn't its."""
    faults = []
    for bind in binds:
        chord, keys, act = bind["chord"], bind["keys"], bind["act"]
        if act and act not in acts:
            faults.append(f"{layer}: {chord} does {act}, which the language doesn't name")
            continue
        if act and keys not in acts[act]["keys"]:
            ways = ", ".join(sorted(acts[act]["keys"]))
            faults.append(f"{layer}: {chord} isn't one of {act}'s keys ({ways})")
            continue
        taken = owner(acts, keys, acts[act]["modes"] if act else modes)
        if taken and taken != act:
            does = f"does {act}" if act else f"is its own ({bind['does']})"
            faults.append(f"{layer}: {chord} {does}, but the language gives it to {taken}")
    return faults


def missing_faults(layer: str, kinds: list[str], binds: list[Bind], acts: Acts) -> list[str]:
    """Find an act the surface's kinds hold that it leaves out."""
    bound = {bind["act"] for bind in binds}
    return [
        f"{layer}: a {', '.join(sorted(act['kinds'] & set(kinds)))} has {name}, which it leaves out"
        for name, act in acts.items()
        if act["kinds"] & set(kinds) and name not in bound
    ]


def faults(language: dict[str, Any], layers: list[Layer], source: str) -> list[str]:
    """Find every way the key map strays from the language."""
    acts = acts_of(language)
    found = language_faults(acts)
    for layer in layers:
        name, kinds, binds = layer["name"], layer.get("kinds", []), layer["binds"]
        if name == source:
            # Only what Neovim binds everywhere in normal mode; a plugin's
            # window and the other modes are its own.
            found += bind_faults(name, [b for b in binds if b["when"] == ""], acts, {"normal"})
        elif kinds:
            modes = {"normal", "insert"} if "field" in kinds else {"normal"}
            found += bind_faults(name, binds, acts, modes)
            found += missing_faults(name, kinds, binds, acts)
        else:
            found += bind_faults(name, [b for b in binds if b["act"]], acts, {"normal"})
    return found


def main() -> None:
    """Read the language and the key map, and say every fault."""
    language_path, keys_path, source = sys.argv[1:]
    with Path(language_path).open("rb") as file:
        language = tomllib.load(file)
    with Path(keys_path).open(encoding="utf-8") as file:
        layers = json.load(file)
    found = faults(language, layers, source)
    for fault in found:
        print(fault)
    if found:
        print(
            "The key map strays from the key language (modules/keys/language.toml): "
            "change the binding, or the language where Neovim's keys.nix changed."
        )
        sys.exit(1)


if __name__ == "__main__":
    main()

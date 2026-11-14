"""Hold the key map to the key language (keys-speak-the-language).

Reads the language (language.toml), the key map every layer writes
(keys.json) and every name Neovim takes for a command of its own
(neovim-commands.lua's table), and fails, naming each, on:

- a chord a layer binds that the language gives to another act;
- an act a surface's kinds hold that the surface leaves out;
- a chord bound to an act that isn't one of the act's keys;
- two acts given one key in one mode, or one name on the command line;
- a command, or a name between its short form and its whole one, that
  Neovim takes for another, or one of Neovim's own shortened otherwise;
- an act that changes something with a key the find line reaches, one
  that ends or starts something with Return in any mode, and a list's act
  taking h, l or Backspace, which step into and out of nothing;
- an arrow in normal or visual mode that isn't its letter's motion: an
  act a surface's kinds hold naming one, since its letter brings it, or
  a modal layer of ours binding one held, in a sequence, or to anything
  but its letter's act. An arrow after the leader, as Space and the
  right arrow, overlays Vim's own keys and is allowed;
- Escape given to any act but stepping back, or a layer of ours whose
  own Escape closes or quits, since Escape does what Vim's does and
  closing is :q, ZZ or Space w alone;
- a mode, kind or field the language doesn't name, a command with no short
  form or one that doesn't begin it, an undoing of nothing, and a guard on
  an act that would never ask.

Every surface, a layer that names its kinds, is held to the first three.
The source, Neovim, is held to the first and third for each key it binds
everywhere in normal mode and each it binds on the command line, so a
change to keys.nix fails until the language agrees. A layer that names no
kinds yet speaks the language only where a chord names an act.

Usage: speaks-the-language.py LANGUAGE KEYS SOURCE NEOVIM_COMMANDS
"""

import json
import re
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

# What an act may say of itself.
FIELDS = {
    "keys",
    "modes",
    "kinds",
    "takes",
    "command",
    "short",
    "changes",
    "undoable",
    "guarded",
    "costly",
    "range",
    "ends",
    "starts",
}
# The modes an arrow is its letter's motion in and nothing else (Joel,
# 2026-11-05: "In normal mode, in a buffer, arrows move the cursor"), and
# the arrows.
LETTERED = {"normal", "visual"}
ARROWS = {"up", "down", "left", "right"}
# The leader, after which an arrow overlays Vim's own keys (Joel,
# 2026-11-05: "leader stuff can overlay default stuff").
LEADER = "space"
# The layers of ours with a normal mode, held to that.
MODAL = {"inlyne", "mpv", "neovim", "reader", "swayimg", "zathura"}
# Each arrow and the letter whose motion it is.
LETTERS = {"down": "j", "up": "k", "left": "h", "right": "l"}
# The one act Escape is, as Vim's: it cancels what's begun and leaves a
# mode, and otherwise does nothing (Joel, 2026-11-05: "fuck escape
# quitting").
ESCAPE = "escape"
STEPS_BACK = "back"
# What a layer's own chord says when it closes or quits.
CLOSING = re.compile(r"\b(close|closes|quit|quits|exit|exits)\b", re.IGNORECASE)
# Where Neovim's keys on the command line are bound, in the key map's words
# (modules/neovim/home.nix, scopes).
COMMAND_LINE = "on the command line"


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


def modes_of(act: dict[str, Any]) -> set[str]:
    """Read the modes an act is pressed in: normal, unless it says."""
    return set(act.get("modes", ["normal"]))


def arrowed(keys: str) -> bool:
    """Say whether keys, spelled as the key map's, press an arrow the leader doesn't begin.

    One is held to it alone, held with a modifier or in another sequence,
    as g and the left arrow; the launcher's own table is held the same way
    (tests/query-keys.qml).
    """
    pressed = keys.split(" ")
    return pressed[0] != LEADER and any(key.split("+")[-1] in ARROWS for key in pressed)


def acts_of(language: dict[str, Any]) -> Acts:
    """Read each act's keys, spelled as the key map's, its modes and kinds.

    An act with a letter an arrow is, in normal or visual mode, takes the
    arrow too, as its motion.
    """
    acts = {}
    for name, act in language["acts"].items():
        keys = {spelled(key) for key in act.get("keys", [])}
        if modes_of(act) & LETTERED:
            keys |= {arrow for arrow, letter in LETTERS.items() if letter in keys}
        acts[name] = {"keys": keys, "modes": modes_of(act), "kinds": set(act["kinds"])}
    return acts


def owner(acts: Acts, keys: str, modes: set[str]) -> str | None:
    """Find the act the language gives these keys to in any of these modes."""
    for name, act in acts.items():
        if keys in act["keys"] and act["modes"] & modes:
            return name
    return None


def key_faults(acts: Acts) -> list[str]:
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


def shape_faults(language: dict[str, Any]) -> list[str]:
    """Find a field, mode or kind the language doesn't name, or a flag amiss."""
    faults = []
    for name, act in language["acts"].items():
        faults += [
            f"{name} has {field}, which the language doesn't know"
            for field in sorted(set(act) - FIELDS)
        ]
        faults += [
            f"{name} is pressed in {mode} mode, which the language doesn't name"
            for mode in sorted(modes_of(act) - set(language["modes"]))
        ]
        faults += [
            f"{name} belongs to a {kind}, which the language doesn't name"
            for kind in sorted(set(act["kinds"]) - set(language["kinds"]))
        ]
        faults += flag_faults(name, act)
    return faults


def flag_faults(name: str, act: dict[str, Any]) -> list[str]:
    """Find a flag of an act's amiss: one saying it changes, guards or costs where it can't."""
    said = {"undoable": "can be undone", "ends": "ends something", "starts": "starts something"}
    faults = [
        f"{name} {says}, but changes nothing"
        for flag, says in said.items()
        if act.get(flag) and not act.get("changes")
    ]
    if act.get("guarded"):
        if not act.get("changes"):
            faults.append(f"{name} guards itself, but it never asks, since it changes nothing")
        elif act.get("undoable"):
            faults.append(f"{name} guards itself, but it never asks, since it can be undone")
    if act.get("costly"):
        if not act.get("changes"):
            faults.append(f"{name} costs a lot, but it changes nothing")
        elif act.get("undoable"):
            faults.append(f"{name} costs a lot, but it can be undone")
        elif act.get("guarded"):
            faults.append(f"{name} costs a lot, but it guards itself")
    return faults


def arrow_faults(language: dict[str, Any]) -> list[str]:
    """Find an act of a surface's naming an arrow, which only its letter's motion takes.

    An act no kind holds is only a layer's, as his Neovim's splits are, and
    is held where the layer binds it.
    """
    faults = []
    for name, act in language["acts"].items():
        if not act["kinds"]:
            continue
        for mode in sorted(modes_of(act) & LETTERED):
            faults += [
                f"{name} takes {keys} in {mode} mode, where an arrow is only its letter's motion"
                for keys in sorted(spelled(key) for key in act.get("keys", []))
                if arrowed(keys)
            ]
    return faults


def arrow_bind_faults(layer: Layer, acts: Acts) -> list[str]:
    """Find an arrow a modal layer of ours binds in normal as anything but its letter's motion."""
    name = layer["name"]
    if name not in MODAL:
        return []
    faults = []
    normal = [b for b in layer["binds"] if b.get("when", "") == ""]
    for bind in normal:
        if not arrowed(bind["keys"]):
            continue
        act = bind["act"]
        letter = LETTERS.get(bind["keys"])
        if letter and act in acts and letter in acts[act]["keys"]:
            continue
        # A program's own motion, as mpv's seeking: the arrow does just what
        # the layer's letter does.
        if letter and any(
            b["keys"] == letter and b["act"] == act and b["does"] == bind["does"] for b in normal
        ):
            continue
        given = f"to {act}" if act else "its own act"
        faults.append(
            f"{name}: {bind['chord']} gives an arrow {given}, where an arrow is only its"
            " letter's motion"
        )
    return faults


def return_faults(language: dict[str, Any]) -> list[str]:
    """Find an end or a start given Return, or a list's act taking h, l or Backspace.

    What ends or takes away, and what calls someone or starts something
    new, runs only from d or the command line, never from Return in any
    mode; and nothing on a list steps in or out by h, l or Backspace,
    which Vim's lists never do.
    """
    faults = []
    for name, act in language["acts"].items():
        keys = sorted(spelled(key) for key in act.get("keys", []))
        for flag in ("ends", "starts"):
            if act.get(flag):
                faults += [
                    f"{name} {flag} something, but Return reaches it by {key} in {mode} mode"
                    for key in keys
                    if key.split(" ")[-1] == "enter"
                    for mode in sorted(modes_of(act))
                ]
        if "list" in act["kinds"]:
            faults += [
                f"{name} takes {key} on a list, where h, l and Backspace mean nothing"
                for key in keys
                if key in {"h", "l", "backspace"} and modes_of(act) & LETTERED
            ]
    return faults


def escape_faults(language: dict[str, Any], layers: list[Layer], source: str) -> list[str]:
    """Find Escape given to any act but stepping back, or a layer's own Escape that closes.

    A layer's Escape bound to one of the language's acts is held by the
    bind's own check, which only back's keys pass; this holds what a layer
    binds by its own words. His Neovim keeps its own.
    """
    faults = [
        f"{name} takes escape, where Escape only steps back, as Vim's does"
        for name, act in language["acts"].items()
        if name != STEPS_BACK and ESCAPE in {spelled(key) for key in act.get("keys", [])}
    ]
    for layer in layers:
        if layer["name"] == source:
            continue
        faults += [
            f'{layer["name"]}: {bind["chord"]} does "{bind["does"]}", where Escape never'
            " closes or quits"
            for bind in layer["binds"]
            if bind["keys"] == ESCAPE and not bind["act"] and CLOSING.search(bind["does"])
        ]
    return faults


def find_faults(language: dict[str, Any]) -> list[str]:
    """Find an act that changes something with a key the find line reaches."""
    return [
        f"{name} changes things, but the find line reaches it by {keys}"
        for name, act in language["acts"].items()
        if act.get("changes") and "find" in modes_of(act)
        for keys in sorted(spelled(key) for key in act.get("keys", []))
    ]


def vim_short(neovim: dict[str, str], command: str) -> str:
    """Find the fewest letters Neovim takes for one of its commands."""
    short = command
    while len(short) > 1 and neovim.get(short[:-1]) == command:
        short = short[:-1]
    return short


def command_faults(language: dict[str, Any], neovim: dict[str, str]) -> list[str]:
    """Find a command shortened amiss, named twice, or named as Neovim's.

    Every name from an act's short form to its whole command is the act's,
    so no two acts may share one. A command Neovim has keeps Neovim's short
    form; any other may be none of the names Neovim takes.
    """
    faults = []
    names: dict[str, str] = {}
    for name, act in language["acts"].items():
        command = act.get("command")
        if command is None:
            continue
        short = act.get("short")
        if short is None:
            faults.append(f"{name}'s command, {command}, has no short form")
            continue
        if not command.startswith(short):
            faults.append(f"{name}'s short form {short} doesn't begin its command, {command}")
            continue
        spoken = [command[:length] for length in range(len(short), len(command) + 1)]
        for word in spoken:
            other = names.setdefault(word, name)
            if other != name:
                faults.append(f"the language gives :{word} to both {other} and {name}")
        if neovim.get(command) == command:
            own = vim_short(neovim, command)
            if own != short:
                faults.append(
                    f"{name}'s :{command} is Neovim's, whose short form is {own}, not {short}"
                )
            continue
        faults += [
            f"{name}'s :{word} is Neovim's :{neovim[word]}" for word in spoken if word in neovim
        ]
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
    """Find an act with keys the surface's kinds hold that it leaves out."""
    bound = {bind["act"] for bind in binds}
    return [
        f"{layer}: a {', '.join(sorted(act['kinds'] & set(kinds)))} has {name}, which it leaves out"
        for name, act in acts.items()
        if act["keys"] and act["kinds"] & set(kinds) and name not in bound
    ]


def layer_faults(layer: Layer, acts: Acts, source: str) -> list[str]:
    """Find every way one layer of the key map strays from the language."""
    name, kinds, binds = layer["name"], layer.get("kinds", []), layer["binds"]
    if name == source:
        # What Neovim binds everywhere in normal mode, and on the command
        # line; a plugin's window and the other modes are its own.
        return bind_faults(
            name, [b for b in binds if b["when"] == ""], acts, {"normal"}
        ) + bind_faults(name, [b for b in binds if b["when"] == COMMAND_LINE], acts, {"command"})
    if kinds:
        # A surface is in every mode an act its kinds hold is pressed in,
        # so a page selects in visual mode and a list finds in find mode.
        modes = {"normal"}.union(
            *(act["modes"] for act in acts.values() if act["kinds"] & set(kinds))
        )
        return bind_faults(name, binds, acts, modes) + missing_faults(name, kinds, binds, acts)
    return bind_faults(name, [b for b in binds if b["act"]], acts, {"normal"})


def faults(
    language: dict[str, Any], layers: list[Layer], source: str, neovim: dict[str, str]
) -> list[str]:
    """Find every way the language or the key map strays."""
    found = shape_faults(language) + arrow_faults(language) + find_faults(language)
    found += return_faults(language)
    found += escape_faults(language, layers, source)
    found += command_faults(language, neovim)
    acts = acts_of(language)
    found += key_faults(acts)
    for layer in layers:
        found += layer_faults(layer, acts, source)
        found += arrow_bind_faults(layer, acts)
    return found


def main() -> None:
    """Read the language, the key map and Neovim's commands, and say every fault."""
    language_path, keys_path, source, neovim_path = sys.argv[1:]
    with Path(language_path).open("rb") as file:
        language = tomllib.load(file)
    with Path(keys_path).open(encoding="utf-8") as file:
        layers = json.load(file)
    with Path(neovim_path).open(encoding="utf-8") as file:
        neovim = json.load(file)
    found = faults(language, layers, source, neovim)
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

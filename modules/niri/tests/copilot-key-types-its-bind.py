"""Whether the Copilot key types, in the keymap niri builds, the key its dictation bind names.

The key sends Super, Shift and F23, and niri matches a bind by the keysym a key types with no
modifiers held, in the keymap libxkbcommon builds from niri's xkb settings and xkeyboard-config.
A new xkeyboard-config can change that keysym, as 2.49 did from F23 to XF86TouchpadOff, and the
bind then matches nothing: the key stops starting dictation, and only a desktop VM test saw it.
This reads the files niri reads and builds the keymap with the libxkbcommon niri links, in
seconds.

usage: copilot-key-types-its-bind.py <libxkbcommon.so> <niri's configuration folder>
"""

import ctypes
import re
import sys
from pathlib import Path

# The kernel's KEY_F23, which the Copilot key sends, as xkb numbers keys: eight past.
F23 = 193 + 8
# The fields of xkb_rule_names, in its order, as niri's xkb block names them.
FIELDS = ("rules", "model", "layout", "variant", "options")
# xkb_keysym_from_name's flag for matching a name whatever its case, as niri reads a bind's key.
CASE_INSENSITIVE = 1


class RuleNames(ctypes.Structure):
    """libxkbcommon's xkb_rule_names: what a keymap is built from, each field unset when None."""

    _fields_ = [(field, ctypes.c_char_p) for field in FIELDS]


def settings(folder: Path) -> dict[str, str]:
    """Read niri's xkb settings, from whichever of its files holds the xkb block."""
    for kdl in sorted(folder.glob("*.kdl")):
        block = re.search(r"\bxkb \{(.*?)\}", kdl.read_text(), re.DOTALL)
        if block:
            return dict(re.findall(r'^\s*(\w+) "([^"]*)"', block.group(1), re.MULTILINE))
    return {}


def bound_key(folder: Path) -> str:
    """Find the key of the chord under Shift that holds dictation, as the Copilot key's is."""
    binds = (folder / "binds.kdl").read_text()
    for line in re.finditer(r'^\s*(\S+) .*\{ spawn "dictation" "hold"; \}', binds, re.MULTILINE):
        chord = line.group(1)
        if "Shift+" in chord:
            return chord.rsplit("+", 1)[1]
    sys.exit("No chord under Shift runs dictation hold, so the Copilot key starts nothing.")


def library(path: str) -> ctypes.CDLL:
    """Open libxkbcommon, with the types of the calls this makes."""
    xkb = ctypes.CDLL(path)
    xkb.xkb_context_new.argtypes = [ctypes.c_int]
    xkb.xkb_context_new.restype = ctypes.c_void_p
    xkb.xkb_keymap_new_from_names.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(RuleNames),
        ctypes.c_int,
    ]
    xkb.xkb_keymap_new_from_names.restype = ctypes.c_void_p
    xkb.xkb_state_new.argtypes = [ctypes.c_void_p]
    xkb.xkb_state_new.restype = ctypes.c_void_p
    xkb.xkb_state_key_get_one_sym.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    xkb.xkb_state_key_get_one_sym.restype = ctypes.c_uint32
    xkb.xkb_keysym_get_name.argtypes = [ctypes.c_uint32, ctypes.c_char_p, ctypes.c_size_t]
    xkb.xkb_keysym_from_name.argtypes = [ctypes.c_char_p, ctypes.c_int]
    xkb.xkb_keysym_from_name.restype = ctypes.c_uint32
    return xkb


def typed(xkb: ctypes.CDLL, chosen: dict[str, str]) -> int:
    """Say the keysym F23 types with nothing held, in the keymap niri's settings build."""
    names = RuleNames(*(chosen.get(field, "").encode() or None for field in FIELDS))
    keymap = xkb.xkb_keymap_new_from_names(xkb.xkb_context_new(0), ctypes.byref(names), 0)
    if not keymap:
        sys.exit(f"libxkbcommon built no keymap from niri's xkb settings, {chosen}.")
    return int(xkb.xkb_state_key_get_one_sym(xkb.xkb_state_new(keymap), F23))


def main() -> None:
    """Fail, saying both keysyms, unless the key types what its bind names."""
    path, configuration = sys.argv[1:]
    folder = Path(configuration)
    xkb = library(path)
    key = bound_key(folder)
    wanted = int(xkb.xkb_keysym_from_name(key.encode(), CASE_INSENSITIVE))
    sym = typed(xkb, settings(folder))
    name = ctypes.create_string_buffer(64)
    xkb.xkb_keysym_get_name(sym, name, len(name))
    if sym != wanted:
        sys.exit(
            f"The Copilot key types {name.value.decode()}, but its bind names {key}, "
            "so holding it starts no dictation."
        )
    print(f"the Copilot key types {name.value.decode()}, as its bind names")


if __name__ == "__main__":
    main()

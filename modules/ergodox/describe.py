"""The board described for the shell: each layer by its name, and on it each
key where it sits, what its cap says, what it does in words and what holding
it does. The shell draws the overlay and finds keys in the launcher from
this (modules/quickshell/Keyboard.qml).

    describe.py LAYOUT INFO NAME... > shell.json

LAYOUT is `qmk c2json`'s reading of keymap.c, INFO the board's info.json,
which places LAYOUT_ergodox_pretty's keys, and the names are the layers' in
keymap.c's order. A keycode with no words here stops the build, so a key
added to the layout is never drawn blank or found by its QMK name.
"""

import json
import re
import sys

# Each keycode as its cap says it, five characters at most, and in words, for
# finding it. A key that types a character says it on its cap.
KEYS = {
    "KC_ESCAPE": ("Esc", "escape"),
    "KC_TAB": ("Tab", "tab"),
    "KC_ENTER": ("Ret", "return, enter"),
    "KC_SPACE": ("Space", "space"),
    "KC_BSPC": ("Bksp", "backspace"),
    "KC_DELETE": ("Del", "delete"),
    "KC_INSERT": ("Ins", "insert"),
    "KC_HOME": ("Home", "home"),
    "KC_END": ("End", "end"),
    "KC_PAGE_UP": ("PgUp", "page up"),
    "KC_PGDN": ("PgDn", "page down"),
    "KC_UP": ("\u2191", "up arrow"),
    "KC_DOWN": ("\u2193", "down arrow"),
    "KC_LEFT": ("\u2190", "left arrow"),
    "KC_RIGHT": ("\u2192", "right arrow"),
    "KC_LEFT_SHIFT": ("Shift", "shift"),
    "KC_RIGHT_SHIFT": ("Shift", "shift"),
    "KC_LEFT_GUI": ("Super", "super"),
    "KC_RIGHT_GUI": ("Super", "super"),
    "KC_MEH": ("Meh", "meh, ctrl alt shift held together"),
    "KC_APPLICATION": ("Menu", "menu key"),
    "CW_TOGG": ("CapsW", "caps word, the next word in capitals"),
    "KC_AUDIO_VOL_DOWN": ("Vol-", "volume down"),
    "KC_AUDIO_VOL_UP": ("Vol+", "volume up"),
    "KC_MEDIA_PREV_TRACK": ("Prev", "previous track"),
    "KC_MEDIA_PLAY_PAUSE": ("Play", "play or pause"),
    "KC_MEDIA_NEXT_TRACK": ("Next", "next track"),
    "KC_BRIGHTNESS_DOWN": ("Dim", "screen dimmer"),
    "KC_BRIGHTNESS_UP": ("Brite", "screen brighter"),
    "KC_PSCR": ("PrtSc", "print screen, a picture of a region"),
    "KC_SYSTEM_POWER": ("Power", "power, sleeps the machine"),
    "MAC_LOCK": ("Lock", "lock the screen"),
    "MAC_DND": ("DND", "do not disturb"),
    "QK_LLCK": ("LLock", "layer lock, keeps the layer on"),
    "QK_BOOT": ("Boot", "bootloader, restarts the board to be flashed"),
    "RGB_SPD": ("Spd-", "lights slower"),
    "RGB_SPI": ("Spd+", "lights faster"),
    "RGB_MODE_FORWARD": ("Mode", "lights' next pattern"),
    "RGB_HUD": ("Hue-", "lights' colour back"),
    "RGB_HUI": ("Hue+", "lights' colour on"),
    "RGB_SAD": ("Sat-", "lights paler"),
    "RGB_SAI": ("Sat+", "lights deeper"),
    "KC_KP_ENTER": ("Ent", "keypad enter"),
    "KC_PLUS": ("+", "plus"),
    "KC_PIPE": ("|", "pipe, vertical bar"),
    "KC_LBRC": ("[", "left bracket, open square bracket"),
    "KC_RBRC": ("]", "right bracket, close square bracket"),
    "KC_LCBR": ("{", "left brace, open curly brace"),
    "KC_RCBR": ("}", "right brace, close curly brace"),
    "KC_LPRN": ("(", "left parenthesis, open paren"),
    "KC_RPRN": (")", "right parenthesis, close paren"),
    "KC_LABK": ("<", "less than, left angle bracket"),
    "KC_RABK": (">", "greater than, right angle bracket"),
    "KC_CIRC": ("^", "caret, circumflex"),
    "KC_EXLM": ("!", "exclamation mark, bang"),
    "KC_QUES": ("?", "question mark"),
    "KC_DLR": ("$", "dollar sign"),
    "KC_EQUAL": ("=", "equals"),
    "KC_ASTR": ("*", "asterisk, star"),
    "KC_QUOTE": ("'", "apostrophe, single quote"),
    "KC_DQUO": ('"', "double quote"),
    "KC_GRAVE": ("`", "backtick, grave"),
    "KC_TILD": ("~", "tilde"),
    "KC_BSLS": ("\\", "backslash"),
    "KC_SLASH": ("/", "slash"),
    "KC_AT": ("@", "at sign"),
    "KC_COMMA": (",", "comma"),
    "KC_DOT": (".", "full stop, period, dot"),
    "KC_MINUS": ("-", "minus, hyphen, dash"),
    "KC_UNDS": ("_", "underscore"),
    "KC_HASH": ("#", "hash, number sign"),
    "KC_PERC": ("%", "percent"),
    "KC_AMPR": ("&", "ampersand, and"),
    "KC_SCLN": (";", "semicolon"),
    "KC_COLN": (":", "colon"),
    "KC_KP_PLUS": ("+", "keypad plus"),
    "KC_KP_MINUS": ("-", "keypad minus"),
    "KC_KP_ASTERISK": ("*", "keypad times"),
    "KC_KP_SLASH": ("/", "keypad divide"),
    "KC_KP_DOT": (".", "keypad point"),
}
for n in range(1, 25):
    KEYS[f"KC_F{n}"] = (f"F{n}", f"f{n}, function key {n}")
for n in range(10):
    KEYS[f"KC_KP_{n}"] = (str(n), f"keypad {n}")
for letter in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
    KEYS[f"KC_{letter}"] = (letter, letter.lower())

# What holding a mod-tap's modifier does.
MODIFIERS = {"LCTL": "Ctrl", "RCTL": "Ctrl", "LALT": "Alt", "RALT": "Alt",
             "LGUI": "Super", "RGUI": "Super", "LSFT": "Shift", "RSFT": "Shift"}

# Where a key is, by its place in LAYOUT_ergodox_pretty: the rows, top to
# bottom, and the fingers by column, outermost first, which the right hand
# mirrors.
ROWS = [(0, "top row"), (14, "upper row"), (28, "home row"), (40, "lower row"),
        (54, "bottom row"), (64, "thumb")]
FINGERS = ["outer column", "little finger", "ring finger", "middle finger",
           "index finger", "index finger, reaching in", "inner column"]
THUMBS = {
    64: "the small key above the long ones", 67: "the small key above the long ones",
    65: "the top small key", 66: "the top small key",
    68: "the middle small key", 69: "the middle small key",
    72: "the bottom small key", 73: "the bottom small key",
    70: "the outer long key", 75: "the outer long key",
    71: "the inner long key", 74: "the inner long key",
}


def where(index, key):
    hand = "left hand" if key["x"] < 8 else "right hand"
    row = [name for start, name in ROWS if index >= start][-1]
    if row == "thumb":
        return f"{hand} thumb, {THUMBS[index]}"
    # Columns stand a key's width apart from the outer one, which is 1.5 wide.
    column = key["x"] if key["x"] < 8 else 17 - key["x"] - key.get("w", 1)
    finger = FINGERS[min(len(FINGERS) - 1, max(0, round(column - 0.5)))]
    return f"{hand}, {row}, {finger}"


def describe(code):
    """The cap, the words and what holding it does, for one keycode."""
    tap_hold = re.fullmatch(r"(MT|LT)\((\w+),\s*(\w+)\)", code)
    if tap_hold:
        kind, hold, tap = tap_hold.groups()
        cap, words = KEYS[tap]
        held = MODIFIERS[hold.removeprefix("MOD_")] if kind == "MT" else int(hold)
        return cap, words, held
    if code not in KEYS:
        sys.exit(f"describe.py has no words for {code}; add it to KEYS.")
    cap, words = KEYS[code]
    return cap, words, None


def main():
    layout_file, info_file, *names = sys.argv[1:]
    layers = json.load(open(layout_file))["layers"]
    places = json.load(open(info_file))["layouts"]["LAYOUT_ergodox_pretty"]["layout"]
    if len(names) != len(layers):
        sys.exit(f"{len(names)} layer names for {len(layers)} layers.")
    described = []
    for number, (name, codes) in enumerate(zip(names, layers)):
        keys = []
        for index, (code, place) in enumerate(zip(codes, places)):
            key = {"x": place["x"], "y": place["y"], "w": place.get("w", 1),
                   "h": place.get("h", 1), "where": where(index, place)}
            if code == "KC_TRANSPARENT":
                key["falls"] = True
            elif code != "KC_NO":
                cap, words, held = describe(code)
                key.update(cap=cap, words=words)
                if isinstance(held, int):
                    key["holds"] = names[held]
                elif held:
                    key["holds"] = held
            keys.append(key)
        described.append({"name": name, "keys": keys})
    json.dump({"layers": described}, sys.stdout, ensure_ascii=False, indent=1)


main()

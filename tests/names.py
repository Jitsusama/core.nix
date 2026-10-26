"""Check that core's names say the glossary's words, not the names it replaces.

    names GLOSSARY NAMES

GLOSSARY is docs/glossary.md, read as glossary.py reads it. NAMES is a JSON
list of names: every check's, and every jitsusama.* option's. A name is split
into words at hyphens, dots and capitals, and a name it says that the
glossary replaces fails, printed with the word that replaces it, unless one
of the glossary's terms holds it there ("line" inside "status line"), the
glossary's In code spells that word on its own as code's ("the `backlight`
program"), or the name is listed below: as another sense of the word, or as a
rename its owner hasn't made yet. An option is listed by its path or a part
of it. A listed name the check no longer finds fails too, so the lists only
shrink.
And so does the check itself, should one of its rules stop refusing what it's
there for, or refuse what sits beside it.
"""

import json
import re
import sys
from pathlib import Path

from glossary import Thing, read

# Names whose word means something the glossary doesn't replace, often a
# program's own word for its own thing.
OTHER_SENSE = {
    "a-quiet-file-says-nothing": "quiet is FileView printing no errors, not Do not disturb",
    "bluetooth-keeps-its-power-limits": "the limits are a radio's SAR table, not an allowance",
    "jitsusama.kernel.cpu": "the CPU Clang's -march and -mtune name, in Clang's word",
    "jitsusama.kernel.packageOverlays": "an overlay is Nix's, not a meter",
    "jitsusama.theme.icons": "the icon theme is GTK's, named as it names it",
    "jitsusama.theme.palette": "a theme's palette is its colours, not the launcher",
    "jitsusama.theme.syntax.operator": "an operator is a code token, such as =, not a person",
    "platform-profiles-reach-the-legacy-file": "the kernel's platform_profile file, by its name",
    "quickshell-restarts-with-its-files": "a program restarts; the machine isn't rebooted",
    "the-panel-slows-on-a-low-battery": "the panel's refresh rate slows, not lag",
    "the-splash-keeps-systemd-quiet": "quiet is the kernel's quiet argument, not Do not disturb",
    "vm-apps-come-first": "app is systemd's app.slice, by its name",
}

# Names waiting on a rename, each with the quest that owns it.
WAITING = {
    "icon-marks-past-the-basic-plane": "Launcher (Y2YBTS): icon to mark",
    "the-margins-glyphs-stand-level": "Launcher (Y2YBTS): glyph to mark",
    "jitsusama.theme.glyphs": "Look and programs (QGB4ON): glyph to mark",
    "only-external-monitors-reach-the-seat": "Repos (Y9XISP): monitor to screen",
    "palette-graphs-draw-in-braille": "Launcher (Y2YBTS): palette to launcher",
    "palette-rows-keep-their-words-apart": "Launcher (Y2YBTS): palette to launcher",
    "jitsusama.quickshell.quiet.whileSharing": "Launcher (Y2YBTS): quiet to Do not disturb",
    "jitsusama.dellXps14.cameraModule": "Repos (Y9XISP): camera module to camera board",
    "jitsusama.spend-watch.detectors.quiet": "Repos (Y9XISP): quiet, for notes only",
    "jitsusama.spend-watch.notices.budget": "Repos (Y9XISP): budget, for notices a week",
    "machines-yield-to-what-the-user-works-in": "Repos (Y9XISP): user to person",
    "nix-builds-yield-to-the-user": "Repos (Y9XISP): user to person",
    "work-yields-to-what-the-user-works-in": "Repos (Y9XISP): user to person",
    "vm-the-line-stays-quick": "Repos (Y9XISP): line to connection",
    "xdg-open-opens-each-kind-in-its-app": "Look and programs (QGB4ON): app to program",
}


def tokens(text: str) -> list[str]:
    """Split a name or a term into its words, in lower case."""
    spaced = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", text)
    return [word for word in re.split(r"[^A-Za-z0-9']+", spaced.lower()) if word]


def same(words: list[str], old: list[str]) -> bool:
    """Say whether words say old, a plural counting as its word."""
    return len(words) == len(old) and all(
        word in (was, was + "s", was + "es") for word, was in zip(words, old, strict=True)
    )


def held(words: list[str], start: int, length: int, terms: list[list[str]]) -> bool:
    """Say whether a longer term covers the words from start, as status line covers line."""
    for term in terms:
        if len(term) <= length:
            continue
        first = max(0, start + length - len(term))
        for at in range(first, start + 1):
            if same(words[at : at + len(term)], term):
                return True
    return False


def spelled(things: list[Thing]) -> list[list[str]]:
    """Give the words of each span the glossary's In code puts in backticks."""
    return [tokens(span) for thing in things for span in re.findall(r"`([^`]+)`", thing.code)]


def listed(name: str, names: set[str]) -> bool:
    """Say whether a name is among names, or an option below one of them."""
    return any(name == entry or name.startswith(entry + ".") for entry in names)


def replaced(name: str, things: list[Thing]) -> list[str]:
    """Say each name the glossary replaces that a name says, with the word that replaces it."""
    words = tokens(name)
    terms = [tokens(thing.word) for thing in things]
    code = spelled(things)
    out: list[str] = []
    for thing in things:
        for old in thing.not_:
            old_words = tokens(old)
            if any(same(span, old_words) for span in code):
                continue
            for start in range(len(words) - len(old_words) + 1):
                if not same(words[start : start + len(old_words)], old_words):
                    continue
                if not held(words, start, len(old_words), terms):
                    out.append(f'"{name}" says "{old}", which the glossary calls "{thing.word}".')
    return out


def drift(things: list[Thing], names: list[str]) -> list[str]:
    """Say each name that says a replaced name, and each listed name no longer found."""
    kept = set(OTHER_SENSE) | set(WAITING)
    out: list[str] = []
    for name in sorted(set(names)):
        if not listed(name, kept):
            out.extend(replaced(name, things))
    out.extend(
        f'"{entry}" is listed in tests/names.py but no longer found: take it off.'
        for entry in sorted(kept)
        if not any(listed(name, {entry}) for name in names)
    )
    return out


GLOSSARY = (
    "## Network\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| connection | the link out | line | `jitsusama.wan.lineRate` |\n"
    "| status line | the line under a session | status bar | |\n"
    "| person | the human | user | |\n"
    "| light | the keyboard's light | backlight | |\n"
    "| brightness | how bright a screen is | | the `backlight` program |\n"
)

# A stand-in name for each rule, breaking it once, beside the listed names
# so that the lists aren't what fails.
LISTED = sorted(set(OTHER_SENSE) | set(WAITING))
REFUSED: list[tuple[str, list[str]]] = [
    ("a replaced name in a check's name", ["the-line-stays-quick", *LISTED]),
    ("a replaced name made plural", ["the-users-sign-in", *LISTED]),
    ("a replaced name in an option's name", ["jitsusama.shaper.userRate", *LISTED]),
    ("a replaced name in a name the glossary spells whole", ["jitsusama.wan.lineRate", *LISTED]),
    ("a listed name no longer found", LISTED[1:]),
]

# What sits beside each rule's edge and isn't refused: a replaced name inside
# a term, a word the glossary spells on its own in code, a word inside
# another, and an option below a listed one.
ALLOWED = [
    "the-status-line-says-its-quest",
    "backlight-passes-clippy",
    "userspace-keeps-its-homes",
    "jitsusama.theme.syntax.operator.bold",
    *LISTED,
]


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    things, _ = read(GLOSSARY)
    out = [f"The check lets {why} through." for why, names in REFUSED if not drift(things, names)]
    out.extend(f"The check refuses what it shouldn't: {said}" for said in drift(things, ALLOWED))
    return out


def main(argv: list[str]) -> None:
    """Print every name that says a replaced name, and fail if there are any."""
    things, _ = read(Path(argv[0]).read_text())
    found = refuses() + drift(things, json.loads(Path(argv[1]).read_text()))
    for said in found:
        print(said)
    if found:
        sys.exit(f"{len(found)} names say what the glossary replaces.")


if __name__ == "__main__":
    main(sys.argv[1:])

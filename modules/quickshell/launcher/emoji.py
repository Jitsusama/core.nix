"""Build the launcher's emoji from Unicode's list and CLDR's English keywords.

It prints one line of JSON for each emoji, [emoji, name, keywords, group],
in Unicode's order. Skin tones are folded into the emoji they tone, which
takes the chosen tone when it has one, so a search finds each emoji once.

    python3 emoji.py emoji-test.txt annotations.xml derived.xml tone > out
"""

import json
import re
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

TONES = {
    "light": "\U0001f3fb",
    "medium-light": "\U0001f3fc",
    "medium": "\U0001f3fd",
    "medium-dark": "\U0001f3fe",
    "dark": "\U0001f3ff",
}
TONING = set(TONES.values())

# A fully-qualified emoji's line in emoji-test.txt: its code points, then
# after the # the emoji, the version it came in, and its name.
LINE = re.compile(r"^([0-9A-F ]+?)\s*;\s*fully-qualified\s*#\s*(\S+)\s+E[\d.]+\s+(.+)$")


def bare(text: str) -> str:
    """Drop the variation selector, which asks for an emoji's colour form."""
    return text.replace("\ufe0f", "")


def keywords(*paths: str) -> dict[str, list[str]]:
    """Read CLDR's keywords for each emoji, from each annotations file."""
    found: dict[str, list[str]] = {}
    for path in paths:
        # CLDR's own files, from the Nix store, not anyone's input.
        for note in ET.parse(path).iter("annotation"):  # noqa: S314
            if note.get("type") != "tts" and note.text:
                words = [word.strip() for word in note.text.split("|")]
                found.setdefault(bare(note.get("cp", "")), []).extend(words)
    return found


def emoji(test: str, tone: str) -> tuple[list[tuple[str, str, str]], dict[str, str]]:
    """Read Unicode's list: each untoned emoji, its name and group, in order.

    Beside them, each emoji toned once by the chosen tone, by the bare emoji
    it stands in for.
    """
    group = ""
    order: list[tuple[str, str, str]] = []
    toned: dict[str, str] = {}
    for text in Path(test).read_text(encoding="utf-8").splitlines():
        if text.startswith("# group:"):
            group = text.split(":", 1)[1].strip()
            continue
        found = LINE.match(text)
        if not found:
            continue
        shown, name = found.group(2), found.group(3)
        marks = [mark for mark in shown if mark in TONING]
        if marks:
            if tone in TONES and marks == [TONES[tone]]:
                untoned = "".join(mark for mark in shown if mark not in TONING)
                toned.setdefault(bare(untoned), shown)
            continue
        order.append((shown, name, group))
    return order, toned


def main(argv: list[str]) -> None:
    """Print each emoji's line, toned as asked."""
    test, annotations, derived, tone = argv[:4]
    words = keywords(annotations, derived)
    order, toned = emoji(test, tone)
    for shown, name, group in order:
        found = [word for word in dict.fromkeys(words.get(bare(shown), [])) if word != name]
        line = [toned.get(bare(shown), shown), name, " ".join(found), group]
        print(json.dumps(line, ensure_ascii=False))


if __name__ == "__main__":
    main(sys.argv[1:])

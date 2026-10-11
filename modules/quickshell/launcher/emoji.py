# Builds the launcher's emoji from Unicode's list and CLDR's English
# keywords: one line of JSON for each emoji, [emoji, name, keywords, group],
# in Unicode's order. Skin tones are folded into the emoji they tone, which
# takes the chosen tone when it has one, so a search finds each emoji once.
#
#     python3 emoji.py emoji-test.txt annotations.xml derived.xml tone > out
import json
import re
import sys
import xml.etree.ElementTree as ElementTree

test, annotations, derived, tone = sys.argv[1:5]
tones = {
    "light": "\U0001f3fb",
    "medium-light": "\U0001f3fc",
    "medium": "\U0001f3fd",
    "medium-dark": "\U0001f3fe",
    "dark": "\U0001f3ff",
}
toning = set(tones.values())


def bare(text):
    return text.replace("\ufe0f", "")


keywords = {}
for path in (annotations, derived):
    for note in ElementTree.parse(path).iter("annotation"):
        if note.get("type") != "tts" and note.text:
            words = [w.strip() for w in note.text.split("|")]
            keywords.setdefault(bare(note.get("cp")), []).extend(words)

line = re.compile(r"^([0-9A-F ]+?)\s*;\s*fully-qualified\s*#\s*(\S+)\s+E[\d.]+\s+(.+)$")
group = ""
order = []
toned = {}
for text in open(test, encoding="utf-8"):
    if text.startswith("# group:"):
        group = text.split(":", 1)[1].strip()
        continue
    found = line.match(text)
    if not found:
        continue
    emoji, name = found.group(2), found.group(3)
    marks = [c for c in emoji if c in toning]
    if marks:
        # Only an emoji toned once, by the chosen tone, stands in for it.
        if tone in tones and marks == [tones[tone]]:
            toned.setdefault(bare("".join(c for c in emoji if c not in toning)), emoji)
        continue
    order.append((emoji, name, group))

for emoji, name, group in order:
    words = [w for w in dict.fromkeys(keywords.get(bare(emoji), [])) if w != name]
    shown = toned.get(bare(emoji), emoji)
    print(json.dumps([shown, name, " ".join(words), group], ensure_ascii=False))

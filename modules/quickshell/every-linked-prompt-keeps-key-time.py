"""Hold each prompt a link opens to key time (every-linked-prompt-keeps-key-time).

Reads every QML file under the folder given, tests aside, and fails, naming
each, on a Prompt whose linked() its file calls but whose own block sets no
keyNow.

A link's quiet (Prompt.qml's linked) drops every key, Escape among them,
until a time read on keyNow's clock, which is the wall clock unless the
prompt is given one. Stepped back, a wall clock would hold that quiet for as
long as it went back, so a prompt a link opens is given KeyTimes' monotonic
clock, as the launcher's and Super+S's line are.

Only the Prompt's own block counts: a keyNow set on a child inside it times
that child, not the prompt. Braces are counted with strings and trailing
comments left out, as neither opens nor ends a block.

Usage: every-linked-prompt-keeps-key-time.py ROOT
"""

import re
import sys
from pathlib import Path

# A Prompt made where it stands, its block opening; PolkitPrompt is another type.
PROMPT = re.compile(r"(?<!\w)Prompt\s*\{")
# A string of any of QML's three quotes, which may hold a brace or a //.
STRING = re.compile(r"""(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`(?:[^`\\]|\\.)*`)""")
# The id a block gives itself, and its keyNow, each set at the block's own level.
ID = re.compile(r"(?<![\w.])id\s*:\s*([A-Za-z_]\w*)")
KEY_NOW = re.compile(r"(?<![\w.])keyNow\s*:")


def code(line: str) -> str:
    """Give the line with its strings emptied and no trailing comment, as neither sets a block."""
    return re.sub(r"(^|\s)//.*$", r"\1", STRING.sub('""', line))


def own(text: str, start: int) -> str:
    """Give what the block opened just before start sets at its own level, children left out.

    Every newline is kept, so what two lines set never runs together.
    """
    depth = 1
    kept: list[str] = []
    for char in text[start:]:
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                break
        elif depth == 1 or char == "\n":
            kept.append(char)
    return "".join(kept)


def unkept(path: Path, name: str) -> list[str]:
    """Find each Prompt a file links but gives no keyNow of its own, saying where."""
    lines = path.read_text(encoding="utf-8").splitlines()
    text = "\n".join(code(line) for line in lines)
    found: list[str] = []
    for prompt in PROMPT.finditer(text):
        block = own(text, prompt.end())
        named = ID.search(block)
        if named is None or KEY_NOW.search(block):
            continue
        given = named.group(1)
        if re.search(rf"(?<![\w.]){re.escape(given)}\.linked\(", text):
            line = text.count("\n", 0, prompt.start()) + 1
            found.append(f"{name}:{line}: Prompt {given} is linked but given no keyNow")
    return found


def main(root: Path) -> int:
    """Check every file, printing each Prompt unkept, and answer 1 if any is or none was read."""
    read = 0
    stray: list[str] = []
    for path in sorted(root.rglob("*.qml")):
        relative = path.relative_to(root)
        if "tests" in relative.parts:
            continue
        read += 1
        stray.extend(unkept(path, relative.as_posix()))
    if read == 0:
        print(f"{root} holds no QML outside its tests, so no Prompt in it was checked.")
        return 1
    for line in stray:
        print(line)
    if stray:
        print(
            "Give each Prompt above keyNow: () => KeyTimes.now(), the launcher's monotonic "
            "clock, as its link's quiet is timed on it (Prompt.qml's keyNow)."
        )
    return 1 if stray else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))

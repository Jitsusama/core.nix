"""Build the launcher's character names from Python's own copy of Unicode's.

It prints Unicode's version, then a line for each run of characters named
by their code point ("cjk unified ideograph-4e00"), as `range`, the first
and last code points and the name they share before it, then a line for
each other named character: its code point, its name in lower case and,
for one drawn on nothing or not drawn at all, its general category. The
controls, which Unicode names only by their aliases, take those, and the
Hangul syllables are left for the launcher to name by their parts.

    python3 names.py > out
"""

import sys
import unicodedata

# The controls' names, from Unicode's NameAliases.txt, which Python's
# unicodedata reads aliases from but never names a character by.
CONTROLS = [
    "null", "start of heading", "start of text", "end of text", "end of transmission",
    "enquiry", "acknowledge", "alert", "backspace", "character tabulation", "line feed",
    "line tabulation", "form feed", "carriage return", "shift out", "shift in",
    "data link escape", "device control one", "device control two", "device control three",
    "device control four", "negative acknowledge", "synchronous idle",
    "end of transmission block", "cancel", "end of medium", "substitute", "escape",
    "information separator four", "information separator three",
    "information separator two", "information separator one",
]  # fmt: skip
DELETE = 0x7F
C1 = [
    "padding character", "high octet preset", "break permitted here", "no break here",
    "index", "next line", "start of selected area", "end of selected area",
    "character tabulation set", "character tabulation with justification",
    "line tabulation set", "partial line forward", "partial line backward",
    "reverse line feed", "single shift two", "single shift three", "device control string",
    "private use one", "private use two", "set transmit state", "cancel character",
    "message waiting", "start of guarded area", "end of guarded area", "start of string",
    "single graphic character introducer", "single character introducer",
    "control sequence introducer", "string terminator", "operating system command",
    "privacy message", "application program command",
]  # fmt: skip
C1_START = 0x80
# The Hangul syllables, named by their parts rather than listed.
HANGUL = range(0xAC00, 0xD7A4)
# The categories of a character drawn on nothing (a combining mark) or not
# drawn at all (a space, a separator, a format or a control).
UNSEEN = {"Mn", "Me", "Zs", "Zl", "Zp", "Cf", "Cc"}
LAST = 0x10FFFF


def named(code: int) -> str:
    """Say a character's name, an alias for a control, or "" for none."""
    if code < len(CONTROLS):
        return CONTROLS[code]
    if code == DELETE:
        return "delete"
    if C1_START <= code < C1_START + len(C1):
        return C1[code - C1_START]
    return unicodedata.name(chr(code), "").lower()


def main() -> None:
    """Print the version, the runs named by code point, then every other name."""
    runs: list[tuple[int, int, str]] = []
    lines: list[str] = []
    for code in range(LAST + 1):
        name = named(code)
        if not name or code in HANGUL:
            continue
        written = f"-{code:04x}"
        if name.endswith(written):
            shared = name[: -len(written) + 1]
            if runs and runs[-1][1] == code - 1 and runs[-1][2] == shared:
                runs[-1] = (runs[-1][0], code, shared)
            else:
                runs.append((code, code, shared))
            continue
        category = unicodedata.category(chr(code))
        unseen = f"\t{category}" if category in UNSEEN else ""
        lines.append(f"{code:x}\t{name}{unseen}")
    print(f"unicode\t{unicodedata.unidata_version}")
    for first, last, shared in runs:
        print(f"range\t{first:x}\t{last:x}\t{shared}")
    sys.stdout.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()

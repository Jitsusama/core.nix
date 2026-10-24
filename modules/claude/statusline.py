"""Claude Code's status line: the folder, the model and how full the context is.

Claude Code runs this after each turn with its session as JSON on standard input, and shows
the line it prints. home.nix names it in Claude Code's settings, and puts a JSON file of the
theme's colours, by what each part of the line is, in place of the marker below.
"""

import json
import sys
from pathlib import Path
from typing import Any

# Each part's colour, as #rrggbb: folder, model, context and separator.
COLOURS: dict[str, str] = json.loads(Path("@colours@").read_text())

# Marks from the icon font, which the theme's monospaced face carries.
FOLDER = "\uf07b"
MODEL = "\uf2db"
CONTEXT = "\uf019"
SEPARATOR = "\ue0b1"

# Tokens in a thousand, and the count from which a whole number of them reads
# plainly enough.
THOUSAND = 1000
PLAIN_FROM = 10


def paint(text: str, colour: str, *, bold: bool = False) -> str:
    """Give the text in a #rrggbb colour, as a terminal draws it, bold if asked."""
    red, green, blue = (int(colour[start : start + 2], 16) for start in (1, 3, 5))
    weight = "1;" if bold else ""
    return f"\033[{weight}38;2;{red};{green};{blue}m{text}\033[0m"


def thousands(tokens: int) -> str:
    """Give a count of tokens in thousands, to a tenth below ten thousand."""
    if tokens <= 0:
        return "0"
    count = tokens / THOUSAND
    return f"{count:.0f}k" if count >= PLAIN_FROM else f"{count:.1f}k"


def context_tokens(transcript: Path) -> int | None:
    """Give how many tokens the last answer read, cached ones too, or None if there's none.

    That's the context's size as it stands. A line of the transcript that isn't JSON is
    passed over, as a line still being written is.
    """
    try:
        lines = transcript.read_text().splitlines()
    except OSError:
        return None
    read: int | None = None
    for line in lines:
        try:
            entry: Any = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(entry, dict) or entry.get("type") != "assistant":
            continue
        usage = entry.get("message", {}).get("usage")
        if isinstance(usage, dict):
            read = sum(
                usage.get(kind) or 0
                for kind in (
                    "input_tokens",
                    "cache_creation_input_tokens",
                    "cache_read_input_tokens",
                )
            )
    return read


def status(session: dict[str, Any]) -> str:
    """Give the status line for Claude Code's session."""
    folder = Path(session.get("workspace", {}).get("current_dir") or "unknown").name
    model = session.get("model", {})
    named = model.get("display_name") or model.get("id") or "unknown"
    parts = [
        paint(f"{FOLDER} {folder}", COLOURS["folder"], bold=True),
        paint(f"{MODEL} {named}", COLOURS["model"]),
    ]
    transcript = session.get("transcript_path")
    read = context_tokens(Path(transcript)) if transcript else None
    if read is not None:
        parts.append(paint(f"{CONTEXT} {thousands(read)}", COLOURS["context"]))
    return f" {paint(SEPARATOR, COLOURS['separator'])} ".join(parts)


def main() -> None:
    """Print the status line for the session Claude Code gives on standard input."""
    print(status(json.load(sys.stdin)))


if __name__ == "__main__":
    main()

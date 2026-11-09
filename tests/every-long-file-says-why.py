"""Check that every long code file and crowded folder says why, or waits.

    every-long-file-says-why TREE WAITING

A code file past 800 lines is usually several things that grew together, and
a folder of more than 40 entries is one a reader can't scan for the thing they
want. Either is split by what changes for one reason, or says why it stays
whole: a code file in its opening comment, in a sentence beginning "Long
because", and a folder in its README.md or AGENTS.md, in one beginning "Many
entries because". TREE is the repository's tree. WAITING lists, a path a line,
the files and folders that were past their limit before the rule, while
they're split; one that's split, or says why, or is gone, fails until it's
taken off, so the list only shrinks. Every failure is printed before the check
exits 1.
"""

import re
import sys
from pathlib import Path

MOST_LINES = 800
MOST_ENTRIES = 40

# What counts as code: what a module runs, configures as a program, or tests.
CODE = {
    ".bt",
    ".c",
    ".cpp",
    ".h",
    ".js",
    ".lua",
    ".mjs",
    ".nix",
    ".py",
    ".qml",
    ".rs",
    ".sh",
    ".ts",
    ".zsh",
}


def line_count(text: str) -> int:
    """Count a file's lines, the newline ending the last not making another."""
    return text.count("\n") + (0 if text.endswith("\n") or not text else 1)


def opening_comment(text: str) -> str:
    """Give the comment a file opens with, as one line of words.

    A shebang, blank lines and QML's pragmas may come first. The comment is a
    run of line comments, a block comment or a docstring, and runs of each may
    follow one another.
    """
    lines = text.splitlines()
    at = 1 if lines and lines[0].startswith("#!") else 0
    said: list[str] = []
    while at < len(lines):
        line = lines[at].strip()
        closing = {'"""': '"""', "'''": "'''", "/*": "*/"}.get(
            line[:3] if line[:3] in ('"""', "'''") else line[:2]
        )
        if closing:
            # A block runs on to the line that closes it, which may be its first.
            end = at
            while closing not in (lines[end].strip()[3:] if end == at else lines[end]):
                end += 1
                if end == len(lines):
                    break
            said.extend(lines[at : end + 1])
            at = end + 1
        elif line.startswith(("#", "//", "--")):
            said.append(line)
            at += 1
        elif not said and (not line or line.startswith("pragma ")):
            at += 1
        else:
            break
    words = re.sub(
        r"(#|//[!/]?|--|/\*+|\*+/|^\s*\*|\"\"\"|''')", " ", "\n".join(said), flags=re.MULTILINE
    )
    return " ".join(words.split())


def says_why_long(text: str) -> bool:
    """Whether a code file's opening comment has a sentence beginning "Long because"."""
    return re.search(r"(^|[.:;!?] )Long because\b", opening_comment(text)) is not None


def says_why_many(folder: Path) -> bool:
    """Whether a folder's README.md or AGENTS.md has a sentence beginning "Many entries because"."""
    for note in ("README.md", "AGENTS.md"):
        path = folder / note
        if path.is_file() and re.search(
            r"\bMany entries because\b", " ".join(path.read_text().split())
        ):
            return True
    return False


def over(tree: Path) -> tuple[list[str], list[str], list[str]]:
    """Give the files and folders past their limit, and those that say why."""
    files: list[str] = []
    folders: list[str] = []
    reasoned: list[str] = []
    for folder in sorted([tree, *(p for p in tree.rglob("*") if p.is_dir())]):
        if ".git" in folder.relative_to(tree).parts:
            continue
        name = folder.relative_to(tree).as_posix()
        entries = [entry for entry in folder.iterdir() if entry.name != ".git"]
        if len(entries) > MOST_ENTRIES:
            (reasoned if says_why_many(folder) else folders).append(name)
        for entry in sorted(entries):
            if entry.is_file() and entry.suffix in CODE:
                text = entry.read_text(errors="replace")
                if line_count(text) > MOST_LINES:
                    path = entry.relative_to(tree).as_posix()
                    (reasoned if says_why_long(text) else files).append(path)
    return files, folders, reasoned


def main() -> None:
    """Say each file and folder past its limit without a reason, and each waiting one no longer."""
    try:
        tree, waiting_list = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-long-file-says-why TREE WAITING")
    waiting = {line.strip() for line in waiting_list.read_text().splitlines() if line.strip()}
    files, folders, reasoned = over(tree)
    said = [
        f"{path} is past {MOST_LINES} lines: split it by what changes for one reason, or say in "
        'its opening comment, in a sentence beginning "Long because", why it stays whole.'
        for path in files
        if path not in waiting
    ]
    said.extend(
        f"{folder} holds more than {MOST_ENTRIES} entries: group them in folders by what they're "
        'for, or say in its README.md, in a sentence beginning "Many entries because", why they '
        "stay together."
        for folder in folders
        if folder not in waiting
    )
    still = set(files) | set(folders)
    said.extend(
        f"{path} waits to be split, but it's split, says why or is gone: "
        "take it off the waiting list."
        for path in sorted(waiting - still)
    )
    for line in said:
        print(line, file=sys.stderr)
    print(
        f"{len(files) + len(folders)} past their limit without a reason, {len(reasoned)} with one."
    )
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()

"""Refuse a live page any script the sites' policy won't run.

The sites' server sends each page under its names with script-src 'self'
(src/http.rs), so a script written into a page, a handler written as an
attribute or a javascript: address does nothing there, and says so only in
the browser's console. A page built with one would pass every other check
and break only once Joel opened it, so localSite's build runs this over its
pages and fails on any, naming each by page and line.
"""

import re
import sys
from html.parser import HTMLParser
from pathlib import Path

# A script's types that hold data rather than run, which the policy leaves be.
DATA = frozenset({"application/json", "application/ld+json", "speculationrules", "text/plain"})
# The attributes an address in may be followed or sent to, and so run.
ADDRESSES = frozenset({"action", "formaction", "href", "src", "xlink:href"})
# What a browser drops from an address before reading its scheme.
DROPPED = re.compile(r"[\x00-\x20]")
# What a fault ends with: why, and what to do instead.
INSTEAD = "the sites run only scripts served beside a page (script-src 'self'): serve it as a file"


class Scripts(HTMLParser):
    """The scripts a page holds that the policy won't run, each by its line."""

    def __init__(self) -> None:
        """Start with none found."""
        super().__init__()
        self.found: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Note this element's script written in, handler or javascript: address."""
        line, _ = self.getpos()
        given = dict(attrs)
        kind = (given.get("type") or "").strip().lower()
        if tag == "script" and "src" not in given and kind not in DATA:
            self.found.append(f"{line}: a script written into the page")
        for name, value in attrs:
            if name.startswith("on"):
                self.found.append(f"{line}: {name}=, a handler written as an attribute")
            elif name in ADDRESSES and DROPPED.sub("", value or "").lower().startswith(
                "javascript:"
            ):
                self.found.append(f"{line}: a javascript: address in {name}=")

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Read a self-closed element as one opened."""
        self.handle_starttag(tag, attrs)


def faults(folder: Path) -> list[str]:
    """Say each script the pages at the top of `folder` hold that won't run."""
    said = []
    for page in sorted(folder.glob("*.html")):
        scripts = Scripts()
        scripts.feed(page.read_text())
        scripts.close()
        said += [f"{page.name}:{found}; {INSTEAD}" for found in scripts.found]
    return said


def main() -> int:
    """Read the pages of the folder given, and fail if any holds a script that won't run."""
    said = faults(Path(sys.argv[1]))
    for fault in said:
        print(fault, file=sys.stderr)
    return 1 if said else 0


if __name__ == "__main__":
    sys.exit(main())

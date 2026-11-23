"""Hold every live page to the kit's head, as sites.md writes it.

The seven pages read as one family only while each opens with the same head:
an eyebrow of two phrases, the title, a lede, the standing line and the quiet
row, in that order, inside one `<header class="page-head">` that holds the
page's first heading, styled by the kit's head.css. Only the standing line is
a live region, so a screen reader hears the state change and never a lede
being rewritten. This reads each page's markup as written, since every page
writes its head there; what a script later puts in it is the page's own tests'
to hold.

A page not yet moved to the head is named as waiting, and passes; the list
only shrinks, so a waiting page that keeps to the head now fails until it is
taken off. Run as `heads.py [--waiting Name,Name] Name=path ...`; it prints
what each page must change and fails when any must.
"""

import argparse
import sys
from html.parser import HTMLParser
from pathlib import Path

# The head's parts, in the order sites.md writes them, as the tag and class
# each is drawn with.
PARTS = [("p", "eyebrow"), ("h1", None), ("p", "lede"), ("p", "standing"), ("div", "quiet")]

# Elements that never hold others, so they never wait for an end tag.
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source"}

# The roles a screen reader speaks the changes of, beside aria-live.
LIVE_ROLES = {"status", "alert", "log", "marquee", "timer"}


class Element:
    """One element as written: its tag, its attributes, the elements in it."""

    def __init__(self, tag: str, attributes: dict[str, str], parent: Element | None) -> None:
        """Take the element's tag and attributes, under its parent."""
        self.tag = tag
        self.attributes = attributes
        self.parent = parent
        self.children: list[Element] = []

    def classes(self) -> list[str]:
        """Give the classes it's marked with."""
        return self.attributes.get("class", "").split()

    def is_part(self, tag: str, name: str | None) -> bool:
        """Say whether it's drawn as the part with this tag and class."""
        return self.tag == tag and (name is None or name in self.classes())

    def within(self) -> list[Element]:
        """Give every element inside it, in the order they're written."""
        found = []
        for child in self.children:
            found.append(child)
            found.extend(child.within())
        return found

    def live(self) -> bool:
        """Say whether a screen reader speaks its changes."""
        live = self.attributes.get("aria-live", "off") != "off"
        return live or self.attributes.get("role") in LIVE_ROLES

    def inside(self, ancestor: Element) -> bool:
        """Say whether it sits somewhere within `ancestor`."""
        parent = self.parent
        while parent is not None:
            if parent is ancestor:
                return True
            parent = parent.parent
        return False


class Reader(HTMLParser):
    """Read markup into elements, closing what a missing end tag leaves open."""

    def __init__(self) -> None:
        """Start from an empty document."""
        super().__init__(convert_charrefs=True)
        self.document = Element("#document", {}, None)
        self.open = self.document

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Open an element under the one open now."""
        element = Element(tag, {name: value or "" for name, value in attrs}, self.open)
        self.open.children.append(element)
        if tag not in VOID:
            self.open = element

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Add an element written closed, which holds nothing."""
        self.open.children.append(
            Element(tag, {name: value or "" for name, value in attrs}, self.open)
        )

    def handle_endtag(self, tag: str) -> None:
        """Close the nearest open element of this tag, and any left open in it."""
        element: Element | None = self.open
        while element is not None and element.tag != tag:
            element = element.parent
        if element is not None and element.parent is not None:
            self.open = element.parent


def read(markup: str) -> Element:
    """Give the document `markup` writes."""
    reader = Reader()
    reader.feed(markup)
    reader.close()
    return reader.document


def said(part: tuple[str, str | None]) -> str:
    """Name a part as its markup reads, for a person to write it."""
    tag, name = part
    return f"<{tag}>" if name is None else f'<{tag} class="{name}">'


def head_faults(markup: str) -> list[str]:
    """Say what `markup`'s head must change to be the kit's, or nothing."""
    everything = read(markup).within()
    heads = [element for element in everything if "page-head" in element.classes()]
    if len(heads) != 1:
        return [f'it draws {len(heads)} heads: open it with one <header class="page-head">']
    head = heads[0]
    faults = []
    if head.tag != "header":
        faults.append(f'its head is a <{head.tag}>: draw it as <header class="page-head">')
    titles = [element for element in everything if element.tag == "h1"]
    if not titles or not titles[0].inside(head):
        faults.append("its first <h1> sits outside its head: make it the head's title")
    drawn = [(child.tag, next(iter(child.classes()), None)) for child in head.children]
    parts = [
        next((part for part in PARTS if child.is_part(*part)), (child.tag, None))
        for child in head.children
    ]
    if parts != PARTS:
        shown = ", ".join(said(part) for part in drawn) or "nothing"
        wanted = ", ".join(said(part) for part in PARTS)
        faults.append(f"its head holds {shown}: hold {wanted}, in that order and nothing else")
    for child in head.children:
        faults.extend(part_faults(child))
    return faults + speaking_faults(head)


def part_faults(part: Element) -> list[str]:
    """Say what one part of the head must change within it, or nothing."""
    inner = part.within()
    if part.is_part("p", "eyebrow"):
        stops = [element for element in inner if element.is_part("span", "sep")]
        if len(stops) != 1:
            return [
                (
                    f"its eyebrow has {len(stops) + 1} phrases: say two, what this is"
                    ' <span class="sep">·</span> whose or where'
                )
            ]
    if part.is_part("p", "standing") and sum(element.live() for element in inner) != 1:
        return ['its standing line needs one role="status" for the feed\'s state']
    if part.is_part("div", "quiet") and not any(e.is_part("p", "keys") for e in inner):
        return ['its quiet row has no <p class="keys">: put the key line first in it']
    return []


def speaking_faults(head: Element) -> list[str]:
    """Say how much of the head speaks as it changes, beyond the standing line's status."""
    faults = []
    standing = [child for child in head.children if child.is_part("p", "standing")]
    speaking = [
        element
        for element in head.within()
        if element.live() and not any(element.inside(line) for line in standing)
    ]
    if speaking or head.live():
        faults.append(
            f"{len(speaking) + head.live()} more of its head speaks as it changes:"
            " only the standing line's status may be live"
        )
    return faults


def page_faults(markup: str) -> list[str]:
    """Say what a whole page must change to open with the kit's head."""
    links = [
        element
        for element in read(markup).within()
        if element.tag == "link" and element.attributes.get("href", "").endswith("kit/head.css")
    ]
    faults = [] if links else ["it doesn't link the kit's head.css, which draws the head"]
    return faults + head_faults(markup)


def main() -> int:
    """Judge each page named, saying what any must change."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--waiting", default="", help="pages not yet moved, comma-separated")
    parser.add_argument("pages", nargs="+", help="Name=path of each live page's markup")
    arguments = parser.parse_args()
    waiting = {name for name in arguments.waiting.split(",") if name}
    named = dict(page.split("=", 1) for page in arguments.pages)
    lines = [f"{name} waits, but isn't a live page here" for name in waiting - named.keys()]
    for name, path in named.items():
        faults = page_faults(Path(path).read_text())
        if name in waiting and not faults:
            lines.append(f"{name} opens with the kit's head now: take it off the waiting list")
        elif name not in waiting:
            lines.extend(f"{name}: {fault}" for fault in faults)
    for line in lines:
        print(line)
    return 1 if lines else 0


if __name__ == "__main__":
    sys.exit(main())

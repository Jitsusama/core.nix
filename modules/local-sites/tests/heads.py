"""Hold every live page to the kit's head, as sites.md writes it.

The seven pages read as one family only while each opens with the same head:
an eyebrow of two phrases, the title, a lede, the standing line and the quiet
row, in that order, inside one `<header class="page-head">` that is the page's
first header, inside no other, and holds its first heading, styled by the
kit's head.css linked in its `<head>` as a sheet every screen applies. Only
the standing line's status is a live region, polite and heard, and nothing
around the head is live, so a screen reader hears the state change and never
a lede being rewritten.

This reads each page's markup as written, since every page writes its head
there, and as a browser parses it where a head would meet the difference:
void elements hold nothing, a `<p>` ends where a block starts, a non-void
element written closed stays open outside SVG and MathML, a `<head>` left
unwritten or unclosed is implied where the parser implies it, and what
`<template>` or `<noscript>` holds, or what's `hidden`, is never drawn. What
is under `aria-hidden` is never heard. What a script later puts in the head
is the page's own tests' to hold; Home's standing line is the one its build
fills (home.py), so its template writes the status around what's filled.

A page not yet moved to the head is named as waiting, and passes. A waiting
page that keeps to the head fails until it is taken off the list, and a name
that wasn't waiting when the gate began fails, so no page joins it later.
Putting a moved page back, or a name into `FIRST_WAITING`, is the way past
the gate, which review alone holds. Run as
`heads.py [--waiting=Name,Name] Name=path ...`; it prints what each page must
change and fails when any must.
"""

import argparse
import sys
from html.parser import HTMLParser
from pathlib import Path

# The head's parts, in the order sites.md writes them, as the tag and class
# each is drawn with.
PARTS = [("p", "eyebrow"), ("h1", None), ("p", "lede"), ("p", "standing"), ("div", "quiet")]

# The pages that waited for the head when the gate began. A page that joins
# the sites opens with the head, so it never waits. This only shrinks, as the
# waiting list does: a name added here is a way past the gate, review's alone.
FIRST_WAITING = {"Code", "Forage", "Home", "Machines", "Map", "Murmurs", "Network"}

# Elements that never hold others, so they never wait for an end tag.
VOID = {
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "source",
    "track",
    "wbr",
}

# The blocks whose start ends a `<p>` left open, as HTML's parser ends it.
ENDS_A_P = {
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
    "xmp",
}

# What a parser puts in the page's `<head>` when it's written before the body
# outside one, and what else a `<head>` may hold without ending.
HEAD_ONLY = {"base", "link", "meta", "style", "title"}
HEAD_HOLDS = HEAD_ONLY | {"head", "noscript", "script", "template"}

# The headings, any of which may be the page's first.
HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}

# The elements a `<p>` left open outside them isn't ended from within.
P_SCOPE = {"button", "caption", "html", "marquee", "object", "table", "td", "template", "th"}

# Where an element written closed is closed, as it is in SVG and MathML.
FOREIGN = {"svg", "math"}

# What holds markup never drawn as the page's own.
UNDRAWN = {"template", "noscript"}

# The roles ARIA knows; of a role list, a browser takes the first it knows.
ROLES = {
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "comment",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "suggestion",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
}

# The roles a screen reader speaks the changes of, beside aria-live.
LIVE_ROLES = {"status", "alert", "log", "marquee", "timer"}

# The sheet that draws the head, as a mounted page links it and as Home does
# from the root.
SHEETS = {"../local-sites/kit/head.css", "/local-sites/kit/head.css"}

# What a link's media may say for every screen to apply it.
EVERY_SCREEN = {"", "all", "screen"}


class Element:
    """One element as written: its tag, its attributes, what's in it."""

    def __init__(self, tag: str, attributes: dict[str, str], parent: Element | None) -> None:
        """Take the element's tag and attributes, under its parent."""
        self.tag = tag
        self.attributes = attributes
        self.parent = parent
        self.children: list[Element] = []
        self.content: list[Element | str] = []

    def classes(self) -> list[str]:
        """Give the classes it's marked with."""
        return self.attributes.get("class", "").split()

    def is_part(self, tag: str, name: str | None) -> bool:
        """Say whether it's drawn as the part with this tag and class."""
        return self.tag == tag and (name is None or name in self.classes())

    def within(self) -> list[Element]:
        """Give every element drawn inside it, in the order they're written."""
        found = []
        for child in self.children:
            # A hidden link or meta still does its work; only what's drawn is hidden.
            if "hidden" in child.attributes and child.tag not in HEAD_ONLY:
                continue
            found.append(child)
            if child.tag not in UNDRAWN:
                found.extend(child.within())
        return found

    def text(self) -> str:
        """Give the words written in it, in order."""
        return "".join(each if isinstance(each, str) else each.text() for each in self.content)

    def role(self) -> str | None:
        """Give the role a browser takes it as: the first it knows, or an output's status."""
        written = self.attributes.get("role", "").lower().split()
        known = next((role for role in written if role in ROLES), None)
        return known or ("status" if self.tag == "output" else None)

    def heard(self) -> bool:
        """Say whether a screen reader hears it: nothing under aria-hidden is."""
        element: Element | None = self
        while element is not None:
            if element.attributes.get("aria-hidden", "").strip().lower() == "true":
                return False
            element = element.parent
        return True

    def live(self) -> bool:
        """Say whether a screen reader speaks its changes; an explicit aria-live decides."""
        if not self.heard():
            return False
        said = self.attributes.get("aria-live", "").strip().lower()
        if said in {"polite", "assertive", "off"}:
            return said != "off"
        return self.role() in LIVE_ROLES

    def interrupts(self) -> bool:
        """Say whether it cuts in on what's being read, as an alert does."""
        return self.attributes.get("aria-live", "").strip().lower() == "assertive"

    def inside(self, ancestor: Element) -> bool:
        """Say whether it sits somewhere within `ancestor`."""
        parent = self.parent
        while parent is not None:
            if parent is ancestor:
                return True
            parent = parent.parent
        return False


class Reader(HTMLParser):
    """Read markup into elements, closing what a browser would leave closed."""

    def __init__(self) -> None:
        """Start from an empty document."""
        super().__init__(convert_charrefs=True)
        self.document = Element("#document", {}, None)
        self.open = self.document
        self.head: Element | None = None
        self.bodied = False

    def add(self, tag: str, attrs: list[tuple[str, str | None]]) -> Element:
        """Put an element under the one open now."""
        element = Element(tag, {name: value or "" for name, value in attrs}, self.open)
        self.open.children.append(element)
        self.open.content.append(element)
        return element

    def foreign(self) -> bool:
        """Say whether the open element sits in SVG or MathML."""
        element: Element | None = self.open
        while element is not None:
            if element.tag in FOREIGN:
                return True
            element = element.parent
        return False

    def end_a_p(self) -> None:
        """End a `<p>` left open, as a block starting after it does."""
        element: Element | None = self.open
        while element is not None and element.tag not in P_SCOPE:
            if element.tag == "p" and element.parent is not None:
                self.open = element.parent
                return
            element = element.parent

    def in_head(self) -> bool:
        """Say whether the open element sits in the page's `<head>`."""
        element: Element | None = self.open
        while element is not None:
            if element is self.head:
                return True
            element = element.parent
        return False

    def head_for(self, tag: str) -> Element | None:
        """Give the `<head>` a parser puts `tag` in when written outside one before the body."""
        if tag not in HEAD_ONLY or self.bodied or self.open.tag not in {"#document", "html"}:
            return None
        if self.head is None:
            self.head = self.add("head", [])
        return self.head

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Open an element under the one open now, where a parser would put it."""
        if tag == "body" or (tag not in HEAD_HOLDS and self.in_head()):
            self.bodied = True
            while self.in_head() and self.open.parent is not None:
                self.open = self.open.parent
        if tag in ENDS_A_P:
            self.end_a_p()
        home = self.head_for(tag)
        was = self.open
        if home is not None:
            self.open = home
        element = self.add(tag, attrs)
        if tag == "head":
            self.head = element
        if tag not in VOID:
            self.open = element
        elif home is not None:
            self.open = was

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Add an element written closed: closed in SVG or MathML, else left open."""
        if tag in VOID or tag in FOREIGN or self.foreign():
            self.add(tag, attrs)
        else:
            self.handle_starttag(tag, attrs)

    def handle_endtag(self, tag: str) -> None:
        """Close the nearest open element of this tag, and any left open in it."""
        element: Element | None = self.open
        while element is not None and element.tag != tag:
            element = element.parent
        if element is not None and element.parent is not None:
            self.open = element.parent

    def handle_data(self, data: str) -> None:
        """Keep the words written where they're written."""
        self.open.content.append(data)


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
    banners = [element for element in everything if element.tag == "header"]
    if banners and banners[0] is not head:
        drawn = (banners[0].tag, next(iter(banners[0].classes()), None))
        where = "around" if head.inside(banners[0]) else "above"
        faults.append(f"it draws a {said(drawn)} {where} its head: let the kit's head be its first")
    titles = [element for element in everything if element.tag in HEADINGS]
    if not titles or not titles[0].inside(head):
        first = f"<{titles[0].tag}>" if titles else "heading"
        faults.append(
            f"its first heading, {first}, sits outside its head: make the head's title first"
        )
    if any(isinstance(each, str) and each.strip() for each in head.content):
        faults.append("its head writes words outside its parts: keep every word in a part")
    drawn_parts = [(child.tag, next(iter(child.classes()), None)) for child in head.children]
    parts = [
        next((part for part in PARTS if child.is_part(*part)), (child.tag, None))
        for child in head.children
    ]
    if parts != PARTS:
        shown = ", ".join(said(part) for part in drawn_parts) or "nothing"
        wanted = ", ".join(said(part) for part in PARTS)
        faults.append(f"its head holds {shown}: hold {wanted}, in that order and nothing else")
    for child in head.children:
        faults.extend(part_faults(child))
    return faults + speaking_faults(head)


def phrases(element: Element, stop: Element) -> list[str]:
    """Give the words written in `element` on each side of `stop`."""
    sides = [""]
    for each in element.content:
        if each is stop:
            sides.append("")
        elif isinstance(each, str):
            sides[-1] += each
        elif stop.inside(each):
            inner = phrases(each, stop)
            sides[-1] += inner[0]
            sides.extend(inner[1:])
        else:
            sides[-1] += each.text()
    return sides


# What an eyebrow says, for a person to write it so.
EYEBROW = 'say two, what this is <span class="sep">·</span> whose or where'


def eyebrow_faults(eyebrow: Element) -> list[str]:
    """Say what the eyebrow must change to be two phrases split by the kit's dot."""
    stops = [element for element in eyebrow.within() if element.is_part("span", "sep")]
    if not stops and "·" in eyebrow.text():
        return ['its eyebrow writes its dot as text: write it as <span class="sep">·</span>']
    if len(stops) != 1:
        count = len(stops) + 1
        return [f"its eyebrow has {count} phrase{'' if count == 1 else 's'}: {EYEBROW}"]
    sides = phrases(eyebrow, stops[0])
    if any("·" in side for side in sides):
        return ["its eyebrow writes a dot as text beside the kit's: keep the one sep"]
    if any(not side.strip() for side in sides):
        return [f"its eyebrow has an empty phrase: {EYEBROW}"]
    return []


def part_faults(part: Element) -> list[str]:
    """Say what one part of the head must change within it, or nothing."""
    inner = part.within()
    if part.is_part("p", "eyebrow"):
        return eyebrow_faults(part)
    if part.is_part("p", "standing"):
        live = [element for element in inner if element.live()]
        if len(live) != 1 or live[0].role() != "status" or live[0].interrupts():
            return [
                (
                    'its standing line needs one polite role="status" for the feed\'s state,'
                    " heard, and nothing else live"
                )
            ]
    opens = part.children[0] if part.children else None
    if part.is_part("div", "quiet") and (opens is None or not opens.is_part("p", "keys")):
        return ['its quiet row doesn\'t open with a <p class="keys">: put the key line first in it']
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
    # A live region speaks for all it holds, so one around the head speaks the whole head.
    around = []
    parent = head.parent
    while parent is not None:
        if parent.live():
            around.append(parent)
        parent = parent.parent
    if around:
        faults.append(
            f"{len(around)} around its head speak{'s' if len(around) == 1 else ''} as it"
            " changes: nothing holding the head may be live"
        )
    return faults


def links_the_sheet(element: Element, document: list[Element]) -> bool:
    """Say whether `element` links the kit's head.css, in the page's `<head>`, for every screen."""
    rel = element.attributes.get("rel", "").lower().split()
    href = element.attributes.get("href", "").split("?")[0].split("#")[0]
    media = element.attributes.get("media", "").strip().lower()
    heads = [each for each in document if each.tag == "head"]
    return (
        element.tag == "link"
        and "stylesheet" in rel
        and "alternate" not in rel
        and "disabled" not in element.attributes
        and media in EVERY_SCREEN
        and href in SHEETS
        and any(element.inside(head) for head in heads)
    )


# What a page lacking the kit's sheet must write, for a person to write it so.
NO_SHEET = (
    "it doesn't link the kit's head.css in its <head>: write <link rel=\"stylesheet\""
    ' href="../local-sites/kit/head.css">, from the root "/local-sites/kit/head.css",'
    " for every screen"
)


def page_faults(markup: str) -> list[str]:
    """Say what a whole page must change to open with the kit's head."""
    document = read(markup).within()
    linked = any(links_the_sheet(element, document) for element in document)
    return ([] if linked else [NO_SHEET]) + head_faults(markup)


def main() -> int:
    """Judge each page named, saying what any must change."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--waiting", default="", help="pages not yet moved, comma-separated")
    parser.add_argument("pages", nargs="+", help="Name=path of each live page's markup")
    arguments = parser.parse_args()
    waiting = {name for name in arguments.waiting.split(",") if name}
    named = dict(page.split("=", 1) for page in arguments.pages)
    lines = [f"{name} waits, but isn't a live page here" for name in sorted(waiting - named.keys())]
    lines.extend(
        f"{name} joined the waiting list: a page that joins the sites opens with the head"
        for name in sorted((waiting & named.keys()) - FIRST_WAITING)
    )
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

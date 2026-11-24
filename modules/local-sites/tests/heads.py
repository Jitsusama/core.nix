"""Hold every live page to the kit's head, as sites.md writes it.

The seven pages read as one family only while each opens with the same head:
an eyebrow of two phrases, the title, a lede, the standing line and the quiet
row, in that order, each drawn and heard, inside one `<header class="page-head">`
that is the page's banner: its first header or element marked one, inside no
other and in no section, holding its first heading by tag or role, styled by
the kit's head.css linked in its `<head>` as a sheet every screen applies. Only
the standing line's status is a live region, polite and heard, and nothing
around the head is live, so a screen reader hears the state change and never
a lede being rewritten.

This reads each page's markup as written, since every page writes its head
there, parsed as a browser running scripts parses it, by lexbor, which builds
HTML's own tree: what's implied, ended, moved out of a table, read as
words or left in SVG or MathML is where a browser puts it. What `<template>`,
`<noscript>`, an element read as words, a list of options, a ruby's fallback
or what draws itself (media, a canvas, an object) holds, what SVG or MathML
holds, even where they hold HTML, which fails closed, what a shut `<dialog>`
or `<details>` holds past its summary, and what's
`hidden` or a `popover` no script has shown, unless its own `style` displays it,
or styled out by its own `style`'s display, visibility or opacity, each
declaration's last value read, is never drawn; nor is it heard, and neither is
what's `inert` or under `aria-hidden`. What's in one undrawn is too, though its
own style shows it again, which fails closed. A `<base>` anywhere moving where
links read from fails, since the sheet can't be known to be the kit's. What a
sheet's rules hide, and what a script later puts in the head, are the page's
own tests' to hold; Home's standing line is the one its build fills (home.py),
so its template writes the status around what's filled. The gate reads each
page's source, which matches what's served while each skip link is a line of
its own before the head, since site.sh puts the strip after that line.

Long because it holds each of the head's rules and what a browser draws and
hears of a parsed page, read together: a rule moves when what's drawn does.

A page not yet moved to the head is named as waiting, and passes. A waiting
page that keeps to the head fails until it is taken off the list, and a name
that wasn't waiting when the gate began fails, so no page joins it later.
Putting a moved page back, or a name into `FIRST_WAITING`, is the way past
the gate, which review alone holds. Run as
`heads.py [--waiting=Name,Name] Name=path ...`; it prints what each page must
change and fails when any must.
"""

import argparse
import re
import sys
from pathlib import Path

from selectolax.lexbor import LexborHTMLParser, LexborNode

# The head's parts, in the order sites.md writes them, as the tag and class
# each is drawn with.
PARTS = [("p", "eyebrow"), ("h1", None), ("p", "lede"), ("p", "standing"), ("div", "quiet")]

# The pages that waited for the head when the gate began. A page that joins
# the sites opens with the head, so it never waits. This only shrinks, as the
# waiting list does: a name added here is a way past the gate, review's alone.
FIRST_WAITING = {"Code", "Forage", "Home", "Machines", "Map", "Murmurs", "Network"}

# Where HTML gives way to another language's elements, all they hold read as theirs.
FOREIGN = {"svg", "math"}

# What only does its work in the page's `<head>`, drawn or not.
HEAD_ONLY = {"base", "link", "meta", "style", "title"}

# The headings, any of which may be the page's first.
HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}

# What holds markup never drawn as the page's own: a template, what shows only
# without scripts, what a browser reads as words, a script or another page's,
# what its own sheet never draws, and the fallback of what draws itself. The
# parser reads raw text as words already, so most of these hold none.
UNDRAWN = {
    "audio",
    "canvas",
    "datalist",
    "iframe",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "plaintext",
    "rp",
    "script",
    "style",
    "template",
    "textarea",
    "title",
    "video",
    "xmp",
}

# What draws only its summary, or nothing, until it's open.
SHUT = {"details", "dialog"}

# Where a header is a section's, not the page's banner: in one of these, or
# in an element marked as one, as HTML-AAM maps a header.
SECTIONS = {"article", "aside", "main", "nav", "section"}
SECTION_ROLES = {"article", "complementary", "main", "navigation", "region"}

# What HTML counts as space between words; a no-break space is a word's.
SPACE = " \t\n\f\r"

# The roles a head may be marked with and still be the page's banner.
BANNER = {None, "banner"}

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

# The roles a screen reader speaks the changes of, beside aria-live; a
# marquee's and a timer's are off unless they say otherwise.
LIVE_ROLES = {"status", "alert", "log"}

# What an element's own style may say that draws none of it, as a browser
# reads each declaration, beside an opacity of nothing; a sheet's rules, and a
# clip or a size of nothing, are the live tests' to hold.
STYLED_OUT = {
    ("display", "none"),
    ("visibility", "hidden"),
    ("visibility", "collapse"),
    ("content-visibility", "hidden"),
}

# A declaration's mark of weight, which CSS lets a space split.
IMPORTANT = re.compile(r"!\s*important\s*$")

# A comment in a style, which CSS reads as nothing.
COMMENT = re.compile(r"/\*.*?(?:\*/|$)", re.DOTALL)

# What any property may say.
GLOBAL = {"inherit", "initial", "unset", "revert", "revert-layer"}

# What rolls a display back to the browser's own, which hides what's hidden.
ROLLED_BACK = {"revert", "revert-layer"}

# A display's words, as CSS Display writes it: how it sits among others, how
# it lays out what it holds, and a list item's marker, each at most once, the
# marker only beside a flow; or one word that says it alone.
OUTER = {"block", "inline", "run-in"}
INNER = {"flow", "flow-root", "table", "flex", "grid", "ruby", "math"}
MARKED = {"list-item"}
FLOWS = {"flow", "flow-root"}
ALONE = {
    "contents",
    "inline-block",
    "inline-flex",
    "inline-grid",
    "inline-table",
    "none",
    "ruby-base",
    "ruby-base-container",
    "ruby-text",
    "ruby-text-container",
    "table-caption",
    "table-cell",
    "table-column",
    "table-column-group",
    "table-footer-group",
    "table-header-group",
    "table-row",
    "table-row-group",
}

# What a visibility or content-visibility may say.
VISIBILITIES = {"visibility": {"visible", "hidden", "collapse"}}
VISIBILITIES["content-visibility"] = {"visible", "hidden", "auto"}

# A number, or a percentage, as an opacity is written.
AMOUNT = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?%?")


def valid(name: str, value: str) -> bool:
    """Say whether a browser keeps `value` for `name`, rather than dropping it unread."""
    if value == "":
        return False
    if value in GLOBAL:
        return True
    if name == "display":
        return displays(value.split())
    if name in VISIBILITIES:
        return value in VISIBILITIES[name]
    return True


def displays(words: list[str]) -> bool:
    """Say whether a browser reads `words` as a display, not dropping it."""
    if len(words) == 1 and words[0] in ALONE:
        return True
    kinds = [kind for kind in (OUTER, INNER, MARKED) for word in words if word in kind]
    inner = [word for word in words if word in INNER]
    return (
        len(kinds) == len(words)
        and all(sum(word in kind for word in words) <= 1 for kind in (OUTER, INNER, MARKED))
        and not (MARKED & set(words) and inner and inner[0] not in FLOWS)
    )


def opaque(value: str) -> bool:
    """Say whether an opacity draws anything: more than none; one this can't read fails closed."""
    if value in GLOBAL:
        return True
    if AMOUNT.fullmatch(value) is None:
        return False
    return float(value.rstrip("%")) > 0


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

    def styled(self) -> dict[str, str]:
        """Give what its own style declares, each property's value as a browser takes it."""
        # The last declaration of a property wins, but not over an important one.
        # One a browser can't read, empty or of no value its property takes,
        # is dropped, and what was declared before it stands.
        declared: dict[str, tuple[str, bool]] = {}
        style = COMMENT.sub(" ", self.attributes.get("style", ""))
        for name, colon, value in (each.partition(":") for each in style.split(";")):
            said = value.strip(SPACE).lower()
            weighty = IMPORTANT.search(said) is not None
            key = name.strip(SPACE).lower()
            kept = IMPORTANT.sub("", said).strip(SPACE)
            if colon and valid(key, kept) and (weighty or not declared.get(key, ("", False))[1]):
                declared[key] = (kept, weighty)
        return {key: value for key, (value, _) in declared.items()}

    def hides(self) -> bool:
        """Say whether it draws none of itself: hidden, a popover never shown, or styled out."""
        # A hidden link or meta still does its work; only what's drawn is hidden.
        if self.tag in HEAD_ONLY:
            return False
        styled = self.styled()
        # hidden and an unshown popover hide by the browser's display: none,
        # which the element's own display beats, unless it rolls back to the
        # browser's; hidden=until-found doesn't.
        held = ("hidden" in self.attributes or "popover" in self.attributes) and (
            styled.get("display", "revert") in ROLLED_BACK
            or self.attributes.get("hidden", "").strip(SPACE).lower() == "until-found"
        )
        return (
            held
            or any(each in STYLED_OUT for each in styled.items())
            or not opaque(styled.get("opacity", "1"))
        )

    def within(self) -> list[Element]:
        """Give every element drawn inside it, in the order they're written."""
        found = []
        for child in self.children:
            if child.hides():
                continue
            found.append(child)
            if child.tag in SHUT and "open" not in child.attributes:
                summary = next((each for each in child.shown() if each.tag == "summary"), None)
                if child.tag == "details" and summary is not None:
                    found.append(summary)
                    found.extend(summary.within())
            # SVG and MathML draw none of what they hold as the page's own,
            # HTML they hold again included: a description, a symbol or a
            # frame of no size draws nothing, which fails closed.
            elif child.tag not in UNDRAWN and child.tag not in FOREIGN:
                found.extend(child.within())
        return found

    def shown(self) -> list[Element]:
        """Give the children drawn in it, leaving out those hidden."""
        return [child for child in self.children if not child.hides()]

    def written(self) -> list[Element]:
        """Give every element parsed inside it, drawn or not, but for what's read as words."""
        found = []
        for child in self.children:
            found.append(child)
            if child.tag not in UNDRAWN:
                found.extend(child.written())
        return found

    def every(self) -> list[Element]:
        """Give every element parsed inside it, wherever it's held, in order."""
        found = []
        for child in self.children:
            found.append(child)
            found.extend(child.every())
        return found

    def words(self) -> str:
        """Give the words drawn and heard in it, in order."""
        return "".join(
            each if isinstance(each, str) else each.words()
            for each in self.content
            if isinstance(each, str) or each.says()
        )

    def says(self) -> bool:
        """Say whether what it holds is drawn and heard where it sits, as a child's words."""
        return not (
            self.hides()
            or self.tag in UNDRAWN
            or (self.tag in SHUT and "open" not in self.attributes)
            or self.attributes.get("aria-hidden", "").strip(SPACE).lower() == "true"
            or "inert" in self.attributes
        )

    def role(self) -> str | None:
        """Give the role a browser takes it as: the first it knows, or an output's status."""
        written = self.attributes.get("role", "").lower().split()
        known = next((role for role in written if role in ROLES), None)
        return known or ("status" if self.tag == "output" else None)

    def heard(self) -> bool:
        """Say whether a screen reader hears it: nothing undrawn, inert or under aria-hidden is."""
        element: Element | None = self
        while element is not None:
            if element.attributes.get("aria-hidden", "").strip(SPACE).lower() == "true":
                return False
            if element.hides() or "inert" in element.attributes:
                return False
            element = element.parent
        return True

    def live(self) -> bool:
        """Say whether a screen reader speaks its changes; an explicit aria-live decides."""
        if not self.heard():
            return False
        said = self.attributes.get("aria-live", "").strip(SPACE).lower()
        if said in {"polite", "assertive", "off"}:
            return said != "off"
        return self.role() in LIVE_ROLES

    def section(self) -> Element | None:
        """Give the section it sits in, by its tag or the role it's marked with, if any."""
        section = self.parent
        while section is not None and not (
            section.tag in SECTIONS or section.role() in SECTION_ROLES
        ):
            section = section.parent
        return section

    def is_banner(self) -> bool:
        """Say whether it's heard as the page's banner: marked one, or a header in no section."""
        role = self.role()
        return role == "banner" or (
            self.tag == "header" and role is None and self.section() is None
        )

    def is_heading(self) -> bool:
        """Say whether it's drawn or heard as a heading: h1 to h6, or marked one."""
        return self.tag in HEADINGS or self.role() == "heading"

    def interrupts(self) -> bool:
        """Say whether it cuts in on what's being read, as an alert does."""
        return self.attributes.get("aria-live", "").strip(SPACE).lower() == "assertive"

    def inside(self, ancestor: Element) -> bool:
        """Say whether it sits somewhere within `ancestor`."""
        parent = self.parent
        while parent is not None:
            if parent is ancestor:
                return True
            parent = parent.parent
        return False


def adopt(node: LexborNode, parent: Element) -> None:
    """Put what the parser read as `node`, and all it holds, under `parent`, in order."""
    if node.is_text_node:
        parent.content.append(node.text_content or "")
        return
    # A comment or the doctype is no element, and says nothing.
    if not node.is_element_node:
        return
    tag = (node.tag or "").lower()
    attributes = {str(name): value or "" for name, value in node.attributes.items()}
    element = Element(tag, attributes, parent)
    parent.children.append(element)
    parent.content.append(element)
    # A template's content sits apart from the page's tree, which a script
    # copies out to use; it's read as the template's own.
    holders = [node]
    if tag == "template":
        holders = [
            fragment
            for fragment in node.template_fragments()
            if fragment.parent is not None and fragment.parent.mem_id == node.mem_id
        ]
    for holder in holders:
        for child in holder.iter(include_text=True):
            adopt(child, element)


def read(markup: str) -> Element:
    """Give the document `markup` writes, as a browser running scripts parses it."""
    document = Element("#document", {}, None)
    # lexbor builds an html element for any markup, even none, so a page
    # always has a root; a missing one would be a parser fault, said here.
    root = LexborHTMLParser(markup, scripting=True).root
    if root is None:
        message = "lexbor built no html element"
        raise ValueError(message)
    adopt(root, document)
    return document


def said(part: tuple[str, str | None]) -> str:
    """Name a part as its markup reads, for a person to write it."""
    tag, name = part
    return f"<{tag}>" if name is None else f'<{tag} class="{name}">'


def named(element: Element) -> str:
    """Name an element by its tag and first class, as `said` writes a part."""
    return said((element.tag, next(iter(element.classes()), None)))


def head_faults(markup: str) -> list[str]:
    """Say what `markup`'s head must change to be the kit's, or nothing."""
    document = read(markup)
    everything = document.within()
    heads = [element for element in everything if "page-head" in element.classes()]
    if len(heads) != 1:
        return [f'it draws {len(heads)} heads: open it with one <header class="page-head">']
    head = heads[0]
    faults = []
    if head.tag != "header":
        faults.append(f'its head is a <{head.tag}>: draw it as <header class="page-head">')
    elif head.role() not in BANNER:
        faults.append(
            f'its head is marked role="{head.role()}": leave the header its own role, the banner'
        )
    section = head.section()
    if section is not None:
        marked = section.role()
        where = f"<{section.tag}>"
        if marked in SECTION_ROLES:
            where = f'<{section.tag} role="{marked}">'
        faults.append(
            f"its head sits in a {where}, where a header is no banner:"
            " draw it before, as the page's own"
        )
    faults.extend(banner_faults(head, everything, document.every()))
    faults.extend(title_faults(head, everything))
    if any(isinstance(each, str) and each.strip(SPACE) for each in head.content):
        faults.append("its head writes words outside its parts: keep every word in a part")
    return faults + parts_faults(head) + speaking_faults(head)


def banner_faults(head: Element, everything: list[Element], written: list[Element]) -> list[str]:
    """Say what the page must change for the head to be its one banner, its skip link before."""
    faults = []
    banners = [element for element in everything if element.is_banner()]
    if banners and banners[0] is not head:
        if head.inside(banners[0]):
            where = "around"
        elif everything.index(banners[0]) < everything.index(head):
            where = "above"
        else:
            where = "below"
        faults.append(
            f"it draws a {named(banners[0])} {where} its head: let the kit's head be its one banner"
        )
    # A banner inside the head is a second, as a header in a header is heard.
    later = banners[1:]
    if banners and banners[0] is head and later:
        where = "inside" if later[0].inside(head) else "below"
        faults.append(
            f"it draws a {named(later[0])} {where} its head as well:"
            " let the kit's head be its one banner"
        )
    # site.sh puts the strip after a skip link's line whether it's drawn or not.
    skips = [element for element in written if element.is_part("a", "skip")]
    if any(written.index(skip) > written.index(head) for skip in skips):
        faults.append(
            "its skip link sits in or after its head, where the strip site.sh puts after it"
            " would split what holds it: write it before the head"
        )
    return faults


def title_faults(head: Element, everything: list[Element]) -> list[str]:
    """Say what the page must change for the head's title to be its first heading."""
    titles = [element for element in everything if element.is_heading()]
    title = next((child for child in head.shown() if child.tag == "h1"), None)
    if not titles:
        return ["it draws no heading: make the head's title its first"]
    if titles[0] is title:
        return []
    first = named(titles[0])
    if titles[0].inside(head):
        return [f"its first heading is a {first} in its head: make the head's title its first"]
    return [f"its first heading, {first}, sits outside its head: make the head's title first"]


def parts_faults(head: Element) -> list[str]:
    """Say what the head's parts must change: their order, their showing, what's in each."""
    faults = []
    parts = [
        next((part for part in PARTS if child.is_part(*part)), (child.tag, None))
        for child in head.children
    ]
    if parts != PARTS:
        held = ", ".join(named(child) for child in head.children) or "nothing"
        wanted = ", ".join(said(part) for part in PARTS)
        faults.append(f"its head holds {held}: hold {wanted}, in that order and nothing else")
    shown = head.shown()
    hidden = [child for child in head.children if child not in shown]
    if hidden:
        faults.append(
            f"its head hides its {', '.join(named(child) for child in hidden)}: draw every part"
        )
    if not head.heard():
        return [*faults, "its head is hidden from screen readers: let them hear it"]
    # The standing line's own fault says when its status goes unheard.
    unheard = [child for child in shown if not child.heard() and not child.is_part("p", "standing")]
    if unheard:
        faults.append(
            f"its head hides its {', '.join(named(child) for child in unheard)}"
            " from screen readers: let them hear every part"
        )
    title = next((child for child in shown if child.tag == "h1"), None)
    if title is not None and title.role() not in {None, "heading"}:
        faults.append(f'its title is marked role="{title.role()}": leave the <h1> a heading')
    for child in shown:
        if child not in unheard:
            faults.extend(part_faults(child))
    return faults


def phrases(element: Element, stop: Element) -> list[str]:
    """Give the words drawn and heard in `element` on each side of `stop`."""
    sides = [""]
    for each in element.content:
        if each is stop:
            sides.append("")
        elif isinstance(each, str):
            sides[-1] += each
        elif stop.inside(each):
            # What's unheard around the dot still splits it, but says nothing.
            inner = phrases(each, stop) if each.says() else ["", ""]
            sides[-1] += inner[0]
            sides.extend(inner[1:])
        elif each.says():
            sides[-1] += each.words()
    return sides


# What an eyebrow says, for a person to write it so.
EYEBROW = 'say two, what this is <span class="sep">·</span> whose or where'


def eyebrow_faults(eyebrow: Element) -> list[str]:
    """Say what the eyebrow must change to be two phrases split by the kit's dot."""
    stops = [element for element in eyebrow.within() if element.is_part("span", "sep")]
    if not stops and "·" in eyebrow.words():
        return ['its eyebrow writes its dot as text: write it as <span class="sep">·</span>']
    if len(stops) != 1:
        count = len(stops) + 1
        return [f"its eyebrow has {count} phrase{'' if count == 1 else 's'}: {EYEBROW}"]
    sides = phrases(eyebrow, stops[0])
    if any("·" in side for side in sides):
        return ["its eyebrow writes a dot as text beside the kit's: keep the one sep"]
    if any(not side.strip(SPACE) for side in sides):
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
    drawn = part.shown()
    opens = drawn[0] if drawn else None
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
    # A browser fetches no sheet of another type, and applies none whose digest
    # fails; the kit's sheet changes as the kit does, so a digest would go stale.
    kind = element.attributes.get("type", "").split(";")[0].strip().lower()
    head = next((each for each in document if each.tag == "head"), None)
    return (
        element.tag == "link"
        and "stylesheet" in rel
        and "alternate" not in rel
        # A titled sheet is one of a set, which another set's title may switch off.
        and "title" not in element.attributes
        and "disabled" not in element.attributes
        and "integrity" not in element.attributes
        and kind in {"", "text/css"}
        and media in EVERY_SCREEN
        and href in SHEETS
        and head is not None
        and element.inside(head)
    )


# What a page lacking the kit's sheet must write, for a person to write it so.
NO_SHEET = (
    "it doesn't link the kit's head.css in its <head>: write <link rel=\"stylesheet\""
    ' href="../local-sites/kit/head.css">, from the root "/local-sites/kit/head.css",'
    " for every screen"
)


# What a page whose links a base moves must change.
BASED = (
    "its <base href> moves where its links read from, so head.css may not be the kit's:"
    " take the base out"
)


# What a page whose skip link shares its line must change, since site.sh puts
# the strip after the line holding it, which would put it inside what follows.
SKIP_LINE = (
    "its skip link shares its line, and the strip goes after that line:"
    " write the link, from its <a to its </a>, on a line of its own"
)


def skip_line_faults(markup: str) -> list[str]:
    """Say whether a skip link's line holds more than it, which the strip would split."""
    for line in markup.splitlines():
        if '<a class="skip"' not in line:
            continue
        alone = line.strip(SPACE)
        if not (
            alone.startswith('<a class="skip"')
            and alone.endswith("</a>")
            and alone.count("</a>") == 1
        ):
            return [SKIP_LINE]
    return []


def page_faults(markup: str) -> list[str]:
    """Say what a whole page must change to open with the kit's head."""
    parsed = read(markup)
    document = parsed.within()
    # A browser reads links from the first base wherever it's parsed, drawn or not.
    if any(element.tag == "base" and "href" in element.attributes for element in parsed.written()):
        sheet = [BASED]
    elif any(links_the_sheet(element, document) for element in document):
        sheet = []
    else:
        sheet = [NO_SHEET]
    return sheet + skip_line_faults(markup) + head_faults(markup)


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

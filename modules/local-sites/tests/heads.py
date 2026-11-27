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
words or left in SVG or MathML is where a browser puts it. It walks that tree
twice, and each walk settles a doubt its own way.

The strict walk finds the head and its parts, and settles a doubt toward
undrawn, so a head that may not show fails. To it, what `<template>`,
`<noscript>`, an element read as words, a list of options, a ruby's fallback
or what draws itself (media, a canvas, an object, a meter or a progress bar)
holds, what SVG or MathML holds, even where they hold HTML, what a shut
`<dialog>` or `<details>` holds past its first summary, and an HTML element
`hidden` or a `popover` no script has shown, unless its own `style` displays
it, or styled out by its own `style`'s display, visibility, content-visibility
or opacity, each declaration's last value read, is never drawn; nor is it
heard, and neither is what's `inert` or under `aria-hidden`. What's in one
undrawn is too, though its own style shows it again.

The generous walk seeks what may not be there beside the head, a rival
banner, heading or live region, and settles a doubt toward drawn, so a rival
that may show fails. To it, HTML in SVG or MathML, a canvas's or an object's
fallback, what a meter or a progress bar holds, an opacity of nothing, one
shown again under visibility: hidden, a style this can't read, and what a
person can show with no script all count: what's hidden until found or held
past a shut `<details>`'s summary, which find-in-page opens, a `popover`,
which a `popovertarget` shows, and a shut `<dialog>` a button's `commandfor`
shows as a modal. It leaves out only what's surely undrawn and unheard:
display: none and what keeps it, as `hidden`, a tag HTML's own sheet hides or
audio with no controls do, what media or a template holds, a shut `<dialog>`
no button opens, an element SVG doesn't know, and what content-visibility:
hidden skips on a box it applies to. A style is doubted only where the value
that wins is one this can't read: one a browser drops, or one a later
readable value or an important one beats, settles nothing. A rival only this
walk finds is named as one the page may draw, with why it counts.

A `<base>` anywhere moving where links read from fails, since the sheet can't
be known to be the kit's. So does a page writing a shadow root template,
marking `aria-owns`, or drawing another document in an `<iframe>`, an `<embed>`
or an `<object>` with data, since what a browser draws or a screen reader
hears there isn't the tree this reads. What a sheet's rules hide, and what a
script later puts in the head, are the page's own tests' to hold; Home's
standing line is the one its build fills (home.py), so its template writes the
status around what's filled. The gate reads each page's source, which matches
what's served while each skip link is a line of its own before the head, since
site.sh puts the strip after that line.

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

# The namespace an element is parsed into, as HTML's parser puts it.
HTML = "html"

# Where SVG or MathML holds HTML again, as HTML's parser reads what's in them:
# SVG's integration points, MathML's text integration points, which keep
# their own two glyphs, and an annotation of HTML.
HOLDS_HTML = {"svg": {"foreignobject", "desc", "title"}, "math": {"mi", "mo", "mn", "ms", "mtext"}}
MATH_GLYPHS = {"mglyph", "malignmark"}
HTML_ANNOTATIONS = {"text/html", "application/xhtml+xml"}

# What SVG holds that it doesn't know as one of its own, so draws none of, of
# those HTML would read as a banner or a section.
UNKNOWN_TO_SVG = {"article", "aside", "footer", "header", "main", "nav", "section"}

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
    "meter",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "plaintext",
    "progress",
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

# What a button's command may say to open a shut dialog, with no script.
OPENING_COMMANDS = {"show-modal"}

# What draws itself, but whose fallback a browser may draw or a screen reader hear.
FALLBACKS = {"canvas", "object"}

# What draws itself as a bar and none of what it holds, which the strict walk
# leaves out; the generous one still reads it, as a reader may hear it.
GAUGES = {"meter", "progress"}

# What an element's own style may say that this can't read, of what draws it.
DOUBTED = ("display", "visibility", "content-visibility", "opacity")

# What HTML's own sheet draws none of, beside what's hidden; audio without
# controls is one too.
SHEET_HIDDEN = {"datalist", "noembed", "noframes", "rp"}

# What draws itself and never what it holds, unlike a canvas's or an
# object's fallback, which a browser may draw or a screen reader hear.
MEDIA = {"audio", "video"}

# The tags HTML's own sheet draws as a block, which content-visibility
# applies to; one not here, a span or a custom element, is inline.
BLOCKS = {
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
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
    "html",
    "legend",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
}

# Where a header is a section's, not the page's banner: in one of these, or
# in an element marked as one, as HTML-AAM maps a header.
SECTIONS = {"article", "aside", "main", "nav", "section"}
SECTION_ROLES = {"article", "complementary", "main", "navigation", "region"}

# What HTML counts as space between words; a no-break space is a word's.
SPACE = " \t\n\f\r"

# The roles a head may be marked with and still be the page's banner.
BANNER = {None, "banner"}

# The roles ARIA knows, DPUB's and SVG's graphics roles among them; of a role
# list, a browser takes the first it knows and skips any other.
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
    "doc-abstract",
    "doc-acknowledgments",
    "doc-afterword",
    "doc-appendix",
    "doc-backlink",
    "doc-biblioentry",
    "doc-bibliography",
    "doc-biblioref",
    "doc-chapter",
    "doc-colophon",
    "doc-conclusion",
    "doc-cover",
    "doc-credit",
    "doc-credits",
    "doc-dedication",
    "doc-endnote",
    "doc-endnotes",
    "doc-epigraph",
    "doc-epilogue",
    "doc-errata",
    "doc-example",
    "doc-footnote",
    "doc-foreword",
    "doc-glossary",
    "doc-glossref",
    "doc-index",
    "doc-introduction",
    "doc-noteref",
    "doc-notice",
    "doc-pagebreak",
    "doc-pagefooter",
    "doc-pageheader",
    "doc-pagelist",
    "doc-part",
    "doc-preface",
    "doc-prologue",
    "doc-pullquote",
    "doc-qna",
    "doc-subtitle",
    "doc-tip",
    "doc-toc",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "graphics-document",
    "graphics-object",
    "graphics-symbol",
    "grid",
    "gridcell",
    "group",
    "heading",
    "image",
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
    "sectionfooter",
    "sectionheader",
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

# The roles whose children a screen reader hears only as their words, never
# as a heading, a banner or a status, and the tags that have one unmarked.
FLATTENING = {
    "button",
    "checkbox",
    "image",
    "img",
    "math",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "option",
    "progressbar",
    "radio",
    "scrollbar",
    "separator",
    "slider",
    "switch",
    "tab",
}
IMPLICIT = {"button": "button", "meter": "meter", "option": "option", "progress": "progressbar"}

# The roles a browser ignores on what takes focus, taking its own instead, as
# ARIA says of none and presentation; generic is read the same way, which
# fails closed, as a holder flattening what it holds hears less.
IGNORED_ON_FOCUS = {"none", "presentation", "generic"}

# What a template carries to attach a shadow root as it's parsed, the old
# name beside the new.
SHADOW_ROOTS = {"shadowrootmode", "shadowroot"}

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
# marker only beside a flow; or one word that says it alone. Only those
# Chromium keeps, as the live tests draw with it: it drops run-in and the
# ruby containers, so hidden holds there.
OUTER = {"block", "inline"}
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
    "ruby-text",
    "table-caption",
    "table-cell",
    "table-column",
    "table-column-group",
    "table-footer-group",
    "table-header-group",
    "table-row",
    "table-row-group",
}

# The displays content-visibility applies to that say their kind alone: an
# atomic inline's, or a table cell's; an inline's, contents or another of a
# table's insides, it skips nothing of.
CONTAINING = {"inline-block", "inline-flex", "inline-grid", "inline-table", "table-cell"}

# What a visibility or content-visibility may say.
VISIBILITIES = {"visibility": {"visible", "hidden", "collapse"}}
VISIBILITIES["content-visibility"] = {"visible", "hidden", "auto"}

# A number, or a percentage, as an opacity is written.
AMOUNT = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?%?")


# How a browser takes a declaration: keeping a value this reads, dropping one
# it can't parse, so what was declared before stands, or keeping one this
# can't read, which settles nothing here.
KEPT, DROPPED, UNREAD = "kept", "dropped", "unread"

# What a value reads only once the page is drawn, which a browser keeps unread.
SUBSTITUTED = re.compile(r"(?<![\w-])(?:var|env|attr|if)\(")


def judged(name: str, value: str) -> str:
    """Say how a browser takes `value` for `name`: kept as read, dropped, or kept unread here."""
    if value == "":
        return DROPPED
    if name != "display" and name not in VISIBILITIES:
        return KEPT
    if SUBSTITUTED.search(value):
        return UNREAD
    if value in GLOBAL:
        return KEPT
    known = displays(value.split()) if name == "display" else value in VISIBILITIES[name]
    if known:
        return KEPT
    # A vendor's own value, as -webkit-box is, Chromium may keep; it drops any other.
    return UNREAD if value.startswith("-") else DROPPED


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

    def __init__(
        self,
        tag: str,
        attributes: dict[str, str],
        parent: Element | None,
        namespace: str = HTML,
    ) -> None:
        """Take the element's tag and attributes, under its parent, in the namespace parsed into."""
        self.tag = tag
        self.attributes = attributes
        self.parent = parent
        self.namespace = namespace
        self.html = namespace == HTML
        self.children: list[Element] = []
        self.content: list[Element | str] = []

    def classes(self) -> list[str]:
        """Give the classes it's marked with."""
        return self.attributes.get("class", "").split()

    def is_part(self, tag: str, name: str | None) -> bool:
        """Say whether it's drawn as the part with this tag and class."""
        return self.tag == tag and (name is None or name in self.classes())

    def cascaded(self, kinds: set[str]) -> dict[str, tuple[str, str]]:
        """Give each property's winning value in its own style, of those `kinds`, with its kind."""
        # The last declaration of a property wins, but not over an important one;
        # a dropped one never does, so what was declared before it stands.
        won: dict[str, tuple[str, str, bool]] = {}
        style = COMMENT.sub(" ", self.attributes.get("style", ""))
        for name, colon, value in (each.partition(":") for each in style.split(";")):
            said = value.strip(SPACE).lower()
            weighty = IMPORTANT.search(said) is not None
            key = name.strip(SPACE).lower()
            kept = IMPORTANT.sub("", said).strip(SPACE)
            kind = judged(key, kept)
            if colon and kind in kinds and (weighty or not won.get(key, ("", "", False))[2]):
                won[key] = (kept, kind, weighty)
        return {key: (value, kind) for key, (value, kind, _) in won.items()}

    def styled(self) -> dict[str, str]:
        """Give what its own style declares, each property's value as a browser takes it.

        A value this can't read is left out, and what was declared before it
        stands, which fails closed for the head: hidden stays hidden.
        """
        return {key: value for key, (value, _) in self.cascaded({KEPT}).items()}

    def doubts(self, name: str) -> bool:
        """Say whether `name`'s winning value in its style is one this can't read.

        One a browser drops settles nothing, and one before a later value
        this reads, or under an important one, is beaten.
        """
        return self.cascaded({KEPT, UNREAD}).get(name, ("", KEPT))[1] == UNREAD

    def until_found(self) -> bool:
        """Say whether it's hidden until found, which find-in-page or a fragment link shows."""
        return self.html and self.attributes.get("hidden", "").strip(SPACE).lower() == "until-found"

    def kept(self) -> bool:
        """Say whether HTML's own sheet gives it display: none that no person shows without script.

        Hidden, a tag the sheet hides and audio with no controls are; what's
        hidden until found and a popover are shown with none. HTML's sheet
        draws only HTML's elements, so an SVG element marked hidden is drawn.
        """
        if not self.html:
            return False
        hidden = "hidden" in self.attributes and not self.until_found()
        silent = self.tag == "audio" and "controls" not in self.attributes
        return hidden or silent or self.tag in SHEET_HIDDEN

    def held(self) -> bool:
        """Say whether HTML's own sheet draws none of it until a script or a person shows it."""
        # The element's own display beats the browser's, unless it rolls back
        # to it; hidden=until-found draws nothing it holds whatever it says.
        rolled_back = self.styled().get("display", "revert") in ROLLED_BACK
        popover = self.html and "popover" in self.attributes
        return ((self.kept() or popover) and rolled_back) or self.until_found()

    def hides(self) -> bool:
        """Say whether it draws none of itself: held by HTML's own sheet, or styled out."""
        # A hidden link or meta still does its work; only what's drawn is hidden.
        if self.tag in HEAD_ONLY:
            return False
        styled = self.styled()
        return (
            self.held()
            or any(each in STYLED_OUT for each in styled.items())
            or not opaque(styled.get("opacity", "1"))
        )

    def gone(self) -> bool:
        """Say whether, beyond doubt, nothing draws it or anything it holds, and none hears it."""
        # Only display: none and what keeps it do, and an element SVG doesn't
        # know; an opacity of nothing is still heard, a child may show itself
        # again under visibility, and a person may show what's hidden until
        # found or a popover.
        if self.tag in HEAD_ONLY:
            return False
        if self.namespace == "svg" and self.tag in UNKNOWN_TO_SVG:
            return True
        styled = self.styled()
        kept = self.kept() and styled.get("display", "revert") in ROLLED_BACK
        return (kept or styled.get("display") == "none") and not self.doubts("display")

    def contains(self) -> bool:
        """Say whether content-visibility applies to its box, as CSS Containment says it does.

        It does to a block's, an atomic inline's or a table cell's, by its tag or
        its own display, and not to an inline's, contents or a table's insides,
        nor where this can't read the display, which fails closed for rivals.
        """
        if not self.html or self.doubts("display"):
            return False
        display = self.styled().get("display", "revert")
        if display in ROLLED_BACK:
            return self.tag in BLOCKS
        words = display.split()
        if display in GLOBAL or (len(words) == 1 and words[0] in ALONE):
            return display in CONTAINING
        inner = next((word for word in words if word in INNER), "flow")
        outer = next(
            (word for word in words if word in OUTER),
            "inline" if inner in {"ruby", "math"} else "block",
        )
        return outer == "block" or inner not in {"flow", "ruby", "math"}

    def unseen(self, *, above: bool) -> bool:
        """Say whether it's drawn invisible, by its own visibility or that of what holds it."""
        if self.doubts("visibility"):
            return False
        said = self.styled().get("visibility")
        if said in {"hidden", "collapse"}:
            return True
        if said in {"visible", "initial"}:
            return False
        return above

    def within(self) -> list[Element]:
        """Give every element drawn inside it, in the order they're written."""
        found = []
        for child in self.children:
            if child.hides():
                continue
            found.append(child)
            if child.tag in SHUT and "open" not in child.attributes:
                # Shut details draw their first summary alone, and none if it's hidden.
                summary = next((each for each in child.children if each.tag == "summary"), None)
                if child.tag == "details" and summary is not None and not summary.hides():
                    found.append(summary)
                    found.extend(summary.within())
            # SVG and MathML draw none of what they hold as the page's own,
            # HTML they hold again included: a description, a symbol or a
            # frame of no size draws nothing, which fails closed for the head.
            # What may not be there is sought in rivals(), which reads them.
            elif child.tag not in UNDRAWN and child.tag not in FOREIGN:
                found.extend(child.within())
        return found

    def rivals(self, *, above: bool = False) -> list[Element]:
        """Give every element inside it a browser may draw or a screen reader hear, in order.

        This is the generous walk: what the page may not hold beside its head
        is sought here, so each doubt counts it, as the module says, a shut
        details' content and a popover among them. Only what's surely undrawn
        and unheard is left out.
        """
        found = []
        for child in self.children:
            if child.gone():
                continue
            unseen = child.unseen(above=above)
            if not unseen:
                found.append(child)
            skipped = (
                child.styled().get("content-visibility") == "hidden"
                and not child.doubts("content-visibility")
                and child.contains()
            )
            if child.html and (
                child.tag in {"template", *MEDIA}
                or (child.tag == "dialog" and "open" not in child.attributes and not child.opens())
                or skipped
            ):
                continue
            found.extend(child.rivals(above=unseen))
        return found

    def opens(self) -> bool:
        """Say whether a person may open it with no script: a popover, or a dialog a button shows.

        A button's commandfor naming its id, with show-modal for its command,
        opens a dialog when pressed, wherever the button sits on the page.
        """
        if "popover" in self.attributes:
            return True
        name = self.attributes.get("id", "")
        page = self
        while page.parent is not None:
            page = page.parent
        return name != "" and any(
            each.html
            and each.tag == "button"
            and each.attributes.get("commandfor") == name
            and each.attributes.get("command", "").lower() in OPENING_COMMANDS
            for each in page.written()
        )

    def why(self) -> str:
        """Say why the generous walk counts it though the strict one doesn't, nearest first."""
        element: Element | None = self
        while element is not None:
            doubt = element.doubt()
            if doubt is not None:
                return doubt
            element = element.parent
        return "where a browser may draw it"

    def doubt(self) -> str | None:
        """Say what about it alone leaves what it holds to doubt, or nothing."""
        if self.tag in FOREIGN:
            return f"held in {'SVG' if self.tag == 'svg' else 'MathML'}"
        if not self.html:
            return None
        styled = self.styled()
        shut = "open" not in self.attributes
        invisible = styled.get("visibility") in {"hidden", "collapse"}
        skips = styled.get("content-visibility") == "hidden"
        doubts = {
            f"a <{self.tag}>'s fallback": self.tag in FALLBACKS,
            f"held in a <{self.tag}>": self.tag in GAUGES,
            "hidden until found": self.until_found(),
            "a popover": "popover" in self.attributes and self.held(),
            "past a shut <details>'s summary": self.tag == "details" and shut,
            "in a <dialog> a button opens": self.tag == "dialog" and shut,
            "a style this can't read": any(self.doubts(name) for name in DOUBTED),
            "drawn at no opacity": not opaque(styled.get("opacity", "1")),
            "shown again under visibility: hidden": invisible,
            "in a box content-visibility: hidden skips nothing of": skips,
        }
        return next((doubt for doubt, holds in doubts.items() if holds), None)

    def shown(self) -> list[Element]:
        """Give the children drawn in it, leaving out those hidden."""
        return [child for child in self.children if not child.hides()]

    def written(self) -> list[Element]:
        """Give every element parsed into the page's tree inside it, drawn or not.

        A template's content is a fragment of its own, apart from the tree;
        raw text holds no elements, so what media, canvas or an object holds
        is read here.
        """
        found = []
        for child in self.children:
            found.append(child)
            if child.tag != "template":
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

    def focusable(self) -> bool:
        """Say whether it may take focus: a button, or what's marked with a tabindex."""
        return self.html and (self.tag == "button" or "tabindex" in self.attributes)

    def flattens(self) -> bool:
        """Say whether a screen reader hears what it holds only as its words."""
        role = self.role()
        if role in IGNORED_ON_FOCUS and self.focusable():
            role = None
        return (role or IMPLICIT.get(self.tag)) in FLATTENING

    def silenced(self) -> bool:
        """Say whether it's marked so no screen reader hears it: aria-hidden or inert."""
        hidden = self.attributes.get("aria-hidden", "").strip(SPACE).lower() == "true"
        return hidden or "inert" in self.attributes

    def heard(self) -> bool:
        """Say whether a screen reader hears it as itself: nothing undrawn, inert or flattened is.

        Neither is what's under aria-hidden, or held by a role, a button say,
        whose children are heard only as its words.
        """
        if self.silenced() or self.hides():
            return False
        element = self.parent
        while element is not None:
            if element.silenced() or element.hides() or element.flattens():
                return False
            element = element.parent
        return True

    def may_be_heard(self) -> bool:
        """Say whether a screen reader may hear it: nothing surely gone or silenced holds it."""
        element: Element | None = self
        while element is not None:
            if element.silenced() or element.gone():
                return False
            element = element.parent
        return True

    def speaks(self) -> bool:
        """Say whether it's marked to speak its changes; an explicit aria-live decides."""
        said = self.attributes.get("aria-live", "").strip(SPACE).lower()
        if said in {"polite", "assertive", "off"}:
            return said != "off"
        return self.role() in LIVE_ROLES

    def live(self) -> bool:
        """Say whether a screen reader surely speaks its changes."""
        return self.heard() and self.speaks()

    def may_speak(self) -> bool:
        """Say whether a screen reader may speak its changes."""
        return self.may_be_heard() and self.speaks()

    def section(self) -> Element | None:
        """Give the section it sits in, by its tag or the role it's marked with, if any."""
        section = self.parent
        while section is not None and not (
            (section.html and section.tag in SECTIONS) or section.role() in SECTION_ROLES
        ):
            section = section.parent
        return section

    def is_banner(self) -> bool:
        """Say whether it's heard as the page's banner: marked one, or a header in no section."""
        role = self.role()
        return role == "banner" or (
            self.html and self.tag == "header" and role is None and self.section() is None
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
    element = Element(tag, attributes, parent, namespace_of(tag, parent))
    parent.children.append(element)
    parent.content.append(element)
    # A template's content sits apart from the page's tree, which a script
    # copies out to use; it's read as the template's own. SVG's is no template.
    holders = [node]
    if tag == "template" and element.html:
        holders = [
            fragment
            for fragment in node.template_fragments()
            if fragment.parent is not None and fragment.parent.mem_id == node.mem_id
        ]
    for holder in holders:
        for child in holder.iter(include_text=True):
            adopt(child, element)


def namespace_of(tag: str, parent: Element) -> str:
    """Give the namespace HTML's parser puts an element written `tag` in, under `parent`.

    SVG and MathML begin at their own tags, and HTML again inside what holds it
    there. What HTML ends them at, a <p> say, the parser has already moved out.
    """
    if tag in FOREIGN:
        return tag
    if parent.html:
        return HTML
    if tag in MATH_GLYPHS and parent.namespace == "math":
        return parent.namespace
    encoding = parent.attributes.get("encoding", "").strip(SPACE).lower()
    annotates = parent.tag == "annotation-xml" and encoding in HTML_ANNOTATIONS
    if annotates or parent.tag in HOLDS_HTML.get(parent.namespace, set()):
        return HTML
    return parent.namespace


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
    # What may not be there beside the head is sought among all a browser
    # may draw or hear, which holds all drawn, the head included.
    rivals = document.rivals()
    drawn = set(everything)
    faults.extend(banner_faults(head, rivals, document.every(), drawn))
    faults.extend(title_faults(head, rivals, drawn))
    if any(isinstance(each, str) and each.strip(SPACE) for each in head.content):
        faults.append("its head writes words outside its parts: keep every word in a part")
    return faults + parts_faults(head) + speaking_faults(head)


def doubted(element: Element, drawn: set[Element]) -> tuple[str, str]:
    """Give the verb and the doubt a fault names a rival with: drawn, or why it may be."""
    if element in drawn:
        return "draws", ""
    return "may draw", f" ({element.why()})"


def banner_faults(
    head: Element, everything: list[Element], written: list[Element], drawn: set[Element]
) -> list[str]:
    """Say what the page must change for the head to be its one banner, its skip link before.

    A rival only the generous walk finds is named as one a browser may draw, and why.
    """
    faults = []
    banners = [element for element in everything if element.is_banner()]
    if banners and banners[0] is not head:
        if head.inside(banners[0]):
            where = "around"
        elif everything.index(banners[0]) < everything.index(head):
            where = "above"
        else:
            where = "below"
        verb, why = doubted(banners[0], drawn)
        faults.append(
            f"it {verb} a {named(banners[0])} {where} its head{why}:"
            " let the kit's head be its one banner"
        )
    # A banner inside the head is a second, as a header in a header is heard.
    later = banners[1:]
    if banners and banners[0] is head and later:
        where = "inside" if later[0].inside(head) else "below"
        verb, why = doubted(later[0], drawn)
        faults.append(
            f"it {verb} a {named(later[0])} {where} its head as well{why}:"
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


def title_faults(head: Element, everything: list[Element], drawn: set[Element]) -> list[str]:
    """Say what the page must change for the head's title to be its first heading.

    A heading only the generous walk finds is named as one it may be, and why.
    """
    titles = [element for element in everything if element.is_heading()]
    title = next((child for child in head.shown() if child.tag == "h1"), None)
    if not titles:
        return ["it draws no heading: make the head's title its first"]
    if titles[0] is title:
        return []
    first = named(titles[0])
    if titles[0] not in drawn:
        where, mend = "outside", "make the head's title first"
        if titles[0].inside(head):
            where, mend = "in", "make the head's title its first"
        return [f"its first heading may be a {first} {where} its head ({titles[0].why()}): {mend}"]
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
        # Its one status is drawn and surely heard, and nothing else may speak.
        live = [element for element in part.rivals() if element.may_speak()]
        if (
            len(live) != 1
            or live[0] not in inner
            or not live[0].live()
            or live[0].role() != "status"
            or live[0].interrupts()
        ):
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
        for element in head.rivals()
        if element.may_speak() and not any(element.inside(line) for line in standing)
    ]
    if speaking or head.may_speak():
        faults.append(
            f"{len(speaking) + head.may_speak()} more of its head speaks as it changes:"
            " only the standing line's status may be live"
        )
    # A live region speaks for all it holds, so one around the head speaks the whole head.
    around = []
    parent = head.parent
    while parent is not None:
        if parent.may_speak():
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


# What a page writing a shadow root must change, since its host then draws
# what it holds only where a slot in that root puts it. A template marked
# with the old name no longer attaches one, but is refused all the same.
SHADOWED = (
    "it writes a shadow root template, whose host would draw what it holds only where"
    " a <slot> puts it: write the page in its own tree"
)


# What a page marking aria-owns must change, since a screen reader then hears
# what it names inside the owner, not where it's written.
OWNED = (
    "it marks aria-owns, so a screen reader hears what it names somewhere other"
    " than where it's written: write each element where it's heard"
)


# What a page drawing another document must change, since what that document
# draws, a banner or a heading say, can't be read from this one's markup.
FRAMED = (
    "it draws another document in an <iframe>, an <embed> or an <object> with data,"
    " whose banner and headings can't be read here: draw the page in its own markup"
)


def frames(element: Element) -> bool:
    """Say whether `element` draws another document in the page: a frame, an embed or an object."""
    return element.html and (
        element.tag in {"iframe", "embed"}
        or (element.tag == "object" and "data" in element.attributes)
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
    # The parser here builds no shadow tree, so a page writing one is named;
    # nor does it read what aria-owns moves, or another document drawn in this.
    shadowed = [
        SHADOWED
        for element in parsed.every()
        if element.tag == "template" and SHADOW_ROOTS & element.attributes.keys()
    ][:1]
    written = parsed.written()
    owned = [OWNED] if any("aria-owns" in element.attributes for element in written) else []
    framed = [FRAMED] if any(frames(element) for element in written) else []
    return sheet + shadowed + owned + framed + skip_line_faults(markup) + head_faults(markup)


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

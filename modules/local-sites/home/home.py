"""Builds the home of Joel's local sites from the sites.json beside it.

It writes the list, its llms.txt and Markdown twin, and the strip at the top
of every page whose row asks for one, and puts the home's own scripts and
styles in place beside them. A row may name pages of its site that say their
own kind in the strip, as "pages": {"tree.html": "test"}. The home's
theme-color takes each look's ground from the kit's theme.css, so it turns
with the theme.

usage: home.py <sources> <root>, where sources holds the home's page and its
files, and root is the sites' folder. It rewrites index.html, index.md and
llms.txt whole, and in each stripped page only what sits between the strip's
two markers, so a run with nothing changed leaves every file as it was. A
root with no sites.json yet is left alone.
"""

import html
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

KINDS = {"mock": "Mock", "record": "Record", "explainer": "Explainer", "test": "Test", "app": "App"}
STATUS = {"withdrawn": "turned down, kept", "settled": "settled", "growing": ""}
# What the home serves of its own, copied in since the server follows no
# link out of its root.
FILES = ("home.js", "home.css", "theme.js", "strip.css")
OPEN, CLOSE = "<!-- sites strip: home.py writes this -->", "<!-- /sites strip -->"
GROUND = re.compile(r"--background:\s*([^;]+);")
LIGHT = "@media (prefers-color-scheme: light)"


@dataclass(frozen=True)
class Site:
    """One row of sites.json: a site, or one page of a site's own."""

    path: str
    name: str
    what: str
    asks: str
    when: str
    kind: str
    status: str
    strip: bool
    pages: dict[str, str]

    @classmethod
    def read(cls, row: dict[str, Any]) -> Site:
        """Read a row as sites.json writes it, an absent field taken as empty."""
        pages = row.get("pages") or {}
        return cls(
            path=str(row["path"]),
            name=str(row["name"]),
            what=str(row.get("what") or ""),
            asks=str(row.get("asks") or ""),
            when=str(row.get("when") or ""),
            kind=str(row["kind"]),
            status=str(row.get("status") or "growing"),
            strip=bool(row.get("strip")),
            pages={str(page): str(kind) for page, kind in pages.items()},
        )

    @property
    def kept(self) -> bool:
        """Whether it sits under Kept for Reference, turned down or settled."""
        return self.status in {"withdrawn", "settled"}

    def says(self, kind: str) -> str:
        """Its kind, then how it stands when that's worth saying."""
        status = STATUS.get(self.status, "")
        return KINDS[kind] + (f" · {status}" if status else "")


def row(site: Site) -> str:
    """Write one site's line in the home's list."""
    e = html.escape
    asks = f'<span class="asks">{e(site.asks)}</span>' if site.asks else ""
    return (
        f'    <li><a href="{e(site.path)}"><span class="name">{e(site.name)}</span>'
        f'<span class="when">{e(site.when)}</span>'
        f'<span class="kind">{e(site.says(site.kind))}</span>'
        f'<span class="what">{e(site.what)}</span>{asks}</a></li>'
    )


def home(page: str, sites: list[Site]) -> str:
    """Write the home's list into the page's shell."""
    fresh = [site for site in sites if not site.kept]
    kept = [site for site in sites if site.kept]
    listing = ['  <ul class="sites" id="sites" aria-label="Sites">', *map(row, fresh), "  </ul>"]
    if kept:
        listing += [
            '  <h2 class="shelf">Kept for Reference</h2>',
            '  <ul class="sites" aria-label="Kept for Reference">',
            *map(row, kept),
            "  </ul>",
        ]
    start = page.index('  <ul class="sites"')
    end = page.index("</main>")
    return page[:start] + "\n".join(listing) + "\n" + page[end:]


def grounds(page: str, root: Path) -> str:
    """Fill the page's theme-color with each look's ground from the kit's theme.css.

    The kit writes the dark look first and the light one under its media query,
    so the first ground on each side of that query is the look's own. With no
    kit, the page names no ground rather than one that would drift from it.
    """
    theme = root / "kit" / "theme.css"
    css = theme.read_text(encoding="utf-8") if theme.exists() else ""
    dark_side, _, light_side = css.partition(LIGHT)
    dark, light = GROUND.search(dark_side), GROUND.search(light_side)
    if not (dark and light):
        return "".join(line for line in page.splitlines(keepends=True) if "-ground@" not in line)
    return page.replace("@dark-ground@", dark[1].strip()).replace(
        "@light-ground@", light[1].strip()
    )


def llms(root: Path, sites: list[Site]) -> str:
    """Write the home's llms.txt, each site linking its own twin where it has one."""
    lines = [
        "# Joel's local sites",
        "",
        "> Every site made for Joel on this laptop, newest first, each saying what it is",
        "> and what it asks of him. Built from sites.json.",
        "",
    ]
    for site in sites:
        target = site.path + ("llms.txt" if site.path.endswith("/") else "")
        if target.endswith(".html") and (root / f"{target[:-5]}.md").exists():
            target = f"{target[:-5]}.md"
        asks = f" Asks: {site.asks}" if site.asks else ""
        lines.append(
            f"- [{site.name}]({target}): {KINDS[site.kind].lower()}, {site.when}. {site.what}{asks}"
        )
    return "\n".join(lines) + "\n"


def twin(sites: list[Site]) -> str:
    """Write the home's Markdown twin: the same list, for a reader that takes text."""
    lines = [
        "# Your Sites",
        "",
        "Everything made just for you lives here, on this laptop, newest first.",
        "",
    ]
    for site in sites:
        asks = f" Asks: {site.asks}" if site.asks else ""
        lines.append(
            f"- [{site.name}]({site.path}) ({KINDS[site.kind].lower()}, {site.when}): "
            f"{site.what}{asks}"
        )
    return "\n".join(lines) + "\n"


def strip(site: Site, page: str, kind: str) -> str:
    """Write the strip a page opens with: the way home, its kind, when, and its ask."""
    e = html.escape
    way_home = "../" * page.count("/") or "./"
    status = STATUS.get(site.status, "")
    bits = [
        f'<a href="{way_home}">All your sites</a>',
        f"<span>{e(KINDS[kind])}</span>",
        f"<span>as of {e(site.when)}</span>",
    ]
    if status:
        bits.append(f"<span>{e(status)}</span>")
    if site.asks:
        bits.append(f'<span class="asks">Asks: {e(site.asks)}</span>')
    return (
        f'{OPEN}\n<link rel="stylesheet" href="{way_home}strip.css">\n'
        f'<nav class="sites-strip" aria-label="Your sites">{" ".join(bits)}</nav>\n{CLOSE}'
    )


def stripped(text: str, block: str) -> str:
    """Put a page's strip in place, replacing the old one or putting a new one first."""
    if OPEN in text:
        old = re.escape(OPEN) + ".*?" + re.escape(CLOSE)
        return re.sub(old, lambda _: block, text, flags=re.DOTALL)
    # After the skip link, so it stays the first thing a Tab reaches.
    skip = re.search(r"<a class=\"skip\"[^\n]*\n", text)
    at = skip.end() if skip else text.index("<body>\n") + len("<body>\n")
    return text[:at] + "\n" + block + "\n" + text[at:]


def write(path: Path, text: str) -> None:
    """Write a file only when it changes, so a run with nothing new touches nothing.

    It writes a copy beside the file and renames it over, so the server never
    sends a page half written.
    """
    if not path.exists() or path.read_text(encoding="utf-8") != text:
        fresh = path.with_name(f".{path.name}.new")
        fresh.write_text(text, encoding="utf-8")
        fresh.replace(path)


def build(sources: Path, root: Path) -> str:
    """Build the home in root from its sources and root's sites.json, saying what it did."""
    record = root / "sites.json"
    if not record.exists():
        return "home: no sites.json yet, so nothing to build"
    # A record caught half written, or broken by a slip, leaves the last good
    # build serving and says why, since the next change to it builds again.
    try:
        rows = json.loads(record.read_text(encoding="utf-8"))["sites"]
        sites = [Site.read(row) for row in rows]
    except (ValueError, KeyError, TypeError, AttributeError) as error:
        return f"home: sites.json can't be read ({error}), so the last build stays"
    for name in FILES:
        write(root / name, (sources / name).read_text(encoding="utf-8"))
    shell = (sources / "index.html").read_text(encoding="utf-8")
    write(root / "index.html", grounds(home(shell, sites), root))
    write(root / "llms.txt", llms(root, sites))
    write(root / "index.md", twin(sites))

    pages: dict[str, tuple[Site, str]] = {}
    for site in (site for site in sites if site.strip):
        pages[site.path + ("index.html" if site.path.endswith("/") else "")] = (site, site.kind)
        pages.update({site.path + page: (site, kind) for page, kind in site.pages.items()})
    for page, (site, kind) in pages.items():
        path = root / page
        if path.exists():
            write(path, stripped(path.read_text(encoding="utf-8"), strip(site, page, kind)))
    kept = sum(site.kept for site in sites)
    return f"home: {len(sites) - kept} sites, {kept} kept; strips on {len(pages)} pages"


if __name__ == "__main__":
    if len(sys.argv) != 3:  # noqa: PLR2004 - the program and its two folders
        sys.exit("usage: home.py <sources> <root>")
    print(build(Path(sys.argv[1]), Path(sys.argv[2])))

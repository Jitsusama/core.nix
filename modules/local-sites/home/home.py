"""Builds the home of Joel's local sites from the sites.json beside it.

It writes the list, its llms.txt and Markdown twin, and the strip at the top
of every page whose row asks for one, and puts the home's own scripts and
styles in place beside them. A row may name pages of its site that say their
own kind in the strip, as "pages": {"tree.html": "test"}. The home's
theme-color takes each look's ground from the kit's theme.css, so it turns
with the theme. A document, any row but an app, takes its keys and theme
switch from the kit's /local-sites/kit/document.js, so what it says names
every document loading a script file of its own, which is a copy that
won't learn what the kit does next.

A live site's row comes from its declaration in Nix rather than the record,
so what the home says of it changes as the site does. The rows the Nix
config declares, as declared.json writes them, stand in for the record's row
at the same path, however it is spelled, or go first if it has none.

usage: home.py <sources> <root> [<declared>], where sources holds the home's
page and its files, root is the sites' folder, and declared is the rows the
Nix config declares. It rewrites index.html, index.md and
llms.txt whole, and in each stripped page only what sits between the strip's
two markers, so a run with nothing changed leaves every file as it was.
Without declared rows, a root with no sites.json yet is left alone; with
them, they make the home, the root made if need be.
"""

import html
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

KINDS = {"mock": "Mock", "record": "Record", "explainer": "Explainer", "test": "Test", "app": "App"}
ROMAN = ("I", "II", "III", "IV", "V", "VI")
# Where the shell's sites go, from this marker to the end of its main.
BEDS = "  <!-- the sites: home.py writes these -->"
STATUS = {"withdrawn": "turned down, kept", "settled": "settled", "growing": ""}
# What the home serves of its own, copied in since the server follows no
# link out of its root.
FILES = ("home.js", "home.css", "theme.js", "strip.css")
OPEN, CLOSE = "<!-- sites strip: home.py writes this -->", "<!-- /sites strip -->"
GROUND = re.compile(r"--background:\s*([^;]+);")
# A script file a page loads, by its address however it's quoted.
SCRIPT_SRC = re.compile(r"<script\b[^>]*?\bsrc\s*=\s*([\"']?)([^\"'\s>]+)\1", re.IGNORECASE)
# Where the kit's own scripts are served from, the only ones a document loads.
KIT = "/local-sites/kit/"
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


@dataclass(frozen=True)
class Bed:
    """One part of the home: the sites of some kinds, under a heading with a turn."""

    anchor: str
    name: str
    turn: str
    kinds: frozenset[str]


# The home's beds, in the order it reads: what runs live, what explains,
# what asks to be tried, and what records. Anything settled or turned down
# goes under Kept, whatever its kind.
BEDS_IN_ORDER = (
    Bed("live", "Live", "as it happens.", frozenset({"app"})),
    Bed("explained", "Explained", "in one page.", frozenset({"explainer"})),
    Bed("to-try", "To try", "and say.", frozenset({"mock", "test"})),
    Bed("on-record", "On record", "as it was.", frozenset({"record"})),
)
KEPT = Bed("kept", "Kept", "for reference.", frozenset(KINDS))


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


def beds(sites: list[Site]) -> list[tuple[Bed, list[Site]]]:
    """Sort the sites into the home's beds, in order, leaving out the empty ones."""
    fresh = [site for site in sites if not site.kept]
    placed = [(bed, [site for site in fresh if site.kind in bed.kinds]) for bed in BEDS_IN_ORDER]
    placed.append((KEPT, [site for site in sites if site.kept]))
    return [(bed, rows) for bed, rows in placed if rows]


def bed(number: str, bed: Bed, rows: list[Site]) -> list[str]:
    """Write one bed: its numbered heading with its turn, then its sites a row each."""
    return [
        f'  <section class="bed" aria-labelledby="{bed.anchor}">',
        (
            f'    <h2 id="{bed.anchor}"><span class="numeral">{number}.</span> '
            f"{bed.name}, <em>{bed.turn}</em></h2>"
        ),
        f'    <ul class="sites" aria-label="{bed.name}">',
        *("  " + line for line in map(row, rows)),
        "    </ul>",
        "  </section>",
    ]


def standing(sites: list[Site]) -> str:
    """Say in one line how the home stands: how many grow, how many ask, how many are kept."""
    fresh = [site for site in sites if not site.kept]
    asking = sum(bool(site.asks) for site in fresh)
    kept = len(sites) - len(fresh)
    said = [f"{len(fresh)} growing", f"{asking} asking something of you"]
    if kept:
        said.append(f"{kept} kept")
    glyph = '<span class="glyph" aria-hidden="true"></span>'
    return glyph + ' <span class="sep">·</span> '.join(said)


def home(page: str, sites: list[Site]) -> str:
    """Write the home's beds, its contents and its standing line into the page's shell."""
    placed = beds(sites)
    listing = [line for n, (b, rows) in enumerate(placed) for line in bed(ROMAN[n], b, rows)]
    contents = "\n".join(
        f'      <li><a href="#{b.anchor}"><span class="numeral">{ROMAN[n]}</span>'
        f"<em>{b.name}</em></a></li>"
        for n, (b, _) in enumerate(placed)
    )
    start = page.index(BEDS) + len(BEDS) + 1
    end = page.index("</main>")
    page = page[:start] + "\n".join(listing) + "\n" + page[end:]
    return page.replace("@contents@", contents).replace("@standing@", standing(sites))


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
        # An app's own line says how live it is, so the strip says only when
        # it was built, never a word that reads as how it stands.
        f"<span>{'built' if kind == 'app' else 'as of'} {e(site.when)}</span>",
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


def inside(root: Path, page: str) -> Path | None:
    """Find the file a row names under root, or None when the name would lead out of it."""
    path = (root / page).resolve()
    return path if path.is_relative_to(root.resolve()) else None


def own_scripts(root: Path, sites: list[Site]) -> list[str]:
    """Name each document whose pages load a script file of their own, with the files.

    A page that can't be read is passed over rather than failing the build,
    since this only reports.
    """
    owning = []
    for site in (site for site in sites if site.kind != "app"):
        own = site.path + ("index.html" if site.path.endswith("/") else "")
        scripts = []
        for page in [own, *(site.path + page for page in site.pages)]:
            path = inside(root, page)
            try:
                text = path.read_text(encoding="utf-8", errors="replace") if path else ""
            except OSError:
                continue
            sources = (source for _, source in SCRIPT_SRC.findall(text))
            scripts += [s for s in sources if not s.startswith(KIT) or ".." in s]
        if scripts:
            # Each named once, though several of its pages load it.
            owning.append(f"{site.path} ({', '.join(dict.fromkeys(scripts))})")
    return owning


def write(path: Path, text: str) -> None:
    """Write a file only when it changes, so a run with nothing new touches nothing.

    It writes a copy beside the file and renames it over, so the server never
    sends a page half written.
    """
    if not path.exists() or path.read_text(encoding="utf-8") != text:
        fresh = path.with_name(f".{path.name}.new")
        fresh.write_text(text, encoding="utf-8")
        fresh.replace(path)


def declaring(recorded: list[Site], declared: list[Site]) -> list[Site]:
    """Put each declared row in its recorded row's place, or first when none is recorded.

    Paths match however a session spelled them ("grove", "./grove/"), and a
    path the record holds twice keeps only its first place, so a declared
    site is listed once.
    """
    by_path = {folder(site.path): site for site in declared}
    recorded_paths = {folder(site.path) for site in recorded}
    placed = [site for site in declared if folder(site.path) not in recorded_paths]
    taken: set[str] = set()
    for site in recorded:
        key = folder(site.path)
        if key not in by_path:
            placed.append(site)
        elif key not in taken:
            placed.append(by_path[key])
            taken.add(key)
    return placed


def folder(path: str) -> str:
    """Say a row's path as one folder name and a slash, however it was spelled."""
    return "/".join(part for part in path.split("/") if part not in {"", "."}) + "/"


def build(sources: Path, root: Path, declared: Path | None = None) -> str:
    """Build the home in root from its sources, root's sites.json and the declared rows.

    It says what it did.
    """
    record = root / "sites.json"
    # With no record yet, the declared rows alone make the home, making the
    # sites' folder on a machine's first switch, before any document is there.
    if not record.exists():
        if not declared:
            return "home: no sites.json yet, so nothing to build"
        root.mkdir(parents=True, exist_ok=True)
    # A record caught half written, or broken by a slip, leaves the last good
    # build serving and says why, since the next change to it builds again.
    try:
        rows = json.loads(record.read_text(encoding="utf-8"))["sites"] if record.exists() else []
        sites = [Site.read(row) for row in rows]
    except (ValueError, KeyError, TypeError, AttributeError) as error:
        return f"home: sites.json can't be read ({error}), so the last build stays"
    # Nix wrote the declared rows, so a fault in them is the build's, and fails it.
    if declared:
        live = [Site.read(row) for row in json.loads(declared.read_text(encoding="utf-8"))["sites"]]
        sites = declaring(sites, live)
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
        path = inside(root, page)
        if path and path.exists():
            write(path, stripped(path.read_text(encoding="utf-8"), strip(site, page, kind)))
    kept = sum(site.kept for site in sites)
    said = f"home: {len(sites) - kept} sites, {kept} kept; strips on {len(pages)} pages"
    owning = own_scripts(root, sites)
    return said + (f"; scripts of their own in {', '.join(owning)}" if owning else "")


if __name__ == "__main__":
    # The program and its two folders, then the declared rows when given.
    folders, with_rows = 3, 4
    if len(sys.argv) not in {folders, with_rows}:
        sys.exit("usage: home.py <sources> <root> [<declared>]")
    rows = Path(sys.argv[folders]) if len(sys.argv) == with_rows else None
    print(build(Path(sys.argv[1]), Path(sys.argv[2]), rows))

"""Builds the home of Joel's local sites from the sites.json beside it.

It writes the list, its llms.txt and Markdown twin, and the strip at the top
of every page whose row asks for one, and puts the home's own styles in
place beside them; its keys and theme switch are the kit's
/local-sites/kit/home.js. A row may name pages of its site that say their
own kind in the strip, as "pages": {"tree.html": "test"}. The home reads
the theme's web kit from the one copy the sites share, /local-sites/web/,
and its theme-color takes each look's ground from that kit's theme.css, and
its favicon each look's tint, so both turn with the theme. A document, any
row but an app, takes its keys and theme switch from the kit's
/local-sites/kit/document.js, so what it says names every document loading
a script file of its own, which is a copy that won't learn what the kit
does next.

A live site's row comes from its declaration in Nix rather than the record,
so what the home says of it changes as the site does. The rows the Nix
config declares, as declared.json writes them, stand in for the record's row
at the same path, however it is spelled, or go first if it has none. Every
row the home lists, each as it was written, goes into rows.json beside it,
for the pages that read what each site is home to, so they read a live
site's row as its declaration says it rather than the record's. A kind a
declared row is home to stays its home: a recorded row's claim on it, by name
or by pattern, is left out of rows.json and said, since the readers would
otherwise give the kind to whichever row came first, or fault both. rows.json
says each claim taken, beside its rows, for the web-design skill's reader to
name in sites.json.

usage: home.py [--into=<home>] [--kept=<record>] <sources> <root> <theme>
[<declared>], where sources holds the home's page and its files, root is the
sites' folder, theme is the web kit's theme.css the sites share, and declared is
the rows the Nix config declares. The home's own files go into home where it's
given, a folder sessions don't write, which the server serves ahead of the
documents as the sites' own rather than as a document, so the home can read
Joel's feeds; any copy of them left in root from before is taken away. Each
record that reads is kept at record where it's given, another place sessions
don't write, to build from when sites.json won't read and no build is there. It
rewrites index.html, index.md, rows.json and llms.txt whole, and in each
stripped page only what sits between the strip's two markers, so a run with
nothing changed leaves every file as it was. Without declared rows, a root with
no sites.json yet is left alone; with them, they make the home, the root made
if need be.
"""

import html
import json
import math
import re
import sys
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Any, NoReturn

KINDS = {"mock": "Mock", "record": "Record", "explainer": "Explainer", "test": "Test", "app": "App"}
ROMAN = ("I", "II", "III", "IV", "V", "VI")
# Where the shell's sites go, from this marker to the end of its main.
BEDS = "  <!-- the sites: home.py writes these -->"
STATUS = {"withdrawn": "turned down, kept", "settled": "settled", "growing": ""}
# What the home serves of its own, copied in since the server follows no
# link out of its root.
FILES = ("home.css", "strip.css")
# What the home served of its own before its keys and switch were the kit's,
# taken away so no page can go on loading a copy that won't learn.
GONE = ("home.js", "theme.js")
# What the home writes of its own, beside FILES, which go where it's told.
OWN = ("favicon.svg", "index.html", "llms.txt", "index.md", "rows.json")
INTO = "--into="
KEEP = "--kept="
# What a record that won't read raises as it's read.
UNREADABLE = (ValueError, KeyError, TypeError, AttributeError)
OPEN, CLOSE = "<!-- sites strip: home.py writes this -->", "<!-- /sites strip -->"
GROUND = re.compile(r"--background:\s*([^;]+);")
# What a site marks its emphasis in, unless it takes another hue: the
# action's, which the home's own styles call --tint.
TINT = re.compile(r"--action:\s*([^;]+);")
# A colour as a stylesheet writes one, slash alpha included, and nothing that
# could close the mark's <style>, start another rule or open a comment. A var()
# is left out too: a favicon can't read the page's properties.
COLOUR = re.compile(r"(?!.*(?:var\(|/\*))[#\w(). ,%/-]+")
# A script file a page loads, by its address however it's quoted.
SCRIPT_SRC = re.compile(r"<script\b[^>]*?\bsrc\s*=\s*([\"']?)([^\"'\s>]+)\1", re.IGNORECASE)
# Where the kit's own scripts are served from, the only ones a document loads.
KIT = "/local-sites/kit/"
# A step of a site's path the home links to: nothing a browser reads as a
# scheme, host, space or escape, and no dot first: the step system's homes keep,
# which home.nix's astray spells again for the rows Nix declares.
STEP = re.compile(r"[A-Za-z0-9_~-][A-Za-z0-9._~-]*")
LIGHT = "@media (prefers-color-scheme: light)"
USAGE = "usage: home.py [--into=<home>] [--kept=<record>] <sources> <root> <theme> [<declared>]"


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
    # The row as it was written, every field kept, for rows.json.
    row: dict[str, Any] = field(compare=False, repr=False)

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
            row=row,
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
    """Say in one line how the home stands: how many grow, how many ask, how many are kept.

    Only the words: the page's standing line holds its glyph and the status
    they're written in, as every live page's head does.
    """
    fresh = [site for site in sites if not site.kept]
    asking = sum(bool(site.asks) for site in fresh)
    kept = len(sites) - len(fresh)
    said = [f"{len(fresh)} growing", f"{asking} asking something of you"]
    if kept:
        said.append(f"{kept} kept")
    return ' <span class="sep">·</span> '.join(said)


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


def looks(theme: Path) -> dict[str, str]:
    """Each look's ground and tint in the kit's theme.css, by the placeholder each fills.

    The kit writes the dark look first and the light one under its media query,
    so the first of each on either side of that query is the look's own. What
    the kit doesn't name, or doesn't write as a colour, or a theme that isn't
    there, is left out, and a value refused as no colour is said on stderr.
    """
    css = theme.read_text(encoding="utf-8") if theme.exists() else ""
    dark_side, _, light_side = css.partition(LIGHT)
    found = {}
    for look, side in (("dark", dark_side), ("light", light_side)):
        for role, pattern in (("ground", GROUND), ("tint", TINT)):
            named = pattern.search(side)
            if not named:
                continue
            if COLOUR.fullmatch(value := named[1].strip()):
                found[f"@{look}-{role}@"] = value
            else:
                print(f"home: the {look} look's {role}, {value!r}, is no colour", file=sys.stderr)
    return found


def grounds(page: str, theme: Path) -> str:
    """Fill the page's theme-color with each look's ground from the kit's theme.css.

    Without both, the page names no ground rather than one that would drift
    from the kit's.
    """
    kit = looks(theme)
    if not {"@dark-ground@", "@light-ground@"} <= kit.keys():
        return "".join(line for line in page.splitlines(keepends=True) if "-ground@" not in line)
    return page.replace("@dark-ground@", kit["@dark-ground@"]).replace(
        "@light-ground@", kit["@light-ground@"]
    )


def marked(mark: str, theme: Path) -> str:
    """Draw the home's mark, its favicon, in each look's tint from the kit's theme.css.

    A favicon can't read the page's properties, so the colours are written in.
    A look the kit names no tint for takes the other look's, and where it names
    neither the mark draws in currentColor, which a favicon draws black.
    """
    kit = looks(theme)
    dark, light = kit.get("@dark-tint@"), kit.get("@light-tint@")
    mark = mark.replace("@dark-tint@", dark or light or "currentColor")
    return mark.replace("@light-tint@", light or dark or "currentColor")


def llms(root: Path, sites: list[Site]) -> str:
    """Write the home's llms.txt, each site linking its own twin where it has one."""
    lines = [
        "# Joel's local sites",
        "",
        "> Every site made for Joel on this machine, newest first, each saying what it is",
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
        "Everything made just for you lives here, on this machine, newest first.",
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


def ours(path: object) -> bool:
    """Whether a row's path names a page among the sites' own, as a link from the home.

    Only plain steps pass, after the ./ some sessions open with, so no
    scheme, host, backslash, step up or escape can ride in, nor whitespace a
    browser drops before reading the rest, nor a path that isn't a string.
    The system's homes take one ./ as well. A reader of rows.json sees it as written.
    """
    if not isinstance(path, str):
        return False
    steps = path.removeprefix("./").removesuffix("/")
    return bool(steps) and all(STEP.fullmatch(step) for step in steps.split("/"))


def refused(constant: str) -> NoReturn:
    """Refuse NaN and the infinities, which Python reads but no strict reader of rows.json does."""
    message = f"{constant} isn't JSON"
    raise ValueError(message)


def finite(number: str) -> float:
    """Read a number, refusing one too large to be anything but an infinity."""
    if math.isfinite(read := float(number)):
        return read
    return refused(number)


def drawable(rows: object) -> None:
    """Refuse a lone surrogate anywhere in rows, which no page draws and no strict reader takes."""
    try:
        json.dumps(rows, ensure_ascii=False).encode("utf-8")
    except UnicodeEncodeError:
        message = "a lone surrogate isn't text"
        raise ValueError(message) from None


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


def live(declared: Path) -> list[Site]:
    """Read the rows Nix declares, refusing one whose path leads off the sites."""
    sites = [Site.read(row) for row in json.loads(declared.read_text(encoding="utf-8"))["sites"]]
    if strays := [site.row["path"] for site in sites if not ours(site.row["path"])]:
        message = f"declared rows {', '.join(map(repr, strays))} lead off the sites"
        raise ValueError(message)
    return sites


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


def kind_of(home: object) -> str | None:
    """Say the kind a home claims, a kind's name or a pattern's kind, or None.

    A home of any other shape claims nothing, as its readers take it, so a
    session's slip is left as it was written rather than failing the build.
    """
    kind = home.get("kind") if isinstance(home, dict) else home
    return kind if isinstance(kind, str) else None


def leaving(sites: list[Site], declared: list[Site]) -> tuple[list[Site], list[dict[str, str]]]:
    """Leave each kind a declared row is home to with it, taking it from the recorded rows.

    It gives each claim it took as its kind, the recorded row's path as
    written and the path of the declared row home to the kind, the first
    when two are, for rows.json to say and the log to quote, since only the
    home knows which rows are declared.
    """
    owners: dict[str, str] = {}
    for site in declared:
        for home in site.row.get("homes") or []:
            kind = kind_of(home)
            if kind is not None:
                owners.setdefault(kind, site.row["path"])
    # The declared rows themselves, the very ones declaring placed.
    theirs = {id(site) for site in declared}
    left: list[Site] = []
    taken: list[dict[str, str]] = []
    for site in sites:
        homes = site.row.get("homes")
        if id(site) in theirs or not isinstance(homes, list):
            left.append(site)
            continue
        kept = [home for home in homes if kind_of(home) not in owners]
        for home in homes:
            kind = kind_of(home)
            if home in kept or kind is None:
                continue
            claim = {"kind": kind, "from": site.row["path"], "to": owners[kind]}
            # A row claiming a kind twice, by name and by pattern, is said once.
            if claim not in taken:
                taken.append(claim)
        left.append(site if kept == homes else replace(site, row={**site.row, "homes": kept}))
    return left, taken


def folder(path: str) -> str:
    """Say a row's path as one folder name and a slash, however it was spelled."""
    return "/".join(part for part in path.split("/") if part not in {"", "."}) + "/"


@dataclass(frozen=True)
class Where:
    """Where the home's own files go, and where the last record that read is kept."""

    into: Path | None = None
    kept: Path | None = None


def rows_of(text: str) -> list[Site]:
    """Read a record's sites, refusing one the home can't draw."""
    rows = json.loads(text, parse_constant=refused, parse_float=finite)["sites"]
    drawable(rows)
    return [Site.read(row) for row in rows]


def kept_rows(kept: Path | None) -> list[Site] | None:
    """Read the sites of the last record that read, if one was kept and still reads."""
    if kept is None or not kept.exists():
        return None
    try:
        return rows_of(kept.read_text(encoding="utf-8"))
    except UNREADABLE:
        return None


def sites_read(
    record: Path, own: Path, kept: Path | None, *, declared: bool
) -> tuple[list[Site] | None, str]:
    """Read the record's sites and what to say of them, or None where the last build stays.

    A record caught half written, or broken by a slip, leaves the last good
    build serving and says why, since the next change to it builds again.
    Where no build is there to keep, as after a reboot empties the runtime
    folder the home is built into, the last record that read is built, kept
    where no session writes and a reboot doesn't reach; with none kept, the
    declared rows alone make it. Either way the sites still answer until the
    record reads again, and the record is said broken. A record that reads is
    kept, where it's told.
    """
    try:
        text = record.read_text(encoding="utf-8") if record.exists() else '{"sites": []}'
        sites = rows_of(text)
    except UNREADABLE as error:
        built = (own / "index.html").exists()
        last = None if built else kept_rows(kept)
        if last is not None:
            return (
                last,
                f"; sites.json can't be read ({error}), so the last record that read is built",
            )
        if built or not declared:
            return None, f"home: sites.json can't be read ({error}), so the last build stays"
        return [], f"; sites.json can't be read ({error}), and no build was there to keep"
    if kept is not None and record.exists():
        kept.parent.mkdir(parents=True, exist_ok=True)
        write(kept, text)
    return sites, ""


def build(
    sources: Path,
    root: Path,
    theme: Path,
    declared: Path | None = None,
    *,
    where: Where | None = None,
) -> str:
    """Build the home from its sources, root's sites.json, the theme and declared rows.

    Its own files go where says into, and into root otherwise, and a record
    that reads is kept where says kept. It says what it did.
    """
    where = where or Where()
    own = where.into or root
    record = root / "sites.json"
    # With no record yet, the declared rows alone make the home, making the
    # sites' folder on a machine's first switch, before any document is there.
    if not record.exists():
        if not declared:
            return "home: no sites.json yet, so nothing to build"
        root.mkdir(parents=True, exist_ok=True)
    read, unread = sites_read(record, own, where.kept, declared=declared is not None)
    if read is None:
        return unread
    sites = read
    # Nix wrote the declared rows, so a fault in them is the build's, and fails it.
    rows_declared = live(declared) if declared else []
    sites = declaring(sites, rows_declared) if declared else sites
    # A row's path becomes the home's link to it, so one that would lead
    # anywhere but a page of the sites' own is left out, and said.
    strays = [site.row["path"] for site in sites if not ours(site.row["path"])]
    sites = [site for site in sites if ours(site.row["path"])]
    # Only then, so a claim is said taken from a row the readers would see.
    sites, taken = leaving(sites, rows_declared)
    own.mkdir(parents=True, exist_ok=True)
    for name in FILES:
        write(own / name, (sources / name).read_text(encoding="utf-8"))
    mark = (sources / "favicon.svg").read_text(encoding="utf-8")
    write(own / "favicon.svg", marked(mark, theme))
    shell = (sources / "index.html").read_text(encoding="utf-8")
    write(own / "index.html", grounds(home(shell, sites), theme))
    # Only once the page that no longer loads them is in place.
    for name in GONE:
        (root / name).unlink(missing_ok=True)
    write(own / "llms.txt", llms(root, sites))
    write(own / "index.md", twin(sites))
    # What the home took is said beside the rows, since they don't say which
    # are declared; the system's and Code's readers read only "sites", and
    # the web-design skill's reader names each claim in "taken".
    listed = {"sites": [site.row for site in sites], "taken": taken}
    write(own / "rows.json", json.dumps(listed, indent=2) + "\n")
    left_behind(root, own)

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
    said += f"; scripts of their own in {', '.join(owning)}" if owning else ""
    said += f"; left out {', '.join(map(repr, strays))}, no path of ours" if strays else ""
    left = ", ".join(f"{each['kind']!r} from {each['from']!r}" for each in taken)
    return said + (f"; homes left to the declared sites: {left}" if taken else "") + unread


def left_behind(root: Path, own: Path) -> None:
    """Take away the copies of the home's own files a build before left in root.

    Once the home is written into a folder of its own, the server serves
    that one, so a copy in root would only be a stale page among the
    documents, looking like the home.
    """
    if own.resolve() == root.resolve():
        return
    for name in (*FILES, *OWN):
        (root / name).unlink(missing_ok=True)


if __name__ == "__main__":
    # Where the home's own files go, if it's said, then the program's two
    # folders and the theme, then the declared rows when given.
    args = sys.argv[1:]
    into = Path(args.pop(0).removeprefix(INTO)) if args and args[0].startswith(INTO) else None
    kept = Path(args.pop(0).removeprefix(KEEP)) if args and args[0].startswith(KEEP) else None
    given, with_rows = 3, 4
    if len(args) not in {given, with_rows}:
        sys.exit(USAGE)
    rows = Path(args[given]) if len(args) == with_rows else None
    theme = Path(args[2])
    # A caller still on the old shape hands the declared rows where the theme
    # now goes, which would read as a theme naming nothing.
    if theme.suffix != ".css":
        sys.exit(f"home: the theme {theme} isn't a stylesheet; {USAGE}")
    if not theme.exists():
        print(f"home: the theme {theme} isn't there, so no look is named", file=sys.stderr)
    print(build(Path(args[0]), Path(args[1]), theme, rows, where=Where(into=into, kept=kept)))

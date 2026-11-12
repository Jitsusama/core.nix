"""The desktop test's looks area for Joel's programs' own pages.

Here the pages his programs hold in the launcher are shot at his two scales, on a screen as large
as the laptop's, as the looks area shoots the launcher's own: 1.5, as the HP draws it, and 2, as
the laptop does. Each is walked short, waiting only until the shot shows what it's named for,
since what each does is proven in its own area (VKCF3Z, 2026-11-08: every shot at both scales,
since each has caught something the other didn't). It's an area of its own, rather than more of
looks, since the pyramid keeps a VM test to twenty subtests and about two minutes a subtest
(docs/testing.md), and on its own it runs beside looks rather than after it.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it; Passwords' goes last, since its copy stays on
the clipboard until it clears.

Long because each program's page is shot in the state that shows it whole, so each subtest has a
fixture of its own beside it making that state and taking it away, and the area stays one file
for one machine: split, each part would boot a machine of its own to shoot the same pages.
"""

from __future__ import annotations

import shlex
from datetime import timedelta
from functools import partial
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# imported for Drive's test to ask for.
from desktop_session import LAUNCHER_OPEN, google, retry  # noqa: F401

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# The scales Joel's screens are drawn at, as niri spells them.
JOELS_SCALES = ("1.5", "2")

# A repository and a machine for kitty's page to list, as launcher-moves-windows gives it; the
# machine is under .test, which nothing in the VM answers, and nothing here runs a row.
REPOSITORY = "tide-tables"
MACHINE = "ferry"
SSH_CONFIG = "Host ferry\n    HostName ferry.harbour.test\n    User deckhand\n    Port 2200\n"

# Where tuple-call keeps the calls that ended, for Tuple's Recent.
TUPLE_KEPT = "~/.local/state/tuple-call/recent.jsonl"

# How long the reader and zathura are given to take a key, or to let their windows go.
TAKING = timedelta(seconds=30)

# Screenshots as niri names them, for swayimg's page to lead with their gallery.
SCREENSHOTS = (
    "Screenshot from 2026-11-09 09-14-02.png",
    "Screenshot from 2026-11-09 11-40-57.png",
)
# Folders in the home swayimg opened pictures in, each with its pictures: the first only
# stepped to in its gallery, the rest opened, the last of all the newest.
LOOKED_AT = {
    "Pictures/Harbour": ("the-quay.png", "ferry-at-dawn.png", "lighthouse.png"),
    "Pictures/Charts": ("soundings.png", "tide-chart.png"),
}

# Where in the home the reader reads markdown files here, and where it keeps where each was
# left (modules/reader/src/place.rs).
NOTES = "Documents/Harbour"
PLACES = ".local/state/reader/places"
# The title of the page left part way, which its row says it was left under; how many lines
# each of its headings holds, so half a screen is a small part of it, as a document's is; and
# the act reading from its mark.
PAGE_TITLE = "A Page"
LINES = 120
MARK = "Read From Mark a"

# How many pages the book has (tests/desktop.nix), and the names it's open under in zathura,
# each with the page it's on.
BOOK_PAGES = 12
OPEN_ON = (("almanac.pdf", 5), ("logbook.pdf", 11))

# Where in the home Inkscape's new drawings go, and the two it opened lately there: one Joel
# saved under a name of his own, a map's size, and one New Drawing made from the template,
# named as it names one.
DRAWINGS = "Pictures/Drawings"
SAVED = "Harbour Map.svg"
SAVED_SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900"></svg>\n'
FRESH = "Drawing 2026-11-09 10.12.03.svg"

# Where in the home GIMP saved the pictures it edited lately, the last the newest, and the name
# GIMP says each it opens under on the desktop's list of files opened lately: its long name,
# which its entry keeps among its keywords now that it goes by GIMP, so its Recent is found.
EDITED = "Pictures/Posters"
EDITS = ("quay-retouched.png", "ferry-poster.png")
GIMP = "GNU Image Manipulation Program"

# Where in the home the Neovims' files are, by name, the first the one left changed, and the
# lines each holds, enough for a drawer's preview to show several.
SOUNDINGS = "soundings"
SOUNDED = ("tides.txt", "currents.txt", "reefs.txt")
SOUNDS = (
    "Soundings off the quay, at low water.",
    "North pier: 4 fathoms",
    "South pier: 3 fathoms",
    "Harbour mouth: 7 fathoms",
)

# Windows standing in for Google's programs in Chrome, as launcher-moves-windows' are: a kitty
# given the app id and title Chrome gives Mail's own window, and one given Chrome's app id and a
# Docs tab's title, each by the layer Shortcuts opens on and its shot's name.
GOOGLE_WINDOWS = [
    ("chrome-mail.google.com__-Default", "Inbox - joel@example.com - Gmail", "Mail", "mail"),
    ("google-chrome", "Notes - Google Docs - Google Chrome", "Docs", "docs-tab"),
]


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> str:
    """Set the screen's scale, with the launcher closed, and give it as a shot's name ends."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")
    return scale.replace(".", "-")


def drawer_on(title: str, shown: Json) -> bool:
    """Say whether the drawer is out, drawing the thing titled."""
    drawer = shown["drawer"]
    return bool(drawer["out"] is True and drawer["title"] == title)


def figures_on(title: str, shown: Json) -> bool:
    """Say whether the drawer draws every fact of the thing titled, none left for its keys."""
    drawer = shown["drawer"]
    return drawer_on(title, shown) and len(drawer["lines"]["facts"]) == len(drawer["facts"])


def previewed(title: str, shown: Json) -> bool:
    """Say whether the drawer is out, drawing the thing titled and previewing it."""
    preview = shown["drawer"]["preview"] or {}
    return drawer_on(title, shown) and bool(preview.get("picture") or preview.get("lines"))


def named(name: str, shown: Json) -> bool:
    """Say whether the selected row is the one named."""
    return (shown["selected"] or {}).get("name") == name


@pytest.mark.usefixtures("google")
def test_drives_places_and_recent_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Drive's program's drawer, the program holding its places, and a file in Recent.

    Typed, drive leads with the disk's room, so the selection goes down to the program, whose
    drawer says what its places hold and what its copies kept here take.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("drive")
        desktop.launcher_until(lambda shown: "Drive" in shown["rows"])
        desktop.down_to("Drive")
        # Each size keeps its unit beside it by a no-break space, as every figure does.
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"]
                and "Kept Here" in shown["drawer"]["drawnFacts"]
                and "412\u00a0MB of 2.00\u00a0GB" in shown["drawer"]["values"]
            )
        )
        desktop.shoot(f"launcher-drive-at-{at}")
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Starred" in shown["rows"])
        desktop.shoot(f"launcher-drive-places-at-{at}")
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(drawer_on, "menu.pdf"))
        desktop.shoot(f"launcher-drive-recent-at-{at}")
        desktop.close_launcher()


def test_spotify_and_its_podcasts_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Spotify's page signed in, holding Recent and Podcasts, and a show in Podcasts."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("spotify")
        desktop.launcher_until(partial(named, "Spotify"))
        desktop.into_program()
        # Spotify is read as it's stepped into, so its lists come once the stand-in answers.
        desktop.launcher_until(lambda shown: {"Recent", "Podcasts"} <= set(shown["rows"]))
        desktop.shoot(f"launcher-spotify-at-{at}")
        desktop.down_to("Podcasts")
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["The Show"])
        desktop.shoot(f"launcher-spotify-podcasts-at-{at}")
        desktop.close_launcher()
    # The stand-in spotify-recent notes each run it's asked (tests/desktop.nix).
    desktop.as_account("rm -f ~/asked")


@pytest.fixture
def stopped_part_way(desktop: Desktop) -> Iterator[str]:
    """Give a file mpv stopped part way, as mpv writes it quitting, and take it away after."""
    clip = f"/home/{desktop.account}/Videos/stopped.mkv"
    later = f"/home/{desktop.account}/.local/state/mpv/watch_later"
    # Where mpv stopped in it, named by its path's hash, as opens-files' mpv test writes it.
    kept = f"{later}/$(printf %s {clip} | md5sum | cut -c 1-32 | tr a-f A-F)"
    desktop.as_account(
        f"mkdir -p {later} ~/Videos && mpv --no-config --really-quiet --ovc=ffv1"
        f" 'av://lavfi:testsrc=duration=60:size=64x64:rate=2' --o={clip}"
        f" && printf '# %s\\nstart=12.000000\\nspeed=1.500000\\n' {clip} > {kept}"
    )
    yield clip
    desktop.as_account(f"rm -f {clip} {kept}")


def test_what_mpv_stopped_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], stopped_part_way: str
) -> None:
    """Shoot mpv's Recent on a file it stopped part way, its drawer saying what's left."""
    title = stopped_part_way.rsplit("/", 1)[-1]
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("mpv")
        desktop.launcher_until(partial(named, "mpv Media Player"))
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(drawer_on, title))
        desktop.shoot(f"launcher-mpv-stopped-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def pictures_looked_at(desktop: Desktop) -> Iterator[None]:
    """Give the screenshots' folder two screenshots, and swayimg pictures it opened in two more.

    Each picture opened is said to the desktop's list of files opened lately as swayimg says
    the one it opens (modules/swayimg/init.lua); the first of each folder was only stepped to
    in its gallery, so only its count says it. After, the screenshots and the folders go, and
    their entries on the list go unlisted with them, as it lists only what's still there.
    """
    home = f"/home/{desktop.account}"
    shots = [shlex.quote(f"{desktop.pictures}/{name}") for name in SCREENSHOTS]
    folders = [f"{home}/{folder}" for folder in LOOKED_AT]
    there = [f"{home}/{folder}/{name}" for folder, names in LOOKED_AT.items() for name in names]
    opened = [
        f"{home}/{folder}/{name}" for folder, names in LOOKED_AT.items() for name in names[1:]
    ]
    desktop.as_account(
        f"mkdir -p {desktop.pictures} {' '.join(folders)}"
        f' && for to in {" ".join(shots + there)}; do cp {desktop.values["picture"]} "$to"; done'
        f" && recent-files add swayimg {' '.join(opened)}"
    )
    yield
    desktop.as_account(f"rm -f {' '.join(shots)} && rm -rf {' '.join(folders)}")


def galleries_counted(shown: Json) -> bool:
    """Say whether swayimg's page holds the screenshots' gallery and each folder's, counted."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    galleries = [read for read in reads if read.startswith("Look Through ")]
    return len(galleries) == len(LOOKED_AT) + 1 and not any("\u2026" in read for read in galleries)


def selects(key: str, shown: Json) -> bool:
    """Say whether the selected row is the thing this key names, as a program's row."""
    return (shown["selected"] or {}).get("is") == key


@pytest.mark.usefixtures("pictures_looked_at")
def test_swayimgs_galleries_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot swayimg's page, led by the screenshots' gallery, each folder's gallery after it.

    How many pictures each folder holds is read as swayimg is stepped into, so the page is
    shot once every gallery says its count.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("swayimg")
        desktop.launcher_until(partial(selects, "program:swayimg"))
        desktop.into_program()
        desktop.launcher_until(galleries_counted)
        desktop.shoot(f"launcher-swayimg-at-{at}")
        desktop.close_launcher()


def markdown(title: str, section: str) -> str:
    """Give a markdown file's words: its title and a section after it, each over its lines."""

    def under(heading: str) -> list[str]:
        """Give the paragraphs under a heading, enough that the page runs several screens."""
        lines = range(1, LINES + 1)
        return [f"Line {n} under {heading}, kept as the tide came in." for n in lines]

    return "\n\n".join([f"# {title}", *under(title), f"## {section}", *under(section)]) + "\n"


def pressed(desktop: Desktop, log: str, act: str, *keys: str) -> str:
    """Press the keys making an act in the reader, and give its key log's line once it's done.

    Each act's own lines are counted, by the name the reader logs it under, so a key it took
    earlier and logs late is never taken for this one.
    """
    machine = desktop.machine
    line = shlex.quote(f"key {act} ")
    # grep counts none as a failure, which the driver's shell would end on.
    counted = f"grep -c -F {line} {log} || true"
    before = int(machine.succeed(counted))
    for key in keys:
        machine.send_key(key)
    machine.wait_until_succeeds(f"[ $({counted}) -gt {before} ]", timeout=TAKING)
    return machine.succeed(f"grep -F {line} {log} | tail -n 1")


def read_in_the_reader(desktop: Desktop, path: str, *acts: tuple[str, ...]) -> None:
    """Open a file in the reader, go to its end and do each act, then close it by ZZ.

    Its keys are logged (READER_KEYS_LOG), as its own check reads them, and each act is its
    name in the log then the keys making it. Keys are pressed once it says it holds the
    keyboard, and G again until it goes past the top, as that check presses it, since G goes
    nowhere until a page is set; each is taken before the next is pressed, so none lands
    after the acts. As it closes, the reader keeps where the file was left, saying how far
    down it is once the whole page is set, which is waited for.
    """
    machine = desktop.machine
    log = f"/run/user/1000/reader-{path.rsplit('/', 1)[-1]}.log"
    read = shlex.quote('READER_KEYS_LOG=1 exec reader "$1" 2> "$2"')
    desktop.as_account(f"niri msg action spawn -- sh -c {read} sh {path} {log}")
    focused = shlex.quote('"app_id":"reader"')
    reading = desktop.in_session(f"niri msg --json focused-window | grep -qF {focused}")
    machine.wait_until_succeeds(reading, timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(f"grep -qx 'keys: focused' {log}", timeout=TAKING)

    def at_the_end(_: bool) -> bool:
        """Press G, and say whether the reader went past the top of a page it has set."""
        return " heading 0.0 pt" not in pressed(desktop, log, "Last", "shift-g")

    retry(at_the_end, timeout=TAKING)
    for act in acts:
        pressed(desktop, log, *act)
    machine.send_key("shift-z")
    machine.send_key("shift-z")
    machine.wait_until_fails(reading, timeout=TAKING)
    places = f"/home/{desktop.account}/{PLACES}"
    kept = f'grep -hF \'"file":"{path}"\' {places}/*.json'
    machine.wait_until_succeeds(f"{kept} | grep -qF '\"through\"'", timeout=TAKING)
    # What it kept is what Recent says, so the log says it beside the shots.
    print(f"the reader kept {machine.succeed(kept)}")
    machine.succeed(f"rm -f {log}")


@pytest.fixture
def markdown_read(desktop: Desktop) -> Iterator[str]:
    """Read two markdown files in the reader, one to its end and one part way, marked.

    The reader keeps where each was left as it closes, as on a machine, and the launcher
    lists them through reader --places. The tide log is read to its end; the page is marked
    a under its section and then left half a screen into its title's lines, read last. It
    gives the page. After, the files go, and so do their places.
    """
    folder = f"/home/{desktop.account}/{NOTES}"
    tides = f"{folder}/tide-log.md"
    page = f"{folder}/a-page.md"
    desktop.as_account(
        f"mkdir -p {folder}"
        f" && printf %s {shlex.quote(markdown('The Tide Log', 'Slack Water'))} > {tides}"
        f" && printf %s {shlex.quote(markdown(PAGE_TITLE, 'Further On'))} > {page}"
    )
    read_in_the_reader(desktop, tides)
    # To the top, to the next heading, the section, two lines into it and mark a there, since
    # a mark on a heading's own line is kept under the heading before it, back to the top,
    # and down half a screen.
    read_in_the_reader(
        desktop,
        page,
        ("First", "g", "g"),
        ("HeadingOn", "bracket_right", "bracket_right"),
        ("Down", "j"),
        ("Down", "j"),
        ("SetMark('a')", "m", "a"),
        ("First", "g", "g"),
        ("HalfDown", "ctrl-d"),
    )
    yield page
    places = f"/home/{desktop.account}/{PLACES}"
    desktop.as_account(
        f"rm -rf {folder} && (grep -lF {folder}/ {places}/*.json || true) | xargs -r rm -f"
    )


def left_under(name: str, heading: str, shown: Json) -> bool:
    """Say whether the file named is selected, its drawer out, saying it was left under this."""
    side = str((shown["selected"] or {}).get("side", ""))
    return drawer_on(name, shown) and side.startswith(f"{heading} \u00b7 ")


def test_the_readers_recent_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], markdown_read: str
) -> None:
    """Shoot the reader's Recent, each file saying where it was left, and a mark's row in one.

    Its places are read as the reader is stepped into, so the page leads Recent saying where
    it was left once reader --places has answered.
    """
    name = markdown_read.rsplit("/", 1)[-1]
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("reader")
        desktop.launcher_until(partial(selects, "program:reader"))
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(left_under, name, PAGE_TITLE))
        desktop.shoot(f"launcher-reader-recent-at-{at}")
        desktop.follow()
        desktop.launcher_until(lambda shown: MARK in shown["rows"])
        desktop.down_to(MARK)
        desktop.shoot(f"launcher-reader-mark-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def documents_open(desktop: Desktop, values: dict[str, Any]) -> Iterator[None]:
    """Open the book twice in zathura, under two names, each on a page of its own.

    Each opens once the one before has its window, so the last is the one focused, and the
    page leads with it, whichever zathura starts sooner. After, each window closes and its
    document goes.
    """
    machine = desktop.machine
    zathuras = 'niri msg --json windows | grep -oF \'"app_id":"org.pwmt.zathura"\' | wc -l'
    for opened, (name, page) in enumerate(OPEN_ON, start=1):
        desktop.as_account(
            f"cp {values['book']} ~/{name}"
            f" && niri msg action spawn -- zathura --page={page} ~/{name}"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $({zathuras}) -eq {opened} ]"), timeout=timedelta(seconds=60)
        )
    yield
    for window in desktop.windows():
        if window["app_id"] == "org.pwmt.zathura":
            desktop.close(window)
    machine.wait_until_succeeds(desktop.in_session(f"[ $({zathuras}) -eq 0 ]"), timeout=TAKING)
    desktop.as_account(f"rm -f {' '.join(f'~/{name}' for name, _ in OPEN_ON)}")


def each_on_its_page(shown: Json) -> bool:
    """Say whether zathura's page lists each document open, saying the page it's on."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    return all(
        any(read.startswith(name) and f"p. {page} of {BOOK_PAGES}" in read for read in reads)
        for name, page in OPEN_ON
    )


@pytest.mark.usefixtures("documents_open")
def test_zathuras_page_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot zathura's page, each document open saying the page it's on.

    zathura is asked over its bus as its page opens, so it's shot once each has answered.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("zathura")
        desktop.launcher_until(partial(selects, "program:org.pwmt.zathura"))
        desktop.into_program()
        desktop.launcher_until(each_on_its_page)
        desktop.shoot(f"launcher-zathura-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def a_picture_copied(desktop: Desktop) -> Iterator[None]:
    """Copy a picture, once the shell has heard it, and copy something else over it after."""
    with desktop.told("heard copy Image 256 \u00d7 256"):
        desktop.as_account(
            f"wl-copy --type image/png < {desktop.values['picture']} > /dev/null 2>&1"
        )
    yield
    with desktop.told("heard copy after the picture"):
        desktop.as_account("echo -n 'after the picture' | wl-copy > /dev/null 2>&1")


@pytest.fixture
def drawings_opened(desktop: Desktop) -> Iterator[None]:
    """Give Inkscape two drawings it opened lately, and take them away after.

    Each drawing is said to the desktop's list of files opened lately under Inkscape's own
    name, as it says each it opens, the saved map first and the new one after, newest.
    """
    folder = f"/home/{desktop.account}/{DRAWINGS}"
    template = f"/home/{desktop.account}/.config/inkscape/templates/default.svg"
    saved = shlex.quote(f"{folder}/{SAVED}")
    fresh = shlex.quote(f"{folder}/{FRESH}")
    desktop.as_account(
        f"mkdir -p {folder} && printf %s {shlex.quote(SAVED_SVG)} > {saved}"
        f" && cp {template} {fresh} && recent-files add org.inkscape.Inkscape {saved} {fresh}"
    )
    yield
    desktop.as_account(f"rm -rf {folder}")


@pytest.mark.usefixtures("a_picture_copied", "drawings_opened")
def test_inkscapes_page_and_its_recent_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Inkscape's page, with a picture copied, and its Recent led by the newest drawing."""
    acts = {"New Drawing", "New Drawing From the Copied Picture", "Recent"}
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("inkscape")
        desktop.launcher_until(partial(selects, "program:org.inkscape.Inkscape"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: acts <= set(shown["rows"]))
        desktop.shoot(f"launcher-inkscape-at-{at}")
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(drawer_on, FRESH))
        desktop.shoot(f"launcher-inkscape-recent-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def pictures_edited(desktop: Desktop) -> Iterator[None]:
    """Give the screenshots' folder a screenshot, and GIMP two pictures it edited lately.

    Each picture is said to the desktop's list of files opened lately under the name GIMP says
    them under, one at a time, as GIMP opens them, so the last is the newest. After, the
    screenshot and the pictures go, and their entries on the list go unlisted with them.
    """
    folder = f"/home/{desktop.account}/{EDITED}"
    shot = shlex.quote(f"{desktop.pictures}/{SCREENSHOTS[-1]}")
    edits = [shlex.quote(f"{folder}/{name}") for name in EDITS]
    added = " && ".join(f"recent-files add {shlex.quote(GIMP)} {edit}" for edit in edits)
    desktop.as_account(
        f"mkdir -p {desktop.pictures} {folder}"
        f' && for to in {shot} {" ".join(edits)}; do cp {desktop.values["picture"]} "$to"; done'
        f" && {added}"
    )
    yield
    desktop.as_account(f"rm -f {shot} && rm -rf {folder}")


@pytest.mark.usefixtures("a_picture_copied", "pictures_edited")
def test_gimps_page_and_its_recent_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot GIMP's page holding every act, and its Recent led by the picture edited last.

    A picture is copied and a screenshot taken, so New Image From the Copied Picture and Edit
    the Newest Screenshot both stand there, beside its Recent, whose newest picture is shot
    with its drawer out showing it.
    """
    acts = {"New Image From the Copied Picture", "Edit the Newest Screenshot", "Recent"}
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("gimp")
        desktop.launcher_until(partial(selects, "program:gimp"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: acts <= set(shown["rows"]))
        desktop.shoot(f"launcher-gimp-at-{at}")
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(previewed, EDITS[-1]))
        desktop.shoot(f"launcher-gimp-recent-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def neovims_open(desktop: Desktop) -> Iterator[str]:
    """Open two Neovims in kitty windows, three files between them, and copy one file's path.

    The first holds two files in two tabs, the first changed and unsaved, and the other one,
    as opens-files' walk has them; a repository under ~/src and the path copied, at a line,
    give Neovim's page Open a File by Name and Open the Copied Path. It gives the changed
    file's name. After, each Neovim quits unsaved, something else is copied over the path, and
    the files and the repository go.
    """
    folder = f"/home/{desktop.account}/{SOUNDINGS}"
    tides, currents, reefs = (f"{folder}/{name}" for name in SOUNDED)
    desktop.as_account(
        f"mkdir -p {folder} && printf '%s\\n' {shlex.join(SOUNDS)}"
        f" | tee {tides} {currents} {reefs} > /dev/null && git init -q ~/src/{REPOSITORY}"
    )
    neovims: list[tuple[int, str]] = []
    try:
        changed = "+call setline(1, 'shoals')"
        neovims.append(
            desktop_session.neovim_in_kitty(desktop, folder, "-p", tides, currents, changed)
        )
        neovims.append(desktop_session.neovim_in_kitty(desktop, folder, reefs))
        desktop.as_account(f"echo -n {reefs}:3 | wl-copy > /dev/null 2>&1")
        yield SOUNDED[0]
    finally:
        desktop_session.neovims_quit(desktop, neovims)
        with desktop.told("heard copy after the path"):
            desktop.as_account("echo -n 'after the path' | wl-copy > /dev/null 2>&1")
        desktop.as_account(f"rm -rf {folder} ~/src/{REPOSITORY}")


def holds_each_open(shown: Json) -> bool:
    """Say whether Neovim's page holds its two acts and a row for every file open."""
    acts = {"Open a File by Name", "Open the Copied Path"}
    return acts | set(SOUNDED) <= set(shown["rows"])


def test_neovims_page_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], neovims_open: str
) -> None:
    """Shoot Neovim's page, its acts and every file two Neovims hold, the changed one's drawer out.

    What's open is asked of each Neovim over its own server as the launcher opens, so the
    page is shot once every file is a row.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("neovim")
        desktop.launcher_until(partial(selects, "program:nvim"))
        desktop.into_program()
        desktop.launcher_until(holds_each_open)
        desktop.down_to(neovims_open)
        desktop.launcher_until(partial(previewed, neovims_open))
        desktop.shoot(f"launcher-neovim-at-{at}")
        desktop.close_launcher()


def latest_said(name: str, shown: Json) -> bool:
    """Say whether Chrome's page holds its Tabs and Recent, and the latest download by name.

    Until its downloads are first read, the act waits in its place saying an ellipsis.
    """
    reads = [str(thing["reads"]) for thing in shown["things"]]
    latest = [read for read in reads if read.startswith(desktop_session.LATEST_DOWNLOAD)]
    held = {"Tabs", "Recent"} <= set(shown["rows"])
    return held and len(latest) == 1 and name in latest[0]


@pytest.fixture
def chrome_with_a_download(desktop: Desktop) -> Iterator[str]:
    """Give the stand-in Chrome its tab, the sites it visited and a download, its window open.

    It gives the download's name. After, the window closes, and the lists and the file go.
    """
    desktop_session.chrome_visited(desktop)
    downloaded = desktop_session.chrome_downloaded(desktop)
    try:
        desktop_session.chrome_opened(desktop)
        yield downloaded.rsplit("/", 1)[-1]
    finally:
        desktop_session.chrome_gone(desktop)
        desktop_session.chrome_download_gone(desktop, downloaded)


def test_chromes_page_and_its_download_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], chrome_with_a_download: str
) -> None:
    """Shoot Chrome's page holding its tab and the latest download, and the download in Recent.

    Its downloads are read as Chrome is stepped into, so the page is shot once Open the
    Latest Download stands on it; in its Recent, among the sites it visited, the download's
    drawer says its kind and previews it.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("chrome")
        desktop.launcher_until(partial(selects, "program:google-chrome"))
        desktop.into_program()
        desktop.launcher_until(partial(latest_said, chrome_with_a_download))
        desktop.shoot(f"launcher-chrome-at-{at}")
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(lambda shown: chrome_with_a_download in shown["rows"])
        desktop.down_to(chrome_with_a_download)
        desktop.launcher_until(partial(previewed, chrome_with_a_download))
        desktop.shoot(f"launcher-chrome-recent-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def a_tuple_call_ended(desktop: Desktop) -> Iterator[None]:
    """Have a Tuple call with Ada end, on the stand-in's stream, as takes-calls has one.

    After, tuple-call forgets it once its file goes and it restarts, and Tuple starts again
    where it ran.
    """
    tuple_was = desktop.machine.execute(desktop.in_session("systemctl --user is-active tuple"))
    desktop.tuple_follows()
    now = int(desktop.as_account("date +%s%3N"))
    connected = {"connection_state": "connected", "audio_enabled": True}
    ada = connected | {"id": 7, "full_name": "Ada Brightwater", "email": "ada@example.com"}
    call = {
        "id": "a-call",
        "local": connected | {"is_current_user": True},
        "participants": [ada],
        "started_at": now - 9 * 60 * 1000,
    }
    with desktop.told("drawn margin"):
        desktop.tuple_streams(call)
    desktop.tuple_streams(None)
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"grep -q Brightwater {TUPLE_KEPT}"), timeout=timedelta(seconds=10)
    )
    yield
    desktop.as_account(
        f"rm -f {TUPLE_KEPT} && systemctl --user stop tuple-answers"
        " && systemctl --user restart tuple-call"
    )
    if tuple_was[0] == 0:
        desktop.as_account("systemctl --user start tuple")


@pytest.mark.usefixtures("a_tuple_call_ended")
def test_tuples_recent_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Tuple's Recent on the call that ended, its drawer out."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("tuple")
        desktop.launcher_until(partial(named, "Tuple"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Recent" in shown["rows"])
        desktop.down_to("Recent")
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Ada Brightwater" in shown["rows"])
        desktop.down_to("Ada Brightwater")
        desktop.launcher_until(partial(drawer_on, "Ada Brightwater"))
        desktop.shoot(f"launcher-tuple-recent-at-{at}")
        desktop.close_launcher()


def test_the_vms_are_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Programs' VMs on the stand-in virsh's web, shut off, its drawer out."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("vms")
        desktop.launcher_until(partial(named, "VMs"))
        desktop.into_program()
        # Every figure fits, none left for its keys; its UUID and security label are
        # identifiers, so they're whole on as many lines as they take, broken at their hyphens,
        # rather than one word running past the drawer's edge, which the shot shows. At scale
        # 2 that takes the line its note on what's known only while it runs had, so the note
        # is what Ctrl+W l reaches there.
        desktop.launcher_until(partial(figures_on, "web"))
        desktop.shoot(f"launcher-vms-at-{at}")
        desktop.close_launcher()
    # The stand-in virsh notes each run it's asked (tests/desktop.nix).
    desktop.as_account("rm -f $XDG_RUNTIME_DIR/virsh-asked")


@pytest.fixture
def a_repository_and_a_machine(desktop: Desktop) -> Iterator[None]:
    """Give kitty a repository in ~/src and a machine in ~/.ssh/config, and take them after.

    The shell reads kitty's repositories and machines ahead, as the launcher closes, so it's
    opened and closed once they're made.
    """
    desktop.as_account(
        f"git init -q ~/src/{REPOSITORY} && mkdir -p -m 700 ~/.ssh && test ! -e ~/.ssh/config"
        f" && printf %s {shlex.quote(SSH_CONFIG)} > ~/.ssh/config"
    )
    desktop.answer("kitty")
    desktop.close_launcher()
    yield
    desktop.as_account(
        f"rm -rf ~/src/{REPOSITORY} && rm -f ~/.ssh/config && (rmdir ~/.ssh || true)"
    )


@pytest.mark.usefixtures("a_repository_and_a_machine")
def test_kittys_repositories_and_machines_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot kitty's New Window in a Repository and SSH to a Machine, each stepped into."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("kitty")
        desktop.launcher_until(partial(named, "kitty"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "SSH to a Machine" in shown["rows"])
        desktop.down_to("New Window in a Repository")
        desktop.step_in()
        desktop.launcher_until(lambda shown: REPOSITORY in shown["rows"])
        desktop.shoot(f"launcher-kitty-repositories-at-{at}")
        desktop.step_out()
        desktop.down_to("SSH to a Machine")
        desktop.step_in()
        desktop.launcher_until(lambda shown: MACHINE in shown["rows"])
        desktop.down_to(MACHINE)
        desktop.launcher_until(partial(drawer_on, MACHINE))
        desktop.shoot(f"launcher-kitty-ssh-at-{at}")
        desktop.close_launcher()


def on_layer(layer: str, shown: Json) -> bool:
    """Say whether Shortcuts holds the layer named, as its path ends."""
    return cast("list[str]", shown["path"])[-1:] == [layer]


def test_shortcuts_over_google_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Shortcuts opened by Super+Shift+/ over Mail's window and over a Docs tab."""
    machine = desktop.machine
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        for app, title, layer, shot in GOOGLE_WINDOWS:
            named_so = f"--class {shlex.quote(app)} --title {shlex.quote(title)}"
            desktop.as_account(f"niri msg action spawn -- kitty {named_so}")
            machine.wait_until_succeeds(
                desktop.in_session(f"niri msg --json windows | grep -qF {shlex.quote(title)}"),
                timeout=desktop_session.STARTING,
            )
            window = desktop.window_titled(title)
            try:
                desktop.as_account(f"niri msg action focus-window --id {window['id']}")
                machine.send_key("meta_l-shift-slash")
                desktop.launcher_ready()
                desktop.launcher_until(partial(on_layer, layer))
                desktop.shoot(f"launcher-shortcuts-over-{shot}-at-{at}")
                desktop.close_launcher()
            finally:
                desktop.close(window)


def test_a_terminal_programs_drawer_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot btop's drawer, which says it runs in a terminal and names its entry's file.

    Its entry's file is an identifier, whole on as many lines as it takes, broken at its
    slashes, which the shot shows.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("btop")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and str(shown["drawer"]["title"]).startswith("btop")
                and "Runs In" in shown["drawer"]["facts"]
                and "Entry File" in shown["drawer"]["facts"]
                and len(shown["drawer"]["lines"]["facts"]) == len(shown["drawer"]["facts"])
            )
        )
        desktop.shoot(f"launcher-btop-drawer-at-{at}")
        desktop.close_launcher()


def test_passwords_and_a_copy_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Passwords, an entry read in full, the margin's mark once copied, and 1Password.

    1Password is asked through the stand-in op (tests/desktop.nix) as the place opens, and
    until it answers the place leads with 1Password locked, so it's opened again once it has.
    """
    favourite = "Lighthouse Console"
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher collection passwords")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: favourite in shown["rows"])
        desktop.close_launcher()
        desktop.as_account("qs ipc call launcher collection passwords")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: named(favourite, shown) and shown["drawer"]["shown"])
        desktop.shoot(f"launcher-passwords-place-at-{at}")
        # Stepping into it reads it in full, which its drawer then says.
        desktop.follow()
        desktop.launcher_until(lambda shown: "Copy the Password" in shown["rows"])
        desktop.step_out()
        desktop.launcher_until(
            lambda shown: (
                named(favourite, shown)
                and shown["drawer"]["shown"]
                and "Health" in shown["drawer"]["drawnFacts"]
            )
        )
        desktop.shoot(f"launcher-passwords-entry-at-{at}")
        # Return copies its password and the launcher goes, the margin wearing the clipboard's
        # mark till it clears.
        since = desktop.mark()
        desktop.machine.send_key("ret")
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.reported("holds margin ", lambda marks: "clipboard" in marks.split(), since)
        desktop.shoot(f"margin-password-copied-at-{at}")
        # 1Password's program, let in, holds the place's Check Every Password after its vaults,
        # and Needing Attention only once a check has found something, which none has here.
        desktop.answer("1password")
        desktop.launcher_until(partial(named, "1Password"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Check Every Password" in shown["rows"])
        assert "Needing Attention" not in desktop.showing()["rows"], desktop.showing()["rows"]
        desktop.shoot(f"launcher-onepassword-at-{at}")
        desktop.close_launcher()

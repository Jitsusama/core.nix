"""The desktop test's says what happens area.

Here notices say what happens: who sent each, its drawer and actions by the keys and the
pointer, what opening one brings forward, and where they sit.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import functools
import json
import math
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import FROM_THE_CORNER, KEY_PACE, LAUNCHER_OPEN, NOTICE_LINE, NOTICES, retry
from PIL import Image, ImageChops, ImageStat

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


# The scales Joel's screens are drawn at, as niri spells them.
JOELS_SCALES = ("1.5", "2")

# The theme's glyph for a call, a handset (modules/theme/glyphs.nix), as the shell's front card
# says the glyph it wears.
CALL_GLYPH = "\U000f03f2"

# How much of the detail under a card must be left, below which niri blurs it. A band blurred
# there kept about a fifth of what the card's shadow alone keeps (2026-11-06, at 2).
UNBLURRED = 0.5

# What a search says going round an end, and with nothing sought before, as Vim's do
# (modules/quickshell/launcher/words.json, field.hints).
WRAPPED_BOTTOM = "search hit BOTTOM, continuing at TOP"
WRAPPED_TOP = "search hit TOP, continuing at BOTTOM"
E35 = "E35: No previous regular expression"
E486_ZZZ = "E486: Pattern not found: zzz"


def test_niris_settings_failing_to_load_is_said_until_they_load_again(desktop: Desktop) -> None:
    """Niri's settings failing to load is said until they load again."""
    machine = desktop.machine
    # niri keeps the settings it had when a load fails, and says so for a
    # few seconds only. A setting it doesn't know stands in for one only a
    # newer niri knows, as srgb-gamut was to the niri running on 2026-10-24.
    settings = f"/home/{desktop.account}/.config/niri/config.kdl"
    meant = machine.succeed(f"readlink {settings}").strip()
    with desktop.told("shows card The window manager kept its old settings"):
        machine.succeed(
            f"cp --remove-destination {meant} {settings}"
            f" && echo 'a-setting-niri-does-not-know' >> {settings}"
            f" && chown {desktop.account} {settings}"
        )
    with desktop.told("gone notifications"):
        machine.succeed(f"ln -sfn {meant} {settings}")
    machine.wait_until_fails(
        desktop.in_session("test -e $XDG_RUNTIME_DIR/niri-settings-notice"),
        timeout=timedelta(seconds=10),
    )


def test_a_notice_that_came_and_went_is_found_again_in_the_launcher(desktop: Desktop) -> None:
    """A notice that came and went is found again in the launcher."""
    machine = desktop.machine
    # The notice the first notification's subtest left to go by itself,
    # sent and let go here when that one hasn't run.
    if machine.execute(desktop.notice_kept('.summary == "Build finished"'))[0] != 0:
        with desktop.told("drawn notifications"):
            desktop.as_account("notify-send 'Build finished' 'the laptop is ready'")
        machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=timedelta(seconds=30),
        )
    # One its sender says not to keep isn't, though it's the newest match.
    desktop.as_account("notify-send --transient 'Build finished twice' 'not kept'")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Home's Now says one was missed earlier, since it left by itself.
    desktop.shoot("launcher-home-missed")
    # Its row is named by its title alone (VKCF3Z, 2026-11-08); the drawer
    # below shows it's the one kept, not the newer one sent as transient.
    desktop.type_in("build fin", selecting="Build finished")
    # Its drawer says each part once: the summary as its title, then the
    # body, not the row's name over both again (VKCF3Z, 2026-11-02).
    desktop.launcher_until(lambda shown: shown["drawer"]["out"])
    drawn = desktop.showing()["drawer"]
    assert (drawn["title"], drawn["body"]) == ("Build finished", "the laptop is ready"), drawn
    desktop.shoot("launcher-notices")
    # Nothing sent it that can be gone to, so Return steps in, as the right
    # arrow does in normal mode, where copying what it said comes last of
    # its acts, never first.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][-1:] == ["Copy What It Said"] and shown["rows"][0] != "Copy What It Said"
        )
    )
    desktop.down_to("Copy What It Said")
    machine.send_key("ret")

    def told_again(_: bool) -> bool:
        """Say whether the clipboard holds what the notice said."""
        said = desktop.as_account("wl-paste --no-newline")
        return said == "Build finished\nthe laptop is ready"

    retry(told_again, timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # With none on screen, Super+N opens all of them, which counts
    # them seen.
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    desktop.shoot("launcher-notices-all")
    desktop.close_launcher()


# The drawer's line pitch, which its view's tops past its rest lie on, so what it shows stays on
# the card's lines (vm-desktop-looks-plates measures them) and a line scrolled to the top takes the
# eyebrow's baseline, its capitals the inset under the frame as the head's are (VKCF3Z,
# 2026-11-05). Ctrl+E and Ctrl+Y scroll it a whole line, as Vim's do, so past a title a pitch and
# a half tall they scroll two pitches.
PITCH = 24
# A box's rounding, which a baseline may be off its place by and still be on it.
ROUNDING = 0.5
# The cursor's rule stands a mark from the letters, which begin the inset from the frame's inside
# edge, its hairline border left out (docs/design.md, the ladder).
MARK = 8
INSET = 24
BORDER = 1


def a_line_on(drawer: Json, past: int = 0) -> bool:
    """Say whether the drawer is scrolled past a point to a whole line on the eyebrow's baseline.

    The eyebrow scrolls with the rest, so where it rests is where it is now and how far the
    drawer has scrolled. The first of the words' lines at or under that baseline is on it, so
    no bare rows are left over the first line shown. Above the words, among its facts, each a
    label and a value on the drawer's pitch, it rests on a pitch from the fact it's in.
    """
    scrolled = int(drawer["scrolled"])
    lines = drawer["lines"]
    rest = lines["eyebrow"] + drawer["scrolled"]
    first = lines["words"] + max(0, math.ceil((rest - ROUNDING - lines["words"]) / PITCH)) * PITCH
    within = [fact for fact in lines.get("facts") or [] if fact is not None]
    within = [fact for fact in within if fact <= rest + ROUNDING]
    if within and rest < lines["words"] - ROUNDING:
        apart = (rest - max(within)) % PITCH
        on = min(apart, PITCH - apart) < ROUNDING
    else:
        on = abs(first - rest) < ROUNDING
    return scrolled > past and on and held(drawer)


# The sequences whose first key waits Joel's timeoutlen for the rest, and how many times one
# is pressed before its coming apart is a failure, and how long each try waits.
WAITING = frozenset(
    {
        "spc right",
        "spc left",
        "spc ?",
        "Z Z",
        "g g",
        "g v",
        "z a",
        "ctrl-w l",
        "ctrl-w h",
        "ctrl-w w",
    }
)
TRIES = 3
TRIED = timedelta(seconds=10)


class InTheDrawer:
    """The launcher holding a long notice's drawer, and the keys that go into it and move there.

    Its notice is a deploy's log of forty lines, so the drawer holds more than fits.
    """

    TITLE = "Deploy log"
    LINES = 40
    FIRST = "line 1 of the deploy went by"
    # More lines than its facts take between its title and its words: Arrived, Left and
    # Urgency, each a name and a value, a value wrapped included.
    FACT_LINES = 12
    # The lines its prose shows while the drawer's fold is shut (Drawer.qml's proseMost).
    SHOWN_FOLDED = 5

    def __init__(self, desktop: Desktop) -> None:
        """Send the notice, and open the launcher on its row."""
        self.desktop = desktop
        machine = desktop.machine
        # A subtest that failed with the launcher open, or at another scale, leaves it closed
        # and at 1 for this one.
        if machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
            desktop.close_launcher()
        desktop.as_account("niri msg output winit scale 1")
        body = "\n".join(f"line {at} of the deploy went by" for at in range(1, self.LINES + 1))
        desktop.as_account(f"notify-send {shlex.quote(self.TITLE)} {shlex.quote(body)}")
        machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=timedelta(seconds=60),
        )
        self.open()

    def open(self) -> None:
        """Open the launcher on the notice's row, its drawer out, in normal mode."""
        desktop = self.desktop
        desktop.machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10)
        )
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("deploy log")
        desktop.launcher_until(
            lambda shown: shown["drawer"]["out"] and shown["drawer"]["title"] == self.TITLE
        )
        desktop.normal()
        self.unfold()
        self.rows = desktop.showing()
        assert self.rows["drawer"]["cursor"] is None, self.rows["drawer"]

    def unfold(self) -> None:
        """Open the drawer's fold, which holds the log past its first five lines, by za.

        A drawer's prose shows five lines while its fold is shut (VKCF3Z's wrap rule 4), the
        last cut with an ellipsis; the fold stays as left from opening to opening, so only the
        first of these subtests finds it shut.
        """
        drawer = self.drawer()
        if drawer["folded"] is False:
            return
        assert drawer["bodyLines"] == self.SHOWN_FOLDED, drawer
        assert str(drawer["bodyDrawn"]).endswith("\u2026"), drawer
        self.pressed_until(
            "z a",
            lambda shown: (
                shown["drawer"]["folded"] is False and shown["drawer"]["bodyLines"] >= self.LINES
            ),
        )

    def press(self, keys: str) -> None:
        """Press each key named, all in one batch, as Desktop.press_at_once does."""
        self.desktop.press_at_once(keys)

    def pressed_until(
        self, keys: str, test: Callable[[Json], object], again: Callable[[], None] | None = None
    ) -> None:
        """Press the keys, and wait until the launcher passes the test.

        A sequence whose first key waits Joel's 300 ms for the rest is pressed again when it
        came apart, after `again` puts back what its second key did alone. Sent as one batch,
        its keys reach the guest together, so this is only for a machine so loaded that niri
        stalls between reading them: what a busy machine does to keys, not what the shell does.
        """
        for _ in range(TRIES - 1 if keys in WAITING else 0):
            self.press(keys)
            try:
                self.desktop.launcher_until(test, TRIED)
            except AssertionError:
                print(f"{keys} came apart, the wait letting go between them: pressed again")
                if again:
                    again()
            else:
                return
        self.press(keys)
        self.desktop.launcher_until(test)

    def drawer(self) -> Json:
        """Give the drawer as the launcher reports it."""
        return cast("Json", self.desktop.showing()["drawer"])

    def until(self, keys: str, test: Callable[[Json], object]) -> Json:
        """Press the keys, and wait until the drawer, holding the keys, passes the test."""
        self.pressed_until(
            keys, lambda shown: shown["drawer"]["cursor"] is not None and test(shown["drawer"])
        )
        return self.drawer()

    def into(self, keys: str = "ctrl-w l") -> Json:
        """Go into the drawer with the keys, Vim's Ctrl+W l unless named, and give its cursor.

        The l alone steps into the notice's row, so the launcher opens on it again before
        another try.
        """

        def again() -> None:
            self.desktop.close_launcher()
            self.open()

        self.pressed_until(
            keys,
            lambda shown: shown["mode"] == "page" and shown["drawer"]["cursor"] is not None,
            again,
        )
        return cast("Json", self.drawer()["cursor"])

    def to_the_words(self) -> int:
        """Move the cursor down past its title and its facts to the log's first line.

        A j that crosses from a fact into the words keeps the cursor where it is across, in
        another face, so a letter counted along a line is only the same letter among the log's
        own lines. Gives the line it stands on.
        """
        for _ in range(self.FACT_LINES):
            cursor = self.drawer()["cursor"]
            if str(cursor["says"]).startswith(self.FIRST):
                return int(cursor["line"])
            self.until("j", one_below(int(cursor["line"])))
        message = f"the log's first line is past its facts: {self.drawer()}"
        raise AssertionError(message)

    def back(self, keys: str) -> None:
        """Press the keys, and wait for the keys back on the rows, the selection unmoved."""
        self.pressed_until(
            keys, lambda shown: shown["mode"] == "normal" and not shown["drawer"]["keys"]
        )
        shown = self.desktop.showing()
        assert shown["open"], shown
        assert shown["selected"]["name"] == self.rows["selected"]["name"], shown
        assert shown["drawer"]["cursor"] is None, shown["drawer"]


def one_below(line: int) -> Callable[[Json], bool]:
    """Say whether the drawer's cursor stands on the line below this one."""
    return lambda drawer: bool(drawer["cursor"]["line"] == line + 1)


def held(drawer: Json) -> bool:
    """Say whether the drawer's cursor is wholly inside its view, whose top cuts no line.

    Vim's topline is always a whole line, so a view scrolled to a pitch partway through the
    title shows it as slivers (VKCF3Z, 2026-11-05).
    """
    top, height = int(drawer["cursor"]["top"]), int(drawer["cursor"]["height"])
    return top >= 0 and top + height <= int(drawer["height"]) and not drawer["cut"]


def test_a_notices_drawer_says_who_sent_it_and_its_keys_scroll_it(desktop: Desktop) -> None:
    """A notice's drawer says who sent it, and gone into, its keys scroll it.

    The foot names no Shift with the arrows, which scroll nothing now: Ctrl+W l goes in, three
    Ctrl+E scroll it three of its lines, each whole, and Ctrl+W h comes back, the selection
    where it was.
    """
    page = InTheDrawer(desktop)
    drawer = page.drawer()
    # From the program it came through, since the desktop's own is what For You's path names,
    # and a notice, filing no kind of its own; never a time, which is an Arrived fact.
    assert drawer["eyebrow"] == "from notify-send \u00b7 notice", drawer
    assert drawer["more"], f"forty lines fit the drawer: {drawer}"
    # Its foot counts each of the log's lines it hides as one, the drawer showing no more of
    # them than its pitch fits in its height.
    hidden = re.fullmatch(r"(\d+) more \u00b7 Ctrl\+W l", drawer["moreSays"])
    assert hidden, drawer
    assert int(hidden[1]) >= page.LINES - int(drawer["height"]) // PITCH, drawer
    assert "\u21e7" not in page.rows["selected"]["told"], page.rows["selected"]
    desktop.shoot("launcher-plate")
    title = drawer["lines"]["title"]
    page.into()
    one = int(page.until("ctrl-e", a_line_on)["scrolled"])
    two = int(page.until("ctrl-e", lambda drawer: a_line_on(drawer, one))["scrolled"])
    scrolled = page.until("ctrl-e", lambda drawer: a_line_on(drawer, two))
    assert round(title - scrolled["lines"]["title"]) == scrolled["scrolled"], scrolled
    page.back("ctrl-w h")
    desktop.close_launcher()


def into_in_sight(desktop: Desktop, page: InTheDrawer, scale: str) -> Json:
    """Take the keys into the drawer just opened at a scale, and return where its cursor is.

    The card and the drawer are held in sight on the way: the card inside the screen however
    small, its rows fewer, and at 2 the drawer taking the card's place once the keys go in,
    its title on one line and the card's foot still under it (VKCF3Z, 2026-11-05).
    """
    card = desktop.showing()["card"]
    assert card["top"] > 0, f"at {scale} it's off the top: {card}"
    assert card["foot"] < card["screen"], f"at {scale} it's off the bottom: {card}"
    assert card["left"] > 0, f"at {scale} it's off the left: {card}"
    assert card["right"] < card["across"], f"at {scale} it's off the right: {card}"
    rows = page.drawer()
    assert scale != "2" or rows["over"], f"at 2 the drawer is squeezed beside: {rows}"
    assert rows["over"] != rows["shown"], f"at {scale} the drawer is out as it lies: {rows}"
    at = page.into()
    inside = page.drawer()
    lines = inside["lines"]
    assert lines["title"] == lines["titleLast"], f"at {scale} the title breaks: {inside}"
    if inside["over"]:
        assert inside["shown"], f"at {scale} the drawer isn't shown inside: {inside}"
        assert inside["at"] == inside["cardAt"], f"at {scale} it's off the card: {inside}"
        assert inside["width"] == inside["cardWidth"], f"at {scale} it's narrower: {inside}"
        assert inside["height"] < inside["cardHeight"], f"at {scale} it hides the foot: {inside}"
    return at


def test_space_right_takes_the_keys_into_a_drawer_and_space_left_brings_them_back(
    desktop: Desktop,
) -> None:
    """Ctrl+W l takes the keys into the drawer, and Ctrl+W h gives them back, as in Vim.

    Ctrl+W w goes either way, ZZ and :q come back too, and his Neovim's Space and an arrow
    overlay them; a bare arrow inside is its letter's move (arrows_move_in_the_drawer).

    Its frame takes the accent the card's gives up, and the cursor stands on the title's first
    letter with nothing scrolled, its line marked by a rule in the margin left of the letters.
    Escape only lets a count go, and q does nothing; ZZ, :q and Ctrl+W h each
    give the keys back to the rows, the selection where it was. Its pictures at 1 and both of
    Joel's scales, on the title and down in the words, are for a person to judge the rule and
    the frames by: this screen is small enough that at 2 the card leaves the drawer no room.
    There it waits until the keys go in and then takes the card's place, its title on one line,
    rather than squeezed beside it until a word breaks (VKCF3Z, 2026-11-05: "Depl/oy/log").
    Then Space ? lists the drawer's own keys (listed_in_the_drawer), and d c p x and q say why
    they change nothing (refused_in_the_drawer), kept in this subtest so the file holds to the
    pyramid's twenty (docs/testing.md).
    """
    page = InTheDrawer(desktop)
    came = page.into()
    assert came["says"] == page.TITLE, came
    assert came["letter"] == 0, came
    # The foot names the way back and the drawer's keys, not the row's Return, which does
    # nothing in the drawer.
    told = desktop.showing()["selected"]["told"]
    assert told == ["Ctrl+W h back", "Space ? keys"], f"in the drawer the foot says {told}"
    assert came["on"] == page.TITLE[0], came
    # The rule is the drawer's pitch tall on every level, on its line's capitals, a mark from them.
    rule = came["rule"]
    assert rule["height"] == PITCH, came
    assert came["top"] < rule["top"] + PITCH / 2 < came["top"] + came["height"], came
    assert rule["width"] > 0, came
    assert rule["x"] + rule["width"] == BORDER + INSET - MARK, came
    drawer = page.drawer()
    assert drawer["scrolled"] == 0, drawer
    assert drawer["frame"] != page.rows["drawer"]["frame"], drawer
    # Each scale is set with the launcher closed, so it opens laid out for it, as
    # desktop-looks sets its own.
    for scale in (*JOELS_SCALES, "1"):
        desktop.close_launcher()
        desktop.as_account(f"niri msg output winit scale {scale}")
        page.open()
        into_in_sight(desktop, page, scale)
        named = scale.replace(".", "-")
        desktop.shoot(f"launcher-drawer-keys-at-{named}")
        # Down among the log's lines, below its facts, where a letter along one is the same
        # letter along the next.
        words = page.to_the_words()

        def five_down(drawer: Json, below: int = words + 5) -> bool:
            return bool(drawer["cursor"]["line"] == below)

        down = page.until("5 j", five_down)
        # The title scrolled away whole, and the foot names no Return, which does nothing here.
        assert held(down), f"at {scale} five down cuts a line: {down}"
        told = desktop.showing()["selected"]["told"]
        assert told == ["Ctrl+W h back", "Space ? keys"], f"at {scale} the foot says {told}"
        # Four along, or the line's last letter where a narrow drawer wraps it shorter.
        page.until(
            "l l l l",
            lambda drawer: (
                drawer["cursor"]["letter"] == min(len("line"), len(drawer["cursor"]["says"]) - 1)
            ),
        )
        desktop.shoot(f"launcher-drawer-cursor-at-{named}")
        # Scrolled a line further, for a person to judge the top's inset by: the first line
        # shown on the eyebrow's baseline, its capitals the inset under the frame.
        scrolled = page.until("ctrl-e", lambda drawer: drawer["scrolled"] > 0 and held(drawer))
        assert a_line_on(scrolled), f"at {named} bare rows lie over the first line: {scrolled}"
        desktop.shoot(f"launcher-drawer-scrolled-at-{named}")
    page.press("3")
    desktop.launcher_until(lambda shown: shown["counted"] == "3")
    page.press("esc")
    desktop.launcher_until(lambda shown: shown["counted"] == "" and shown["mode"] == "page")
    # Escape at rest and q alone leave the keys where they are: j still moves the cursor, one
    # line.
    line, letter = page.drawer()["cursor"]["line"], page.drawer()["cursor"]["letter"]
    came = page.until("esc q j", lambda drawer: drawer["cursor"]["line"] == line + 1)
    assert came["cursor"]["letter"] == min(letter, len(came["cursor"]["says"]) - 1), came
    arrows_move_in_the_drawer(page)
    page.back("Z Z")
    # Vim's window keys go in and come back: Ctrl+W l in and h out, Ctrl+W w either way, and
    # his Neovim's Space and an arrow over them (Joel: "leader stuff can overlay default stuff").
    for into, back in (
        ("ctrl-w l", ": q ret"),
        ("ctrl-w l", "ctrl-w h"),
        ("ctrl-w w", "ctrl-w w"),
        ("spc right", "spc left"),
    ):
        page.into(into)
        page.back(back)
    desktop.close_launcher()
    listed_in_the_drawer(desktop)
    refused_in_the_drawer(desktop)


def arrows_move_in_the_drawer(page: InTheDrawer) -> None:
    """In the drawer, each arrow is its letter's move, a count before it too, as in a buffer.

    Down and Up go a line as j and k do, and Right and Left a letter as l and h (Joel,
    2026-11-05: "In normal mode, in a buffer, arrows move the cursor").
    """
    line = int(page.drawer()["cursor"]["line"])
    page.until("2 down up", lambda drawer: drawer["cursor"]["line"] == line + 1)
    at = page.drawer()["cursor"]
    right = min(int(at["letter"]) + 1, len(at["says"]) - 1)
    page.until("right", lambda drawer: drawer["cursor"]["letter"] == right)
    page.until("left", lambda drawer: drawer["cursor"]["letter"] == max(right - 1, 0))


E21 = "E21: Cannot make changes"


def refused_in_the_drawer(desktop: Desktop) -> None:
    """In the drawer, d c p x say Vim's E21 and q says how to close, the keys staying there.

    A page can't be changed (VKCF3Z's spec v2, the Reader): x, p and S say it at once, and every
    other key Vim's nv_edit refuses there, i a o and their capitals entering no insert; d and c
    once their motion comes, r once its letter does, and visual's change keys as its choice
    ends, each where the line was and none as "not here", the cursor unmoved. q says what it
    says on the rows. Escape, a key that says nothing, is pressed between, so each word is seen
    to come of its own key.
    """
    page = InTheDrawer(desktop)
    page.into()
    line = page.drawer()["cursor"]["line"]

    def said(shown: Json) -> bool:
        return bool(
            shown["commandSaid"] == E21 and shown["mode"] == "page" and shown["saying"] == []
        )

    at_once = ("x", "p", "S", "D", "C", "X", "P", "s", "J", "~", "i", "a", "I", "A", "o", "O")
    waiting = ("d j", "c c", "r x", "r 5")
    chosen = tuple(f"{v} {key}" for v, key in zip("vVvVvVvVvV", "dxcpDCXPsS", strict=True))
    for keys in (*at_once, *waiting, *chosen, "v J", "V ~"):
        page.pressed_until(keys, said)
        assert page.drawer()["cursor"]["line"] == line, (keys, page.drawer())
        page.pressed_until("esc", lambda shown: shown["commandSaid"] == "")
    page.pressed_until(
        "q",
        lambda shown: (
            shown["saying"] == ["q does nothing", ":q closes"] and shown["mode"] == "page"
        ),
    )
    page.back("ctrl-w h")
    desktop.close_launcher()


def listed_in_the_drawer(desktop: Desktop) -> None:
    """In the drawer, Space ? lists the drawer's own keys in it, the keys staying there.

    The cursor stands on the list's title, and j moves down it. Space ? again gives the row's
    drawer back, and Ctrl+W h, bringing the keys back to the rows, puts the
    list away with them.
    """
    page = InTheDrawer(desktop)
    page.into()

    def listed(shown: Json) -> bool:
        cursor = shown["drawer"]["cursor"]
        return bool(
            shown["listing"]
            and shown["mode"] == "page"
            and cursor is not None
            and cursor["says"] == "The Drawer's Keys"
        )

    # The pair come apart leaves ? alone, which begins a search back, so Escape lets it go.
    def unsought() -> None:
        page.press("esc")

    page.pressed_until("spc ?", listed, unsought)
    line = page.drawer()["cursor"]["line"]
    page.until("j", lambda drawer: drawer["cursor"]["line"] == line + 1)
    page.pressed_until(
        "spc ?",
        lambda shown: not shown["listing"] and shown["drawer"]["title"] == page.TITLE,
        unsought,
    )
    page.pressed_until("spc ?", listed, unsought)
    page.back("ctrl-w h")
    assert not desktop.showing()["listing"], desktop.showing()
    desktop.close_launcher()


def test_in_a_drawer_the_keys_move_its_cursor_as_vims_do(desktop: Desktop) -> None:
    """In the drawer, j k h l and a count move its cursor, and the view follows it.

    j and k move it a line, after a count as many; h and l along the line; gg and G to its
    first and last line; Ctrl+E and Ctrl+Y scroll a line; Ctrl+D and Ctrl+U half the view;
    Ctrl+F, Ctrl+B, Page Down and Page Up nearly a whole one; and H, L and M to the view's top,
    bottom and middle lines, the view staying. Shift and an arrow do nothing. Then its search
    and its visual, each as a page's is in Vim (searched_as_vim_does,
    chosen_and_copied_as_vim_does), kept in this subtest so the file holds to the pyramid's
    twenty (docs/testing.md).
    """
    page = InTheDrawer(desktop)
    title = page.into()["line"]
    page.until("j", lambda drawer: drawer["cursor"]["line"] == title + 1)
    page.until("3 j", lambda drawer: drawer["cursor"]["line"] == title + 4)
    page.until("k", lambda drawer: drawer["cursor"]["line"] == title + 3)
    page.until("l l", lambda drawer: drawer["cursor"]["letter"] == 1 + 1)
    page.until("h", lambda drawer: drawer["cursor"]["letter"] == 1)
    last = page.until("G", lambda drawer: drawer["cursor"]["line"] == drawer["cursor"]["lines"] - 1)
    assert last["scrolled"] > 0, last
    # Said whole, a string pytest never shortens, so a failure shows where the view stood.
    assert held(last), f"G left the cursor outside the view or its top cut: {last}"
    # The drawer's last line, its last act, since its acts come after its words (drawers.md).
    assert last["cursor"]["says"], last
    acts = [str(act) for act in desktop.showing()["drawer"]["acts"]]
    assert acts, desktop.showing()["drawer"]
    assert acts[-1].endswith(last["cursor"]["says"]), (acts, last)
    page.until("g g", lambda drawer: drawer["cursor"]["line"] == 0 and drawer["scrolled"] == 0)
    page.until("ctrl-e", a_line_on)
    page.until("ctrl-y", lambda drawer: drawer["scrolled"] == 0)
    half = page.until("ctrl-d", lambda drawer: drawer["scrolled"] > 0 and held(drawer))
    page.until("ctrl-u", lambda drawer: drawer["scrolled"] == 0)
    paged = page.until("ctrl-f", lambda drawer: drawer["scrolled"] > half["scrolled"])
    assert held(paged), paged
    page.until("ctrl-b", lambda drawer: drawer["scrolled"] == 0)
    page.until("pgdn", lambda drawer: drawer["scrolled"] == paged["scrolled"])
    page.until("pgup", lambda drawer: drawer["scrolled"] == 0)
    page.until("ctrl-d", lambda drawer: drawer["scrolled"] == half["scrolled"])
    # H, L and M each move it from the line before, and the view stays.
    lines = [page.drawer()["cursor"]["line"]]
    for key in ("shift-h", "shift-l", "shift-m"):
        drawer = page.until(key, lambda drawer: drawer["cursor"]["line"] != lines[-1])
        assert held(drawer), drawer
        assert drawer["scrolled"] == half["scrolled"], drawer
        lines.append(drawer["cursor"]["line"])
    assert lines[1] < lines[3] < lines[2], lines
    # Shift and an arrow move nothing, so the k after them moves from where H M L left it.
    page.until(
        "shift-down shift-up k",
        lambda drawer: (
            drawer["cursor"]["line"] == lines[-1] - 1 and drawer["scrolled"] == half["scrolled"]
        ),
    )
    page.back("ctrl-w h")
    desktop.close_launcher()
    searched_as_vim_does(desktop)
    chosen_and_copied_as_vim_does(desktop)


def said_round_and_nowhere(page: InTheDrawer) -> None:
    """In the drawer, on the title a search went round to, the foot says each end it goes round.

    / came past the bottom, N goes back past the top to the title, its only capital, and what's
    found nowhere says E486, the cursor staying, each as Vim says it, until the next key, when
    the foot's keys come back.
    """
    page.desktop.launcher_until(lambda shown: shown["saying"] == [WRAPPED_BOTTOM])
    page.pressed_until(
        "shift-n",
        lambda shown: shown["saying"] == [WRAPPED_TOP] and shown["drawer"]["cursor"]["line"] == 1,
    )
    page.pressed_until(
        "/ z z z ret",
        lambda shown: (
            shown["saying"] == [E486_ZZZ]
            and shown["drawer"]["cursor"]["line"] == 1
            and shown["drawer"]["search"]["landing"] == "missing"
        ),
    )
    page.pressed_until(
        "j", lambda shown: shown["saying"] == [] and shown["drawer"]["cursor"]["line"] > 1
    )


def searched_as_vim_does(desktop: Desktop) -> None:
    """In the drawer, / and ? type what to seek, and Return puts the cursor where it's found.

    What's typed is washed as it's typed; Return puts the cursor on its first letter and the
    view follows; n goes on to the next, N back to the one before, and a count as many; ? with
    nothing typed seeks the last back; a capital asks for one, and the search goes round past
    the last line to the title, and N back round the top, the foot saying each as Vim's
    wrapscan does. With nothing sought n says E35, and what's found nowhere says E486, the
    cursor staying. Escape lets one being typed go, the keys staying in the drawer,
    as Backspace does with nothing left; Ctrl+W takes the word before and Ctrl+C lets it go,
    as on Vim's command line. The foot counts the match it would land on, and the one the
    cursor stands on, of how many. Its pictures at both of Joel's scales show the matches
    washed, for a person to judge the wash by.
    """
    page = InTheDrawer(desktop)
    page.into()
    page.pressed_until("n", lambda shown: shown["saying"] == [E35])
    page.press("/ l i n e spc 3 0")
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["washing"] == "line 30")
    typed = desktop.showing()["modeSays"]
    assert typed == "/line 30 \u00b7 1 of 1", f"typing, the foot says {typed}"
    found = page.until("ret", lambda drawer: drawer["cursor"]["says"].startswith("line 30 "))
    assert found["cursor"]["letter"] == 0, found
    assert found["scrolled"] > 0, found
    assert held(found), found
    assert found["search"] == {
        "typing": "",
        "last": "line 30",
        "washing": "line 30",
        "landing": "",
        "found": [1, 1],
    }, found

    def on(line: int) -> Callable[[Json], bool]:
        said = f"line {line} of the "
        return lambda drawer: (
            drawer["cursor"]["says"].startswith(said) and drawer["cursor"]["letter"] == len(said)
        )

    # Its own line's deploy is after the cursor, on its first letter.
    landed = page.until("/ d e p l o y ret", on(30))
    at, of = landed["search"]["found"]
    # The next is counted one on, of as many, until the cursor leaves a match.
    page.until("n", on(31))
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["found"] == [at + 1, of])
    counted = desktop.showing()["modeSays"]
    assert counted == f"/deploy \u00b7 {at + 1} of {of}", f"on a match, the foot says {counted}"
    # A letter on, it stands on none, and the foot counts nothing.
    page.until("l", lambda drawer: drawer["search"]["found"] is None)
    desktop.launcher_until(lambda shown: shown["modeSays"] == "")
    page.until("h", on(31))
    page.until("shift-n", on(30))
    page.until("3 n", on(33))
    page.until("? ret", on(32))
    # Smartcase: only the title holds a capital, so the search goes round to it.
    came = page.until("/ shift-d e p l o y ret", lambda drawer: drawer["cursor"]["line"] == 1)
    assert came["cursor"]["says"] == page.TITLE, came
    assert came["search"]["landing"] == "wrapped", came
    said_round_and_nowhere(page)
    # A search let go moves nothing, and the keys stay: j moves from where it stood.
    page.press("/ z z")
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["typing"] == "zz")
    line = page.drawer()["cursor"]["line"]
    page.until("esc j", lambda drawer: drawer["cursor"]["line"] == line + 1)
    page.until("? backspace j", lambda drawer: drawer["cursor"]["line"] == line + 2)
    # Its line edits as Vim's does: Ctrl+W takes the word before, and Ctrl+C lets it go.
    page.press("/ l i n e spc z z ctrl-w")
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["typing"] == "line ")
    page.until("ctrl-c j", lambda drawer: drawer["cursor"]["line"] == line + 3)
    searched_at_each_scale(page)


def searched_at_each_scale(page: InTheDrawer) -> None:
    """At each scale, a search from the title lands on the first line's deploy, and zzz on none.

    The cursor stands on the title's own Deploy, so the first line's deploy comes next, without
    going round: where a narrow drawer breaks that line, it's on a line of its own. Its
    pictures show the matches washed, and E486 said in the foot, for a person to judge.
    """
    desktop = page.desktop

    def first(title: int, drawer: Json) -> bool:
        cursor = drawer["cursor"]
        return bool(
            cursor["line"] > title
            and str(cursor["says"])[cursor["letter"] :].startswith("deploy")
            and drawer["search"]["landing"] == ""
        )

    for scale in (*JOELS_SCALES, "1"):
        desktop.close_launcher()
        desktop.as_account(f"niri msg output winit scale {scale}")
        page.open()
        # The title's line, which is the first where no eyebrow is shown.
        at = int(page.into()["line"])
        page.until("/ d e p l o y ret", functools.partial(first, at))
        named = scale.replace(".", "-")
        desktop.shoot(f"launcher-drawer-search-at-{named}")
        page.pressed_until("/ z z z ret", lambda shown: shown["saying"] == [E486_ZZZ])
        desktop.shoot(f"launcher-drawer-missing-at-{named}")
    page.back("ctrl-w h")
    desktop.close_launcher()


def chosen_and_copied_as_vim_does(desktop: Desktop) -> None:
    """In the drawer, v chooses letters from the cursor's and V lines, and y copies them.

    The moves extend what's chosen, and it's counted as the mode word counts it; y puts it on
    the clipboard as markdown, the title after its #, and goes back to the page with the
    cursor at its start, the flash it gives what it took ending; o goes to its other end,
    Escape lets it go, and gv brings it back. V grounds each line whole, a band across the
    measure, where v grounds only the letters. Its pictures at both of Joel's scales show
    letters chosen over two lines, for a person to judge the selection's ground and the block.
    """
    page = InTheDrawer(desktop)
    title = page.into()["line"]

    def chose(by: str, count: int) -> Callable[[Json], bool]:
        return lambda drawer: drawer["chosen"]["by"] == by and drawer["chosen"]["count"] == count

    came = page.until("v", chose("letters", 1))
    assert came["chosen"]["taken"] == [[title, 0, 1]], came
    # Letters are grounded as they're chosen, with no band.
    assert came["chosen"]["bands"] == [], came
    # Choosing, the foot names no key: nothing names Escape, which only leaves a mode, and
    # VIS and the count say where you are (VKCF3Z, 2026-11-05).
    told = desktop.showing()["selected"]["told"]
    assert told == [], f"in visual the foot says {told}"
    page.until("l l l l l", chose("letters", len("Deploy")))
    page.pressed_until(
        "y",
        lambda shown: shown["mode"] == "page" and shown["drawer"]["chosen"]["copied"] == "# Deploy",
    )
    desktop.machine.wait_until_succeeds(
        desktop.in_session("[ \"$(wl-paste --no-newline)\" = '# Deploy' ]"),
        timeout=timedelta(seconds=10),
    )
    desktop.launcher_until(lambda shown: not shown["drawer"]["chosen"]["flashing"])
    back = page.drawer()
    assert back["cursor"]["line"] == title, back
    assert back["cursor"]["letter"] == 0, back
    # Lines below the title and its facts, so what's copied holds no "log" to be found ahead
    # of the notice when the launcher next looks for its title.
    # V grounds whole lines, each one band across the measure and a mark past each side, that
    # meets the next.
    words = page.to_the_words()
    lines = page.until("shift-v j", chose("lines", 2))["chosen"]
    measure = lines["measure"] + 2 * MARK
    assert [(band["x"], band["width"]) for band in lines["bands"]] == [(-MARK, measure)] * 2, lines
    upper, lower = lines["bands"]
    assert upper["top"] + upper["height"] == lower["top"], lines
    page.until("o", lambda drawer: drawer["cursor"]["line"] == words)
    page.pressed_until(
        "esc", lambda shown: shown["mode"] == "page" and shown["drawer"]["chosen"]["by"] == ""
    )
    page.until("g v", chose("lines", 2))
    page.pressed_until(
        "y",
        lambda shown: (
            shown["drawer"]["chosen"]["copied"]
            == "line 1 of the deploy went by\nline 2 of the deploy went by"
        ),
    )
    # Y copies the whole line, however few of its letters are chosen, as Vim's v_Y.
    page.until("v l", chose("letters", 2))
    page.pressed_until(
        "shift-y",
        lambda shown: shown["drawer"]["chosen"]["copied"] == "line 1 of the deploy went by",
    )
    for scale in (*JOELS_SCALES, "1"):
        desktop.close_launcher()
        desktop.as_account(f"niri msg output winit scale {scale}")
        page.open()
        page.into()
        page.until("v l l l l l j", chose("letters", 2))
        desktop.shoot(f"launcher-drawer-chosen-at-{scale.replace('.', '-')}")
    page.pressed_until("esc", lambda shown: shown["mode"] == "page")
    page.back("ctrl-w h")
    desktop.close_launcher()


def test_a_notices_action_is_done_by_the_keys_and_one_not_acted_on_stays_unread(
    desktop: Desktop,
) -> None:
    """A notice's action is done by the keys, and one not acted on stays unread."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # It stays a minute, so it's still at the front when the keys come.
    with desktop.told("shows card Lunch"):
        desktop.as_account("notify-send --expire-time=60000 'Lunch' 'at noon'")
    # notify-send waits for the answer, so it runs on its own. The
    # shot waits for its card's frames to settle, which is when
    # Qt would have warned of a layout that never does.
    with desktop.told("shows card Recording saved"):
        desktop.as_account(
            "notify-send --action=play='Play it' 'Recording saved' 'x.mp4'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    desktop.shoot("notice-with-action")
    # Qt only warns when a layout never settles, which left the
    # card's words piled at its top.
    log = desktop.shell_said("polish() loop")
    assert "polish() loop" not in log, "a notice's layout never settles"
    # Held, the stack is its lines, which wait while it holds the
    # keys, the cursor on the card in front, the older; the newer is line
    # 1. Tab is no key of normal's, nor Backspace, h or l, and each says so: a card's acts are
    # commands, never a row stepped into (Joel, 2026-11-05: "where in vi does that step into
    # shit?"). :forg lets the older go unread, the cursor going on to the newer, whose card
    # lists what it offers as commands under its words, and : puts them first in the menu;
    # Tab chooses it and Return does it, with a token to come forward, giving the keys back.
    desktop.listen()
    # Its status line reaches the frame, so its text centres between the
    # hairline and the frame, as the launcher's does (docs/design.md: text
    # in a band centres in it).
    with (
        desktop.told("keys notices"),
        desktop.told("notice cursor Lunch"),
        desktop.told("notice foot ends at the frame"),
    ):
        desktop.hold_notices()
    desktop.shoot("notices-held")
    for key in ["tab", "backspace", "h", "l"]:
        with desktop.told("notice says not here"):
            machine.send_key(key)
    with (
        desktop.told("notice forgets 1"),
        desktop.told("notice cursor Recording saved"),
        desktop.told("notice offers :play"),
    ):
        machine.send_chars(":forg\n", delay=KEY_PACE)
    desktop.shoot("notices-held-verbs")
    with desktop.told("notice menu chose play"):
        machine.send_chars(":", delay=KEY_PACE)
        machine.send_key("tab")
    with desktop.told("lets notices go"):
        machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx play /tmp/answer", timeout=timedelta(seconds=15))
    desktop.tokened()
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=10)
    )
    # The history is kept on disk, the one acted on read and the one
    # only let go still unread.
    machine.wait_until_succeeds(
        desktop.notice_kept('.summary == "Recording saved" and .read'),
        timeout=timedelta(seconds=10),
    )
    machine.succeed(desktop.notice_kept('.summary == "Lunch" and (.read | not)'))


def test_a_notice_copied_from_its_card_says_copied(desktop: Desktop) -> None:
    """Copying what a notice said from its acts or by yy says so, and the clipboard holds it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Build log"):
        desktop.as_account("notify-send --expire-time=60000 'Build log' 'all green'")
    with desktop.told("keys notices"):
        desktop.hold_notices()
    # It offers nothing, so its card lists no command, and :y copies what it said, keeping
    # the keys.
    with desktop.told("copied card Build log"):
        machine.send_chars(":y\n", delay=KEY_PACE)
    assert desktop.as_account("wl-paste --no-newline") == "Build log\nall green"
    # yy copies it too, as Vim yanks a line.
    desktop.as_account("wl-copy nothing")
    with desktop.told("copied card Build log"):
        machine.send_chars("yy")
    assert desktop.as_account("wl-paste --no-newline") == "Build log\nall green"
    # Two more arrive while it's held, newer, so they're lines 1 and 2.
    for summary, said in [("Test log", "all run"), ("Deploy log", "all out")]:
        with desktop.told(f"shows card {summary}"):
            desktop.as_account(f"notify-send --expire-time=60000 '{summary}' '{said}'")
    for scale in JOELS_SCALES:
        found_chosen_and_undone(desktop, scale)
    desktop.as_account("niri msg output winit scale 1")
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )


def found_chosen_and_undone(desktop: Desktop, scale: str) -> None:
    """Find among three held notices, choose all three, copy them, be done and undo.

    At one of Joel's scales, for a person to look at the lines, the find, visual over three,
    the mode word, the foot and the : menu by: / narrows the lines as it's typed, Escape keeps
    what was found and a fresh / with nothing clears it; V k k chooses three and y copies
    them; V j j then dd is done with all three, and u brings the cards back; : opens the menu,
    which Tab chooses in; and Space ? lists every key, by mode, until Escape puts them away.
    """
    machine = desktop.machine
    desktop.as_account(f"niri msg output winit scale {scale}")
    named = scale.replace(".", "-")
    found_among_them(desktop, named)
    with desktop.told("notice cursor Build log"):
        machine.send_chars("G")
    with desktop.told("notice selects 1"):
        machine.send_chars("V")
    with desktop.told("notice selects 3"):
        machine.send_chars("kk", delay=KEY_PACE)
    desktop.shoot(f"notice-visual-at-{named}")
    with desktop.told("copied card Deploy log"):
        machine.send_chars("y")
    copied = desktop.as_account("wl-paste --no-newline")
    assert copied == "Deploy log\nall out\n\nTest log\nall run\n\nBuild log\nall green", copied
    # Joel's line: V j j then dd marks three read and takes them away. y left the cursor on
    # the first copied, line 1.
    with desktop.told("notice selects 3"):
        machine.send_chars("Vjj", delay=KEY_PACE)
    with desktop.told("notice done 3"):
        machine.send_chars("dd", delay=KEY_PACE)
    desktop.shoot(f"notice-done-at-{named}")
    for summary in ["Build log", "Test log", "Deploy log"]:
        machine.wait_until_succeeds(
            desktop.notice_kept(f'.summary == "{summary}" and .read'),
            timeout=timedelta(seconds=10),
        )
    with desktop.told("notice brought back 3"):
        machine.send_key("u")
    with desktop.told("notice pending d"):
        machine.send_chars("d")
    desktop.shoot(f"notice-pending-at-{named}")
    # The : takes the head, where the find's / stands, and its completions are the list's rows
    # on its columns, as the launcher's are (Joel, 2026-11-06: "they need to share more code
    # and UI/UX design").
    with desktop.told("notice mode command"), desktop.told("notice command stands in the head"):
        machine.send_key("esc")
        machine.send_chars(":")
    desktop.shoot(f"notice-menu-at-{named}")
    with desktop.told("notice menu chose delete"):
        machine.send_key("tab")
    with desktop.told("notice mode normal"):
        machine.send_key("esc")
    with desktop.told("notice lists keys"):
        machine.send_chars(" ?", delay=KEY_PACE)
    desktop.shoot(f"notice-keys-at-{named}")
    with desktop.told("notice hides keys"):
        machine.send_key("esc")


def found_among_them(desktop: Desktop, named: str) -> None:
    """Show the lines, then find Deploy among them, shot at the scale named.

    / narrows the lines as it's typed, Escape keeps what was found, and a fresh / left empty
    lists them all again, the cursor staying on what was found, as Vim's / leaves it.
    """
    machine = desktop.machine
    # The cursor on the oldest, Build log, line 3, by way of the newest, so it moves.
    with desktop.told("notice cursor Build log"):
        machine.send_chars("ggG", delay=KEY_PACE)
    desktop.shoot(f"notice-lines-at-{named}")
    with desktop.told("notice finds Deploy in 1"), desktop.told("notice cursor Deploy log"):
        machine.send_chars("/Deploy", delay=KEY_PACE)
    desktop.shoot(f"notice-find-at-{named}")
    # Where what it found opens nothing, Return lands on it in normal, as Vim's / lands on its
    # match, rather than saying it isn't here (VKCF3Z, 2026-11-06).
    with desktop.told("notice mode normal"), desktop.told("notice lands on Deploy"):
        machine.send_key("ret")
    desktop.shoot(f"notice-found-at-{named}")
    with desktop.told("notice mode normal"):
        machine.send_chars("/")
        machine.send_key("esc")


def test_a_click_on_a_notice_about_nothing_to_open_puts_it_away(desktop: Desktop) -> None:
    """A click on a notice about nothing to open puts it away, and never does what it offers.

    What it offers the keys reach, since Joel wants keys first and the line that offered it
    was clutter (look 3, 2026-11-03).
    """
    since = desktop.mark()
    x, y = desktop.offer_to_play("notice-to-put-away")
    desktop.click(x, y)
    assert desktop.answered() == "", "a click on the notice did what it offers"
    # It wasn't missed, so For You's tray never once lit for it: a notice
    # arriving or leaving mustn't flash it.
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert not re.search("holds margin [^\\n]*\\bforYou\\b", said), (
        "the margin flashed For You's tray for a notice"
    )


def test_a_click_on_a_notice_opens_what_its_about_with_a_token_to_come_forward(
    desktop: Desktop,
) -> None:
    """A click on a notice opens what it's about, with a token to come forward."""
    desktop.listen()
    x, y = desktop.offer_to_play("notice-to-open", offers="--action=default=Open")
    desktop.click(x, y)
    assert desktop.answered() == "default", "a click on the message didn't open what it's about"
    desktop.tokened()


def test_a_notice_is_signed_by_the_name_joel_knows_what_sent_it_by(desktop: Desktop) -> None:
    """A notice is signed by the name Joel knows what sent it by, never its program's.

    Its pictures at both of Joel's scales, alone and held with its acts listed as commands, are
    look 3's notice (2026-11-03), for a person to measure its room by. So are three notices on
    the arrival line, the front one drawn and the two behind it counted, and what's playing in
    the corner. Once what played has gone, niri blurs only under the line.
    """
    machine = desktop.machine
    try:
        for scale in JOELS_SCALES:
            signed_and_stacked_at(desktop, scale)
    finally:
        desktop.as_account("systemctl --user stop player || true")
        desktop.as_account("qs ipc call notices putAway")
        # Those resting off the screen outlive putting away, which takes only what's shown, so
        # each is closed as its sender would close it, by the number it was given.
        desktop.as_account(
            "for id in $(cat /tmp/rested 2>/dev/null); do busctl --user call"
            " org.freedesktop.Notifications /org/freedesktop/Notifications"
            " org.freedesktop.Notifications CloseNotification u $id || true; done;"
            " rm -f /tmp/rested"
        )
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account("qs ipc call notices seen")
    desktop.as_account("niri msg output winit scale 1")


def signed_and_stacked_at(desktop: Desktop, scale: str) -> None:
    """Sign a notice and shoot it, held, as a stack, under what's playing, and that alone.

    Each scale's notice comes after the last scale's track played alone, so nothing is blurred
    past the card.
    """
    machine = desktop.machine
    desktop.as_account(f"niri msg output winit scale {scale}")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # One whose time is up rests off the screen, still open, while the next stands alone; its
    # frame, never drawn, mustn't be blurred under the next card (Joel, 2026-11-09: "shadows
    # where things don't exist").
    with desktop.told("shows card Rested Off the Screen"):
        desktop.as_account(
            "notify-send --print-id --expire-time=1000 'Rested Off the Screen' 'by itself'"
            " >> /tmp/rested"
        )
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    with desktop.told("shows card Tokens Run Out in About an Hour"):
        desktop.as_account(
            "notify-send --app-name=tokens --expire-time=60000"
            " --action=default='See What Spends' --action=pause='Pause the Orchestrator'"
            " 'Tokens Run Out in About an Hour' 'At $300 an hour, 92% used. See what"
            " spends them, or pause the session spending most.' > /tmp/answer 2>&1"
            " < /dev/null &"
        )
    signed = json.loads(desktop.as_account("qs ipc call notices front")).get("signed")
    assert signed == "AI Spend", f"the tokens' notice is signed {signed}"
    blurs_only_what_is_drawn(desktop, "beside one resting")
    # Named without the point, which the driver would take for where the
    # name ends and its kind begins.
    named = scale.replace(".", "-")
    desktop.shoot(f"notice-at-{named}")
    under, shown = under_the_card(desktop)
    # Held, a row is the pop-up's notice in a list and a launcher row in its place: its number on
    # the inset, its mark in the mark column, and every line of it on the text column (Joel,
    # 2026-11-06: "they need to share more code and UI/UX design to stop them from constantly
    # drifting").
    with (
        desktop.told("keys notices"),
        desktop.told("notice offers :pause"),
        desktop.told("notice rows start on the inset"),
    ):
        desktop.hold_notices()
    desktop.shoot(f"notice-held-at-{named}")
    with desktop.told("notice selects 1"):
        machine.send_chars("V")
    desktop.shoot(f"notice-held-visual-at-{named}")
    with desktop.told("notice mode normal"):
        machine.send_key("esc")
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.held_still()
    kept = shown / detail(desktop.screen_now().crop(under))
    print(f"UNDER THE CARD at {under}, {kept:.2f} of the detail was left")
    assert kept > UNBLURRED, f"under the card, at {under}, only {kept:.2f} of the detail was left"
    pictured_linked_and_quiet_at(desktop, named)
    # Collected once it ends, since a stopped player exits failed and would
    # keep the unit's name from the next scale's.
    with desktop.told("drawn notifications"):
        desktop.as_account(
            "systemd-run --user --collect --unit=player mpv --ao=null --no-video"
            f" --loop {desktop.values['track']}"
        )
    desktop.shoot(f"notices-playing-at-{named}")
    # And alone, its player's mark in the mark column where a notice's
    # stands (VKCF3Z, 2026-11-06).
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_succeeds(
        desktop.in_session("test \"$(qs ipc call notices front)\" = '{}'"),
        timeout=timedelta(seconds=30),
    )
    desktop.shoot(f"playing-at-{named}")
    desktop.as_account("systemctl --user stop player")


def pictured_linked_and_quiet_at(desktop: Desktop, named: str) -> None:
    """Stack three notices, the front with a picture and a link, the next sent at low urgency.

    The server says it draws markup, links and pictures, so each reaches the front card; held,
    the low one is quieter than the rest; and the front one's drawer in the launcher draws the
    same, its acts opening the link (VKCF3Z, 2026-11-07).
    """
    machine = desktop.machine
    link = "https://docs.google.com/d/notes"
    for summary, body, more in [
        (
            "Standup in Ten Minutes",
            f'The usual room, with <a href="{link}">the notes</a> from Tuesday.',
            f"--hint=string:image-path:{desktop.values['wide']}",
        ),
        ("Backup Finished", "", "--urgency=low"),
        ("Tokens Run Out in About an Hour", "At $300 an hour, 92% used.", ""),
    ]:
        with desktop.told(f"shows card {summary}"):
            desktop.as_account(
                f"notify-send --expire-time=60000 {more} {shlex.quote(summary)} {shlex.quote(body)}"
            )
    front = json.loads(desktop.as_account("qs ipc call notices front"))
    assert front.get("pictured"), f"the standup's picture isn't on its card: {front}"
    assert front.get("links") == [link], front
    desktop.shoot(f"notices-stacked-at-{named}")
    with desktop.told("keys notices"):
        desktop.hold_notices()
    desktop.shoot(f"notices-held-quiet-at-{named}")
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    # Put away, so the launcher takes the keys; the history keeps each.
    blurs_only_under_the_cards(desktop)
    if machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("standup ten", selecting="Standup in Ten Minutes")
    desktop.launcher_until(lambda shown: shown["drawer"]["out"])
    # Gone into, as at these scales the drawer takes the card's place only then.
    desktop.normal()
    desktop.press_at_once("ctrl-w l")
    desktop.launcher_until(lambda shown: shown["mode"] == "page")
    desktop.shoot(f"notice-drawer-at-{named}")
    desktop.press_at_once("ctrl-w h")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Open the Link" in shown["rows"])
    desktop.shoot(f"notice-acts-at-{named}")
    desktop.close_launcher()


def blurs_only_what_is_drawn(desktop: Desktop, when: str) -> None:
    """Hold what niri blurs, and where the pointer reaches, to the frames of what's drawn.

    A card resting off the screen, or past the last edge, is tracked but never drawn, and a
    card moves with the stack, which a frame given the card alone follows only when told (Joel,
    2026-11-09: "shadows where things don't exist").
    """
    desktop.held_still()
    front = json.loads(desktop.as_account("qs ipc call notices front"))
    # Gathered in the order they're tracked, meant in the order they stand, so held as sets.
    blurred, drawn = sorted(front["blurred"]), sorted(front["drawn"])
    assert blurred == drawn, f"{when}, niri blurs {blurred} where {drawn} are drawn"


def blurs_only_under_the_cards(desktop: Desktop) -> None:
    """Blur only under the arrival line, with more notices waiting than it draws.

    Five notices wait and the line draws the front one alone, so what niri blurs, and where
    the pointer reaches, is the line's frame and nothing of the four behind it. Under its foot,
    a blur would take out the detail a shadow only darkens, so the room is held to the same
    room once the notices are put away (Joel, 2026-11-09: "shadows where things don't exist").
    """
    machine = desktop.machine
    for summary in ["Fourth in Line", "Fifth in Line"]:
        with desktop.told(f"shows card {summary}"):
            desktop.as_account(f"notify-send --expire-time=60000 {shlex.quote(summary)} 'behind'")
    blurs_only_what_is_drawn(desktop, "with four waiting behind the line")
    under, shown = under_the_card(desktop)
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    machine.wait_until_fails(desktop.in_session(NOTICE_LINE), timeout=timedelta(seconds=30))
    desktop.held_still()
    kept = shown / detail(desktop.screen_now().crop(under))
    print(f"UNDER THE LINE at {under}, {kept:.2f} of the detail was left")
    assert kept > UNBLURRED, f"under the line, at {under}, only {kept:.2f} was left"


def under_the_card(desktop: Desktop) -> tuple[tuple[int, int, int, int], float]:
    """Give the room under the stack's foot, in the screen's points, and how much detail it holds.

    A card shown after what played alone, at a new scale, came in where it would stand under
    the track, and the column moved it up a frame later, which its window's regions didn't
    follow; left there, they blurred a band under the card to the window's foot (Y2YBTS,
    2026-11-06). A blur takes the detail out of what's under it, where a shadow only darkens
    it, so it's held to the same room once the card has gone. Read just after a new scale,
    before the card came, the room once held no detail at all (2026-11-06).
    """
    desktop.held_still()
    shown = desktop.screen_now()
    card = json.loads(desktop.as_account("qs ipc call notices front"))
    logical = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]
    wide, high, scale = logical["width"], logical["height"], logical["scale"]
    space = json.loads(desktop.as_account("cat ~/.config/jitsusama/theme.json"))["space"]
    inset, tie = space["rung"]["inset"], space["rung"]["tie"]
    # The room under the stack, past its last edge, as far as its shadow reaches, and inside
    # the front card's sides.
    under = (
        int((card["left"] * wide + inset) * scale),
        int((card["foot"] * high + tie) * scale),
        int((card["right"] * wide - inset) * scale),
        int((card["foot"] * high + inset) * scale),
    )
    return under, detail(shown.crop(under))


def detail(picture: Image.Image) -> float:
    """Give how far, from 0 to 255, each of a picture's points is from the one beside it."""
    grey = picture.convert("L")
    beside = ImageChops.difference(
        grey.crop((1, 0, grey.width, grey.height)), grey.crop((0, 0, grey.width - 1, grey.height))
    )
    return float(ImageStat.Stat(beside).mean[0])


def test_a_notice_from_calls_is_signed_as_a_call_and_wears_a_calls_glyph(desktop: Desktop) -> None:
    """A notice call-transcripts sends is signed Calls, in a call's glyph and a person's hue.

    Calls is the name its notices go by (notices.rs); unknown, a notice is signed by its own
    name but wears the bell any program's would, in a program's hue.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    with desktop.told("shows card Standup Is Written Down"):
        desktop.as_account(
            "notify-send --app-name=Calls --hint=string:x-answers:call --expire-time=60000"
            " --action=default='Open the Transcript' -- 'Standup Is Written Down'"
            " 'Its transcript is in its folder.' > /dev/null 2>&1 < /dev/null &"
        )
    front = json.loads(desktop.as_account("qs ipc call notices front"))
    assert (front.get("signed"), front.get("glyph"), front.get("hue")) == (
        "Calls",
        CALL_GLYPH,
        "person",
    ), f"Calls' notice is {front}"
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account("qs ipc call notices seen")


def test_return_on_a_notice_in_the_launcher_opens_what_its_about_with_a_token(
    desktop: Desktop,
) -> None:
    """Return on a notice in the launcher opens what it's about, with a token."""
    machine = desktop.machine
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    desktop.listen()
    with desktop.told("shows card Thread with Sam"):
        desktop.as_account(
            "notify-send --expire-time=60000 --action=default=Open 'Thread with Sam' 'see it'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    # Super+N opens the notices in the launcher on the card in front,
    # where Return opens it.
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Sam")
    )
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=timedelta(seconds=15))
    desktop.tokened()


def test_a_notices_commands_are_what_its_launcher_row_holds(
    desktop: Desktop,
) -> None:
    """A notice's commands, on its card and in its menu, are what its launcher row holds."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.listen()
    with desktop.told("shows card Thread with Ana"):
        desktop.as_account(
            "notify-send --expire-time=60000 --action=default=Open --action=play='Play it'"
            " 'Thread with Ana' 'see it' > /tmp/answer 2>&1 < /dev/null &"
        )
    # Its card lists what it offers, and : its menu with Return's Open first, then Play it.
    with desktop.told("keys notices"), desktop.told("notice offers :play"):
        desktop.hold_notices()
    with desktop.told("notice menu chose open"):
        machine.send_chars(":", delay=KEY_PACE)
        machine.send_key("tab")
    with desktop.told("notice menu chose play"):
        machine.send_key("tab")
    with desktop.told("notice mode normal"):
        machine.send_key("esc")
    # :e Notices hands on to the history, which opens on its first row,
    # the newest notice, as every place opens on its first. Its acts are
    # commands, not parts (VKCF3Z, 2026-11-05), so the foot offers no
    # Tab into them, and Return opens it as its card's does.
    desktop.notices_history()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Ana")
    )
    desktop.normal()
    told = desktop.showing()["selected"]["told"]
    assert "Tab in" not in told, f"a notice offers Tab into its acts: {told}"
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=timedelta(seconds=15))


def test_a_notice_opened_brings_its_sender_forward_when_it_stays_behind(
    desktop: Desktop,
) -> None:
    """A notice opened brings its sender forward when it doesn't come by itself.

    notify-send ignores the token it's handed, as Slack and Chrome often do, so its
    window, a kitty going by the sender's name, only comes if the shell brings it.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.as_account("niri msg action spawn -- kitty --class stand-in-chat --title 'Chat'")
    machine.wait_until_succeeds(
        desktop.in_session('niri msg --json windows | grep -qF \'"title":"Chat"\''),
        timeout=timedelta(seconds=30),
    )
    elsewhere = desktop.opened("Elsewhere")
    with desktop.told("shows card Thread with Kim"):
        desktop.as_account(
            "notify-send --app-name=stand-in-chat --expire-time=60000 --action=default=Open"
            " 'Thread with Kim' 'see it' > /tmp/answer 2>&1 < /dev/null &"
        )
    with desktop.told("keys notices"):
        desktop.hold_notices()
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=timedelta(seconds=15))
    retry(
        lambda _: desktop.window_titled("Chat")["is_focused"],
        timeout=timedelta(seconds=10),
    )
    desktop.close(desktop.window_titled("Chat"))
    desktop.close(elsewhere)


def test_a_notice_whose_time_is_up_still_does_what_it_offers_from_the_launcher(
    desktop: Desktop,
) -> None:
    """A notice whose time is up still does what it offers from the launcher."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # It says no time of its own, so it goes by itself after the ten
    # seconds one offering something has, unanswered; and notify-send,
    # which stops waiting once a time it was given is up, waits on.
    with desktop.told("rests Quarry take"):
        desktop.as_account(
            "notify-send --expire-time=0 --action=play='Play it' 'Quarry take' 'z.mp4'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=10))
    # Gone from the screen but still open: closing it would have ended
    # notify-send, as it ends a program's hold on what it offered.
    machine.succeed("pgrep -f '[n]otify-send .*--action=play'")
    desktop.listen()
    # With none on screen, Super+N opens the history, newest first.
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    # Its row is one line; its drawer reads it whole, the summary as its
    # title over the body.
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"]
            and (shown["drawer"]["title"], shown["drawer"]["body"]) == ("Quarry take", "z.mp4")
        )
    )
    # With nothing to open or go to, Return steps in, and what it offers
    # leads; copying what it said is last.
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["Play it"] and shown["rows"][-1:] == ["Copy What It Said"]
        )
    )
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Play it")
    machine.send_key("ret")
    assert desktop.answered() == "play", "a notice whose time was up lost what it offered"
    desktop.tokened()
    # Every row at once reaches each sender as one row does (FUW70D, 2026-11-09): :%forget
    # lets each still open go unanswered, and :%delete reads each at its sender, where it
    # offers Read, as d does.
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    every_row_reaches_its_sender(desktop, "forget", "")
    every_row_reaches_its_sender(desktop, "delete", "read")


def every_row_reaches_its_sender(desktop: Desktop, command: str, answer: str) -> None:
    """Rest two notices offering Read, run a command on every row, and hear each sender's answer.

    notify-send waits on its notice until it closes and then says the action done, if any, so
    both ending says the command reached the bus for each, and what they said, "" for none,
    says which it was.
    """
    machine = desktop.machine
    titles = [f"Kiln {command} one", f"Kiln {command} two"]
    with desktop.told(f"rests {titles[0]}"), desktop.told(f"rests {titles[1]}"):
        for title in titles:
            desktop.as_account(
                f"notify-send --expire-time=0 --action=read='Mark as read' '{title}' 'fired'"
                f" > '/tmp/{title}' 2>&1 < /dev/null &"
            )
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=10))
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    # Escape gives normal from the find, where : begins a command, and never closes.
    machine.send_key("esc")
    machine.send_chars(f":%{command}\n", delay=KEY_PACE)

    # Not every row can, Mark Them Read and History never, so it asks first, as Vim's :confirm
    # does. How many can depends on what the subtests before left: a notice already closed
    # can't be forgotten, and a whole run left 9 of 26 that could, so only the two here are
    # counted on.
    def asked(shown: Json) -> bool:
        count = cast("int", shown["count"])
        said = re.fullmatch(
            rf"(\d+) of {count} can {command}; {command} them\? \(y/n\)",
            cast("str", shown["question"]),
        )
        return (
            said is not None and len(titles) <= int(said[1]) < count and bool(shown["takesAnswer"])
        )

    desktop.launcher_until(asked)
    machine.send_chars("y")
    desktop.launcher_until(lambda shown: shown["question"] == "")
    # What d is done with is let go as the launcher closes, so u can bring it back till then.
    desktop.close_launcher()
    machine.wait_until_fails(
        f"pgrep -f '[n]otify-send .*Kiln {command}'", timeout=timedelta(seconds=15)
    )
    said = [machine.succeed(f"cat '/tmp/{title}'").strip() for title in titles]
    assert said == [answer, answer], f":%{command} gave its senders {said}, not {answer!r}"


def test_the_notices_sit_the_frames_rung_inside_the_window_under_them(desktop: Desktop) -> None:
    """The notices sit the frames rung inside the window under them, the screen rung at least."""
    machine = desktop.machine
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    # A window of its own, the only one in view, two thirds of the
    # screen wide and centred, so its edge is far from the corner's;
    # it goes again after.
    desktop.as_account("niri msg action spawn -- kitty --title 'Under the notices'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json windows | grep -q '\"Under the notices\"'"),
        timeout=timedelta(seconds=30),
    )
    under = desktop.window_titled("Under the notices")
    desktop.as_account(f"niri msg action focus-window --id {under['id']}")
    screen = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]["width"]
    wide = int(screen * 2 / 3)
    desktop.as_account(f"niri msg action set-column-width {wide}")
    # niri centres a column by the width its window has, and kitty takes
    # its new width only when it next draws, so centring before then
    # leaves the column to grow rightwards from where it was centred.
    retry(
        lambda _: desktop.window_titled("Under the notices")["layout"]["tile_size"][0] >= wide,
        timeout=timedelta(seconds=15),
    )
    desktop.as_account("niri msg action center-column")
    space = json.loads(desktop.as_account("cat ~/.config/jitsusama/theme.json"))["space"]

    # Where it will be once the view rests, from niri.
    def settled(last: bool) -> bool:
        """Say whether niri has the window where it will be once the view rests."""
        layout = desktop.window_titled("Under the notices")["layout"]
        place = layout["tile_pos_in_workspace_view"]
        if last:
            # Where niri has it, and what else is on the screen, to say why when it's elsewhere.
            print(f"UNDER THE NOTICES at {place}, {layout['tile_size']} wide, screen {screen}")
            print(desktop.as_account("niri msg windows"))
            assert place is not None, "niri doesn't say where a tiled window is"
        return place is not None and screen - (place[0] + layout["tile_size"][0]) > FROM_THE_CORNER

    retry(settled, timeout=timedelta(seconds=15))
    with desktop.told("shows card Inside the window"):
        desktop.as_account(
            "notify-send --expire-time=60000 'Inside the window' 'a frame on a frame'"
        )
    desktop.shoot("notice-inside")
    # The nearest window wholly on screen, as the shell picks it.
    ws = desktop.window_titled("Under the notices")["workspace_id"]
    tiles = [
        (w["layout"]["tile_pos_in_workspace_view"], w["layout"]["tile_size"])
        for w in json.loads(desktop.as_account("niri msg --json windows"))
        if w["workspace_id"] == ws and w["layout"]["tile_pos_in_workspace_view"]
    ]
    whole = [(p, s) for p, s in tiles if p[0] >= 0 and p[0] + s[0] <= screen]
    (left, top), (width, _) = min(whole, key=lambda t: (-(t[0][0] + t[1][0]), t[0][1]))
    step, edge = space["rung"]["frames"], space["rung"]["screen"]
    right = int(max(screen - (left + width) + step, edge) + 0.5)
    # A card's own width, which its words' measure sets, leads what the shell says.
    card, *notices_at = (
        int(n)
        for n in desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep -o 'report\\[places notices [0-9 ]*' | tail -n 1"
        ).split()[-4:]
    )
    wanted_at = [
        right,
        int(max(top + step, edge) + 0.5),
        min(card, int(screen - right - (left + step))),
    ]
    assert notices_at == wanted_at, "the notices aren't inside the window"
    desktop.as_account("qs ipc call notices putAway")
    desktop.as_account(f"niri msg action close-window --id {under['id']}")
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q '\"Under the notices\"'"),
        timeout=timedelta(seconds=30),
    )


# Super+Shift+N puts the cards away, and each is still there unread.


def test_every_held_key_does_what_vims_does(desktop: Desktop) -> None:
    """Every key the held notices take does what Vim's does, each told by what the list says.

    Four notices from two senders, the newest on line 1, pressed as EVERY_HELD_KEY lists: the
    motions, the sender's run as a paragraph, d with a motion and an object, ., u and Ctrl+R,
    y, visual's o, gv and x, q, keys with no meaning here, Space u l, the : line's ranges, :g
    and its errors, the find, the menu, the key list, and every way to let go.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    for sender, summary in [
        ("Mail", "Alpha one"),
        ("Mail", "Alpha two"),
        ("Chat", "Bravo one"),
        ("Chat", "Bravo two"),
    ]:
        with desktop.told(f"shows card {summary}"):
            desktop.as_account(
                f"notify-send --app-name={sender} --expire-time=60000 '{summary}' 'words'"
            )
    with desktop.told("keys notices"), desktop.told("notice cursor Alpha one"):
        desktop.hold_notices()
    # Lines: Bravo two, Bravo one, Alpha two, Alpha one. Chat's run is the first two.
    pressed_and_told(desktop, EVERY_HELD_KEY)
    # Every way to let go: ZZ, Space w, Super+N and :q, never Escape (Joel, 2026-11-05: "fuck
    # escape quitting").
    letting_go: list[str | tuple[str, ...]] = ["ZZ", " w", ("meta_l-n",), ":q\n"]
    for keys in letting_go:
        pressed_and_told(desktop, [(keys, "lets notices go")])
        with desktop.told("keys notices"):
            desktop.hold_notices()
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # Twelve, more than the eight lines held notices show, scroll as Vim's lines do (Joel,
    # 2026-11-05: "I can't scroll up and down in a notices list now").
    for number in range(1, 13):
        with desktop.told(f"shows card Feed {number}"):
            desktop.as_account(
                f"notify-send --app-name=Feed --expire-time=60000 'Feed {number}' 'words'"
            )
    with desktop.told("keys notices"):
        desktop.hold_notices()
    pressed_and_told(desktop, HELD_SCROLLS)
    with desktop.told("notice view from 5"):
        desktop.wheel(0.85, 0.15, 3)
    with desktop.told("notice view from 2"):
        desktop.wheel(0.85, 0.15, -1)
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


# Keys pressed on twelve held notices, eight shown with three kept from the view's edges: the
# arrows as j and k, Ctrl+E and Ctrl+Y a line, and zz zt zb about the cursor, which a scroll
# keeps in view. Feed 12 is line 1, and the cursor opens on Feed 1, the oldest.
HELD_SCROLLS: list[tuple[str | tuple[str, ...], str]] = [
    ("gg", "notice cursor Feed 12"),
    ("G", "notice cursor Feed 1"),
    ("gg", "notice cursor Feed 12"),
    (("down",), "notice cursor Feed 11"),
    (("up",), "notice cursor Feed 12"),
    (("ctrl-e",), "notice view from 2"),
    (("ctrl-y",), "notice view from 1"),
    ("zt", "notice view from 2"),
    ("zb", "notice view from 1"),
    ("zz", "notice view from 2"),
]


# Keys pressed on the held notices of test_every_held_key_does_what_vims_does, each with what
# the list says it did: typed as letters, or pressed by QEMU's names where they're a tuple.
EVERY_HELD_KEY: list[tuple[str | tuple[str, ...], str]] = [
    # The motions, the sender's run a paragraph.
    ("k", "notice cursor Alpha two"),
    ("2k", "notice cursor Bravo two"),
    ("j", "notice cursor Bravo one"),
    ("G", "notice cursor Alpha one"),
    ("gg", "notice cursor Bravo two"),
    ("3G", "notice cursor Alpha two"),
    ("H", "notice cursor Bravo two"),
    ("L", "notice cursor Alpha one"),
    ("M", "notice cursor Bravo one"),
    ("}", "notice cursor Alpha two"),
    ("{", "notice cursor Bravo two"),
    (("ctrl-d",), "notice cursor Alpha one"),
    (("ctrl-u",), "notice cursor Bravo two"),
    (("ctrl-f",), "notice cursor Alpha one"),
    (("ctrl-b",), "notice cursor Bravo two"),
    # dip is done with Chat's run; u brings it back, Ctrl+R takes it again, u once more, and
    # . does dip again.
    ("dip", "notice done 2"),
    ("u", "notice brought back 2"),
    (("ctrl-r",), "notice says 2 taken again"),
    ("u", "notice brought back 2"),
    (".", "notice done 2"),
    ("u", "notice brought back 2"),
    ("yj", "copied card Bravo two"),
    ("q", "notice says q does nothing \u00b7 :q closes"),
    ("n", "notice says not here"),
    ("c", "notice says not here"),
    # An arrow is its letter's motion (Joel, 2026-11-05), and Right's l steps into nothing
    # here, so it says so as any key with no meaning does.
    (("right",), "notice says not here"),
    (" ul", "notice numbers absolute"),
    (" ul", "notice numbers relative"),
    # Visual: o goes to its other end, Escape ends it, gv brings it back, x is done with it.
    # The foot's start names each mode in three capitals, NOR again once it ends.
    ("V", "notice shows VIS"),
    ("j", "notice selects 2"),
    ("o", "notice cursor Bravo two"),
    (("esc",), "notice shows NOR"),
    ("gv", "notice mode visual"),
    ("x", "notice done 2"),
    ("u", "notice brought back 2"),
    # The command line: :g, a range, a line number, and Vim's own errors.
    (":g/Bravo/d\n", "notice done 2"),
    ("u", "notice brought back 2"),
    (":3\n", "notice cursor Alpha two"),
    (":1,2forg\n", "notice forgets 2"),
    ("u", "notice brought back 2"),
    (":wifi\n", "notice says E492: Not an editor command: wifi"),
    (":2reply\n", "notice says E481: No range allowed"),
    (":set rnu?\n", "notice says   relativenumber"),
    # The find: i opens it, it narrows as it's typed, Down chooses, and Backspace past its
    # start leaves it.
    ("i", "notice shows FND"),
    ("Alpha", "notice finds Alpha in 2"),
    (("down",), "notice cursor Alpha one"),
    (("backspace",) * 6, "notice mode normal"),
    # The menu: Tab chooses its lines, Ctrl+E puts it away, Escape drops the line.
    (":", "notice shows CMD"),
    (("tab",), "notice menu chose delete"),
    (("ctrl-e", "esc"), "notice mode normal"),
    # F1 lists the keys, j scrolls them, and Escape puts them away.
    (("f1",), "notice lists keys"),
    ("j", "notice lists keys from 2"),
    (("esc",), "notice hides keys"),
    # At rest Escape does nothing, as Vim's, and never lets them go: the notices are still
    # held for the key after it.
    (("esc", "right"), "notice says not here"),
]


def pressed_and_told(desktop: Desktop, steps: list[tuple[str | tuple[str, ...], str]]) -> None:
    """Press each step's keys, typed as letters or by QEMU's names, and wait for what it says."""
    for keys, said in steps:
        with desktop.told(said):
            if isinstance(keys, tuple):
                for key in keys:
                    desktop.machine.send_key(key)
            else:
                desktop.machine.send_chars(keys, delay=KEY_PACE)


def test_a_held_notice_takes_a_reply_typed_on_it(desktop: Desktop) -> None:
    """A notice whose sender takes a reply typed here is answered from :reply's prompt.

    :reply opens a prompt on the card in insert; Escape gives normal over its words, where b
    and cw change one, and Return sends what's written, which the sender hears.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # notify-send offers no inline reply, so the notice is asked for on the bus itself, with
    # systemd's busctl, and what the shell answers is watched there too.
    desktop.as_account("stdbuf -oL busctl --user monitor > /tmp/replied 2>&1 < /dev/null &")
    with desktop.told("shows card Thread with Bo"):
        desktop.as_account(
            "busctl --user call org.freedesktop.Notifications /org/freedesktop/Notifications"
            " org.freedesktop.Notifications Notify susssasa{sv}i"
            " Chat 0 '' 'Thread with Bo' 'lunch?' 2 inline-reply Reply 0 60000"
        )
    with desktop.told("keys notices"):
        desktop.hold_notices()
    with desktop.told("notice mode insert"), desktop.told("notice shows INS"):
        machine.send_chars(":reply\n", delay=KEY_PACE)
    machine.send_chars("on my way", delay=KEY_PACE)
    for scale in JOELS_SCALES:
        desktop.as_account(f"niri msg output winit scale {scale}")
        desktop.shoot(f"notice-reply-at-{scale.replace('.', '-')}")
    desktop.as_account("niri msg output winit scale 1")
    with desktop.told("notice mode prompt"), desktop.told("notice shows NOR"):
        machine.send_key("esc")
    with desktop.told("notice mode insert"):
        machine.send_chars("bcw", delay=KEY_PACE)
    with desktop.told("notice replied Thread with Bo"):
        machine.send_chars("road\n", delay=KEY_PACE)
    machine.wait_until_succeeds(
        "grep -q NotificationReplied /tmp/replied && grep -q 'on my road' /tmp/replied",
        timeout=timedelta(seconds=10),
    )
    with desktop.told("lets notices go"):
        machine.send_chars(":q\n")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


def test_super_shift_n_puts_the_notices_away(desktop: Desktop) -> None:
    """Super+Shift+N puts the notices away."""
    machine = desktop.machine
    with desktop.told("shows card Standup"):
        desktop.as_account("notify-send --urgency=critical 'Standup' 'in five minutes'")
    with desktop.told("gone notifications"):
        machine.send_key("meta_l-shift-n")
    machine.wait_until_succeeds(
        desktop.notice_kept('.summary == "Standup" and (.read | not)'),
        timeout=timedelta(seconds=10),
    )


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

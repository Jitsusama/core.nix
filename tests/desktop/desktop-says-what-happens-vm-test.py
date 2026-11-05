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
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import FROM_THE_CORNER, KEY_PACE, LAUNCHER_OPEN, NOTICES, retry
from test_driver.machine import CHAR_TO_KEY

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
    kept = desktop.as_account(
        "find ~/.local/state/quickshell -name notices.json"
        ' -exec grep -l \'"summary":"Build finished"\' {} + 2>/dev/null; true'
    )
    if not kept.strip():
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
    desktop.photograph("launcher-home-missed")
    desktop.type_in("build fin", selecting="the laptop is ready")
    # Its drawer says each part once: the summary as its title, then the
    # body, not the row's name over both again (VKCF3Z, 2026-11-02).
    desktop.launcher_until(lambda shown: shown["drawer"]["out"])
    drawn = desktop.showing()["drawer"]
    assert (drawn["title"], drawn["body"]) == ("Build finished", "the laptop is ready"), drawn
    desktop.photograph("launcher-notices")
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
    desktop.photograph("launcher-notices-all")
    desktop.close_launcher()


# The drawer's line pitch, which Ctrl+E and Ctrl+Y scroll it by, so what it shows stays on the
# card's lines (vm-desktop-looks measures them).
PITCH = 24
# How far three Ctrl+E scroll it.
SCROLLED = 3 * PITCH
# The sequences whose first key waits Joel's timeoutlen for the rest, and how many times one
# is pressed before its coming apart is a failure, and how long each try waits.
WAITING = frozenset({"spc right", "spc left", "Z Z", "g g", "g v"})
TRIES = 3
TRIED = timedelta(seconds=10)


def keyed(name: str, *, down: bool) -> Json:
    """Give QEMU's input event for a key going down or up, by its name or its number."""
    key = (
        {"type": "number", "data": int(name, 16)}
        if name.startswith("0x")
        else {"type": "qcode", "data": name}
    )
    return {"type": "key", "data": {"down": down, "key": key}}


class InTheDrawer:
    """The launcher holding a long notice's drawer, and the keys that go into it and move there.

    Its notice is a deploy's log of forty lines, so the drawer holds more than fits.
    """

    TITLE = "Deploy log"
    LAST = "line 40 of the deploy went by"

    def __init__(self, desktop: Desktop) -> None:
        """Send the notice, and open the launcher on its row."""
        self.desktop = desktop
        machine = desktop.machine
        # A subtest that failed with the launcher open, or at another scale, leaves it closed
        # and at 1 for this one.
        if machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
            desktop.close_launcher()
        desktop.as_account("niri msg output winit scale 1")
        body = "\n".join(f"line {at} of the deploy went by" for at in range(1, 41))
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
        self.rows = desktop.showing()
        assert self.rows["drawer"]["cursor"] is None, self.rows["drawer"]

    def press(self, keys: str) -> None:
        """Press each key named, a letter by its character and any other by QEMU's name.

        Every key goes down and up in one batch of QEMU's input events, so the guest reads them
        all at once. Sent one sendkey at a time, a loaded builder let niri draw the first key's
        frame before it read the second, and a waiting key's 300 ms ran out between them.
        """
        events: list[Json] = []
        for key in keys.split():
            *held, pressed = CHAR_TO_KEY.get(key, key).split("-")
            chord = [*held, pressed]
            events += [keyed(name, down=True) for name in chord]
            events += [keyed(name, down=False) for name in reversed(chord)]
        client = self.desktop.machine.qmp_client
        assert client is not None
        # The driver says a command's arguments are strings, so the events go through JSON to
        # reach it as they are.
        client.send("input-send-event", json.loads(json.dumps({"events": events})))

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

    def into(self) -> Json:
        """Go into the drawer with Space and the right arrow, and give its cursor.

        The arrow alone steps into the notice's row, so the launcher opens on it again before
        another try.
        """

        def again() -> None:
            self.desktop.close_launcher()
            self.open()

        self.pressed_until(
            "spc right",
            lambda shown: shown["mode"] == "page" and shown["drawer"]["cursor"] is not None,
            again,
        )
        return cast("Json", self.drawer()["cursor"])

    def back(self, keys: str) -> None:
        """Press the keys, and wait for the keys back on the rows, the selection unmoved."""
        self.pressed_until(
            keys, lambda shown: shown["mode"] == "normal" and not shown["drawer"]["keys"]
        )
        shown = self.desktop.showing()
        assert shown["open"], shown
        assert shown["selected"]["name"] == self.rows["selected"]["name"], shown
        assert shown["drawer"]["cursor"] is None, shown["drawer"]


def held(drawer: Json) -> bool:
    """Say whether the drawer's cursor is wholly inside its view."""
    top, height = int(drawer["cursor"]["top"]), int(drawer["cursor"]["height"])
    return top >= 0 and top + height <= int(drawer["height"])


def test_a_notices_drawer_says_who_sent_it_and_its_keys_scroll_it(desktop: Desktop) -> None:
    """A notice's drawer says who sent it and when, and gone into, its keys scroll it.

    The foot names no Shift with the arrows, which scroll nothing now: Space and the right arrow
    go in, three Ctrl+E scroll it three of its lines, and Space and the left arrow come back,
    the selection where it was.
    """
    page = InTheDrawer(desktop)
    drawer = page.drawer()
    # Signed as a card signs it: a program known by no other name by its own, in title case.
    assert re.fullmatch(r"Desktop \u00b7 \d\d:\d\d", drawer["eyebrow"]), drawer
    assert drawer["more"], f"forty lines fit the drawer: {drawer}"
    assert "\u21e7" not in page.rows["selected"]["told"], page.rows["selected"]
    desktop.photograph("launcher-plate")
    title = drawer["lines"]["title"]
    page.into()
    scrolled = page.until("ctrl-e ctrl-e ctrl-e", lambda drawer: drawer["scrolled"] == SCROLLED)
    assert round(title - scrolled["lines"]["title"]) == SCROLLED, scrolled
    page.back("spc left")
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
    """Space and the right arrow take the keys into the drawer, and four keys give them back.

    Its frame takes the accent the card's gives up, and the cursor stands on the title's first
    letter with nothing scrolled, its line marked by a rule in the margin left of the letters.
    Escape only lets a count go, and q does nothing; ZZ, :q and Space with the left arrow each
    give the keys back to the rows, the selection where it was. Its pictures at 1 and both of
    Joel's scales, on the title and down in the words, are for a person to judge the rule and
    the frames by: this screen is small enough that at 2 the card leaves the drawer no room.
    There it waits until the keys go in and then takes the card's place, its title on one line,
    rather than squeezed beside it until a word breaks (VKCF3Z, 2026-11-05: "Depl/oy/log").
    """
    page = InTheDrawer(desktop)
    came = page.into()
    assert came["says"] == page.TITLE, came
    assert came["letter"] == 0, came
    # The foot names the way back, not the row's Return, which does nothing in the drawer.
    told = desktop.showing()["selected"]["told"]
    assert told == ["Space \u2190 back"], f"in the drawer the foot says {told}"
    assert came["on"] == page.TITLE[0], came
    rule = came["rule"]
    assert rule["top"] == came["top"], came
    assert rule["height"] == came["height"], came
    assert rule["x"] > 0, came
    assert rule["width"] > 0, came
    drawer = page.drawer()
    assert drawer["scrolled"] == 0, drawer
    assert drawer["frame"] != page.rows["drawer"]["frame"], drawer
    # Each scale is set with the launcher closed, so it opens laid out for it, as
    # desktop-looks sets its own.
    for scale in (*JOELS_SCALES, "1"):
        desktop.close_launcher()
        desktop.as_account(f"niri msg output winit scale {scale}")
        page.open()
        at = into_in_sight(desktop, page, scale)
        named = scale.replace(".", "-")
        desktop.photograph(f"launcher-drawer-keys-at-{named}")

        def five_down(drawer: Json, below: int = int(at["line"]) + 5) -> bool:
            return bool(drawer["cursor"]["line"] == below)

        page.until("5 j", five_down)
        # Four along, or the line's last letter where a narrow drawer wraps it shorter.
        page.until(
            "l l l l",
            lambda drawer: (
                drawer["cursor"]["letter"] == min(len("line"), len(drawer["cursor"]["says"]) - 1)
            ),
        )
        desktop.photograph(f"launcher-drawer-cursor-at-{named}")
    page.press("3")
    desktop.launcher_until(lambda shown: shown["counted"] == "3")
    page.press("esc")
    desktop.launcher_until(lambda shown: shown["counted"] == "" and shown["mode"] == "page")
    # Escape at rest and q alone leave the keys where they are: j still moves the cursor.
    line = page.drawer()["cursor"]["line"]
    page.until("esc q j", lambda drawer: drawer["cursor"]["line"] == line + 1)
    page.back("Z Z")
    for keys in (": q ret", "spc left"):
        page.into()
        page.back(keys)
    desktop.close_launcher()


def test_in_a_drawer_the_keys_move_its_cursor_as_vims_do(desktop: Desktop) -> None:
    """In the drawer, j k h l and a count move its cursor, and the view follows it.

    j and k move it a line, after a count as many; h and l along the line; gg and G to its
    first and last line; Ctrl+E and Ctrl+Y scroll a line; Ctrl+D and Ctrl+U half the view;
    Ctrl+F, Ctrl+B, Page Down and Page Up nearly a whole one; and H, L and M to the view's top,
    bottom and middle lines, the view staying. Shift and an arrow do nothing.
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
    assert held(last), last
    assert last["cursor"]["says"] == page.LAST, last
    page.until("g g", lambda drawer: drawer["cursor"]["line"] == 0 and drawer["scrolled"] == 0)
    page.until("ctrl-e", lambda drawer: drawer["scrolled"] == PITCH and held(drawer))
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
    page.back("spc left")
    desktop.close_launcher()


def test_in_a_drawer_slash_and_question_put_its_cursor_where_they_land(desktop: Desktop) -> None:
    """In the drawer, / and ? type what to seek, and Return puts the cursor where it's found.

    What's typed is washed as it's typed; Return puts the cursor on its first letter and the
    view follows; n goes on to the next, N back to the one before, and a count as many; ? with
    nothing typed seeks the last back; a capital asks for one, and the search goes round past
    the last line to the title. Escape lets one being typed go, the keys staying in the drawer,
    as Backspace does with nothing left. Its pictures at both of Joel's scales show the matches
    washed, for a person to judge the wash by.
    """
    page = InTheDrawer(desktop)
    page.into()
    page.press("/ l i n e spc 3 0")
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["washing"] == "line 30")
    found = page.until("ret", lambda drawer: drawer["cursor"]["says"].startswith("line 30 "))
    assert found["cursor"]["letter"] == 0, found
    assert found["scrolled"] > 0, found
    assert held(found), found
    assert found["search"] == {
        "typing": "",
        "last": "line 30",
        "washing": "line 30",
        "landing": "",
    }, found

    def on(line: int) -> Callable[[Json], bool]:
        said = f"line {line} of the "
        return lambda drawer: (
            drawer["cursor"]["says"].startswith(said) and drawer["cursor"]["letter"] == len(said)
        )

    # Its own line's deploy is after the cursor, on its first letter.
    page.until("/ d e p l o y ret", on(30))
    page.until("n", on(31))
    page.until("shift-n", on(30))
    page.until("3 n", on(33))
    page.until("? ret", on(32))
    # Smartcase: only the title holds a capital, so the search goes round to it.
    came = page.until("/ shift-d e p l o y ret", lambda drawer: drawer["cursor"]["line"] == 1)
    assert came["cursor"]["says"] == page.TITLE, came
    assert came["search"]["landing"] == "wrapped", came
    # A search let go moves nothing, and the keys stay: j moves from where it stood.
    page.press("/ z z")
    desktop.launcher_until(lambda shown: shown["drawer"]["search"]["typing"] == "zz")
    line = page.drawer()["cursor"]["line"]
    page.until("esc j", lambda drawer: drawer["cursor"]["line"] == line + 1)
    page.until("? backspace j", lambda drawer: drawer["cursor"]["line"] == line + 2)

    # The cursor stands on the title's own Deploy, so the first line's deploy comes next,
    # without going round: where a narrow drawer breaks that line, it's on a line of its own.
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
        desktop.photograph(f"launcher-drawer-search-at-{scale.replace('.', '-')}")
    page.back("spc left")
    desktop.close_launcher()


def test_in_a_drawer_v_and_shift_v_choose_and_y_copies_what_they_chose(desktop: Desktop) -> None:
    """In the drawer, v chooses letters from the cursor's and V lines, and y copies them.

    The moves extend what's chosen, and it's counted as the mode word counts it; y puts it on
    the clipboard as markdown, the title after its #, and goes back to the page with the
    cursor at its start, the flash it gives what it took ending; o goes to its other end,
    Escape lets it go, and gv brings it back. Its pictures at both of Joel's scales show
    letters chosen over two lines, for a person to judge the selection's ground and the block.
    """
    page = InTheDrawer(desktop)
    title = page.into()["line"]

    def chose(by: str, count: int) -> Callable[[Json], bool]:
        return lambda drawer: drawer["chosen"]["by"] == by and drawer["chosen"]["count"] == count

    came = page.until("v", chose("letters", 1))
    assert came["chosen"]["taken"] == [[title, 0, 1]], came
    # Choosing, the foot names Escape, which lets the choice go and leaves the keys in.
    told = desktop.showing()["selected"]["told"]
    assert told == ["Esc back"], f"in visual the foot says {told}"
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
    # Lines below the title, so what's copied holds no "log" to be found ahead of the notice
    # when the launcher next looks for its title.
    page.until("j shift-v j", chose("lines", 2))
    page.until("o", lambda drawer: drawer["cursor"]["line"] == title + 1)
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
    for scale in (*JOELS_SCALES, "1"):
        desktop.close_launcher()
        desktop.as_account(f"niri msg output winit scale {scale}")
        page.open()
        page.into()
        page.until("v l l l l l j", chose("letters", 2))
        desktop.photograph(f"launcher-drawer-chosen-at-{scale.replace('.', '-')}")
    page.pressed_until("esc", lambda shown: shown["mode"] == "page")
    page.back("spc left")
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
    # photograph waits for its card's frames to settle, which is when
    # Qt would have warned of a layout that never does.
    with desktop.told("shows card Recording saved"):
        desktop.as_account(
            "notify-send --action=play='Play it' 'Recording saved' 'x.mp4'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    desktop.photograph("notice-with-action")
    # Qt only warns when a layout never settles, which left the
    # card's words piled at its top.
    log = desktop.shell_said("polish() loop")
    assert "polish() loop" not in log, "a notice's layout never settles"
    # Super+N takes the stack as its lines, which wait while it holds the
    # keys, the cursor on the card in front, the older; the newer is line
    # 1. Tab is no key of normal's, and says so, and Backspace is h, out
    # of nothing. :forg lets the older go unread, the cursor going on to
    # the newer, and the right arrow, as l, steps into its acts,
    # forgetting it first, since it opens nothing, then
    # what it offers, as a launcher row's are (launcher/Notice.js); Down
    # chooses the next and Left steps back, and Return does the one
    # chosen, with a token to come forward, and gives the keys back.
    desktop.listen()
    with desktop.told("keys notices"), desktop.told("notice cursor Lunch"):
        machine.send_key("meta_l-n")
    desktop.photograph("notices-held")
    with desktop.told("notice says not here"):
        machine.send_key("tab")
    machine.send_key("backspace")
    with desktop.told("notice forgets 1"), desktop.told("notice cursor Recording saved"):
        machine.send_chars(":forg\n", delay=KEY_PACE)
    with desktop.told("notice verb Forget It"):
        machine.send_key("right")
    with desktop.told("notice verb Play it"):
        machine.send_key("down")
    desktop.photograph("notices-held-verbs")
    with desktop.told("notice back"):
        machine.send_key("left")
    with desktop.told("notice verb Forget It"):
        machine.send_key("right")
    with desktop.told("notice verb Play it"):
        machine.send_key("down")
    with desktop.told("lets notices go"):
        machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx play /tmp/answer", timeout=timedelta(seconds=15))
    desktop.tokened()
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=10)
    )
    # The history is kept on disk, the one acted on read and the one
    # only let go still unread.
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(
        desktop.in_session(kept + ' | grep -q \'"summary":"Recording saved"[^}]*"read":true\''),
        timeout=timedelta(seconds=10),
    )
    desktop.as_account(kept + ' | grep -q \'"summary":"Lunch"[^}]*"read":false\'')


def test_a_notice_copied_from_its_card_says_copied(desktop: Desktop) -> None:
    """Copying what a notice said from its acts or by yy says so, and the clipboard holds it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Build log"):
        desktop.as_account("notify-send --expire-time=60000 'Build log' 'all green'")
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    # It offers nothing and opens nothing, so forgetting it leads its
    # acts, then whether it's read, the stack's own, and copying what it
    # said last (launcher/Notice.js), which keeps the keys.
    with desktop.told("notice verb Forget It"):
        machine.send_key("l")
    for verb in ["Read", "Forget Them All", "All the Notices", "Copy What It Said"]:
        with desktop.told(f"notice verb {verb}"):
            machine.send_key("j")
    with desktop.told("copied card Build log"):
        machine.send_key("ret")
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
        machine.send_key("esc")
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
    desktop.photograph(f"notice-visual-at-{named}")
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
    desktop.photograph(f"notice-done-at-{named}")
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    for summary in ["Build log", "Test log", "Deploy log"]:
        machine.wait_until_succeeds(
            desktop.in_session(kept + f' | grep -q \'"summary":"{summary}"[^}}]*"read":true\''),
            timeout=timedelta(seconds=10),
        )
    with desktop.told("notice brought back 3"):
        machine.send_key("u")
    with desktop.told("notice pending d"):
        machine.send_chars("d")
    desktop.photograph(f"notice-pending-at-{named}")
    with desktop.told("notice mode command"):
        machine.send_key("esc")
        machine.send_chars(":")
    desktop.photograph(f"notice-menu-at-{named}")
    with desktop.told("notice menu chose delete"):
        machine.send_key("tab")
    with desktop.told("notice mode normal"):
        machine.send_key("esc")
    with desktop.told("notice lists keys"):
        machine.send_chars(" ?", delay=KEY_PACE)
    desktop.photograph(f"notice-keys-at-{named}")
    with desktop.told("notice hides keys"):
        machine.send_key("esc")


def found_among_them(desktop: Desktop, named: str) -> None:
    """Show the lines, then find Deploy among them, photographed at the scale named.

    / narrows the lines as it's typed, Escape keeps what was found, and a fresh / left empty
    lists them all again, the cursor staying on what was found, as Vim's / leaves it.
    """
    machine = desktop.machine
    # The cursor on the oldest, Build log, line 3, by way of the newest, so it moves.
    with desktop.told("notice cursor Build log"):
        machine.send_chars("ggG", delay=KEY_PACE)
    desktop.photograph(f"notice-lines-at-{named}")
    with desktop.told("notice finds Deploy in 1"), desktop.told("notice cursor Deploy log"):
        machine.send_chars("/Deploy", delay=KEY_PACE)
    desktop.photograph(f"notice-find-at-{named}")
    with desktop.told("notice mode normal"):
        machine.send_key("esc")
    desktop.photograph(f"notice-found-at-{named}")
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
    # It wasn't missed, so the margin never once said one waits: a notice
    # arriving or leaving mustn't flash it.
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert "holds margin waiting" not in said, "the margin flashed a waiting notice"


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

    Its pictures at both of Joel's scales, alone, held and stepped into, are look 3's notice
    (2026-11-03), for a person to measure its room by.
    """
    machine = desktop.machine
    for scale in JOELS_SCALES:
        desktop.as_account(f"niri msg output winit scale {scale}")
        machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
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
        # Named without the point, which the driver would take for where
        # the name ends and its kind begins.
        named = scale.replace(".", "-")
        desktop.photograph(f"notice-at-{named}")
        with desktop.told("keys notices"):
            machine.send_key("meta_l-n")
        desktop.photograph(f"notice-held-at-{named}")
        with desktop.told("notice verb Open"):
            machine.send_key("right")
        desktop.photograph(f"notice-stepped-at-{named}")
        with desktop.told("notice back"):
            machine.send_key("esc")
        with desktop.told("lets notices go"):
            machine.send_key("esc")
        desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account("qs ipc call notices seen")
    desktop.as_account("niri msg output winit scale 1")


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
    # Super+N takes the stack, and :e Notices hands on to the history.
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    desktop.notices_history()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Sam")
    )
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=timedelta(seconds=15))
    desktop.tokened()


def test_a_notices_right_arrow_holds_what_return_does_on_its_card_and_in_the_launcher(
    desktop: Desktop,
) -> None:
    """A notice's right arrow holds what Return does first, on its card and in the launcher."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.listen()
    with desktop.told("shows card Thread with Ana"):
        desktop.as_account(
            "notify-send --expire-time=60000 --action=default=Open --action=play='Play it'"
            " 'Thread with Ana' 'see it' > /tmp/answer 2>&1 < /dev/null &"
        )
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    with desktop.told("notice verb Open"):
        machine.send_key("right")
    with desktop.told("notice verb Play it"):
        machine.send_key("down")
    with desktop.told("notice back"):
        machine.send_key("left")
    # :e Notices hands on to the history, which opens on its first row,
    # the newest notice, as every place opens on its first; the right arrow
    # steps into its acts from normal mode, and they hold the same.
    desktop.notices_history()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Ana")
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Open", "Play it"])
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
        machine.send_key("meta_l-n")
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
    # With nothing to open or go to, what it offers leads; copying what it
    # said is last.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["Play it"] and shown["rows"][-1:] == ["Copy What It Said"]
        )
    )
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Play it")
    machine.send_key("ret")
    assert desktop.answered() == "play", "a notice whose time was up lost what it offered"
    desktop.tokened()


def test_the_notices_sit_a_step_inside_the_window_under_them(desktop: Desktop) -> None:
    """The notices sit a step inside the window under them."""
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
            "notify-send --expire-time=60000 'Inside the window' 'a step from its edges'"
        )
    desktop.photograph("notice-inside")
    # The nearest window wholly on screen, as the shell picks it.
    ws = desktop.window_titled("Under the notices")["workspace_id"]
    tiles = [
        (w["layout"]["tile_pos_in_workspace_view"], w["layout"]["tile_size"])
        for w in json.loads(desktop.as_account("niri msg --json windows"))
        if w["workspace_id"] == ws and w["layout"]["tile_pos_in_workspace_view"]
    ]
    whole = [(p, s) for p, s in tiles if p[0] >= 0 and p[0] + s[0] <= screen]
    (left, top), (width, _) = min(whole, key=lambda t: (-(t[0][0] + t[1][0]), t[0][1]))
    step = space["s"]
    wanted_at = [
        int(screen - (left + width) + step + 0.5),
        int(max(top, 0) + step + 0.5),
        min(440, int(width - 2 * step)),
    ]
    notices_at = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[places notices [0-9 ]*' | tail -n 1"
    ).split()[-3:]
    assert [int(n) for n in notices_at] == wanted_at, "the notices aren't a step inside the window"
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
        machine.send_key("meta_l-n")
    # Lines: Bravo two, Bravo one, Alpha two, Alpha one. Chat's run is the first two.
    pressed_and_told(desktop, EVERY_HELD_KEY)
    # Every way to let go: ZZ, Space w, Super+N, Escape at rest and :q.
    letting_go: list[str | tuple[str, ...]] = ["ZZ", " w", ("meta_l-n",), ("esc",), ":q\n"]
    for keys in letting_go:
        pressed_and_told(desktop, [(keys, "lets notices go"), (("meta_l-n",), "keys notices")])
    with desktop.told("lets notices go"):
        machine.send_key("esc")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


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
    (" ul", "notice numbers absolute"),
    (" ul", "notice numbers relative"),
    # Visual: o goes to its other end, Escape ends it, gv brings it back, x is done with it.
    ("Vj", "notice selects 2"),
    ("o", "notice cursor Bravo two"),
    (("esc",), "notice mode normal"),
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
    ("iAlpha", "notice finds Alpha in 2"),
    (("down",), "notice cursor Alpha one"),
    (("backspace",) * 6, "notice mode normal"),
    # The menu: Tab chooses its lines, Ctrl+E puts it away, Escape drops the line.
    (":", "notice mode command"),
    (("tab",), "notice menu chose delete"),
    (("ctrl-e", "esc"), "notice mode normal"),
    # F1 lists the keys, j scrolls them, and Escape puts them away.
    (("f1",), "notice lists keys"),
    ("j", "notice lists keys from 2"),
    (("esc",), "notice hides keys"),
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
        machine.send_key("meta_l-n")
    with desktop.told("notice mode insert"):
        machine.send_chars(":reply\n", delay=KEY_PACE)
    machine.send_chars("on my way", delay=KEY_PACE)
    for scale in JOELS_SCALES:
        desktop.as_account(f"niri msg output winit scale {scale}")
        desktop.photograph(f"notice-reply-at-{scale.replace('.', '-')}")
    desktop.as_account("niri msg output winit scale 1")
    with desktop.told("notice mode prompt"):
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
        machine.send_key("esc")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


def test_super_shift_n_puts_the_notices_away(desktop: Desktop) -> None:
    """Super+Shift+N puts the notices away."""
    machine = desktop.machine
    with desktop.told("shows card Standup"):
        desktop.as_account("notify-send --urgency=critical 'Standup' 'in five minutes'")
    with desktop.told("gone notifications"):
        machine.send_key("meta_l-shift-n")
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(
        desktop.in_session(kept + ' | grep -q \'"summary":"Standup"[^}]*"read":false\''),
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

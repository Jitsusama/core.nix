"""The desktop test's says what happens area.

Here notices say what happens: who sent each, its drawer and actions by the keys and the
pointer, what opening one brings forward, and where they sit.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import FROM_THE_CORNER, LAUNCHER_OPEN, NOTICES, retry

if TYPE_CHECKING:
    from desktop_session import Desktop
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
    # arrow does, where copying what it said comes last of its acts, never
    # first.
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
    machine.send_key("esc")


# How far Shift and an arrow scroll the launcher's drawer: three of its lines of 24, so what
# it shows stays on the card's lines (vm-desktop-looks measures them).
SCROLLED = 3 * 24


def test_a_notices_drawer_says_who_sent_it_and_shift_scrolls_it(desktop: Desktop) -> None:
    """A notice's drawer says who sent it and when, and Shift with the arrows scrolls it.

    Shift and an arrow scroll it three of its lines at a time, said in the foot while it holds
    more than fits, and leave the selection where it was.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    body = "\n".join(f"step {at} of the long build went by" for at in range(1, 41))
    desktop.as_account(f"notify-send 'Long build log' {shlex.quote(body)}")
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=timedelta(seconds=60),
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("long build log")
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] and shown["drawer"]["title"] == "Long build log"
    )
    drawer = desktop.showing()["drawer"]
    # Signed as a card signs it: a program known by no other name by its own, in title case.
    assert re.fullmatch(r"Desktop \u00b7 \d\d:\d\d", drawer["eyebrow"]), drawer
    assert drawer["more"], f"forty lines fit the drawer: {drawer}"
    assert "\u21e7\u2191\u2193 scroll" in desktop.showing()["selected"]["told"], desktop.showing()
    desktop.photograph("launcher-plate")
    # Shift and the arrows scroll it, the list where it was.
    selected = desktop.showing()["selected"]["name"]
    title = drawer["lines"]["title"]
    machine.send_key("shift-down")
    desktop.launcher_until(lambda shown: shown["drawer"]["scrolled"] == SCROLLED)
    scrolled = desktop.showing()
    assert scrolled["selected"]["name"] == selected, scrolled
    assert round(title - scrolled["drawer"]["lines"]["title"]) == SCROLLED, scrolled["drawer"]
    machine.send_key("shift-up")
    desktop.launcher_until(lambda shown: shown["drawer"]["scrolled"] == 0)
    machine.send_key("esc")


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
    # Super+N takes the stack, which waits while it holds the keys.
    # Tab does nothing, being only for completing. The newer notice
    # waits behind the older; letting the older go keeps the keys and
    # brings the newer to the front, the right arrow steps into its
    # verbs, forgetting it first, since Return forgets one with nothing to
    # open, then what it offers, as a launcher row's are
    # (launcher/Notice.js), Down chooses the next and Left steps
    # back, and Return does the one
    # chosen, with a token to come forward, and gives the keys back.
    desktop.listen()
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    desktop.photograph("notices-held")
    machine.send_key("tab")
    machine.send_key("backspace")
    # Held, the card is as tall as its verbs need from the start, so
    # stepping in, through and out of them moves nothing.
    desktop.held_still()
    since = desktop.mark()
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
    moved = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)} | grep -F 'notice stands'; true"
    )
    assert moved.strip() == "", f"the held card changed its height: {moved}"
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
    """Copying what a notice said from its card says so, and the clipboard holds it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Build log"):
        desktop.as_account("notify-send --expire-time=60000 'Build log' 'all green'")
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    # It offers nothing and opens nothing, so forgetting it leads its
    # verbs, as Return does, then whether it's read, the stack's own, and
    # copying what it said last (launcher/Notice.js).
    with desktop.told("notice verb Forget It"):
        machine.send_key("right")
    for verb in ["Read", "Forget Them All", "All the Notices", "Copy What It Said"]:
        with desktop.told(f"notice verb {verb}"):
            machine.send_key("down")
    with desktop.told("copied card Build log"):
        machine.send_key("ret")
    desktop.photograph("notice-copied")
    assert desktop.as_account("wl-paste --no-newline") == "Build log\nall green"
    # yy copies it too, as vi yanks a line, and the stack stays held.
    desktop.as_account("wl-copy nothing")
    with desktop.told("shows card Deploy log"):
        desktop.as_account("notify-send --expire-time=60000 'Deploy log' 'all out'")
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    with desktop.told("copied card Build log"):
        machine.send_key("y")
        machine.send_key("y")
    assert desktop.as_account("wl-paste --no-newline") == "Build log\nall green"
    for scale in JOELS_SCALES:
        searched_selected_and_undone(desktop, scale)
    desktop.as_account("niri msg output winit scale 1")
    with desktop.told("lets notices go"):
        machine.send_key("q")
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )


def searched_selected_and_undone(desktop: Desktop, scale: str) -> None:
    """Search the held Build log and Deploy log cards, select and copy both, forget and undo.

    At one of Joel's scales, for a person to look at the search wash and the selection by: /
    turns to the first card holding what's typed as it's typed, V selects the front card, j
    carries it on to the next, y copies both, dd forgets one, u brings it back, and Space ?
    lists every key in the acts' place until Escape puts them away.
    """
    machine = desktop.machine
    desktop.as_account(f"niri msg output winit scale {scale}")
    named = scale.replace(".", "-")
    with desktop.told("notice front Deploy log"), desktop.told("notice finds Deploy in 1"):
        machine.send_key("slash")
        machine.send_chars("Deploy")
    desktop.photograph(f"notice-search-at-{named}")
    machine.send_key("ret")
    with desktop.told("notice selects 1"):
        machine.send_key("shift-v")
    with desktop.told("notice selects 2"):
        machine.send_key("j")
    desktop.photograph(f"notice-visual-at-{named}")
    with desktop.told("copied card Build log"):
        machine.send_key("y")
    copied = desktop.as_account("wl-paste --no-newline")
    assert copied == "Deploy log\nall out\n\nBuild log\nall green", copied
    with desktop.told("notice front Deploy log"):
        machine.send_key("d")
        machine.send_key("d")
    with desktop.told("notice brought back Build log"):
        machine.send_key("u")
    with desktop.told("notice lists keys"):
        machine.send_key("spc")
        machine.send_key("shift-slash")
    desktop.photograph(f"notice-keys-at-{named}")
    # Thirteen lines scroll in the room of the card's five acts, so G shows the last five.
    with desktop.told("notice lists keys from 9"):
        machine.send_key("shift-g")
    with desktop.told("notice hides keys"):
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
    # Super+N takes the stack, and again hands on to the history.
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
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
    # Super+N again hands on to the history, which opens on its first row,
    # the newest notice, as every place opens on its first; the right arrow
    # steps into its acts, which hold the same.
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Ana")
    )
    machine.send_key("right")
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

"""The desktop test's says what happens area.

Here the desktop says what happens: notices, the margin and what it holds, Tuple's calls and
caps lock. tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Every test asks for the desktop, started once
a run (desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    CAPITALS,
    FROM_THE_CORNER,
    FROM_THE_EDGE,
    LAUNCHER_OPEN,
    MARGIN,
    NEARLY_ALL,
    NOTICES,
    STARTING,
    changed_at_the_top,
    drawn_at,
    margin_points,
    margin_sets,
    retry,
)

if TYPE_CHECKING:
    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


# How far right of a glyph's middle a click lands, in pixels: inside its
# cell, and past where what takes the pointer would end had it stayed half
# a cell to the left, where the row was before it slid.
BESIDE_THE_MIDDLE = 2

# How long the same wait lasts before the VM's margin marks it, in seconds,
# as tests/desktop.nix sets the Lag piece's `lasts`.
LAG_LASTS = 3600


def now_down_to(desktop: Desktop, name: str) -> None:
    """Open Now with Super+O, and move down to this row from Waiting on You.

    Now opens on what wants Joel or is live, else on Waiting on You, never
    on a mode left on, since Return turns one off and a habit mustn't.
    """
    desktop.machine.send_key("meta_l-o")
    desktop.whats_on("Waiting on You")
    desktop.down_to(name)


def stays_showing_it_gone(desktop: Desktop, name: str) -> None:
    """Wait for the launcher to stay open showing a row gone, then close it.

    An act that changes how things stand, rather than taking Joel
    anywhere, keeps the launcher, and the row it ended stays in its place
    marked gone, so nothing under it moves.
    """
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and any(
                thing["gone"] and thing["reads"].split(" \u00b7 ")[0] == name
                for thing in shown["things"]
            )
        )
    )
    desktop.machine.send_key("esc")
    desktop.machine.wait_until_fails(
        desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10)
    )


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
    desktop.as_account("qs ipc call notices putAway")
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )


def test_a_click_on_a_notices_action_does_it_and_anywhere_else_puts_it_away(
    desktop: Desktop,
) -> None:
    """A click on a notice's action does it, and anywhere else puts it away."""
    since = desktop.mark()
    x, y = desktop.offer_to_play("notice-to-click")
    desktop.click(x, y)
    assert desktop.answered() == "play", "a click on the action put the notice away instead"
    # A little above the action is the notice's message.
    x, y = desktop.offer_to_play("notice-to-put-away")
    desktop.click(x, y - 0.04)
    assert desktop.answered() == "", "a click on the message did the action"
    # Neither was missed, so the margin never once said one waits: a
    # notice arriving or leaving mustn't flash it.
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
    # Its default is the card's click, so the one glyph is Play it's.
    x, y = desktop.offer_to_play("notice-to-open", offers="--action=default=Open")
    desktop.click(x, y - 0.04)
    assert desktop.answered() == "default", "a click on the message didn't open what it's about"
    desktop.tokened()


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
    # Super+N again hands on to the history, opened on the newest notice's
    # acts, which hold the same.
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
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
    # Its row is one line; below the rows it reads whole, summary over
    # body.
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("reads") == "Quarry take\nz.mp4"
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
        min(380, int(width - 2 * step)),
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


def test_the_launcher_keeps_the_machine_awake_lid_shut_too_then_lets_it_sleep(
    desktop: Desktop,
) -> None:
    """The launcher keeps the machine awake, lid shut too, then lets it sleep."""
    machine = desktop.machine
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("keep awake")
    desktop.photograph("launcher-keep-awake")
    # Keeping it awake changes something, so Return would stay; Ctrl+Return
    # does it and closes.
    machine.send_key("ctrl-ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.wait_until_succeeds(f"{held} | grep -qw idle", timeout=timedelta(seconds=10))
    machine.wait_until_succeeds(
        f"{held} | grep -q handle-lid-switch", timeout=timedelta(seconds=10)
    )
    # Kept awake, a suspend asked for by the power key or the launcher
    # still goes through, and so does the lid's on battery.
    machine.fail(f"{held} | grep -q sleep")
    # Without power the lid is let go, and taken again when it's back.
    desktop.power("off")
    machine.wait_until_fails(f"{held} | grep -q handle-lid-switch", timeout=timedelta(seconds=10))
    machine.succeed(f"{held} | grep -qw idle")
    desktop.power("on")
    machine.wait_until_succeeds(
        f"{held} | grep -q handle-lid-switch", timeout=timedelta(seconds=10)
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.photograph("launcher-home-awake")
    desktop.type_in("let it sleep")
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


def test_the_lid_is_let_go_when_the_power_goes_though_the_shell_never_hears(
    desktop: Desktop,
) -> None:
    """The lid is let go when the power goes, though the shell never hears it."""
    machine = desktop.machine
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    # The supplies were told of the rules once udev had settled, so the
    # mains is a device systemd tracks, as it is after a switch.
    machine.succeed("systemctl is-active power-supplies-seen.service")
    machine.succeed("systemctl is-active sys-subsystem-power_supply-mains.device")
    desktop.as_account("systemctl --user start caffeinate.service caffeinate-lid.service")
    machine.wait_until_succeeds(
        f"{held} | grep -q handle-lid-switch", timeout=timedelta(seconds=10)
    )
    # The shell frozen stands in for one that missed the power going while
    # the machine slept, as on 2026-10-28, when a laptop ran flat in a bag.
    # Frozen rather than stopped, it's the same shell once thawed: one
    # started again took the moments after it as its first, saying no
    # notice and showing the lag of its own start, which the subtests after
    # this one saw.
    desktop.as_account("systemctl --user freeze quickshell.service")
    try:
        desktop.power("off")
        machine.wait_until_fails(
            f"{held} | grep -q handle-lid-switch", timeout=timedelta(seconds=10)
        )
        machine.succeed(f"{held} | grep -qw idle")
    finally:
        desktop.power("on")
        desktop.as_account("systemctl --user stop caffeinate.service")
        desktop.as_account("systemctl --user thaw quickshell.service")
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc call launcher list > /dev/null"),
            timeout=timedelta(seconds=30),
        )


def test_a_camera_opened_is_said_then_shows_in_the_margin_and_the_launcher_says_by_what(
    desktop: Desktop,
) -> None:
    """A camera opened is said, then shows in the margin, and the launcher says by what."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # What's true as the shell starts isn't news, so it's waited out.
    desktop.reported("settles arrivals", lambda rest: rest == "", "")
    # Written whole, as the camera's module writes it. A program opened
    # it by itself, so it's said as a notice of the camera's.
    since = desktop.mark()
    with desktop.told("shows card chrome has the camera open"):
        machine.succeed(
            "echo chrome > /run/camera-holders/.camera"
            " && mv /run/camera-holders/.camera /run/camera-holders/camera"
        )
    desktop.photograph("notice-camera")
    # A card or a mark: once the card goes, the glyph shows, on air, so
    # strong, and the card wasn't missed, since the glyph says it.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.reported("sets margin ", margin_sets(("camera", "onAir", "strong")), since)
    desktop.photograph("margin-camera")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("camera")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "in use by chrome"
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # Let go, it lingers a moment, muted, then goes.
    since = desktop.mark()
    machine.succeed(
        ": > /run/camera-holders/.camera"
        " && mv /run/camera-holders/.camera /run/camera-holders/camera"
    )
    desktop.reported("sets margin ", margin_sets(("camera", "onAir", "muted")), since)
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_a_usb_cameras_stream_shows_in_the_margin_too_and_leaves_with_the_camera(
    desktop: Desktop,
) -> None:
    """A USB camera's stream shows in the margin too, and leaves with the camera."""
    machine = desktop.machine
    # A camera the machine doesn't name, written as usb-camera-holders
    # writes one: a file named for the camera, there only while it is.
    usb = "/run/camera-holders/MX Brio 705 for Business"
    with desktop.told("drawn margin"):
        machine.succeed(f"echo zoom > '{usb}.1' && mv '{usb}.1' '{usb}'")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("camera")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "in use by zoom")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.succeed(f"rm '{usb}'")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_keep_awake_shows_in_the_margin_and_whats_on_says_it_and_lets_it_sleep(
    desktop: Desktop,
) -> None:
    """Keep Awake shows in the margin, and What's on says it and lets it sleep."""
    machine = desktop.machine
    # Keep Awake was just let go, and the margin leaves as it fades.
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # Its own, so it runs alone: whether something holds the machine awake.
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    # A camera's card from before is gone first, so what changes along the
    # top is the glyph alone.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # Left on, so muted.
    desktop.held_still()
    empty = desktop.screen_now()
    since = desktop.mark()
    with desktop.told("drawn margin"):
        desktop.as_account("systemctl --user start caffeinate.service")
    desktop.reported("sets margin ", margin_sets(("awake", "leftOn", "muted")), since)
    # Super+O opens What's on, on Waiting on You, since a mode left on is
    # never what it opens on; on its row, the mark its keys are on wears
    # the accent, nothing grown; closed, it's as it was.
    glyph = changed_at_the_top(empty, desktop.settled_screen("margin-awake"))
    since = desktop.mark()
    now_down_to(desktop, "Keep Awake")
    desktop.reported("sets margin ", margin_points("awake"), since)
    # Settled, as Joel sees it.
    desktop.held_still()
    desktop.photograph("launcher-whats-on")
    since = desktop.mark()
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.reported("sets margin ", margin_points(""), since)
    # Another mark comes and goes, so the row slides aside and back, and
    # what takes the pointer has to follow it there.
    since = desktop.mark()
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    desktop.reported(
        "holds margin ", lambda rest: sorted(rest.split()) == ["awake", "muted"], since
    )
    desktop.held_still()
    since = desktop.mark()
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
    desktop.reported("holds margin ", lambda rest: rest.split() == ["awake"], since)
    desktop.held_still()
    glyph = (glyph[0] + BESIDE_THE_MIDDLE / desktop.screen_now().width, glyph[1])
    # A click there opens it the same, and Return lets the machine sleep.
    desktop.click(*glyph)
    try:
        desktop.whats_on("Keep Awake")
    except AssertionError:
        # Where the click went, beside where the shell's surfaces are.
        print(f"clicked at {glyph} of the screen")
        print(desktop.as_account("niri msg layers; niri msg --json focused-output"))
        raise
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # Letting it sleep stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Keep Awake")


def test_do_not_disturb_holds_a_notice_shows_it_held_and_ends_from_the_margin(
    desktop: Desktop,
) -> None:
    """Do not disturb holds a notice, shows it held, and ends from the margin."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("do not disturb")
    machine.send_key("ctrl-ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    # The shell decides as the notice arrives, so once it says it held
    # one, there's no card to come.
    with desktop.told("held Held back"):
        desktop.as_account("notify-send 'Held back' 'until it ends'")
    machine.fail(desktop.in_session(NOTICES))
    desktop.photograph("margin-quiet")
    # Its row ends it, and a step in offers the rest, what's held last:
    # An Hour More comes first only when it ends by itself.
    now_down_to(desktop, "Do Not Disturb")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Show What's Held"])
    desktop.type_in("show", selecting="Show What's Held")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["keys"] and shown["held"] == ["notices"])
    desktop.photograph("launcher-held")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    now_down_to(desktop, "Do Not Disturb")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    desktop.as_account("test ! -s $XDG_RUNTIME_DIR/quiet")
    # Ending it stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Do Not Disturb")


def test_a_missed_notice_shows_its_glyph_and_whats_on_shows_the_notices(desktop: Desktop) -> None:
    """A missed notice shows its glyph, and What's on shows the notices."""
    machine = desktop.machine
    # On screen it isn't waiting yet; once it goes by itself, it is, and
    # it wants Joel, so it's strong.
    since = desktop.mark()
    with desktop.told("drawn margin"), desktop.told("shows card Peter"):
        desktop.as_account("notify-send --expire-time=1000 'Peter' 'are you there?'")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.reported("sets margin ", margin_sets(("waiting", "wants", "strong")), since)
    desktop.photograph("margin-waiting")
    machine.send_key("meta_l-o")
    desktop.whats_on("Notices")
    # The time and the date are among Now's own rows, at its top before
    # what comes and goes, the notices among them, each its own answer as
    # it's placed anywhere; the VM has no forecast and no calendar, so the
    # location follows them.
    things = desktop.showing()["things"]
    kinds = [thing["is"] for thing in things]
    at = kinds.index("answer:time")
    time, today = things[at : at + 2]
    assert kinds[at + 2] == "on:location", things
    notices = next(n for n, thing in enumerate(things) if thing["reads"].startswith("Notices"))
    assert at < notices, things
    date = machine.succeed("date '+%A %-d %B %Y'").strip()
    assert time["is"] == "answer:time", time
    assert re.match(r"Time \u00b7 \d\d:\d\d", time["reads"]), time
    assert today["is"] == "answer:today", today
    assert today["reads"].startswith(f"Date \u00b7 {date}"), today
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["keys"] and shown["held"] == ["notices"])
    desktop.photograph("launcher-from-margin")
    machine.send_key("esc")
    # Opening them counted them seen, so nothing's waiting.
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_waiting_that_lasts_a_minute_shows_in_the_margin_quietly(desktop: Desktop) -> None:
    """Waiting that lasts a minute shows in the margin, quietly."""
    machine = desktop.machine
    # lag-watch would write its own window over a stand-in's, so it rests.
    desktop.as_account("systemctl --user stop lag-watch.service")
    lagged = "$XDG_RUNTIME_DIR/lag-watch/status.json"

    def lag_says(worst: str, who: str, ago: int) -> None:
        """Write a window as lag-watch does, renamed in, its run begun so long ago."""
        now = int(machine.succeed("date +%s").strip())
        window = {
            "ended": now,
            "trouble": [f"{who} waited on {worst}"] if worst else [],
            "worst": worst,
            "who": who,
            "since": now - ago if worst else None,
        }
        desktop.as_account(
            f"mkdir -p $(dirname {lagged}) && echo '{json.dumps(window)}' > {lagged}.new"
            f" && mv {lagged}.new {lagged}"
        )

    # A moment's wait says nothing.
    since = desktop.mark()
    with desktop.told("heard lag memory"):
        lag_says("memory", "apps", 10)
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert not re.search(r"holds margin [^\n]*\blag\b", said), "a moment's wait showed"
    # As long as the VM's Lag waits for (tests/desktop.nix's hour) it
    # does, left on, so muted. A run that began that long ago is another
    # run, so the last one ends first.
    with desktop.told("heard lag nothing"):
        lag_says("", "", 0)
    lag_says("memory", "apps", LAG_LASTS + 10)
    desktop.reported("sets margin ", margin_sets(("lag", "leftOn", "muted")), since)
    desktop.photograph("margin-lag")
    # Over, it goes.
    lag_says("", "", 0)
    desktop.reported("holds margin", lambda rest: "lag" not in rest.split(), since)
    desktop.as_account("systemctl --user start lag-watch.service")


def test_the_microphone_muted_shows_in_the_margin_and_return_unmutes_it(desktop: Desktop) -> None:
    """The microphone muted shows in the margin, and Return unmutes it."""
    machine = desktop.machine
    try:
        with desktop.told("drawn margin"):
            desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    except AssertionError:
        # Which source Pipewire calls the default, whether it's muted,
        # and what Pipewire holds for it, beside what Quickshell heard.
        print(desktop.as_account("wpctl status; wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
        print(desktop.as_account("pw-dump stand-in-microphone"))
        raise
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=10)
    )
    desktop.photograph("margin-muted")
    now_down_to(desktop, "Microphone")
    machine.send_key("ret")
    muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
    machine.wait_until_fails(desktop.in_session(muted), timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # Unmuting stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Microphone")
    # Unmuting shows the meter a moment, which the next pictures
    # would hold for as long as the host takes to reach them.
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=10)
    )


# Dictation shows at the bottom centre, never in the margin, and says what
# it heard, from a stand-in for its service (desktop.nix).


def test_dictation_says_what_its_doing_and_when_it_heard_nothing(desktop: Desktop) -> None:
    """Dictation says what it's doing, and when it heard nothing."""
    machine = desktop.machine
    dictation = "niri msg layers | grep -q dictation"
    # The volume's and the microphone's meters share the bottom centre, and
    # show for a moment as the shell first hears them, so they're let go.
    meters = "niri msg layers | grep -Eq 'Namespace: \"(volume|microphone)\"'"
    machine.wait_until_fails(desktop.in_session(meters), timeout=timedelta(seconds=15))
    since = desktop.mark()
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation listening", lambda rest: rest == "", since)
    desktop.wait_for_surface("dictation")
    desktop.photograph("dictation-listening")
    # Typing shows once it lasts, and a press meanwhile says it's still
    # writing; the stand-in types for five seconds.
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation typing", lambda rest: rest == "", since)
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation still writing", lambda rest: rest == "", since)
    desktop.photograph("dictation-still-writing")
    desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
    desktop.photograph("dictation-heard-nothing")
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=10))


# The laptop's Copilot key sends Super, Shift and F23 together, which the
# keyboard's layout reads as F23, and niri's bind for it starts
# dictation. A stand-in key sends the same, through the kernel as the
# laptop's does, since QEMU's keyboard has no F23.


def test_the_copilot_key_starts_dictation(desktop: Desktop) -> None:
    """The Copilot key starts dictation."""
    machine = desktop.machine
    dictation = "niri msg layers | grep -q dictation"
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=15))
    machine.succeed(f"systemd-run --unit=copilot-key {desktop.values['copilot_key']}")
    machine.wait_for_file("/run/copilot-key", timeout=timedelta(seconds=30))
    # A press before the key is open would be lost, so the test waits until
    # it is. Here niri runs nested in Cage, which reads the machine's input
    # devices and hands their keys on to niri, so it's Cage that opens it.
    node = machine.succeed("cat /run/copilot-key.device").strip()
    # Its process isn't named cage exactly, so it's found by its command.
    held = f"find /proc/$(pgrep -o -f bin/cage)/fd -lname {node} | grep -q ."
    try:
        machine.wait_until_succeeds(held, timeout=30)
    except AssertionError:
        # What udev made of the key says why Cage passed it by.
        seen = machine.execute(f"udevadm info {node}", check_return=False)[1]
        pytest.fail(f"Cage never opened the Copilot key; udev says:\n{seen}")
    since = desktop.mark()
    machine.succeed("echo press > /run/copilot-key")
    try:
        desktop.reported("says dictation listening", lambda rest: rest == "", since)
    finally:
        machine.succeed("systemctl stop copilot-key")
    # It's put back as it was: stopped, it types what it heard, nothing here.
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=10))


# Any player that tells the desktop what it plays shows in the margin
# while it plays, and the media keys' command shows what it is.


def test_whats_playing_shows_in_the_margin_and_a_media_key_shows_what_it_is(
    desktop: Desktop,
) -> None:
    """What's playing shows in the margin, and a media key shows what it is."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # A track that starts by itself shows its card a moment, which
    # goes before the margin is photographed.
    with desktop.told("drawn notifications"), desktop.told("drawn margin"):
        desktop.as_account(
            "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
            f" {desktop.values['track']}"
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    desktop.photograph("margin-playing")
    # The play key pauses it, and the card says what was paused.
    with desktop.told("drawn notifications"):
        desktop.as_account("qs ipc call music toggle")
    desktop.photograph("music-card")
    # Paused, nothing's playing, so the margin goes; the card goes by itself.
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account("systemctl --user stop player")


# A call comes before the other apps when the machine is busy.


def test_tuples_daemon_comes_before_the_other_apps(desktop: Desktop) -> None:
    """Tuple's daemon comes before the other apps."""
    comes = desktop.as_account("systemctl --user show tuple --property=CPUWeight --property=Nice")
    assert sorted(comes.split()) == ["CPUWeight=1000", "Nice=-10"]


def test_a_tuple_call_shows_in_the_margin_while_it_lasts(desktop: Desktop) -> None:
    """A Tuple call shows in the margin while it lasts."""
    machine = desktop.machine
    # Only a call writes these lines, so they're written here, with
    # the daemon stopped, since it rewrites its log as it starts.
    desktop.as_account("systemctl --user stop tuple")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    with desktop.told("drawn margin"):
        desktop.tuple_said("call connected")
    desktop.photograph("margin-call")
    desktop.tuple_said("Caller.cpp:985|invalidating call")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_people_coming_and_going_in_a_tuple_call_are_said_and_joel_can_leave(
    desktop: Desktop,
) -> None:
    """People coming and going in a Tuple call are said, and Joel can leave."""
    machine = desktop.machine
    # Whoever comes is "a guest", not a number, which Tuple's contacts
    # would be asked about, and with the daemon stopped it starts one.
    # The call before stops it too, so this one stops it itself and runs
    # alone the same.
    desktop.as_account("systemctl --user stop tuple")
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action={0}'"
    # Those there as Joel joins aren't news, so the call connected a
    # while before the guest comes.
    with desktop.told("drawn margin"):
        desktop.tuple_said("call connected", ago=10)
    # The call is one place, Calls, leading with what can be done in it;
    # Desktop lists no second row for it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("desktop ")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["desktop"] and "Calls" in shown["rows"],
    )
    assert "The Call" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.type_in("calls", selecting="Calls")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Leave the Call" in shown["rows"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Someone joined the call"):
        desktop.tuple_said("peer joined call: a guest")
    desktop.photograph("notice-joined")
    with desktop.told("shows card Someone left the call"):
        desktop.tuple_said("peer left call: a guest")
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so Super+N takes the stack at once: held, it waits.
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    machine.succeed(offered.format("leave"))
    desktop.photograph("notice-alone")
    # Leaving runs tuple call leave, which starts the stopped daemon,
    # whose new log ends the call. Super+N again opens the history on the
    # newest notice's offers, Return's own Open first.
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Open", "Leave the Call"])
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Leave the Call")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=15))
    machine.wait_until_fails(offered.format("leave"), timeout=timedelta(seconds=10))
    desktop.as_account("tuple stop")


def test_a_tuple_call_ringing_wants_joel_and_one_not_answered_is_kept_as_missed(
    desktop: Desktop,
) -> None:
    """A Tuple call ringing wants Joel, and one not answered is kept as missed."""
    machine = desktop.machine
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action=show'"
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Someone is calling"):
        desktop.tuple_said("received incoming call: a caller")
    machine.succeed(offered)
    # The ring's card says it, so the margin's phone waits for it to go.
    desktop.photograph("notice-ringing")
    # The caller gives up: the ring's notice goes, and a missed call stays.
    desktop.tuple_said("Caller.cpp:985|invalidating call")
    machine.wait_until_fails(offered, timeout=timedelta(seconds=10))
    kept = "grep -q 'Missed a call from someone' ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(desktop.in_session(kept), timeout=timedelta(seconds=10))
    # Once it goes by itself it waits, and the margin shows it.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.wait_for_surface("margin")
    machine.send_key("meta_l-n")
    desktop.launcher_ready()
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_a_copied_calls_link_can_be_joined_until_something_else_is_copied(desktop: Desktop) -> None:
    """A copied call's link can be joined until something else is copied."""
    machine = desktop.machine
    link = "https://tuple.app/c/from-a-chat"
    desktop.as_account(f"echo -n 'pair with me? {link}' | wl-copy > /dev/null 2>&1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("join", selecting="Join the call")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

    # A screenshot is copied as an image alone, with no text to read.
    desktop.as_account("printf '\\211PNG\\r\\n' | wl-copy --type image/png > /dev/null 2>&1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("join")

    def joins_nothing(shown: Json) -> bool:
        """Say whether the launcher's selected row joins no Tuple call."""
        row = shown["selected"]
        return row is None or row["is"] != "tuple:join"

    desktop.launcher_until(joins_nothing)
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


def test_tuples_windows_find_their_places_the_face_in_a_corner_a_shared_screen_wide(
    desktop: Desktop,
) -> None:
    """Tuple's windows find their places: the face in a corner, a shared screen wide."""
    machine = desktop.machine

    # Through X11, as Tuple draws them, so niri starts xwayland-satellite
    # for them, and they reach it titled by their part. Tuple's daemon
    # is a service, started with no token from niri to be let take the
    # keys, which xwayland-satellite would hand on, so neither has one.
    def tuple_opens(part: str) -> Json:
        """Open a window as Tuple's part of this name does, and give it as niri lists it."""
        desktop.as_account(
            "niri msg action spawn -- env -u XDG_ACTIVATION_TOKEN"
            f" {desktop.values['tuple_window']} {part}"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f'niri msg --json windows | grep -q \'"title":"{part}"\''),
            timeout=STARTING,
        )
        return desktop.window_titled(part)

    span = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]
    # Tuple's own panel floats.
    tuple_panel = tuple_opens("tuple-main")
    assert tuple_panel["is_floating"], tuple_panel
    # Joel is at work in a window of his own when the other's face
    # comes, and keeps the keys. (xwayland-satellite hands a new X11
    # window the keys while another of its windows has them, which no
    # rule of niri's overrides.)
    desktop.as_account("niri msg action spawn -- kitty --title 'at work'")
    machine.wait_until_succeeds(
        desktop.in_session('niri msg --json focused-window | grep -q \'"title":"at work"\''),
        timeout=STARTING,
    )
    tuple_face = tuple_opens("tuple-webcams")
    assert tuple_face["app_id"] == "Tuple"
    assert tuple_face["is_floating"], tuple_face
    assert not tuple_face["is_focused"], tuple_face
    assert desktop.window_titled("at work")["is_focused"], tuple_face
    left, top, width, height = drawn_at(tuple_face)
    assert (width, height) == (384, 216)
    # In the bottom-right corner, a little in from the screen's edges
    # and whatever niri keeps clear along them.
    assert 0 < span["width"] - (left + width) <= FROM_THE_EDGE, tuple_face
    assert 0 < span["height"] - (top + height) <= FROM_THE_EDGE, tuple_face

    tuple_view = tuple_opens("tuple-screen")
    assert not tuple_view["is_floating"], tuple_view
    assert tuple_view["layout"]["window_size"][0] >= span["width"] * NEARLY_ALL, tuple_view
    desktop.photograph("tuple-windows")
    desktop.as_account("pkill -f '[t]uple-window'")
    desktop.as_account(
        f"niri msg action close-window --id {desktop.window_titled('at work')['id']}"
    )
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q 'tuple-\\|at work'"),
        timeout=timedelta(seconds=30),
    )


def test_caps_lock_shows_in_the_margin_while_its_on(desktop: Desktop) -> None:
    """Caps lock shows in the margin while it's on."""
    machine = desktop.machine
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    desktop.photograph("margin-capitals")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 0 {CAPITALS}", timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_caps_lock_still_shows_after_its_watcher_restarts(desktop: Desktop) -> None:
    """Caps lock still shows after its watcher restarts."""
    machine = desktop.machine
    # Quickshell keeps running, so it must still hear the file.
    machine.succeed("systemctl restart keyboard-lights")
    machine.wait_for_unit("keyboard-lights.service")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    machine.send_key("caps_lock")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


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

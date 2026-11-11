"""The screens test's keep-places area.

niri starts with the lid shut and holds the first frame, typing keeps the session awake, and
sessions and workspaces go back where they sat.

tests/screens.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The machine and each step it's taken
through are screens_session.py's fixtures, which every test asks for by name.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import time
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from screens_session import Screens

# The fixtures are a plugin's, never imported here, so pytest loads the module once and checks
# its asserts as it does a test's.
pytest_plugins = ["screens_session"]

# The monitor beside the laptop's own panel, as screens.nix names it.
MONITOR = "Virtual-1"

# More colours than a screen nothing drew on shows, one or the console's few, and far fewer
# than the wallpaper's.
DRAWN = 64

# How long, in whole seconds as swayidle takes them, the session sits untouched before a
# watcher's swayidle says it's idle, and how many times that keys go on being typed through
# the virtual keyboard, as dictation types its words.
IDLE_AFTER = 5

TYPED_FOR = 3

# What the typing says when wtype itself fails, apart from the session going idle.
WTYPE_FAILED = 2


def kept_on(output: str, workspace: int, column: int) -> dict[str, Any]:
    """Give a place kept on a screen by its connector.

    The VM's two cards describe themselves alike, so each place names its connector, as one kept
    before screens were named by what they are does.
    """
    return {"output": output, "workspace": workspace, "column": column, "tile": 1}


def test_a_lid_shut_as_niri_starts_keeps_the_laptops_screen_dark(shut_at_start: Screens) -> None:
    """Niri started with the lid shut connects the monitor alone, never the laptop's screen.

    On 2026-11-03 a laptop started on a dock with its lid shut had niri connect its panel and
    lay the monitor out beside it, then take the panel away 67 ms later, when it heard the lid.
    """
    assert shut_at_start.connected_shut == [MONITOR], (
        f"niri connected {shut_at_start.connected_shut} with the lid shut, not {MONITOR} alone"
    )


def test_niri_held_the_monitors_first_frame_for_the_desktop(booted: Screens) -> None:
    """The monitor's first frame was held for the wallpaper, or until niri's hold ran out.

    The laptop's screen comes once the lid opens, after the hold, so it isn't held.
    """
    said = booted.niri_said(": first frame drawn ")
    print(said)
    assert re.search(
        rf' {MONITOR}: first frame drawn (once "wallpaper" covered it'
        r"|as its hold ran out), \d+ ms after starting$",
        said,
        re.MULTILINE,
    ), f"niri never said it held {MONITOR}'s first frame: {said}"


def test_typing_through_the_virtual_keyboard_keeps_the_session_awake(booted: Screens) -> None:
    """Keys typed through wtype count as activity, as dictation's do, so the session stays awake."""
    idled = "/run/user/1000/idled-while-typed-to"
    # A swayidle of its own, with a timeout far shorter than the session's, says when niri
    # takes the session as idle. Shift alone types nothing into whatever has the keys.
    typed, said = booted.machine.execute(
        booted.in_niri(
            f"rm -f {idled}"
            " && systemd-run --user --collect --unit=idle-watch"
            f" swayidle timeout {IDLE_AFTER} 'touch {idled}'"
            f" && end=$((SECONDS + {IDLE_AFTER * TYPED_FOR}))"
            " && while [ $SECONDS -lt $end ]; do"
            f" wtype -k Shift_L || exit {WTYPE_FAILED};"
            f" if [ -e {idled} ]; then echo idle after $SECONDS s; exit 1; fi;"
            " sleep 0.5; done"
        ),
        timeout=dt.timedelta(seconds=IDLE_AFTER * TYPED_FOR + 30),
    )
    try:
        assert typed != WTYPE_FAILED, f"wtype couldn't type: {said}"
        assert typed == 0, (
            f"niri took the session as idle while keys were typed through wtype: {said}"
        )
        # Once the typing stops, the watcher must say so, or it was never watching.
        booted.machine.wait_until_succeeds(
            f"test -e {idled}", timeout=dt.timedelta(seconds=IDLE_AFTER * 3)
        )
    finally:
        booted.machine.execute(booted.in_niri("systemctl --user stop idle-watch"))


def test_sessions_reopened_while_their_screen_is_gone_go_to_it_once_its_back(
    booted: Screens,
) -> None:
    """Sessions reopened while the screen they sat on is gone go to it, in order, once it's back.

    A screen can come well after niri does, as the monitor came 52 s after it on 2026-11-02, so
    the sessions kept on it open on the other first. Once it's back, their workspaces must go
    to it in their order, and the session kept on the other stays there.
    """
    machine = booted.machine

    def home(now: dict[str, tuple[str, int, int]]) -> bool:
        """Whether each is on its own screen, in its order there."""
        if len(now) < len(where):
            return False
        left, right, up = now["01c5-left"], now["01c5-right"], now["01c5-up"]
        return (
            now["01c5-there"][0] == "Virtual-1"
            and left[:2] == right[:2] == ("Virtual-2", 1)
            and left[2] < right[2]
            and up[:2] == ("Virtual-2", 2)
        )

    where = {
        "01c5-there": kept_on("Virtual-1", 1, 1),
        "01c5-left": kept_on("Virtual-2", 1, 1),
        "01c5-right": kept_on("Virtual-2", 1, 2),
        "01c5-up": kept_on("Virtual-2", 2, 1),
    }
    boot = booted.hand_over(where)
    try:
        booted.plug("off", "Virtual-2")
        booted.outputs_say("Virtual-2", there=False)
        booted.reopen(where)
        # The screen comes once the first has opened, with the rest to come.
        machine.wait_until_succeeds(
            booted.in_niri("niri msg --json windows | grep -q 'pi --session 01c5-'"),
            timeout=dt.timedelta(seconds=120),
        )
        booted.plug("detect", "Virtual-2")
        booted.outputs_say("Virtual-2", there=True)
        deadline = time.monotonic() + 120
        while not home(now := booted.sessions_sat("01c5-")):
            assert time.monotonic() < deadline, f"not on their own screen in order: {now}"
            time.sleep(1)
    finally:
        booted.handed_back(boot)
        booted.plug("detect", "Virtual-2")
        booted.outputs_say("Virtual-2", there=True)


def test_sessions_reopened_while_every_screen_drops_and_comes_back_go_where_they_sat(
    booted: Screens,
) -> None:
    """Sessions reopened while every screen drops and comes back go where they sat, in order.

    At the 21:55 boot on 2026-11-02 the monitor dropped for three seconds while sessions were
    reopened, and the first session of its second workspace stayed at the end of its first.
    """
    machine = booted.machine
    where = {
        "01c6-there": kept_on("Virtual-1", 1, 1),
        "01c6-left": kept_on("Virtual-2", 1, 1),
        "01c6-right": kept_on("Virtual-2", 1, 2),
        "01c6-up": kept_on("Virtual-2", 2, 1),
        "01c6-down": kept_on("Virtual-2", 2, 2),
    }

    def home(now: dict[str, tuple[str, int, int]]) -> bool:
        """Whether each is on its own screen, its workspace and its order there."""
        if len(now) < len(where):
            return False
        left, right = now["01c6-left"], now["01c6-right"]
        up, down = now["01c6-up"], now["01c6-down"]
        return (
            now["01c6-there"][0] == "Virtual-1"
            and left[:2] == right[:2] == ("Virtual-2", 1)
            and left[2] < right[2]
            and up[:2] == down[:2] == ("Virtual-2", 2)
            and up[2] < down[2]
        )

    boot = booted.hand_over(where)
    try:
        booted.reopen(where)
        # Both screens go once the first has opened, as an undock does, and
        # come back once the third has begun, with the rest to come.
        machine.wait_until_succeeds(
            booted.in_niri("niri msg --json windows | grep -q 'pi --session 01c6-'"),
            timeout=dt.timedelta(seconds=120),
        )
        booted.plug("off")
        for screen in sorted(booted.both):
            booted.outputs_say(screen, there=False)
        # One that opens with no screen to go to holds the next back until a
        # screen comes or its 30 seconds' patience runs out, so the third may
        # wait that long. The shell says when it begins one; a process
        # naming it is no sign, since the stand-in pi becomes a bare sleep
        # within a second, and one begun as the screens went was missed.
        machine.wait_until_succeeds(
            booted.in_niri(
                "journalctl --user -u quickshell -o cat"
                " | grep -qF 'report[resumed session 01c6-right]'"
            ),
            timeout=dt.timedelta(seconds=180),
        )
        booted.plug("detect")
        for screen in sorted(booted.both):
            booted.outputs_say(screen, there=True)
        deadline = time.monotonic() + 120
        while not home(now := booted.sessions_sat("01c6-")):
            assert time.monotonic() < deadline, f"not where they sat: {now}"
            time.sleep(1)
    finally:
        booted.handed_back(boot)
        booted.plug("detect")
        for screen in sorted(booted.both):
            booted.outputs_say(screen, there=True)


def test_reopen_them_all_after_a_reboot_reopens_what_the_boot_left_where_each_sat(
    booted: Screens,
) -> None:
    """Reopen Them All after a reboot reopens what restore.json kept, each where it sat, once.

    The shell starting reopens nothing: Joel reopens them when he asks (2026-11-07, "no, i
    never wanted them automatic"), and at 09:08 that day a switch restarting the shell had
    reopened 36 sessions. restore.json alone says what the boot left, as the harness writes it
    whatever ends a boot, so a reboot nobody listed by hand is offered the same (2026-11-09).
    A window Joel opens as he signs in, as his browser alone on the first at 10:33 on 11-07,
    shares no workspace with a session, and the sessions keep who sits with whom. Since
    2026-11-09 they go back by identity, never by a workspace's number (Joel: one placer puts
    every window back by identity), so they go on the workspaces after his.
    """
    machine = booted.machine
    where = {
        "01c7-there": kept_on("Virtual-1", 1, 1),
        "01c7-left": kept_on("Virtual-2", 1, 1),
        "01c7-right": kept_on("Virtual-2", 1, 2),
        "01c7-up": kept_on("Virtual-2", 2, 1),
    }
    # Each the shell began, by the word it says as it begins one.
    begun = (
        "journalctl --user -u quickshell -o cat"
        " | grep -o 'report\\[resumed session 01c7-[a-z]*' | sort | uniq -c"
    )
    restart = (
        "systemctl --user restart quickshell && until"
        " [ $(ps -o etimes= -p $(systemctl --user show -p MainPID --value quickshell)) -ge 10 ];"
        " do sleep 1; done"
    )

    def counted() -> dict[str, str]:
        """Give how many times the shell began each, by its session."""
        lines = machine.succeed(booted.in_niri(begun)).splitlines()
        return {line.split()[-1]: line.split()[0] for line in lines}

    def home(now: dict[str, tuple[str, int, int]]) -> bool:
        """Whether each listed is on its own screen, its workspace and its order there."""
        if not all(one in now for one in where):
            return False
        # Each by who sits with whom, never by its workspace's number, which
        # Joel's window has taken: left and right together, up on the next,
        # and none with Lantern (Joel, 2026-11-09: one placer puts every
        # window back by identity).
        left, right, up = now["01c7-left"], now["01c7-right"], now["01c7-up"]
        return (
            now["01c7-there"][0] == "Virtual-1"
            and left[:2] == right[:2]
            and left[0] == "Virtual-2"
            and left[2] < right[2]
            and up[:2] == ("Virtual-2", left[1] + 1)
            and lantern() not in {left[:2], up[:2]}
        )

    def lantern() -> tuple[str, int]:
        """Give the screen and workspace Joel's own window is on."""
        listed = json.loads(machine.succeed(booted.in_niri("niri msg --json workspaces")))
        spaces = {one["id"]: one for one in listed}
        windows = json.loads(machine.succeed(booted.in_niri("niri msg --json windows")))
        [space] = [spaces[one["workspace_id"]] for one in windows if one["title"] == "Lantern"]
        return space["output"], space["idx"]

    def windows() -> list[dict[str, Any]]:
        """Give every window niri lays out."""
        listed: list[dict[str, Any]] = json.loads(
            machine.succeed(booted.in_niri("niri msg --json windows"))
        )
        return listed

    def focused() -> list[str]:
        """Give the title of each window niri says has the focus."""
        return [one["title"] for one in windows() if one["is_focused"]]

    # restore.json holds what the boot before left, and the shell kept where each sat.
    boot = booted.hand_over(where)
    try:
        # Joel's own window, opened as he signs in, where two of them sat.
        machine.succeed(booted.in_niri("niri msg action focus-monitor Virtual-2"))
        machine.succeed(booted.in_niri("niri msg action focus-workspace 1"))
        machine.succeed(booted.in_niri("niri msg action spawn -- kitty --title Lantern"))
        machine.wait_until_succeeds(
            booted.in_niri('niri msg --json windows | grep -qF \'"title":"Lantern"\''),
            timeout=dt.timedelta(seconds=120),
        )
        # A shell starting after the reboot reopens nothing of it by itself.
        machine.succeed(booted.in_niri(restart), timeout=dt.timedelta(seconds=90))
        assert counted() == {}, counted()
        # Joel is typing in his own window as he asks.
        [joels] = [one["id"] for one in windows() if one["title"] == "Lantern"]
        machine.succeed(booted.in_niri(f"niri msg action focus-window --id {joels}"))
        assert focused() == ["Lantern"], focused()
        # Reopen Them All, as Joel's press runs it from Agents (Now holds
        # the same row), reopens them, each where it sat.
        reopen_all = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"
        catalogue = json.loads(machine.succeed(booted.in_niri("qs ipc call launcher list")))
        assert reopen_all in [e["id"] for e in catalogue], "no Reopen Them All in Agents"
        ran = machine.succeed(booted.in_niri(f"qs ipc call launcher run {shlex.quote(reopen_all)}"))
        assert '"error"' not in ran, ran
        deadline = time.monotonic() + 180
        while not home(now_sat := booted.sessions_sat("01c7-")):
            assert time.monotonic() < deadline, f"not reopened where they sat: {now_sat}"
            time.sleep(1)
        # None of them took the focus from him: a reopened window takes no
        # keys (item 11's rollback, 2026-11-11: each back unfocused).
        assert focused() == ["Lantern"], focused()
        # Started again, the shell reopens none twice.
        machine.succeed(booted.in_niri(restart), timeout=dt.timedelta(seconds=90))
        assert counted() == dict.fromkeys(where, "1"), counted()
    finally:
        booted.handed_back(boot)
        machine.succeed(booted.in_niri("pkill -f '[k]itty --title Lantern' || true"))


def test_a_screen_coming_back_takes_back_its_workspaces_though_one_took_a_window(
    booted: Screens,
) -> None:
    """A screen coming back takes back each workspace it had, in order, one given a window too.

    An undock puts the monitor's workspaces on the laptop's screen, and niri gives one back
    only if nothing opened in it meanwhile: on 2026-11-03 a session handed over while Joel was
    undocked made his first workspace the laptop's, so redocking would have left it there.
    """
    machine = booted.machine
    opened: list[str] = []

    def open_on(title: str) -> None:
        """Open a window with this title where the focus is, and wait until niri lists it."""
        opened.append(title)
        machine.succeed(booted.in_niri(f"niri msg action spawn -- kitty --title {title}"))
        machine.wait_until_succeeds(
            booted.in_niri(f'niri msg --json windows | grep -qF \'"title":"{title}"\''),
            timeout=dt.timedelta(seconds=120),
        )

    def where() -> dict[str, tuple[str, int]]:
        """Give each window opened here's screen and workspace, by its title."""
        listed = json.loads(machine.succeed(booted.in_niri("niri msg --json workspaces")))
        spaces = {one["id"]: one for one in listed}
        windows = json.loads(machine.succeed(booted.in_niri("niri msg --json windows")))
        return {
            window["title"]: (
                spaces[window["workspace_id"]]["output"],
                spaces[window["workspace_id"]]["idx"],
            )
            for window in windows
            if window["title"] in opened
        }

    try:
        machine.succeed(booted.in_niri("niri msg action focus-monitor Virtual-2"))
        machine.succeed(booted.in_niri("niri msg action focus-workspace 1"))
        open_on("Tide")
        machine.succeed(booted.in_niri("niri msg action focus-workspace 2"))
        open_on("Reef")
        assert where() == {"Tide": ("Virtual-2", 1), "Reef": ("Virtual-2", 2)}, where()
        booted.plug("off", "Virtual-2")
        booted.outputs_say("Virtual-2", there=False)
        reef = next(
            window["id"]
            for window in json.loads(machine.succeed(booted.in_niri("niri msg --json windows")))
            if window["title"] == "Reef"
        )
        machine.succeed(booted.in_niri(f"niri msg action focus-window --id {reef}"))
        open_on("Shell")
        assert where()["Shell"] == where()["Reef"], where()
        booted.plug("detect", "Virtual-2")
        booted.outputs_say("Virtual-2", there=True)
        home = {"Tide": ("Virtual-2", 1), "Reef": ("Virtual-2", 2), "Shell": ("Virtual-2", 2)}
        deadline = time.monotonic() + 60
        while (now := where()) != home:
            assert time.monotonic() < deadline, f"not back on their own screen in order: {now}"
            time.sleep(1)
    finally:
        booted.plug("detect", "Virtual-2")
        booted.outputs_say("Virtual-2", there=True)
        for window in json.loads(machine.succeed(booted.in_niri("niri msg --json windows"))):
            if window["title"] in opened:
                machine.succeed(booted.in_niri(f"niri msg action close-window --id {window['id']}"))


def test_workspaces_moved_from_a_screen_turned_off_stay_where_joel_puts_them(
    booted: Screens,
) -> None:
    """Workspaces moved from a screen turned off stay where Joel puts them, until it's back.

    The laptop's screen is off while its lid is shut on the dock, and niri still lists it. On
    2026-11-03 the shell took it for there, so each change of the workspaces sent the ones it
    had back to their old order, and a workspace put third went sixth again.
    """
    machine = booted.machine
    opened: list[str] = []

    def open_on(title: str) -> int:
        """Open a window with this title where the focus is, and give its id once listed."""
        opened.append(title)
        machine.succeed(booted.in_niri(f"niri msg action spawn -- kitty --title {title}"))
        machine.wait_until_succeeds(
            booted.in_niri(f'niri msg --json windows | grep -qF \'"title":"{title}"\''),
            timeout=dt.timedelta(seconds=120),
        )
        windows = json.loads(machine.succeed(booted.in_niri("niri msg --json windows")))
        return int(next(window["id"] for window in windows if window["title"] == title))

    def order() -> list[str]:
        """Give the windows opened here by their workspaces' order on the first screen."""
        listed = json.loads(machine.succeed(booted.in_niri("niri msg --json workspaces")))
        spaces = {one["id"]: one for one in listed}
        windows = json.loads(machine.succeed(booted.in_niri("niri msg --json windows")))
        mine = [window for window in windows if window["title"] in opened]
        on = [window for window in mine if spaces[window["workspace_id"]]["output"] == "Virtual-1"]
        return [
            window["title"]
            for window in sorted(on, key=lambda window: spaces[window["workspace_id"]]["idx"])
        ]

    try:
        machine.succeed(booted.in_niri("niri msg action focus-monitor Virtual-2"))
        machine.succeed(booted.in_niri("niri msg action focus-workspace 1"))
        open_on("Cove")
        machine.succeed(booted.in_niri("niri msg action focus-workspace 2"))
        shoal = open_on("Shoal")
        # Off, as the lid shut on the dock leaves the laptop's: listed, but with no workspace.
        machine.succeed(booted.in_niri("niri msg output Virtual-2 off"))
        machine.wait_until_succeeds(
            booted.in_niri('! niri msg --json workspaces | grep -qF \'"output":"Virtual-2"\''),
            timeout=dt.timedelta(seconds=60),
        )
        machine.wait_until_succeeds(
            booted.in_niri('niri msg --json windows | grep -qF \'"title":"Shoal"\'')
        )
        # Joel puts Shoal's workspace before Cove's, and changes nothing after.
        machine.succeed(booted.in_niri(f"niri msg action focus-window --id {shoal}"))
        machine.succeed(booted.in_niri("niri msg action move-workspace-to-index 1"))
        assert order().index("Shoal") < order().index("Cove"), order()
        # A change of the workspaces, as a session opening its own does.
        machine.succeed(booted.in_niri("niri msg action focus-workspace-down"))
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            now = order()
            assert now.index("Shoal") < now.index("Cove"), f"put back in their old order: {now}"
            time.sleep(1)
    finally:
        machine.succeed(booted.in_niri("niri msg output Virtual-2 on"))
        booted.outputs_say("Virtual-2", there=True)
        for window in json.loads(machine.succeed(booted.in_niri("niri msg --json windows"))):
            if window["title"] in opened:
                machine.succeed(booted.in_niri(f"niri msg action close-window --id {window['id']}"))


def test_a_niri_from_before_the_hold_takes_the_settings_a_switch_brings(booted: Screens) -> None:
    """A niri started before the hold came in takes the account's settings, as at a switch."""
    older = "/etc/niri-before-the-hold/bin/niri"
    # niri reads its settings again in place when a switch changes them, as validate reads them.
    booted.machine.succeed(booted.in_niri(f"{older} validate -c ~/.config/niri/config.kdl"))
    # The hold asked in the settings, as it once was, which that niri refused, every line of it.
    refused = booted.machine.fail(
        booted.in_niri(
            "d=$(mktemp -d) && cp -L ~/.config/niri/*.kdl $d/ && chmod u+w $d/theme.kdl"
            " && printf 'debug {\\n    hold-first-frame-for-background\\n}\\n' >> $d/theme.kdl"
            f" && {older} validate -c $d/config.kdl 2>&1",
        ),
    )
    assert "hold-first-frame-for-background" in refused, (
        f"a niri from before the hold took a setting only the hold knows: {refused}"
    )


def test_niri_draws_on_both_screens_the_second_cards_too(booted: Screens) -> None:
    """Niri draws on both screens, the second card's too.

    The second card has no render node, so only niri drawing through the card's own node puts
    the wallpaper there; a screen nothing draws on keeps one colour, or the console's few.
    """
    for screen in sorted(booted.both):
        deadline = time.monotonic() + 60
        while (seen := booted.colours_on(screen)) <= DRAWN:
            assert time.monotonic() < deadline, f"{screen} shows {seen} colours, so nothing drew it"
            time.sleep(1)
        booted.machine.log(f"{screen} shows {seen} colours")

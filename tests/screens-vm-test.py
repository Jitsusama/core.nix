"""The shell's surfaces through a screen going and coming back, in a virtual machine.

tests/screens.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test asks for the state it starts from as a
fixture, and each fixture takes the machine one step on from the one it asks for: signed in
with a notice and the margin's marks up, the screens gone and back, the console switched, a
sleep, then two screens apart, and the launcher through the screens going and a sleep. A
fixture's work is done once a run, however many tests ask for it, so any test runs alone with
`-k`, and a whole run takes the steps in the order they were always taken. The account greetd
signs in is screens.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable

    from test_driver.machine import BaseMachine, QemuMachine

# How many reads a second apart must find every surface before a wait trusts them. A screen
# coming back can make the shell remake a surface that was already there, as the second of two
# screens returning does, so one read can see it just before it goes.
STEADY_READS = 2
# How long a wait for the shell's surfaces may take, and a launcher's step.
SURFACING = dt.timedelta(seconds=60)
STEPPING = dt.timedelta(seconds=30)
# Each screen's card, by the id screens.nix gives it, so QEMU can say what that card shows.
CARDS = {"Virtual-1": "first", "Virtual-2": "second"}
# More colours than a screen nothing drew on shows, one or the console's few, and far fewer
# than the wallpaper's.
DRAWN = 64
# What the account's password is, as screens.nix sets it, which lets the lock go after a sleep.
PASSWORD = "correct-horse"  # noqa: S105 (a test machine's, said in screens.nix)
# Every surface the shell keeps up while a notice, Keep Awake and dictation are on.
EVERY_SURFACE = ("wallpaper", "launcher", "notifications", "margin", "dictation")


class Screens:
    """The machine with niri on its two screens, signed in as the account."""

    def __init__(self, machine: QemuMachine, account: str) -> None:
        """Act on that machine, as that account."""
        self.machine = machine
        self.account = account
        # Each screen's connector, by the name niri gives the screen: one on each card.
        self.connectors = {
            path.split("-", 1)[1]: path
            for path in machine.succeed("ls -d /sys/class/drm/card*-Virtual-*").split()
        }
        self.both = set(self.connectors)
        # Where the shell's journal stood before the launcher's tests, set by their fixture.
        self.launcher_since = ""

    def in_niri(self, command: str) -> str:
        """Give a command, run as the account inside its niri session, to run on the machine."""
        surroundings = (
            "export XDG_RUNTIME_DIR=/run/user/1000"
            " DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus;"
            " export $(systemctl --user show-environment"
            " | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)=');"
        )
        return f"su - {self.account} -c " + shlex.quote(f"{surroundings} {command}")

    def surfaces(self) -> set[str]:
        """Give the shell's surfaces niri holds now, by namespace."""
        held = json.loads(self.machine.succeed(self.in_niri("niri msg --json layers")))
        return {layer["namespace"] for layer in held}

    def wait_for_surfaces(self, *namespaces: str, timeout: dt.timedelta = SURFACING) -> None:
        """Wait until niri holds a surface for each namespace and keeps it, saying which didn't."""
        deadline = time.monotonic() + timeout.total_seconds()
        wanted = " && ".join(
            f"niri msg --json layers | grep -qF {shlex.quote(json.dumps(name))}"
            for name in namespaces
        )
        status, _ = self.machine.execute(
            f"timeout {timeout.total_seconds():.0f} sh -c "
            + shlex.quote(f"until {self.in_niri(wanted)} > /dev/null 2>&1; do sleep 1; done"),
            timeout=timeout + dt.timedelta(seconds=30),
        )
        steady = 0
        while True:
            missing = set(namespaces) - self.surfaces()
            steady = 0 if missing else steady + 1
            if steady >= STEADY_READS or time.monotonic() >= deadline:
                break
            time.sleep(1)
        if status != 0 or missing:
            self.machine.screenshot("missing-surfaces")
        assert not missing, f"niri holds no surface for {sorted(missing)}"

    def wait_for_no_surface(self, namespace: str, timeout: dt.timedelta = SURFACING) -> None:
        """Wait until niri holds no surface for a namespace, saying it stayed."""
        deadline = time.monotonic() + timeout.total_seconds()
        while namespace in self.surfaces():
            assert time.monotonic() < deadline, f"niri still holds a surface for {namespace!r}"
            time.sleep(1)

    def held_on(self, namespace: str) -> set[str]:
        """Give the screens niri holds a surface of a namespace on, by name."""
        held = json.loads(self.machine.succeed(self.in_niri("niri msg --json layers")))
        return {layer["output"] for layer in held if layer["namespace"] == namespace}

    def wait_for_held_on(
        self, namespace: str, screens: set[str], timeout: dt.timedelta = SURFACING
    ) -> None:
        """Wait until niri holds a namespace's surfaces on exactly these screens."""
        deadline = time.monotonic() + timeout.total_seconds()
        while (now := self.held_on(namespace)) != screens:
            assert time.monotonic() < deadline, f"{namespace} is on {sorted(now)}, not {screens}"
            time.sleep(1)

    def plug(self, status: str, *named: str) -> None:
        """Force screens' connectors to a status and tell niri, as a cable coming or going does.

        It takes every screen at once, as an undock does, unless some are named. The VM's
        display driver doesn't watch its connectors, so a forced status alone announces nothing;
        niri looks at a card's screens again on any change to it.
        """
        for name in named or self.connectors:
            path = self.connectors[name]
            card = path.rsplit("-", 2)[0]
            self.machine.succeed(
                f"echo {status} > {path}/status && udevadm trigger --action=change --settle {card}"
            )

    def outputs_say(self, screen: str, *, there: bool) -> None:
        """Wait until niri lists a screen among its outputs, or no longer lists it."""
        self.machine.wait_until_succeeds(
            self.in_niri(
                f"{'' if there else '! '}niri msg --json outputs | grep -qF '\"{screen}\"'"
            ),
            timeout=dt.timedelta(seconds=60),
        )

    def displays_say(self, *, named: bool) -> None:
        """Wait until desk-facts' displays.json names the screen, or no longer names it."""
        facts = "/run/user/1000/desk/displays.json"
        self.machine.wait_until_succeeds(
            f"test -s {facts} && {'' if named else '! '}grep -qF '\"Virtual-1\"' {facts}",
            timeout=dt.timedelta(seconds=30),
        )
        self.machine.log(f"displays.json: {self.machine.succeed(f'cat {facts}')}")

    def niri_answers(self, after: str) -> None:
        """Fail, saying where each of niri's threads waits, unless niri answers."""
        answered, _ = self.machine.execute(
            self.in_niri("timeout 10 niri msg --json layers"), timeout=dt.timedelta(seconds=30)
        )
        waits = ""
        if answered != 0:
            _, waits = self.machine.execute(
                "for t in /proc/$(pgrep -o niri)/task/*; do"
                ' echo "$(cat $t/comm): $(cat $t/wchan)"; cat $t/stack; done',
                timeout=dt.timedelta(seconds=60),
            )
        assert answered == 0, f"niri stopped answering after {after}:\n{waits}"

    def listening(self) -> bool:
        """Say whether dictation listens, which its bar shows."""
        return "dictation" in self.surfaces()

    def quiet(self) -> None:
        """Have dictation stop listening, if it listens, and wait until its bar goes.

        The second press types, then says it heard nothing, and the bar goes.
        """
        if self.listening():
            self.machine.succeed(self.in_niri("dictation toggle"))
            self.wait_for_no_surface("dictation")

    def colours_on(self, screen: str) -> int:
        """Count the colours QEMU sees on a screen, in every seventh pixel."""
        with tempfile.TemporaryDirectory() as folder:
            picture = Path(folder) / "screen.ppm"
            said = self.machine.send_monitor_command(f"screendump {picture} {CARDS[screen]}")
            assert picture.exists(), f"QEMU gave no picture of {screen}: {said}"
            image = picture.read_bytes()
        # A binary PPM: its magic, width, height and depth, one blank, then
        # three bytes a pixel.
        header = re.match(rb"P6\s+\d+\s+\d+\s+\d+\s", image)
        assert header, f"{screen}'s picture isn't a PPM"
        pixels = image[header.end() :]
        return len({pixels[at : at + 3] for at in range(0, len(pixels) - 2, 3 * 7)})

    def sleep_and_wake(self) -> None:
        """Put the machine to sleep and wake it at a key, as a laptop sleeps and wakes."""
        # A key wakes a laptop, so the keyboard may wake the machine.
        self.machine.succeed(
            "for w in /sys/bus/serio/devices/*/power/wakeup; do echo enabled > $w; done"
        )
        self.machine.succeed("systemctl suspend")
        self.machine.wait_for_console_text("PM: suspend entry", timeout=dt.timedelta(seconds=60))
        # A key while it's still going to sleep would only stop it going, and
        # the wait is the driver's, since the sleeping machine runs nothing.
        time.sleep(5)
        self.machine.send_key("shift")
        self.machine.wait_for_console_text("PM: suspend exit", timeout=dt.timedelta(seconds=60))
        self.niri_answers("a sleep")

    def unlock(self) -> None:
        """Let the lock go with the account's password, as Joel does after a sleep, if it's up."""
        locked, _ = self.machine.execute(self.in_niri("qs ipc call lock isLocked | grep -qx true"))
        if locked != 0:
            return
        # A letter typed before the lock has the keys goes nowhere.
        self.machine.wait_until_succeeds(
            self.in_niri("journalctl --user -u quickshell -o cat | grep -qF 'report[keys lock]'"),
            timeout=dt.timedelta(seconds=30),
        )
        self.machine.send_chars(f"{PASSWORD}\n")
        self.machine.wait_until_succeeds(
            self.in_niri("qs ipc call lock isLocked | grep -qx false"),
            timeout=dt.timedelta(seconds=30),
        )

    # The launcher is how Joel reaches everything, so it must work on whichever
    # screen he's on after any of these: Super+Space opens it there and gives
    # it the keys, typing finds a row, Return runs it and the launcher closes,
    # and Escape closes it too. The row is Keep Awake, whose unit says it ran;
    # each run turns it the other way, so it's typed by the name it has now.

    def launcher_shows(self) -> dict[str, Any]:
        """Give what the launcher says it shows, from its showing call."""
        return cast(
            "dict[str, Any]",
            json.loads(self.machine.succeed(self.in_niri("qs ipc call launcher showing"))),
        )

    def launcher_until(
        self,
        test: Callable[[dict[str, Any]], object],
        what: str,
        timeout: dt.timedelta = STEPPING,
    ) -> dict[str, Any]:
        """Wait until what the launcher shows passes the test, saying what it never did."""
        deadline = time.monotonic() + timeout.total_seconds()
        while not test(shown := self.launcher_shows()):
            assert time.monotonic() < deadline, f"the launcher never {what}: {shown}"
            time.sleep(0.5)
        return shown

    def awake(self) -> bool:
        """Say whether Keep Awake is on."""
        status, _ = self.machine.execute(
            self.in_niri("systemctl --user is-active --quiet caffeinate.service")
        )
        return status == 0

    def super_space_on(self, screen: str) -> None:
        """Put the keys on a screen and press Super+Space, waiting until the launcher has them."""
        self.machine.succeed(self.in_niri(f"niri msg action focus-monitor {screen}"))
        self.machine.wait_until_succeeds(
            self.in_niri(f"niri msg --json focused-output | grep -qF '\"{screen}\"'"),
            timeout=dt.timedelta(seconds=10),
        )
        self.machine.send_key("meta_l-spc")
        self.launcher_until(
            lambda shown: shown["open"] and shown["keys"], f"took the keys on {screen}"
        )
        self.wait_for_held_on("launcher", {screen}, timeout=dt.timedelta(seconds=10))

    def launcher_works_on(self, screen: str) -> None:
        """Open the launcher on a screen, run Keep Awake and close it, then open it and close it."""
        was = self.awake()
        typed, row = ("let it sleep", "Let It Sleep") if was else ("keep awake", "Keep Awake")
        self.super_space_on(screen)
        self.machine.send_chars(typed)
        self.launcher_until(
            lambda shown: shown["typed"] == typed and (shown["selected"] or {}).get("name") == row,
            f"found {row!r} on {screen}",
        )
        # Keep Awake is a setting, which Return does and stays to show;
        # Ctrl+Return does it and closes.
        self.machine.send_key("ctrl-ret")
        self.launcher_until(lambda shown: not shown["open"], f"closed once {row!r} ran on {screen}")
        deadline = time.monotonic() + 30
        while self.awake() == was:
            assert time.monotonic() < deadline, f"{row!r} did nothing on {screen}"
            time.sleep(0.5)
        self.super_space_on(screen)
        self.machine.send_key("esc")
        self.launcher_until(lambda shown: not shown["open"], f"closed at Escape on {screen}")

    def launcher_works_on_both(self) -> None:
        """Have the launcher open, find and run a row on each screen, in their names' order."""
        for screen in sorted(self.both):
            self.launcher_works_on(screen)

    def gone_while_open(self, *screens: str) -> None:
        """Open the launcher on the first screen, and take these screens while it's open.

        A launcher left on no screen would never open again, so it must close and stand ready.
        """
        self.super_space_on(screens[0])
        self.plug("off", *screens)
        for screen in screens:
            self.outputs_say(screen, there=False)
        self.launcher_until(lambda shown: not shown["open"], f"closed when {screens} went")

    def come_back(self, *screens: str) -> None:
        """Bring these screens back, and wait until the launcher's surface is on one again."""
        self.plug("detect", *screens)
        for screen in screens:
            self.outputs_say(screen, there=True)
        self.wait_for_surfaces("launcher")

    def minute(self) -> str:
        """Give the machine's time to the minute, as the launcher's foot says it."""
        return self.machine.succeed("date +%H:%M").strip()


@dataclass(frozen=True)
class Apart:
    """The two screens with dictation listening on one of them."""

    screens: Screens
    # The screen dictation's bar is on, and the other.
    listening: str
    left: set[str]


@pytest.fixture(scope="module")
def booted(machine: BaseMachine, values: dict[str, Any]) -> Screens:
    """Give the machine signed in, with the wallpaper and the launcher on its screens."""
    # The test's one machine is QEMU's, which takes keys and screenshots.
    qemu = cast("QemuMachine", machine)
    qemu.wait_for_unit("multi-user.target")
    screens = Screens(qemu, values["account"])
    screens.wait_for_surfaces("wallpaper", "launcher", timeout=dt.timedelta(seconds=300))
    assert sorted(screens.connectors) == ["Virtual-1", "Virtual-2"], (
        f"the screens are {screens.connectors}"
    )
    return screens


@pytest.fixture(scope="module")
def noticed(booted: Screens) -> Screens:
    """Give the machine with a critical notice up, one that never goes by itself."""
    booted.machine.succeed(
        booted.in_niri(
            "notify-send --urgency=critical 'Still here' 'A notice that never goes by itself'"
        )
    )
    return booted


@pytest.fixture(scope="module")
def marked(noticed: Screens) -> Screens:
    """Give the machine with Keep Awake's mark in the margin and dictation listening."""
    # Keep Awake is a mark the margin keeps for as long as it's on.
    noticed.machine.succeed(noticed.in_niri("systemctl --user start caffeinate.service"))
    # The stand-in listens from the first press until the next.
    noticed.machine.succeed(noticed.in_niri("dictation toggle"))
    return noticed


@pytest.fixture(scope="module")
def gone(marked: Screens) -> Screens:
    """Give the machine with every screen gone, as at an undock."""
    marked.plug("off")
    marked.machine.wait_until_succeeds(
        marked.in_niri("niri msg --json outputs | grep -qx '{}'"),
        timeout=dt.timedelta(seconds=60),
    )
    return marked


@pytest.fixture(scope="module")
def back(gone: Screens) -> Screens:
    """Give the machine with its screens back."""
    gone.plug("detect")
    gone.machine.wait_until_succeeds(
        gone.in_niri("niri msg outputs | grep -qF Virtual-1"), timeout=dt.timedelta(seconds=60)
    )
    return gone


@pytest.fixture(scope="module")
def switched(back: Screens) -> Screens:
    """Give the machine once the console has been switched away and back."""
    console = back.machine.succeed("cat /sys/class/tty/tty0/active").strip().removeprefix("tty")
    back.machine.succeed(f"chvt 2 && sleep 2 && chvt {console}")
    return back


@pytest.fixture(scope="module")
def slept(switched: Screens) -> Screens:
    """Give the machine once it has slept as a laptop does and woken at a key."""
    sleeps = switched.machine.succeed("cat /sys/power/mem_sleep").strip()
    assert "[s2idle]" in sleeps, f"the machine sleeps by {sleeps}, not as a laptop does"
    switched.sleep_and_wake()
    return switched


@pytest.fixture(scope="module")
def apart(slept: Screens) -> Apart:
    """Give the two screens with Keep Awake on and dictation listening, its bar on one."""
    slept.machine.succeed(slept.in_niri("systemctl --user start caffeinate.service"))
    if not slept.listening():
        slept.machine.succeed(slept.in_niri("dictation toggle"))
    slept.machine.wait_until_succeeds(
        slept.in_niri("niri msg --json layers | grep -qF '\"dictation\"'"),
        timeout=dt.timedelta(seconds=30),
    )
    [listening] = slept.held_on("dictation")
    return Apart(slept, listening, slept.both - {listening})


@pytest.fixture(scope="module")
def one_gone(apart: Apart) -> Apart:
    """Give the two screens once the one dictation's bar is on has gone."""
    apart.screens.plug("off", apart.listening)
    apart.screens.outputs_say(apart.listening, there=False)
    return apart


@pytest.fixture(scope="module")
def one_back(one_gone: Apart) -> Apart:
    """Give the two screens once the one that went has come back."""
    one_gone.screens.plug("detect", one_gone.listening)
    return one_gone


@pytest.fixture(scope="module")
def unlocked(slept: Screens) -> Screens:
    """Give the machine unlocked after its sleep, the shell's journal marked for the launcher.

    The shell warns of what this machine lacks, such as UPower, in its first seconds, so the
    mark waits until it's been up a while, and what the launcher's tests make it say can be read
    apart from what the start and the screens before said.
    """
    slept.unlock()
    slept.machine.wait_until_succeeds(
        slept.in_niri(
            "test $(ps -o etimes= -p $(systemctl --user show -p MainPID --value quickshell)) -ge 20"
        ),
        timeout=dt.timedelta(seconds=60),
    )
    slept.launcher_since = slept.machine.succeed("date +%s").strip()
    return slept


@pytest.fixture(scope="module")
def first_gone(unlocked: Screens) -> Screens:
    """Give the machine once the first screen went with the launcher open on it."""
    unlocked.gone_while_open("Virtual-1")
    return unlocked


@pytest.fixture(scope="module")
def first_back(first_gone: Screens) -> Screens:
    """Give the machine once the first screen has come back."""
    first_gone.come_back("Virtual-1")
    return first_gone


@pytest.fixture(scope="module")
def second_gone(first_back: Screens) -> Screens:
    """Give the machine once the second screen went with the launcher open on it."""
    first_back.gone_while_open("Virtual-2")
    return first_back


@pytest.fixture(scope="module")
def second_back(second_gone: Screens) -> Screens:
    """Give the machine once the second screen has come back."""
    second_gone.come_back("Virtual-2")
    return second_gone


@pytest.fixture(scope="module")
def slept_open(second_back: Screens) -> Screens:
    """Give the machine once it slept with the launcher open and woke, and was unlocked."""
    second_back.super_space_on("Virtual-1")
    second_back.sleep_and_wake()
    # The screens lock before a sleep, so the keys are the lock's until
    # the password lets it go.
    second_back.machine.wait_until_succeeds(
        second_back.in_niri("qs ipc call lock isLocked | grep -qx true"),
        timeout=dt.timedelta(seconds=30),
    )
    second_back.unlock()
    # Whether it's still open after is the shell's to say, but it must hold
    # the keys if it is, and close at Escape.
    shown = second_back.launcher_until(
        lambda shown: not shown["open"] or shown["keys"], "held the keys"
    )
    if shown["open"]:
        second_back.machine.send_key("esc")
        second_back.launcher_until(
            lambda shown: not shown["open"], "closed at Escape after the wake"
        )
    return second_back


def test_a_critical_notice_is_up_before_the_screen_goes(noticed: Screens) -> None:
    """A critical notice is up before the screen goes."""
    noticed.wait_for_surfaces("notifications")


def test_the_margin_shows_a_mark_and_dictation_listens_first(marked: Screens) -> None:
    """The margin shows a mark, and dictation listens, before the screen goes."""
    marked.wait_for_surfaces("margin", "dictation")


def test_desk_facts_says_what_the_screen_is_before_it_goes(booted: Screens) -> None:
    """Desk-facts says what the screen is before it goes."""
    booted.displays_say(named=True)


def test_the_screen_goes_and_niri_lets_it_go(gone: Screens) -> None:
    """The screen goes, and niri lets it go."""
    gone.machine.succeed(gone.in_niri("niri msg --json outputs | grep -qx '{}'"))


def test_desk_facts_no_longer_names_the_screen_once_its_gone(gone: Screens) -> None:
    """Desk-facts no longer names the screen once it's gone."""
    gone.displays_say(named=False)


def test_the_screen_comes_back_and_the_launcher_and_notice_with_it(back: Screens) -> None:
    """The screen comes back, and the launcher and the notice with it."""
    back.wait_for_surfaces("wallpaper", "launcher", "notifications")


def test_the_margins_mark_and_dictations_bar_come_back(back: Screens) -> None:
    """The margin's mark and dictation's bar come back with the screen."""
    back.wait_for_surfaces("margin", "dictation")


def test_desk_facts_names_the_screen_again_once_its_back(back: Screens) -> None:
    """Desk-facts names the screen again once it's back.

    bochs-drm reads the screen's EDID from QEMU at each probe, as a real display's driver does
    at a replug, so the screen comes back whole.
    """
    edid = f"{back.connectors['Virtual-1']}/edid"
    back.machine.log(f"EDID: {back.machine.succeed(f'wc -c < {edid}').strip()} bytes")
    back.displays_say(named=True)


def test_a_console_switch_away_and_back_keeps_them_all(switched: Screens) -> None:
    """A console switch away and back keeps them all."""
    switched.niri_answers("a console switch")
    switched.wait_for_surfaces(*EVERY_SURFACE)


def test_the_machine_sleeps_and_wakes_at_a_key_and_keeps_them_all(slept: Screens) -> None:
    """The machine sleeps and wakes at a key, and keeps them all."""
    said = slept.machine.succeed("journalctl --boot --identifier=systemd-sleep --output=cat")
    assert "System returned from sleep" in said, f"the machine never slept: {said}"
    slept.wait_for_surfaces(*EVERY_SURFACE)


# A surface drawn again says nothing of what it hears. Each of these goes
# by what the shell is told, over the bus, a socket or a unit's state, so
# a connection that died in the sleep and wasn't made again would leave it
# standing.


def test_after_the_wake_a_notice_still_arrives(slept: Screens) -> None:
    """After the wake, a notice still arrives."""
    slept.machine.succeed(slept.in_niri("notify-send 'After the wake' 'Heard over the bus'"))
    slept.machine.wait_until_succeeds(
        slept.in_niri(
            "journalctl --user --unit=quickshell --output=cat"
            " | grep -qF 'report[shows card After the wake]'"
        ),
        timeout=dt.timedelta(seconds=30),
    )


def test_after_the_wake_dictation_still_hears_its_keys(slept: Screens) -> None:
    """After the wake, dictation still hears its keys, and its bar goes when it's done."""
    assert slept.listening(), "dictation stopped listening in the sleep"
    slept.quiet()


def test_after_the_wake_keep_awake_let_go_takes_its_mark_away(slept: Screens) -> None:
    """After the wake, Keep Awake let go takes its mark away."""
    # With nothing else on, the margin goes with its last mark.
    slept.quiet()
    slept.machine.succeed(slept.in_niri("systemctl --user stop caffeinate.service"))
    slept.wait_for_no_surface("margin")


# Joel docks with two screens, and an undock can take either. The margin is
# a surface on each screen; dictation's bar is one, on the screen niri
# gives it, so the screen to take is the one it's on.


def test_on_two_screens_the_margin_is_on_each_and_dictation_on_one(apart: Apart) -> None:
    """On two screens, the margin shows on each and dictation's bar on one."""
    apart.screens.wait_for_held_on("margin", apart.screens.both)
    assert apart.screens.held_on("dictation") == {apart.listening}


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


def test_one_of_two_screens_goes_and_the_margin_and_bar_stay(one_gone: Apart) -> None:
    """One of two screens goes, and the margin and dictation's bar stay on the other."""
    one_gone.screens.wait_for_held_on("margin", one_gone.left)
    one_gone.screens.wait_for_held_on("dictation", one_gone.left)


def test_the_screen_comes_back_and_the_margin_is_on_both_again(one_back: Apart) -> None:
    """The screen comes back, and the margin is on both again."""
    one_back.screens.wait_for_held_on("margin", one_back.screens.both)
    # Still listening through it all, so the second press ends it. The shell remakes the bar as
    # the screens are put back, gone for under a second, so one look can land in that gap.
    one_back.screens.wait_for_surfaces("dictation")
    one_back.screens.quiet()


def test_the_launcher_opens_finds_and_runs_a_row_on_each_screen(unlocked: Screens) -> None:
    """The launcher opens, finds and runs a row on each of two screens."""
    unlocked.launcher_works_on_both()


def test_the_first_screen_goes_with_the_launcher_open_on_it(first_gone: Screens) -> None:
    """The first screen goes with the launcher open on it, and it works on the other."""
    first_gone.launcher_works_on("Virtual-2")


def test_the_first_screen_comes_back_and_the_launcher_works_on_both(first_back: Screens) -> None:
    """The first screen comes back, and the launcher works on both."""
    first_back.launcher_works_on_both()


def test_the_second_screen_goes_with_the_launcher_open_on_it(second_gone: Screens) -> None:
    """The second screen goes with the launcher open on it, and it works on the other."""
    second_gone.launcher_works_on("Virtual-1")


def test_the_second_screen_comes_back_and_the_launcher_works_on_both(
    second_back: Screens,
) -> None:
    """The second screen comes back, and the launcher works on both."""
    second_back.launcher_works_on_both()


def test_both_screens_go_with_the_launcher_open_and_come_back(second_back: Screens) -> None:
    """Both screens go with the launcher open, come back, and it works on both."""
    second_back.gone_while_open(*sorted(second_back.both))
    second_back.come_back(*sorted(second_back.both))
    second_back.launcher_works_on_both()


def test_a_sleep_with_the_launcher_open_and_it_works_after(slept_open: Screens) -> None:
    """A sleep with the launcher open, and after the wake it works on both screens."""
    slept_open.launcher_works_on_both()


def test_after_the_wake_the_launchers_foot_still_tells_the_time(slept_open: Screens) -> None:
    """After the wake, the launcher's foot still tells the time as it passes.

    A clock that stopped in the sleep would still say the minute it stopped at, so the foot
    must follow the minute after the wake.
    """
    woke = slept_open.minute()
    slept_open.machine.log(f"the foot says: {slept_open.launcher_shows()['glance']}")
    deadline = time.monotonic() + 90
    while (now := slept_open.minute()) == woke or now not in slept_open.launcher_shows()["glance"]:
        assert time.monotonic() < deadline, (
            f"the foot says {slept_open.launcher_shows()['glance']!r} at {now}"
        )
        time.sleep(1)


def test_quickshell_warned_of_nothing_as_the_screens_came_and_went(
    unlocked: Screens,
    slept_open: Screens,  # noqa: ARG001 # Asked for so a run alone takes the sleep too.
) -> None:
    """Quickshell warned of nothing as the launcher's screens came and went.

    A surface made where there's no screen, or a binding that throws or assigns the wrong type
    as a screen comes or goes, only warns.
    """
    warned = unlocked.machine.succeed(
        unlocked.in_niri(
            f"journalctl --user -u quickshell --since @{unlocked.launcher_since} --no-pager -o cat"
            " | grep -a -F WARN || true"
        )
    )
    assert warned.strip() == "", f"Quickshell warned:\n{warned}"

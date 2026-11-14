"""The screens test's machine, the steps it's taken through, and what reads it.

Every area of the screens test (tests/screens/) loads this as a plugin, a module of the
driver's Python that tests/screens.nix gives it. Each fixture takes the machine one step on
from the one it asks for: signed in with the lid shut since it started, the lid opened, a
notice and the margin's marks up, the screens gone and back, the console switched, a sleep,
then two screens apart, or the launcher through the screens going and a sleep. A fixture's
work is done once a run, however many tests ask for it, and only for the tests that ask, so
each area takes only the steps its tests need, and any test runs alone with `-k`. The
account greetd signs in is screens.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
import functools
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

# The screen that stands in for the laptop's own panel, which its stand-in lid shuts, as
# screens.nix names it.
LAPTOP = "Virtual-2"

# What the account's password is, as screens.nix sets it, which lets the lock go after a sleep.
PASSWORD = "correct-horse"  # noqa: S105 (a test machine's, said in screens.nix)


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
        # The screens niri said it connected while the lid was shut from the start, set by
        # that fixture before the lid opens.
        self.connected_shut: list[str] = []

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

    def lid(self, *, shut: bool) -> None:
        """Shut the stand-in lid, or open it."""
        self.machine.succeed(f"echo {int(shut)} > /sys/module/stand_in_lid/parameters/shut")

    def lid_shut(self) -> bool:
        """Say whether logind reads the lid as shut."""
        said = self.machine.succeed(
            "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
            " org.freedesktop.login1.Manager LidClosed"
        )
        return said.strip() == "b true"

    def niri_said(self, words: str) -> str:
        """Give the lines niri has written this boot holding these words, its colours taken out."""
        said = self.machine.succeed(
            f"journalctl -b -o cat | grep -a -F {shlex.quote(words)} || true"
        )
        return re.sub(r"\x1b\[[0-9;]*m", "", said)

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
    # it the keys, typing finds a row, Return runs it, and :q closes it, from
    # the normal mode Escape gives, as Escape never closes. The row is Keep
    # Awake, whose unit says it ran; each run turns it the other way, so it's
    # typed by the name it has now.

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

    def close_launcher(self, on: str = "") -> None:
        """Press Escape until normal is at rest, then close the launcher by :q, as Joel does.

        Escape gives normal from the find, lets the key list go, and cancels a count or a key
        begun, but never closes (Joel, 2026-11-05: "fuck escape quitting"), so :q does, from
        any depth. A key sent before this that the launcher hears late moves it on too, so each
        step waits only for something to change. Where it's closed, such as on which screen,
        goes into what a wait that runs out says.
        """
        said = f" {on}" if on else ""

        def where(shown: dict[str, Any]) -> tuple[object, ...]:
            return (
                shown["open"],
                shown["mode"],
                len(shown["held"]),
                bool(shown.get("listing") or shown.get("command") or shown.get("operator")),
                bool(shown.get("counted") or shown.get("begun")),
            )

        def moved(was: tuple[object, ...], shown: dict[str, Any]) -> bool:
            return where(shown) != was

        for _ in range(12):
            was = where(self.launcher_shows())
            if not was[0]:
                return
            _, mode, _, over, begun = was
            if mode == "normal" and not (over or begun):
                self.machine.send_chars(":q\n")
            else:
                self.machine.send_key("esc")
            self.launcher_until(functools.partial(moved, was), f"moved on toward :q{said}")
        pytest.fail(f"the launcher was still open after 12 Escapes{said}: {self.launcher_shows()}")

    def awake(self) -> bool:
        """Say whether Keep Awake is on."""
        status, _ = self.machine.execute(
            self.in_niri("systemctl --user is-active --quiet keep-awake.service")
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
        typed, row = ("let it sleep", "Keep Awake") if was else ("keep awake", "Keep Awake")
        self.super_space_on(screen)
        self.machine.send_chars(typed)
        self.launcher_until(
            lambda shown: shown["typed"] == typed and (shown["selected"] or {}).get("name") == row,
            f"found {row!r} on {screen}",
        )
        # Keep Awake is a setting, which Return in normal does and stays to show.
        # Its acts lie on its row, so Ctrl+] no longer steps into it (#4136).
        self.machine.send_key("esc")
        self.launcher_until(lambda shown: shown["mode"] == "normal", f"normal on {screen}")
        self.machine.send_key("ret")
        deadline = time.monotonic() + 30
        while self.awake() == was:
            assert time.monotonic() < deadline, f"{row!r} did nothing on {screen}"
            time.sleep(0.5)
        self.close_launcher(f"on {screen}")
        self.super_space_on(screen)
        self.close_launcher(f"on {screen}")

    def launcher_works_on_both(self) -> None:
        """Have the launcher open, find and run a row on each screen, in their names' order."""
        for screen in sorted(self.both):
            self.launcher_works_on(screen)

    def line_works_on(self, screen: str) -> None:
        """Bring Super+S's line up on a screen by its chord, holding the keys, and put it away."""
        self.machine.succeed(self.in_niri(f"niri msg action focus-monitor {screen}"))
        self.machine.wait_until_succeeds(
            self.in_niri(f"niri msg --json focused-output | grep -qF '\"{screen}\"'"),
            timeout=dt.timedelta(seconds=10),
        )
        self.machine.send_key("meta_l-s")
        self.launcher_until(
            lambda shown: shown["line"]["up"] and shown["line"]["keys"],
            f"brought the line up with the keys on {screen}",
        )
        self.wait_for_held_on("talk-line", {screen}, timeout=dt.timedelta(seconds=10))
        self.machine.send_key("meta_l-s")
        self.launcher_until(lambda shown: not shown["line"]["up"], f"put the line away on {screen}")

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

    def hand_over(self, where: dict[str, dict[str, Any]]) -> str:
        """Keep places for stand-in sessions from a boot before, and give this boot's id.

        Each stays open (pi-stays) until `handed_back`. A first zsh makes its completions
        first, which on a busy machine can take longer than Reopen Them All waits for a window.
        """
        self.machine.wait_until_succeeds(
            self.in_niri("ls -d ~/.local/state/quickshell/by-shell/*"),
            timeout=dt.timedelta(seconds=60),
        )
        self.machine.succeed(self.in_niri("zsh -lic true"), timeout=dt.timedelta(seconds=300))
        self.machine.succeed(self.in_niri("touch $XDG_RUNTIME_DIR/pi-stays"))
        sessions = [{"id": one, "cwd": f"/home/{self.account}"} for one in where]
        self.keep(
            "~/.local/state/pi/restore.json",
            {"v": 1, "boot": "a-boot-before", "sessions": sessions},
        )
        self.keep(
            "~/.local/state/agents/arrangement.json",
            {"v": 1, "boots": {"a-boot-before": where}},
        )
        return self.machine.succeed("cat /proc/sys/kernel/random/boot_id").strip()

    def reopen(self, where: dict[str, dict[str, Any]]) -> None:
        """Hand the shell the sessions kept, to reopen as Reopen Them All does, and see it."""
        # With the focus, as Joel's own press reopens them: only that puts
        # each in its column, which needs the column focused to move it.
        sessions = [{"id": one, "cwd": f"/home/{self.account}", "focus": True} for one in where]
        handed = shlex.quote(json.dumps({"sessions": sessions}))
        taken = self.machine.succeed(self.in_niri(f"qs ipc call sessions reopen {handed}"))
        assert taken.strip() == "taken", taken

    def handed_back(self, boot: str) -> None:
        """Close the stand-in sessions, and leave nothing kept to reopen."""
        # Its own pattern bracketed, so pkill doesn't match the shell it runs in.
        self.machine.succeed(
            self.in_niri("rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'; true")
        )
        self.keep("~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})

    def keep(self, path: str, read: dict[str, Any]) -> None:
        """Write a file the agents keep, whole, as they do."""
        written = (
            f"mkdir -p $(dirname {path}) && printf %s {shlex.quote(json.dumps(read))} > {path}.q"
            f" && mv {path}.q {path}"
        )
        self.machine.succeed(self.in_niri(written))

    def sessions_sat(self, prefix: str) -> dict[str, tuple[str, int, int]]:
        """Give each stand-in session's screen, workspace and column, by its session."""
        listed = json.loads(self.machine.succeed(self.in_niri("niri msg --json workspaces")))
        spaces = {one["id"]: one for one in listed}
        windows = json.loads(self.machine.succeed(self.in_niri("niri msg --json windows")))
        return {
            # "pi --session <id>", then what it was told, if anything.
            window["title"].split()[2]: (
                spaces[window["workspace_id"]]["output"],
                spaces[window["workspace_id"]]["idx"],
                window["layout"]["pos_in_scrolling_layout"][0],
            )
            for window in windows
            if window["title"].startswith(f"pi --session {prefix}")
        }


@dataclass(frozen=True)
class Apart:
    """The two screens with dictation listening on one of them."""

    screens: Screens
    # The screen dictation's bar is on, and the other.
    listening: str
    left: set[str]


@pytest.fixture(scope="module")
def shut_at_start(machine: BaseMachine, values: dict[str, Any]) -> Screens:
    """Give the machine signed in with its lid shut since it started, the desktop up."""
    # The test's one machine is QEMU's, which takes keys and screenshots.
    qemu = cast("QemuMachine", machine)
    qemu.wait_for_unit("multi-user.target")
    screens = Screens(qemu, values["account"])
    screens.wait_for_surfaces("wallpaper", "launcher", timeout=dt.timedelta(seconds=300))
    assert sorted(screens.connectors) == ["Virtual-1", "Virtual-2"], (
        f"the screens are {screens.connectors}"
    )
    assert screens.lid_shut(), "logind never read the stand-in lid as shut"
    screens.connected_shut = re.findall(
        r"tty: connecting connector: (\S+)$",
        screens.niri_said("connecting connector: "),
        re.MULTILINE,
    )
    return screens


@pytest.fixture(scope="module")
def booted(shut_at_start: Screens) -> Screens:
    """Give the machine signed in with its lid open, the wallpaper on both its screens."""
    shut_at_start.lid(shut=False)
    shut_at_start.outputs_say(LAPTOP, there=True)
    shut_at_start.wait_for_held_on("wallpaper", shut_at_start.both)
    return shut_at_start


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
    noticed.machine.succeed(noticed.in_niri("systemctl --user start keep-awake.service"))
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
    slept.machine.succeed(slept.in_niri("systemctl --user start keep-awake.service"))
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
    # the keys if it is, and close at its :q.
    second_back.launcher_until(lambda shown: not shown["open"] or shown["keys"], "held the keys")
    second_back.close_launcher("after the wake")
    return second_back

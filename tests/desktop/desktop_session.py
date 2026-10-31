"""The desktop every area of the desktop test drives: its boot, and the helpers each area shares.

tests/desktop.nix gives this to the driver as a module of its Python, and its opening comment
says what the test proves. Each area asks for a `Desktop` as a fixture, made once a run by
`started`. A helper two areas use comes here, and nothing here is a test.
"""

from __future__ import annotations

import contextlib
import json
import re
import shlex
import tempfile
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypedDict, cast

import test_driver.machine
import vm_helpers
from PIL import Image, ImageStat

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine, QemuMachine


class Values(TypedDict):
    """What tests/desktop.nix gives the test: the account, the theme and paths in the store."""

    # The account the desktop runs as, and its password.
    account: str
    secret: str
    # The programs tests/desktop.nix writes for the test to run inside the machine.
    asking: str
    ask_portal: str
    choose_a_file: str
    copilot_key: str
    keyboard: str
    touchscreen: str
    tuple_window: str
    # A picture, a document, a text and a track to open, as a download would be.
    picture: str
    page: str
    text: str
    track: str
    # The packages whose programs the test runs by their paths.
    coreutils: str
    dbus: str
    ffmpeg_headless: str
    fontconfig_bin: str
    pipewire: str
    stress_ng: str
    wev: str
    zenity: str
    # The theme's colours the test looks for, and its monospaced face.
    accent: str
    action: str
    sunken: str
    mono_family: str
    # Whether this is the gallery, which keeps pictures for a person to look at.
    photographs: bool


# A JSON object as niri or the shell gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

# A picture holds more colours than this, and a flat colour, or a screen not yet drawn, far
# fewer.
PICTURE_COLOURS = 1000
# The stand-in panel's brightness above which it has followed a brighter room up from the
# dim screen picked in a dim one.
FOLLOWED_THE_ROOM = 15
# How far a pixel can be from the action's colour, summed over red, green and blue, and
# still be its glyph's.
NEAR_THE_ACTION = 40
# How wide and how high one glyph is at most, in pixels.
ONE_GLYPH = 30
# How far in from the screen's right edge, in pixels, the window under the notices rests,
# so its edge is far from the corner's.
FROM_THE_CORNER = 100
# The longest a program may take to start and show its window. A wait
# ends when it does, so this is only a ceiling: on a host busy with
# other machines and builds, a cold kitty or GIMP took past 30
# seconds, and a wait on a program starting fails then, not the step.
STARTING = timedelta(seconds=120)
# How far apart typed keys go. QEMU's virtio keyboard holds about 64 events, four a key,
# and drops what comes while it's full; at the driver's 10 ms a key, a guest starved
# for 180 ms on a loaded builder lost the middle of "quarterly numbers", leaving "qs",
# while its libinput said it was lagging behind (2026-10-30). 50 ms a key holds a stall
# that long.
KEY_PACE = timedelta(milliseconds=50)
# More rows than this name something across everything the launcher is asked for in its
# survey, so the rows were looked at rather than all empty.
ROWS_LOOKED_AT = 100
# The same for the Wi-Fi sign-in window, which starts GTK and WebKit cold.
SIGNING_IN = timedelta(seconds=180)
# How far in, in pixels, from the screen's edges and what niri keeps clear along them, a
# window set in a corner rests at most.
FROM_THE_EDGE = 48
# The share of the screen's width a window set wide takes at least.
NEARLY_ALL = 0.9
# CAP_SYS_NICE's bit among a process's capabilities.
CAP_SYS_NICE = 23

# The launcher's surface is always listed, so whether it's open is what it says of itself.
LAUNCHER_OPEN = """qs ipc call launcher showing | grep -q '"open":true'"""
# The window the launcher's window subtests find by its title.
NOTES = "Notes for Tuesday"

# A session as pi keeps one: its quest's README, front matter first, and
# its presence while it runs; and its window, titled as pi titles its own:
# its address, its quest cut short, its folder, which a hyphen doesn't end.
PI_QUEST = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-TESTQA"
PI_PRESENCE = "~/.local/state/pi/agentic-harness.pi/mesh-workflow/presence"
PI_WINDOW = "\u03c0 - TESTQA A quest of\u2026 - my-tree"

# The margin is there only while something's on, each thing its glyph
# alone, and what each is, in words, is the launcher's What's on: Super+O
# or a click on a glyph opens it, Return does a thing's first action and
# the right arrow steps into the rest.
MARGIN = "niri msg layers | grep -q margin"
# The margin's cells: the theme's gap, open or shut, since nothing grows.
CELL = 12
NOTICES = "niri msg layers | grep -q notifications"

# Tuple's log, which only a call writes, and which the daemon rewrites as
# it starts.
TUPLE_LOG = "~/.local/share/tuple/0/log.txt"

# Where keyboard-lights says whether caps lock is on.
CAPITALS = "/run/keyboard-lights/capslock"

# What ssh-tpm-agent asks before Joel's key signs.
SIGNING = (
    "Allow use of key joel-signing?\\nKey fingerprint SHA256:abc."
    "\\nRequested by ssh-keygen (1234) \u2190 tpm-sign \u2190 git \u2190 zsh"
)


def retry(test: Callable[[bool], bool], timeout: timedelta) -> None:
    """Call the test each second until it says yes or the time is up, through the driver's retry.

    It's looked up as it's called, so the paced retry `pace_the_driver` puts in its place is
    the one used.
    """
    test_driver.machine.retry(test, timeout=timeout)


def margin_sets(*wanted: tuple[str, str, str]) -> Callable[[str], bool]:
    """Test what the margin says it draws: each glyph's mark, kind and colour, left to right."""

    def sets(rest: str) -> bool:
        drawn = [(glyph["mark"], glyph["kind"], glyph["tone"]) for glyph in json.loads(rest)]
        return drawn == list(wanted)

    return sets


def margin_points(pointed: str) -> Callable[[str], bool]:
    """Test the margin's cells stay the gap's size, and only the pointed mark wears the accent.

    While What's on is open, the mark of the row the keys are on wears the accent, and
    nothing grows. An empty pointed means none does.
    """

    def points(rest: str) -> bool:
        drawn = json.loads(rest)
        return bool(drawn) and all(
            glyph["cell"] == CELL and (glyph["hue"] == "accent") == (glyph["mark"] == pointed)
            for glyph in drawn
        )

    return points


def drawn_at(window: Json) -> tuple[int, int, int, int]:
    """Give where a floating window draws on the screen: left, top, width, height."""
    layout = window["layout"]
    left = int(layout["tile_pos_in_workspace_view"][0] + layout["window_offset_in_tile"][0])
    top = int(layout["tile_pos_in_workspace_view"][1] + layout["window_offset_in_tile"][1])
    width, height = layout["window_size"]
    return left, top, width, height


def settles_on(
    wanted: str, read: Callable[[], str], timeout: timedelta = timedelta(seconds=15)
) -> str | None:
    """Give what a reading of the screen settles on.

    It's read until it gives the colour wanted, since niri moves a window to where it says it
    is over a few frames, and the last reading is given if it never does. A reading niri can't
    give yet, a window not yet floating, reads as nothing.
    """
    readings: list[str | None] = [None]

    def settled(_: bool) -> bool:
        try:
            readings.append(read())
        except KeyError, TypeError:
            readings.append(None)
        return readings[-1] == wanted

    with contextlib.suppress(Exception):
        retry(settled, timeout=timeout)
    return readings[-1]


def changed_at_the_top(before: Image.Image, after: Image.Image) -> tuple[float, float]:
    """Give the middle of what changed along the screen's top, as a fraction of it.

    That's a glyph the margin drew, at the top centre, and its shade, which falls round it.
    """
    width, height = after.size
    was, now = before.tobytes(), after.tobytes()

    def moved(x: int, y: int) -> int:
        at = 3 * (y * width + x)
        return sum(abs(was[at + c] - now[at + c]) for c in range(3))

    top = [
        (x, y) for y in range(height // 20) for x in range(width) if moved(x, y) > NEAR_THE_ACTION
    ]
    assert top, "nothing changed along the top"
    xs, ys = [x for x, _ in top], [y for _, y in top]
    return (min(xs) + max(xs)) / 2 / width, (min(ys) + max(ys)) / 2 / height


def started(machine: BaseMachine, values: Values) -> Desktop:
    """Give the desktop on a test's machine, started, with every wait paced to the host.

    An area written for pytest makes its `desktop` fixture of this, once a run. A busy host is
    slower at everything, so every wait allows for how busy it is as it starts.
    """
    assert isinstance(machine, test_driver.machine.QemuMachine), "the desktop is QEMU's"
    desktop = Desktop(machine, values)
    # The machines wait through the driver's machine module's retry, which this replaces; the
    # globals a script would have had are only somewhere to put the copy it also makes.
    vm_helpers.pace_the_driver({}, describe=desktop.described)
    print(
        f"waits allow {vm_helpers.pace(vm_helpers.busyness()):.1f} times as long as they say,"
        " for now"
    )
    desktop.start()
    return desktop


class Desktop:  # The helpers every area shares, as one machine's desktop.
    """The account's desktop on the test's machine, and what every area does with it."""

    def __init__(self, machine: QemuMachine, values: Values) -> None:
        """Drive the desktop on that machine, with what tests/desktop.nix gave the test.

        It's QEMU's, since the desktop is seen on its screen and typed at by its keys.
        """
        self.machine = machine
        self.values = values
        self.account = values["account"]
        # Print opens the launcher's Capture collection, where a picture of the
        # screen, a window or a region is chosen by the keys, and niri says
        # when it's taken.
        self.pictures = f"/home/{self.account}/Pictures/Screenshots"

    def start(self) -> None:
        """Start niri's session and the shell, and wait until the launcher answers.

        Every area past the first few steps needs the shell, so it starts with the session,
        and any area's test can run alone.
        """
        machine = self.machine
        machine.wait_for_unit("multi-user.target")
        # niri builds its software renderer before it opens its socket, which on a
        # busy host has taken past two minutes.
        machine.wait_until_succeeds("ls /run/user/1000/niri.*.sock", timeout=STARTING)
        self.as_account(
            "NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg action spawn --"
            " systemctl --user import-environment WAYLAND_DISPLAY NIRI_SOCKET"
        )
        machine.wait_until_succeeds(
            self.in_session("systemctl --user show-environment | grep -q WAYLAND_DISPLAY")
        )
        # niri sets these as a session, then starts it.
        self.as_account(
            "systemctl --user set-environment XDG_CURRENT_DESKTOP=niri XDG_SESSION_TYPE=wayland"
        )
        self.as_account("systemctl --user start compositor")
        self.as_account("systemctl --user start quickshell")
        machine.wait_until_succeeds(
            self.in_session("qs ipc show | grep -q launcher"), timeout=STARTING
        )

    def in_session(self, command: str) -> str:
        """Give a command as the account, inside its niri session, where qs finds the shell."""
        environment = (
            "export XDG_RUNTIME_DIR=/run/user/1000; "
            "export $(systemctl --user show-environment"
            " | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)='); "
        )
        return f"su - {self.account} -c " + shlex.quote(environment + command)

    def as_account(self, command: str) -> str:
        """Run a command as the account, inside its session, and give what it printed."""
        return self.machine.succeed(self.in_session(command))

    def shell_said(self, *words: str) -> str:
        """Give which of these words the shell's journal holds, one a line.

        It's searched in the machine: by the end of a run the whole journal is
        megabytes, and a busy host has hung the driver bringing it back.
        """
        wanted = " ".join(f"-e {shlex.quote(word)}" for word in words)
        return self.as_account(
            "journalctl --user -u quickshell --no-pager -o cat"
            f" | grep -o -F {wanted} | sort -u; true"
        )

    def brightness(self) -> int:
        """Give the stand-in panel's brightness."""
        return int(self.machine.succeed("cat /sys/class/backlight/stand-in/brightness"))

    # The kernel's stand-in battery has two supplies, mains and USB, and UPower
    # counts the machine on battery only with both off.
    def power(self, state: str) -> None:
        """Turn both of the stand-in's supplies on or off."""
        parameters = "/sys/module/test_power/parameters"
        for supply in ["ac", "usb"]:
            self.machine.succeed(f"echo {state} > {parameters}/{supply}_online")

    def wait_for_surface(self, namespace: str) -> None:
        """Wait until niri lists one of the shell's surfaces by its namespace."""
        self.machine.wait_until_succeeds(
            self.in_session(f"niri msg layers | grep -q {namespace}"),
            timeout=timedelta(seconds=30),
        )

    # The shell reports what it does to its journal (Report.qml), so the test
    # waits on what happened, never on a guess at how long it takes. A point
    # in the journal is marked before the step that should bring a report,
    # and only what's said after it counts, so a report made before the wait
    # begins still ends it.
    def mark(self) -> str:
        """Mark the point the shell's journal has reached."""
        said = self.as_account(
            "journalctl --user --unit=quickshell --lines=1 --show-cursor --output=cat"
        )
        return said.rsplit("-- cursor: ", 1)[1].strip()

    def reported(
        self,
        prefix: str,
        test: Callable[[str], object],
        since: str,
        timeout: timedelta = timedelta(seconds=30),
    ) -> None:
        """Wait until something said after a point starts with the prefix and passes the test.

        The point is one mark gave, or "" for the boot. What the shell did say is printed when
        it never says what's wanted.
        """
        pace = vm_helpers.pace(vm_helpers.busyness())
        deadline = time.monotonic() + timeout.total_seconds() * pace
        start = since

        def never() -> str:
            point = f"--after-cursor={shlex.quote(start)}" if start else "--boot"
            print(
                self.as_account(
                    f"journalctl --user --unit=quickshell --no-pager --output=cat {point}"
                )
            )
            return f"the shell never reported {prefix!r} as wanted"

        while True:
            left = int(deadline - time.monotonic())
            assert left > 0, never()
            status, said = self.machine.execute(
                self.in_session(f"reports {shlex.quote(since)} {left} {shlex.quote(prefix)}"),
                timeout=timedelta(seconds=left + 30),
            )
            assert status == 0, never()
            for line in said.splitlines():
                kind, what = line.split("\t", 1)
                if kind == "cursor":
                    since = what
                elif what.startswith(prefix) and test(what[len(prefix) :]):
                    return

    @contextlib.contextmanager
    def told(self, what: str, timeout: timedelta = timedelta(seconds=30)) -> Iterator[None]:
        """Wait, after what's done inside it, for the shell to say exactly this."""
        since = self.mark()
        yield
        self.reported(what, lambda rest: rest == "", since, timeout)

    def held_still(self, timeout: timedelta = timedelta(seconds=10), span: float = 0.5) -> bool:
        """Say whether the screen holds still for half a second before the time is up.

        Half a second is twice the theme's longest movement: niri composes a frame a moment
        after it's asked to, windows and surfaces move as they come, and a busy host can draw
        two frames in a row the same mid-motion.
        """
        pace = vm_helpers.pace(vm_helpers.busyness())
        deadline = time.monotonic() + timeout.total_seconds() * pace
        with tempfile.TemporaryDirectory() as folder:
            shot = Path(folder) / "shot.ppm"
            last: bytes | None = None
            since = time.monotonic()
            while time.monotonic() < deadline:
                self.machine.send_monitor_command(f"screendump {shot}")
                now = shot.read_bytes()
                if now != last:
                    last, since = now, time.monotonic()
                elif time.monotonic() - since >= span:
                    return True
        return False

    def photograph(self, name: str, timeout: timedelta = timedelta(seconds=10)) -> None:
        """Photograph the screen once it holds still, for a person to look at, in the gallery.

        One that never holds still is kept as it is. Outside the gallery it takes nothing and
        waits for nothing, so a test must never lean on it to let the screen settle.
        """
        if self.values["photographs"]:
            self.held_still(timeout)
            self.machine.screenshot(name)

    def screen_now(self) -> Image.Image:
        """Give the screen as it is, from QEMU's own picture of it."""
        with tempfile.TemporaryDirectory() as folder:
            shot = Path(folder) / "shot.ppm"
            self.machine.send_monitor_command(f"screendump {shot}")
            return Image.open(shot).convert("RGB")

    def settled_screen(self, name: str, timeout: timedelta = timedelta(seconds=10)) -> Image.Image:
        """Give the screen once it holds still, or as it is when it never does, to measure.

        The gallery keeps it too, by its name, for a person to look at.
        """
        self.held_still(timeout)
        image = self.screen_now()
        if self.values["photographs"]:
            image.save(self.machine.out_dir / f"{name}.png")
        return image

    def colours_and_light(
        self, picture: str, part: tuple[float, float, float, float] = (0, 0, 1, 1)
    ) -> tuple[Image.Image, int, float]:
        """Take a picture of the screen, and give how many colours a part holds and how bright.

        A flat colour holds one, and the light is from 0 to 255. The part is a box of fractions
        of the screen: left, top, right and bottom.
        """
        self.machine.screenshot(picture)
        image = Image.open(self.machine.out_dir / f"{picture}.png").convert("RGB")
        w, h = image.size
        box = image.crop((int(part[0] * w), int(part[1] * h), int(part[2] * w), int(part[3] * h)))
        counts = box.getcolors(box.width * box.height)
        assert counts is not None
        return image, len(counts), ImageStat.Stat(box.convert("L")).mean[0]

    def showing(self) -> Json:
        """Give what the launcher shows, from its showing call.

        That's whether it holds the keys, what's typed and held, and the row Return takes.
        Under load the launcher lags the keys sent to it, so each step waits on this.
        """
        return cast("Json", json.loads(self.as_account("qs ipc call launcher showing")))

    def launcher_until(
        self, test: Callable[[Json], object], timeout: timedelta = timedelta(seconds=30)
    ) -> None:
        """Wait until what the launcher shows passes the test.

        The launcher reports its state each time it changes, so a wait reads it once and then
        hears every change.
        """
        since = self.mark()
        if not test(self.showing()):
            self.reported("showing ", lambda said: test(json.loads(said)), since, timeout)

    def launcher_ready(self) -> None:
        """Wait until the launcher is open and holds the keys.

        niri hands a surface the keys a frame after it asks for them, and a letter typed in
        that frame goes to the window underneath.
        """
        self.launcher_until(lambda shown: shown["open"] and shown["keys"])

    def launcher_left_closed(self) -> None:
        """Close the launcher if a test left it open, so the next test starts as every test does.

        A test failing with it open handed it open to every test after it, and each failed
        on what the first left (2026-10-30: one failure in goes-places took 31 more with it).
        """
        left_open, _ = self.machine.execute(self.in_session(LAUNCHER_OPEN))
        if left_open != 0:
            return
        print("the launcher was left open, so it's closed for the next test")
        self.as_account("qs ipc call launcher toggle")
        self.machine.wait_until_fails(self.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

    def type_in(self, text: str, selecting: str = "") -> None:
        """Type into the launcher, and wait until it holds every letter with Return's row settled.

        An answer still being looked up closes the launcher without copying. A row that comes
        from something found later, such as a window or a copy, can be named to wait for too.
        """
        self.machine.send_chars(text, delay=KEY_PACE)

        def holds(shown: Json) -> object:
            row = shown["selected"]
            name = row["name"].lower() if row else ""
            settled = row is None or row["settled"]
            return shown["typed"].endswith(text) and settled and selecting.lower() in name

        self.launcher_until(holds)

    def answer(self, typed: str) -> None:
        """Open the launcher, and ask it what's typed.

        It waits for the launcher to be closed first, since opening is a toggle: an Escape
        sent just before that a loaded guest hears late would otherwise land after the toggle
        had closed the launcher it was meant to close, and nothing would open.
        """
        self.launcher_until(lambda shown: not shown["open"])
        self.as_account("qs ipc call launcher toggle")
        self.launcher_ready()
        self.type_in(typed)

    def step_in(self, key: str = "right") -> None:
        """Press a key that steps into the selected row, and wait until the launcher holds it.

        That's the right arrow into a thing's verbs, or Return into what a verb asks.
        """
        before = len(self.showing()["held"])
        self.machine.send_key(key)
        self.launcher_until(lambda shown: len(shown["held"]) > before)

    def down_to(self, name: str) -> None:
        """Move the selection to the row named, and wait until it's there.

        Rows are counted by name, since what leads a step (Return's own act) moves the rest,
        and from the row selected, which a step in from a match keeps rather than the first.
        """
        shown = self.showing()
        rows = shown["rows"]
        selected = (shown["selected"] or {}).get("name")
        steps = rows.index(name) - (rows.index(selected) if selected in rows else 0)
        for _ in range(abs(steps)):
            self.machine.send_key("down" if steps > 0 else "up")
        self.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == name)

    def whats_on(self, selected: str) -> None:
        """Wait until the launcher holds Now and the keys, with this thing's row selected."""
        self.launcher_until(
            lambda shown: (
                shown["open"]
                and shown["keys"]
                and shown["held"] == ["now"]
                and (shown["selected"] or {}).get("name") == selected
            )
        )

    def windows(self) -> list[Json]:
        """Give every window niri lists."""
        return cast("list[Json]", json.loads(self.as_account("niri msg --json windows")))

    def window_titled(self, title: str) -> Json:
        """Give the window niri lists under this title."""
        return next(window for window in self.windows() if window["title"] == title)

    def opened(self, title: str, program: str = "") -> Json:
        """Open a kitty with this title, running this program or its shell, and give its window.

        It's given once niri lists it.
        """
        self.as_account(f"niri msg action spawn -- kitty --title '{title}' {program}")
        self.machine.wait_until_succeeds(
            self.in_session(f'niri msg --json windows | grep -qF \'"title":"{title}"\''),
            timeout=STARTING,
        )
        return self.window_titled(title)

    def close(self, window: Json) -> None:
        """Close a window niri listed."""
        self.as_account(f"niri msg action close-window --id {window['id']}")

    def focused_kitty(self) -> Json:
        """Give a kitty window, focused, opening one first if there's none."""
        kitties = [window for window in self.windows() if window["app_id"] == "kitty"]
        if not kitties:
            self.as_account("niri msg action spawn -- kitty")
            self.machine.wait_until_succeeds(
                self.in_session('niri msg --json windows | grep -qF \'"app_id":"kitty"\''),
                timeout=STARTING,
            )
            kitties = [window for window in self.windows() if window["app_id"] == "kitty"]
        self.as_account(f"niri msg action focus-window --id {kitties[0]['id']}")
        return kitties[0]

    def notes_window(self) -> Json:
        """Give the notes' window, focused, opening it first if it isn't open."""
        if not any(window["title"] == NOTES for window in self.windows()):
            self.as_account(f"niri msg action spawn -- kitty --title '{NOTES}'")
            self.machine.wait_until_succeeds(
                self.in_session(f"niri msg windows | grep -q '{NOTES}'"), timeout=STARTING
            )
        notes = self.window_titled(NOTES)
        self.as_account(f"niri msg action focus-window --id {notes['id']}")
        return notes

    def commonest_colour_in(self, title: str, picture: str, wanted: str) -> str | None:
        """Give the commonest colour inside a window, a little in from its edges, once wanted.

        niri says where a window is only once it floats, so it floats first. The first GTK
        window has taken over 30 seconds to open on a CI runner building the kernel beside the
        test, so this waits longer than the rest.
        """
        self.machine.wait_until_succeeds(
            self.in_session(f'niri msg --json windows | grep -qF \'"title":"{title}"\''),
            timeout=timedelta(seconds=120),
        )
        self.as_account(
            f"niri msg action move-window-to-floating --id {self.window_titled(title)['id']}"
        )

        def commonest() -> str:
            image = self.settled_screen(picture)
            left, top, width, height = drawn_at(self.window_titled(title))
            inside = image.crop((left + 20, top + 20, left + width - 20, top + height - 20))
            # Pillow gives no counts when there are more colours than asked
            # for, and there can't be more than there are pixels.
            counts = inside.getcolors(inside.width * inside.height)
            assert counts is not None
            _, colour = max(counts)
            # An RGB picture's colours are each a red, a green and a blue.
            return "#{:02x}{:02x}{:02x}".format(*cast("tuple[int, int, int]", colour))

        colour = settles_on(wanted, commonest)
        self.close(self.window_titled(title))
        return colour

    def described(self) -> None:
        """Print the windows and surfaces niri shows, for a wait that failed to say what was.

        It asks without a wait of its own, and says nothing more when niri can't answer.
        """
        _, shown = self.machine.execute(
            self.in_session("niri msg windows; niri msg layers"), timeout=timedelta(seconds=30)
        )
        print(f"niri shows:\n{shown}")

    def meter_shows(self, namespace: str, kind: str, change: str) -> None:
        """Make a change the shell is sure to hear, and photograph the meter that answers it.

        Quickshell hears a node change only once it has bound the node's audio, a moment after
        WirePlumber names it the default, which the meter says, so one change is made after
        that and its meter answers it.
        """
        said = self.as_account(f"wpctl inspect @DEFAULT_AUDIO_{kind}@")
        found = re.search(r'node\.name = "([^"]+)"', said)
        assert found is not None, f"no default {kind.lower()} has a name"
        node = found.group(1)
        self.reported(
            f"hears {namespace} {node}", lambda rest: rest == "", "", timeout=timedelta(seconds=60)
        )
        with self.told(f"drawn {namespace}"):
            self.as_account(change)
        self.photograph(namespace)

    def pi_window(self, address: str, title: str) -> None:
        """Open a window titled so whose program stands in for a pi session's, as pi runs in kitty.

        The program writes its own pid and the kitty window it's in, then sleeps until its
        window closes, so the session's record names a process running while the window is
        open (pi_record).
        """
        name = address.lower()
        owner = f"$XDG_RUNTIME_DIR/{name}-owner"
        self.as_account(
            f"rm -f {owner} && printf '%s\\n' '#!/bin/sh'"
            f" 'echo \"$$ $KITTY_LISTEN_ON $KITTY_WINDOW_ID\" > {owner}.new'"
            f" 'mv {owner}.new {owner}'"
            f" 'exec sleep infinity' > ~/.{name}-pi && chmod +x ~/.{name}-pi"
            f" && echo {shlex.quote(title)} > $XDG_RUNTIME_DIR/{name}-title"
        )
        self.opened(title, f"~/.{name}-pi")
        self.machine.wait_until_succeeds(self.in_session(f"test -s {owner}"), timeout=STARTING)

    def pi_record(self, address: str) -> Json:
        """Give what a session's record says of its process and window, as pi's presence does.

        That's the process pi_window's program wrote, on this boot, since its start, the kitty
        window it's in, and the window's title as its label.
        """
        name = address.lower()
        pid, scope, window = self.as_account(f"cat $XDG_RUNTIME_DIR/{name}-owner").split()
        title = self.as_account(f"cat $XDG_RUNTIME_DIR/{name}-title").strip()
        ticks = self.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{pid}/stat")
        return {
            "title": title,
            "owner": {
                "pid": int(pid),
                "bootId": self.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
                "startTicks": int(ticks),
                "startedAt": int(self.as_account("date +%s%3N")),
            },
            "terminal": {
                "driverId": "kitty",
                "kind": "kitty-window",
                "value": window,
                "scope": scope,
            },
        }

    def presence_says(self, said: Json, address: str = "TESTQA") -> None:
        """Replace a session's presence with this, by rename, as pi does, with its process."""
        name = address.lower()
        whole = {**self.pi_record(address), **said}
        self.as_account(
            f"echo '{json.dumps(whole)}' > {PI_PRESENCE}/.{name}"
            f" && mv {PI_PRESENCE}/.{name} {PI_PRESENCE}/{name}.json"
        )

    def pi_session(self) -> None:
        """Make pi's session, working, with its window and another to be in, unless there.

        Each of pi's tests starts from it, so any of them runs alone.
        """
        self.as_account(
            f"mkdir -p {PI_QUEST} {PI_PRESENCE} && printf '%s\\n' '---' 'title: no' '---'"
            f" '# A quest of its own & whole' > {PI_QUEST}/README.md"
        )
        titles = {window["title"] for window in self.windows()}
        if PI_WINDOW not in titles:
            self.pi_window("TESTQA", PI_WINDOW)
        self.presence_says(
            {
                "session": "01a1face-test",
                "addresses": ["TESTQA"],
                "busy": True,
                "owns": ["QEST-20261022-TESTQA"],
                "worksOn": "Tidy the tests",
                "groups": ["#desk"],
            }
        )
        if "Elsewhere" not in titles:
            self.opened("Elsewhere")

    def copied_then_closed(self, text: str, flags: str = "", served: str | None = None) -> None:
        """Copy from a program that then closes.

        wl-copy in the foreground serves the copy until it's stopped. It's stopped once the
        clipboard hands the copy back, not after a guess: a two-second guess once ended a copy
        the history never heard.
        """
        served = served or f"[ \"$(wl-paste --no-newline 2>/dev/null)\" = '{text}' ]"
        self.as_account(
            f"echo -n '{text}' | wl-copy --foreground {flags} > /dev/null 2>&1 & p=$!; "
            f"for i in $(seq 100); do {served} && break; sleep 0.1; done; kill $p"
        )

    def pasted_the_picture(self, _: bool) -> bool:
        """Say whether the clipboard hands back the picture as it was copied."""
        pasted = self.in_session(f"wl-paste --type image/png | cmp -s - {self.values['picture']}")
        return self.machine.execute(pasted)[0] == 0

    def opens_in(self, path: str, kind: str, app_id: str) -> None:
        """Open a file as a click on it would, and wait for the program meant for it to focus.

        A program's first start here, drawn in software, has taken 17 seconds on a busy host.
        If none does, it says what would open the file, what opening it said, and what's open.
        """
        self.as_account(f"niri msg action spawn -- sh -c 'xdg-open {path} > /tmp/opened 2>&1'")
        try:
            self.machine.wait_until_succeeds(
                self.in_session(
                    f'niri msg --json focused-window | grep -q \'"app_id":"{app_id}"\''
                ),
                timeout=timedelta(seconds=60),
            )
        except Exception:
            print(
                self.as_account(f"xdg-mime query default {kind}; cat /tmp/opened; niri msg windows")
            )
            raise

    def listen(self) -> None:
        """Listen to what the notification server says to programs, as they hear it."""
        dbus = self.values["dbus"]
        self.as_account(
            f"{dbus}/bin/dbus-monitor --session"
            " \"type='signal',interface='org.freedesktop.Notifications'\""
            " > /tmp/signals 2>&1 < /dev/null & echo $! > /tmp/listening"
        )
        # It's listening once it hears a signal of its own.
        probe = (
            f"{dbus}/bin/dbus-send --session --type=signal"
            " /probe org.freedesktop.Notifications.Probe"
        )
        self.machine.wait_until_succeeds(
            self.in_session(f"{probe}; grep -q member=Probe /tmp/signals"),
            timeout=timedelta(seconds=10),
        )

    def tokened(self) -> None:
        """Check that the program was sent a token to bring its window forward, then the choice.

        That's the order a compositor needs.
        """
        said = self.machine.succeed("cat /tmp/signals")
        self.machine.succeed("kill $(cat /tmp/listening)")
        token = re.search(r"member=ActivationToken\n\s+uint32 \d+\n\s+string \"([^\"]+)\"", said)
        assert token is not None, said
        assert said.index("member=ActivationToken") < said.index("member=ActionInvoked"), said

    def offer_to_play(self, picture: str, offers: str = "") -> tuple[float, float]:
        """Show a notice offering to play, and give where its action's glyph is drawn.

        Where is a fraction of the screen, and the glyph is the only thing in the action
        colour. notify-send waits for the answer, so it runs on its own, and the notice stays
        a minute, so only a click closes it sooner.
        """
        self.machine.wait_until_fails(self.in_session(NOTICES), timeout=timedelta(seconds=30))
        with self.told(f"shows card {picture}"):
            self.as_account(
                f"notify-send --expire-time=60000 {offers} --action=play='Play it' '{picture}'"
                " 'y.mp4' > /tmp/answer 2>&1 < /dev/null &"
            )
        hexed = self.values["action"]
        action = [int(hexed[i : i + 2], 16) for i in (1, 3, 5)]
        drawn: list[tuple[int, int]] = []
        size = [1, 1]

        # The card's first frames may not hold the glyph yet.
        def glyph_drawn(last: bool) -> bool:
            image = self.settled_screen(picture)
            pixels = image.tobytes()
            size[:] = [image.width, image.height]
            drawn[:] = [
                (x, y)
                for y in range(image.height)
                for x in range(image.width)
                if sum(abs(pixels[3 * (y * image.width + x) + c] - action[c]) for c in range(3))
                < NEAR_THE_ACTION
            ]
            if last:
                assert drawn, "the action's glyph isn't drawn"
            return bool(drawn)

        retry(glyph_drawn, timeout=timedelta(seconds=15))
        xs, ys = [x for x, _ in drawn], [y for _, y in drawn]
        assert max(xs) - min(xs) < ONE_GLYPH, f"more than one glyph: {drawn}"
        assert max(ys) - min(ys) < ONE_GLYPH, f"more than one glyph: {drawn}"
        return sum(xs) / len(xs) / size[0], sum(ys) / len(ys) / size[1]

    def tablet(self, *events: Json) -> None:
        """Send QEMU's tablet these events, its axes spanning the screen from 0 to 32767.

        The driver says a command's arguments are strings, so the events go through JSON to
        reach it as they are.
        """
        client = self.machine.qmp_client
        assert client is not None
        client.send("input-send-event", json.loads(json.dumps({"events": events})))

    def point(self, x: float, y: float) -> None:
        """Move the pointer there, as a fraction of the screen across and down."""
        self.tablet(
            {"type": "abs", "data": {"axis": "x", "value": int(x * 32767)}},
            {"type": "abs", "data": {"axis": "y", "value": int(y * 32767)}},
        )

    def click(self, x: float, y: float) -> None:
        """Click there, as a fraction of the screen across and down."""
        self.point(x, y)
        # QEMU hands the guest each batch in the order sent, so the press
        # lands where the pointer went.
        self.tablet({"type": "btn", "data": {"down": True, "button": "left"}})
        self.tablet({"type": "btn", "data": {"down": False, "button": "left"}})

    def answered(self) -> str:
        """Give what notify-send said once its notice closed.

        That's the action if one was done, and nothing if the notice was only put away.
        """
        self.machine.wait_until_fails(
            "pgrep -f '[n]otify-send .*--action=play'", timeout=timedelta(seconds=15)
        )
        return self.machine.succeed("cat /tmp/answer").strip()

    def tuple_said(self, message: str, ago: int = 0) -> None:
        """Write a line to Tuple's log as the daemon writes it, as if so many seconds since."""
        line = f"$(date -u -d '-{ago} sec' +%H:%M:%S.000)|1|1|info|{message}"
        self.as_account(f'mkdir -p $(dirname {TUPLE_LOG}) && echo "{line}" >> {TUPLE_LOG}')

    def capture(self) -> None:
        """Press Print, which opens Capture, and wait until its rows are ready for the keys."""
        self.machine.send_key("sysrq")
        self.launcher_until(
            lambda shown: (
                shown["keys"] and shown["held"] == ["captures"] and shown["selected"] is not None
            )
        )

    @contextlib.contextmanager
    def captured(self) -> Iterator[None]:
        """Wait for niri to say it took a picture, after what's done inside this."""
        since = self.mark()
        yield
        self.reported("captured ", lambda path: path.startswith(f"{self.pictures}/"), since)

    def newest_size(self) -> tuple[int, int]:
        """Give the newest picture's width and height.

        A PNG says them in its header, from byte 16.
        """
        newest = f"ls -t {self.pictures}/*.png | head -1"
        size = self.machine.succeed(f"{newest} | xargs -d '\\n' od -An -tu1 -j16 -N8")
        b = [int(n) for n in size.split()]
        return (b[2] * 256 + b[3], b[6] * 256 + b[7])

    # ssh-tpm-agent asks before a key confirmed on use signs, and for a
    # key's PIN, and ssh for a security key's touch, all through the
    # shell's askpass, which answers as OpenSSH's contract says.
    def ask(self, kind: str, prompt: str) -> None:
        """Ask through the askpass, started by the account's systemd as ssh-tpm-agent is.

        That way it has what the agent has.
        """
        self.as_account(
            f"systemd-run --user --collect --quiet -- {self.values['asking']} {kind} '{prompt}'"
        )

    def answered_with(
        self, kind: str, code: int, answer: str = "", timeout: timedelta = timedelta(seconds=30)
    ) -> None:
        """Wait until an ask is answered, and check how it exited and what it gave back."""
        exited = f"$XDG_RUNTIME_DIR/asked-{kind}.exit"
        self.machine.wait_until_succeeds(self.in_session(f"test -e {exited}"), timeout=timeout)
        assert self.as_account(f"cat {exited}").strip() == str(code)
        assert self.as_account(f"cat $XDG_RUNTIME_DIR/asked-{kind}.answer") == answer

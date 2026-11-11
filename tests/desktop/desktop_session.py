"""The desktop every area of the desktop test drives: its boot, and the helpers each area shares.

tests/desktop.nix gives this to the driver as a module of its Python, and its opening comment
says what the test proves. Each area asks for a `Desktop` as a fixture, made once a run by
`started`. A helper two areas use comes here, and nothing here is a test.
"""

from __future__ import annotations

import contextlib
import functools
import io
import itertools
import json
import operator
import re
import shlex
import tempfile
import time
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypedDict, cast

import pytest
import test_driver.machine
import vm_helpers
from PIL import Image, ImageChops, ImageStat

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
    desk_light: str
    ergodox: str
    keyboard: str
    stand_in_sessions: str
    touchscreen: str
    tuple_window: str
    # A picture, a document, a text and a track to open, as a download would be.
    picture: str
    page: str
    text: str
    track: str
    # A wide picture and a short video, for the file picker to preview.
    wide: str
    clip: str
    # The ErgoDox's layout, as its module describes it for the shell.
    ergodox_layout: str
    # warp-cli's answers as warp's own tests read them (modules/warp/contract), all made up, for
    # the stand-in warp-cli to give: only the areas that show WARP, so no other area moves with
    # them.
    warp_contract: str
    # Windows retitled without rest, as a crowd of agents' windows is (desktop/window-storm.py):
    # only keeps-pace, which opens the launcher through the storm.
    window_storm: str
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
    # What a floating frame leaves of the screen across, in logical pixels.
    floating_room: int
    # Whether this is the gallery, which keeps pictures for a person to look at.
    shots: bool


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
# How many keys type_in sends before the launcher holds the ones before them. Pacing alone
# holds a stall of about 64 events' worth of keys: a guest starved for 1.04 s on a loaded
# builder kept "3c:2" and "6" of "3c:22:fb:12:34:56" (2026-11-10). Six keys, a shifted one
# eight events, never fill the queue, however long the stall.
KEYS_IN_FLIGHT = 6
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
# The modes the launcher's line types in: the find, and insert while a row asks for words.
TYPING = ("find", "insert")
# The window the launcher's window subtests find by its title.
NOTES = "Notes for Tuesday"

# A session as pi keeps one: its quest's README, front matter first, and
# its presence while it runs; and its window, titled as pi titles its own:
# its address, its quest cut short, its folder, which a hyphen doesn't end.
PI_QUEST = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-TESTQA"
PI_PRESENCE = "~/.local/state/pi/agentic-harness.pi/mesh-workflow/presence"
PI_WINDOW = "\u03c0 - TESTQA A quest of\u2026 - my-tree"

# The margin is there only while something's on, each thing its glyph
# alone, and what each is, in words, is the launcher's Now: Super+O
# or a click on a glyph opens it, Return does a thing's first action and
# Ctrl+] goes into the rest.
MARGIN = "niri msg layers | grep -q margin"
# The margin's cells: the theme's gap, open or shut, since nothing grows.
CELL = 12
NOTICES = "niri msg layers | grep -q notifications"

# Tuple's log, where tuple-call hears a call ring, which only a call
# writes, and which the daemon rewrites as it starts.
TUPLE_LOG = "~/.local/share/tuple/0/log.txt"
# Tuple's stream as tuple-call follows it in the VM, through the stand-in
# for Tuple's command (desktop/tuple-stand-in.sh), and what it was asked.
TUPLE_STREAM = "$XDG_RUNTIME_DIR/tuple-stream"
TUPLE_ASKED = "$XDG_RUNTIME_DIR/tuple-asked"

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

    While Now is open, the mark of the row the keys are on wears the accent, and
    nothing grows. An empty pointed means none does.
    """

    def points(rest: str) -> bool:
        drawn = json.loads(rest)
        return bool(drawn) and all(
            glyph["cell"] == CELL and (glyph["hue"] == "accent") == (glyph["mark"] == pointed)
            for glyph in drawn
        )

    return points


def changed_where(before: bytes, now: bytes) -> str:
    """Say where two of QEMU's pictures of the screen differ, as the box around it all."""
    box = ImageChops.difference(
        Image.open(io.BytesIO(before)).convert("RGB"), Image.open(io.BytesIO(now)).convert("RGB")
    ).getbbox()
    return f" from {box[0]},{box[1]} to {box[2]},{box[3]}" if box else ""


# How much of the screen a running clock may change and the picture still count as at rest: a
# hundredth of its pixels. A timer's figures change a few hundred each second, two clocks
# under ten thousand at 2, while a fade changes every pixel of what fades, the launcher's
# hundreds of thousands.
TICKS_SHARE = 0.01


def but_the_ticks(before: bytes, now: bytes) -> bool:
    """Say two of QEMU's pictures of the screen are alike but for what a clock changed."""
    was, then = Image.open(io.BytesIO(before)), Image.open(io.BytesIO(now))
    if was.size != then.size:
        return False
    changed = ImageChops.difference(was.convert("RGB"), then.convert("RGB")).convert("L")
    moved = changed.point(lambda level: 255 if level else 0).histogram()[255]
    return moved < TICKS_SHARE * was.width * was.height


def for_you_lit(desktop: Desktop, said: str) -> None:
    """Light For You's tray with a critical notice put away unread, then wait for its mark."""
    with desktop.told(f"shows card {said}"):
        desktop.as_account(f"notify-send --urgency=critical {shlex.quote(said)} 'are you there?'")
    since = desktop.mark()
    desktop.as_account("qs ipc call notices putAway")
    desktop.reported("holds margin ", lambda rest: "forYou" in rest.split(), since)


def for_you_read(desktop: Desktop, *rest: str) -> None:
    """Read every notice, which puts For You's tray out, and wait for the margin to hold the rest.

    Read as Joel reads them, by Mark Them Read in For You's Desktop, where Super+N goes with
    no cards up; its line asks first, since reading them all can't be taken back.
    """
    desktop.machine.send_key("meta_l-n")
    desktop.launcher_until(lambda shown: shown["held"][-1:] == ["Desktop"])
    desktop.down_to("Mark Them Read")
    since = desktop.mark()
    desktop.line_answered("delete")
    desktop.close_launcher()
    # The prefix holds no space, since a margin holding nothing says none after it.
    desktop.reported("holds margin", lambda held: sorted(held.split()) == sorted(rest), since)


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


def keyed(name: str, *, down: bool) -> Json:
    """Give QEMU's input event for a key going down or up, by its name or its number."""
    key = (
        {"type": "number", "data": int(name, 16)}
        if name.startswith("0x")
        else {"type": "qcode", "data": name}
    )
    return {"type": "key", "data": {"down": down, "key": key}}


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
        # niri imports these as a session does, run as its child so each holds
        # what niri set: NIRI_ON_DEMAND_WHEN_ASKED says it gives the keys as a
        # surface asks (modules/niri's on-demand-when-asked.patch).
        self.as_account(
            "NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg action spawn --"
            " systemctl --user import-environment WAYLAND_DISPLAY NIRI_SOCKET"
            " NIRI_ON_DEMAND_WHEN_ASKED"
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

    # Every notice the notices daemon keeps, newest first (modules/notices).
    NOTICES_KEPT = "~/.local/state/notices/notices.json"

    def notice_kept(self, where: str) -> str:
        """Give a command that prints each notice kept that jq's `where` holds of, or fails."""
        return self.in_session(f"jq -ec '.notices[] | select({where})' {self.NOTICES_KEPT}")

    def as_account(self, command: str) -> str:
        """Run a command as the account, inside its session, and give what it printed."""
        return self.machine.succeed(self.in_session(command))

    # What whereabouts keeps of where the laptop is, read back as it starts.
    KEPT_HERE = "$XDG_RUNTIME_DIR/whereabouts/here.json"

    def found_at(self, here: Json | None) -> None:
        """Say where the laptop is in whereabouts' stead, or nowhere for None.

        The VM has no geolocation service, so GeoClue finds nothing and what whereabouts kept
        from before it started stands: this writes that, in the kept file's shape, and starts
        it again, which says it over its socket as a restart would.
        """
        kept = self.KEPT_HERE
        write = f"rm -f {kept}"
        if here is not None:
            write = f"printf %s {shlex.quote(json.dumps(here))} > {kept}"
        self.as_account(f"{write} && systemctl --user restart whereabouts.service")

    def here_kept(self) -> Json | None:
        """Give where whereabouts says the laptop is, as it keeps it, or None for nowhere."""
        said = self.as_account(f"cat {self.KEPT_HERE} 2>/dev/null || true").strip()
        return cast("Json", json.loads(said)) if said else None

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

    def lid(self, *, shut: bool) -> None:
        """Shut the stand-in lid, or open it."""
        self.machine.succeed(f"echo {int(shut)} > /sys/module/stand_in_lid/parameters/shut")

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

    def unsettled(
        self,
        timeout: timedelta = timedelta(seconds=10),
        span: float = 0.5,
        *,
        shell_only: bool = False,
        ticking: bool = False,
    ) -> str | None:
        """Wait for the screen to stand still, giving None once it has, or why it never did.

        Still is the screen's picture staying the same for two spans (vm_helpers.settled).
        Half a second is twice the theme's longest movement; both it and the bound stretch
        with how busy the host is, since a busy host draws a fade slower than that. The
        shell's own count of its frames (Frames.qml) says, when it never stands still, whether
        the shell drew meanwhile, and the pictures say where the screen last changed. Shell
        only reads the count in place of the picture, for a screen where something else
        always moves: still is then the shell drawing the same between looks. Ticking allows
        a running clock's figures to change, the rest of the picture holding still
        (but_the_ticks), since a clock's own frames would change the count with it.
        """
        pace = vm_helpers.pace(vm_helpers.busyness())
        began = time.monotonic()
        with tempfile.TemporaryDirectory() as folder:
            shot = Path(folder) / "shot.ppm"

            def counted() -> dict[str, int]:
                # Before the shell runs, or after it's gone, nothing of its own is drawing.
                status, said = self.machine.execute(
                    self.in_session("qs ipc call frames said && qs ipc call frames start")
                )
                lines = said.strip().splitlines()
                return cast("dict[str, int]", json.loads(lines[0]) if status == 0 and lines else {})

            def screen() -> bytes:
                if shell_only:
                    return json.dumps(counted(), sort_keys=True).encode()
                self.machine.send_monitor_command(f"screendump {shot}")
                return shot.read_bytes()

            def drawn() -> dict[str, int]:
                return {} if shell_only else counted()

            try:
                return vm_helpers.settled(
                    screen,
                    drawn,
                    span * pace,
                    timeout.total_seconds() * pace,
                    where=(vm_helpers.nowhere if shell_only else changed_where),
                    alike=(but_the_ticks if ticking else operator.eq),
                )
            finally:
                # Counting stops, so a shell between checks pays nothing for it.
                _, said = self.machine.execute(self.in_session("qs ipc call frames said"))
                print(
                    f"the screen looked at for {time.monotonic() - began:.1f} s, in spans of"
                    f" {span * pace:.1f} s, the shell drawing {said.strip()} at the last"
                )

    def held_still(self, timeout: timedelta = timedelta(seconds=10), span: float = 0.5) -> bool:
        """Say whether the screen stands still before the time is up (unsettled)."""
        return self.unsettled(timeout, span) is None

    def shoot(
        self,
        name: str,
        timeout: timedelta = timedelta(seconds=10),
        *,
        moving: str = "",
        ticking: str = "",
    ) -> None:
        """Shoot the screen once it stands still, for a person to look at, in the gallery.

        One that never stands still is shot as it is, then fails saying why, so a picture caught
        mid-motion never reaches the person judging the gallery as if it were the design.
        Moving says what in the picture moves by itself, as a camera's view does, so the shot
        waits on the shell's frames alone. Ticking says what clock runs in it, as a timer's
        does, so the shot waits for the rest of the picture to hold still. Outside the gallery
        it takes nothing and waits for nothing, so a test must never lean on it to let the
        screen settle.
        """
        if self.values["shots"]:
            why = self.unsettled(timeout, shell_only=bool(moving), ticking=bool(ticking))
            self.machine.screenshot(name)
            assert why is None, f"{name} was shot moving: {why}"

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
        if self.values["shots"]:
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

    def normal(self) -> None:
        """Put the launcher's line in normal mode, by Escape from the find, and wait for it."""
        if self.showing()["mode"] == "normal":
            return
        self.machine.send_key("esc")
        self.launcher_until(lambda shown: shown["mode"] == "normal")

    def land(self) -> None:
        """Press Return on a row that changes how things stand, from the find, and wait.

        Return there only lands on it, in normal, the row saying what a second Return does,
        so a test presses Return again for it to be done, once a Return no longer comes so soon
        after the landing that the launcher takes it for the same press doubled.
        """
        self.machine.send_key("ret")
        self.launcher_until(lambda shown: shown["mode"] == "normal" and shown["landed"] != "")
        fresh = "qs ipc call launcher showing | grep -qF '\"landedFresh\":false'"
        self.machine.wait_until_succeeds(self.in_session(fresh), timeout=timedelta(seconds=5))

    def line(self, written: str) -> None:
        """Press Return on a row that ends something, and wait for its command on the : line.

        Return on it, from the find or normal, only writes its command there, so a test presses
        Return again for it to be done, once a Return no longer comes so soon after the writing
        that the launcher takes it for the same press doubled.
        """
        self.machine.send_key("ret")
        self.launcher_until(lambda shown: (shown["command"] or {}).get("line") == written)
        fresh = "qs ipc call launcher showing | grep -qF '\"linedFresh\":false'"
        self.machine.wait_until_succeeds(self.in_session(fresh), timeout=timedelta(seconds=5))

    def line_answered(self, written: str, asked: str | None = None) -> None:
        """Press Return on a row that ends something, run its line, and answer its ask yes.

        What can't be taken back asks once the line runs, as d's does, and y does it. Given
        `asked`, the ask says that: the row's own verb and name, never the line's command.
        """
        self.line(written)
        with self.told("ask ready"):
            self.machine.send_key("ret")
        if asked is not None:
            self.launcher_until(lambda shown: shown["question"] == asked)
        with self.told("answered yes"):
            self.machine.send_chars("y")

    def on_find(self) -> None:
        """Put the launcher's line back to typing, by A from normal, and wait for it.

        It types on the find, or in insert while a row asks for words, as a password. A puts
        the caret at the end of what's typed, so what's typed next goes after it.
        """
        if self.showing()["mode"] in TYPING:
            return
        self.machine.send_chars("A")
        self.launcher_until(lambda shown: shown["mode"] in TYPING)

    def notices_history(self) -> None:
        """From the held notices, open every notice in the launcher, as :e Notices does."""
        self.machine.send_chars(":e Notices\n", delay=KEY_PACE)
        self.launcher_ready()

    def close_launcher(self) -> None:
        """Press Escape until normal is at rest, then close the launcher by :q, as Joel does.

        Escape gives normal from the find, lets the key list go if it's shown, and cancels a
        count, a key begun or an operator, but never closes (Joel, 2026-11-05: "fuck escape
        quitting"), so :q does, from any depth. A key sent before this that the launcher hears
        late moves it on too, so each Escape waits only for something to change. Escape does
        nothing in the drawer, as in any window, so with the keys there, on its page or choosing
        on it, :q brings them back to the rows first, a line typed that no wait can come apart.
        """

        # A pending count, key or operator counts, since an Escape cancels
        # it and changes nothing else, as Vim's does.
        def where(shown: Json) -> tuple[object, ...]:
            return (
                shown["open"],
                shown["operator"],
                shown["mode"],
                len(shown["held"]),
                shown.get("listing"),
                shown.get("command") is not None,
                shown["counted"],
                shown["begun"],
            )

        def moved(was: tuple[object, ...], shown: Json) -> bool:
            return where(shown) != was

        for _ in range(12):
            was = where(self.showing())
            if not was[0]:
                return
            _, operator, mode, _, listing, commanding, counted, begun = was
            resting = mode == "normal" and not (operator or listing or commanding)
            if str(mode).startswith("page") or (resting and not (counted or begun)):
                self.machine.send_chars(":q\n")
            else:
                self.machine.send_key("esc")
            self.launcher_until(functools.partial(moved, was))
        still = self.showing()
        message = f"the launcher was still open after 12 Escapes: {still}"
        raise AssertionError(message)

    def type_in(self, text: str, selecting: str = "") -> None:
        """Type into the launcher, and wait until it holds every letter with Return's row settled.

        In normal mode it goes back to typing first, at the end of what's typed. An answer still
        being looked up closes the launcher without copying. A row that comes from something
        found later, such as a window or a copy, can be named to wait for too.
        """
        self.on_find()
        # At most KEYS_IN_FLIGHT go before the launcher holds those sent so far, so a guest
        # starved however long can't drop the middle of what's typed.
        sent = ""

        def holds_sent(shown: Json) -> object:
            return shown["typed"].endswith(sent)

        while len(text) - len(sent) > KEYS_IN_FLIGHT:
            sent += text[len(sent) : len(sent) + KEYS_IN_FLIGHT]
            self.machine.send_chars(sent[-KEYS_IN_FLIGHT:], delay=KEY_PACE)
            self.launcher_until(holds_sent)
        self.machine.send_chars(text[len(sent) :], delay=KEY_PACE)

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

    def step_in(self, key: str = "ret") -> None:
        """Press a key that steps into the selected row, and wait until the launcher holds it.

        That's Return, from the find or normal alike, into a place or a thing with acts, or
        into what a verb asks; - goes back up. A program, a window or a file opens on Return
        instead, so its parts are reached by `follow`, and its acts by : and their commands.
        Return from the find keeps the find, where the next Return on a row that changes
        something only lands, so it ends in normal, as `follow` and `step_out` do. A step into
        what asks, as a password, stays on its line, which Escape would give up.
        """
        before = len(self.showing()["held"])
        self.machine.send_key(key)
        self.launcher_until(lambda shown: len(shown["held"]) > before or bool(shown["asking"]))
        if not self.showing()["asking"]:
            self.normal()

    def follow(self) -> None:
        """Go into the selected row's parts by Ctrl+] from normal, and wait until it's there.

        That's how a row whose Return opens it is gone into, as a mail's sender or a
        program's windows, and how a found row goes to where it lives; - comes back.
        """
        before = len(self.showing()["held"])
        self.normal()
        self.machine.send_key("ctrl-bracket_right")
        self.launcher_until(lambda shown: len(shown["held"]) > before)

    def into_program(self) -> None:
        """Open the selected program's page by Tab, as Joel does, and wait until it's held.

        Every walk to what a program holds goes in here, so they all go by the one key Joel
        keeps for it (2026-11-07: "keep", Return still switching): from the find, where he
        types its name, Tab goes into a row holding parts.
        """
        before = len(self.showing()["held"])
        self.machine.send_key("tab")
        self.launcher_until(lambda shown: len(shown["held"]) > before)

    def step_out(self) -> None:
        """Go up a level by - from normal, and wait until the launcher holds less.

        On the find - is a letter, so normal comes first, as it does for h in Vim.
        """
        before = len(self.showing()["held"])
        self.normal()
        self.machine.send_key("minus")
        self.launcher_until(lambda shown: len(shown["held"]) < before)

    def down_to(self, name: str) -> None:
        """Move the selection to the row named, and wait until it's there.

        Rows are counted by name, since what leads a step (Return's own act) moves the rest,
        and from the row selected, which a step in from a match keeps rather than the first.
        A selection above the rows, an answer's headline or a save's name on the line, is -1.
        """
        shown = self.showing()
        rows = shown["rows"]
        selected = (shown["selected"] or {}).get("name")
        above = -1 if shown["current"] == -1 else 0
        steps = rows.index(name) - (rows.index(selected) if selected in rows else above)
        for _ in range(abs(steps)):
            if steps > 0:
                self.next_row()
            else:
                self.previous_row()
        self.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == name)

    def row_keys(self) -> tuple[str, str]:
        """Give the keys to the next row and the one before, as the launcher's mode takes them.

        Where nothing types, in normal, visual and the drawer, they're j and k, and the arrows
        do nothing (Joel, 2026-11-05: "rely only on vi right feeling things"); on a line being
        typed, the arrows, as Vim's insert and its completion menu take them.
        """
        mode = self.showing()["mode"]
        still = mode in {"normal", "visual"} or mode.startswith("page")
        return ("j", "k") if still else ("down", "up")

    def next_row(self) -> None:
        """Move the launcher's cursor to the next row, by the key its mode takes."""
        self.machine.send_key(self.row_keys()[0])

    def previous_row(self) -> None:
        """Move the launcher's cursor to the row before, by the key its mode takes."""
        self.machine.send_key(self.row_keys()[1])

    def in_now(self, selected: str) -> None:
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
        """Make a change the shell is sure to hear, and shoot the meter that answers it.

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
        self.shoot(namespace)

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
        window it's in, and the window's title as its label; and for TESTQA, the quest
        pi_session lays out, that quest whole and its folder, as the harness says them.
        """
        name = address.lower()
        pid, scope, window = self.as_account(f"cat $XDG_RUNTIME_DIR/{name}-owner").split()
        title = self.as_account(f"cat $XDG_RUNTIME_DIR/{name}-title").strip()
        ticks = self.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{pid}/stat")
        told = (
            {
                "questInfo": {
                    "id": "QEST-20261022-TESTQA",
                    "title": "A quest of its own & whole",
                    "short": "TESTQA",
                    "status": "active",
                },
                "cwd": "/home/jitsusama/my-tree",
            }
            if address == "TESTQA"
            else {}
        )
        return {
            **told,
            # The session's own id, which pi always writes and the node keys each record by: one
            # without it isn't a session to the node. TESTQA's is pi_session's.
            "session": "01a1face-test" if address == "TESTQA" else f"01a1face-{name[-2:]}",
            "title": title,
            # When it was written, which mycelium's node reads every
            # record by: one without it isn't a session to the node.
            "at": datetime.now(UTC).isoformat(),
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
        """Show a notice offering to play, and give where the middle of its card is drawn.

        Where is a fraction of the screen, as the shell says it. notify-send waits for the
        answer, so it runs on its own, and the notice stays a minute, so only a click closes
        it sooner.
        """
        self.machine.wait_until_fails(self.in_session(NOTICES), timeout=timedelta(seconds=30))
        with self.told(f"shows card {picture}"):
            self.as_account(
                f"notify-send --expire-time=60000 {offers} --action=play='Play it' '{picture}'"
                " 'y.mp4' > /tmp/answer 2>&1 < /dev/null &"
            )
        self.settled_screen(picture)
        card = json.loads(self.as_account("qs ipc call notices front"))
        assert card, "the shell says no card is in front"
        return (card["left"] + card["right"]) / 2, (card["top"] + card["bottom"]) / 2

    def press_at_once(self, keys: str) -> None:
        """Press each key named, a letter by its character and any other by QEMU's name.

        Every key goes down and up in one batch of QEMU's input events, so the guest reads them
        all at once, as two keys pressed closer together than a person can are. Sent one
        sendkey at a time, a loaded builder let niri draw the first key's frame before it read
        the second, and a waiting key's 300 ms ran out between them.
        """
        events: list[Json] = []
        for key in keys.split():
            *held, pressed = test_driver.machine.CHAR_TO_KEY.get(key, key).split("-")
            chord = [*held, pressed]
            events += [keyed(name, down=True) for name in chord]
            events += [keyed(name, down=False) for name in reversed(chord)]
        self.input_events(*events)

    def tablet(self, *events: Json) -> None:
        """Send QEMU's tablet these events, its axes spanning the screen from 0 to 32767."""
        self.input_events(*events)

    def input_events(self, *events: Json) -> None:
        """Send QEMU these input events, a key's or the tablet's, as one batch.

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

    def wheel(self, x: float, y: float, notches: int) -> None:
        """Turn the wheel there, as a fraction of the screen, down for a positive count."""
        self.point(x, y)
        button = "wheel-down" if notches > 0 else "wheel-up"
        for _ in range(abs(notches)):
            self.tablet({"type": "btn", "data": {"down": True, "button": button}})
            self.tablet({"type": "btn", "data": {"down": False, "button": button}})

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

    def tuple_follows(self) -> None:
        """Stop Tuple's daemon, answer on its socket in its stead, and wait for tuple-call.

        Tuple's daemon rewrites its log as it starts, so it's stopped while a test writes the
        log, and tuple-call follows Tuple's stream only while something answers on its socket.
        Once it does, it follows the stand-in's.
        """
        self.as_account("systemctl --user stop tuple && systemctl --user start tuple-answers")
        self.machine.wait_until_succeeds(
            self.in_session("pgrep -f '[t]ail -n [+]1 -F .*tuple-stream'"),
            timeout=timedelta(seconds=30),
        )

    def tuple_streams(self, call: Json | None) -> None:
        """Say the call Joel's in on Tuple's stream, as Tuple says it, or that there's none."""
        line = json.dumps({"type": "current_call", "data": call}, separators=(",", ":"))
        self.as_account(f"echo {shlex.quote(line)} >> {TUPLE_STREAM}")

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


# What the launcher's areas share: the harness's files they write, Google's stand-in, and how
# they read what the launcher draws.
HARNESS = "~/.local/state/pi/agentic-harness.pi"
# How long a window may take to start the stand-in pi. The first zsh in the fresh test machine
# makes its completion dump before running anything, and under the builder's load that has
# kept a window's shell busy past a minute, so a subtest run on its own waited it out.
SHELL_STARTS = timedelta(seconds=120)
INBOX = f"{HARNESS}/steer/TESTQA"


def lighthouse_listed(desktop: Desktop) -> None:
    """List Lighthouse as an orchestrator, as the harness would, its inbox empty."""
    listed = {
        "orchestrators": [
            {"quest": "QEST-20261022-TESTQA", "address": "TESTQA", "short": "Lighthouse",
             "title": "Keep the lighthouse lit"},
        ]
    }  # fmt: skip
    desktop.as_account(
        f"mkdir -p {HARNESS}/quest-workflow && echo '{json.dumps(listed)}'"
        f" > {HARNESS}/quest-workflow/.o && mv {HARNESS}/quest-workflow/.o"
        f" {HARNESS}/quest-workflow/orchestrators.json && rm -rf {INBOX}"
    )


def listed_quest(quest: str, title: str, kind: str = "quest", **fields: object) -> Json:
    """Give a quest as the harness's lists hold it, open and driving unless `fields` say not.

    Its kind is said twice, as the harness says it, since the node puts its own over `kind`.
    """
    return {
        "id": quest,
        "address": quest[-6:],
        "title": title,
        "kind": kind,
        "quest.kind": kind,
        "status": "active",
        "priority": "driving",
        "rank": 1,
        "documents": [],
        "sessions": [],
        "start": "/home/jitsusama",
        "updated": "2026-10-22T09:00:00.000Z",
        **fields,
    }


def quests_listed(desktop: Desktop, quests: list[Json], ended: list[Json] | None = None) -> None:
    """Write the harness's lists of open and ended quests, each whole by rename, as it does.

    The realm's node serves both, and the launcher's quests are what it serves.
    """
    workflow = f"{HARNESS}/quest-workflow"
    desktop.as_account(
        f"mkdir -p {workflow}"
        f" && echo {shlex.quote(json.dumps({'quests': quests}))} > {workflow}/.q"
        f" && mv {workflow}/.q {workflow}/quests.json"
        f" && echo {shlex.quote(json.dumps({'quests': ended or []}))} > {workflow}/.e"
        f" && mv {workflow}/.e {workflow}/ended.json"
    )


def started_apart(desktop: Desktop, since: str, said: str) -> list[float]:
    """Give the seconds between each journal line of the shell's saying `said`, since a mark.

    The first is zero, so there's one for every line.
    """
    lines = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=short-unix"
        f" --after-cursor={shlex.quote(since)}"
    ).splitlines()
    times = [float(line.split()[0]) for line in lines if said in line]
    return [0.0] + [later - sooner for sooner, later in itertools.pairwise(times)]


@pytest.fixture
def google(desktop: Desktop) -> Iterator[None]:
    """Start the stand-in Google afresh for one test, and stop it after.

    Its meetings are timed from when it starts, so one left running would come to start during
    a later test, wanting Joel in Now and said as a notice.
    """
    desktop.as_account("systemctl --user restart google.service")
    yield
    desktop.as_account("systemctl --user stop google.service")


def now_rows(desktop: Desktop) -> Json:
    """Open the launcher on Now, and give what it shows once it holds Now's rows."""
    desktop.answer("now")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:on")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["now"] and shown["selected"])
    return desktop.showing()


def repeating(said: Json) -> str:
    """Say each row the walk found whose drawer would only repeat it, one a line."""
    # Joel, 2026-10-29: "we should never show the side panel when it is
    # just repeating the currently selected text". A drawer says more with
    # a fact, a picture, or words that aren't the row's name, side or note.
    return "\n".join(
        drawer["in"]
        for drawer in cast("list[Json]", said["drawers"])
        if drawer["facts"] == 0
        and not drawer["picture"]
        and all(words in ("", *drawer["row"]) for words in (drawer["title"], drawer["body"]))
    )


# Where the agents keep what waits on Joel.
MISSION = "$HOME/.local/state/mission"


def kept(desktop: Desktop, path: str, read: dict[str, Any]) -> None:
    """Write a file the agents keep, whole, as they do."""
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(read))} > {path}.q && mv {path}.q {path}"
    )


def laid_out(desktop: Desktop) -> Json:
    """Give where the launcher draws each word, once two reads in a row agree."""
    was = None
    for _ in range(20):
        now = cast("Json", json.loads(desktop.as_account("qs ipc call launcher laidOut")))
        if now == was:
            return now
        was = now
    return cast("Json", was)


def foot_overlaps(desktop: Desktop) -> list[str]:
    """Give each of the foot's words whose ink runs into the next one's, its one line read left on.

    A key drawn over the glance's marks reads as neither (QIFT5D, 2026-11-06: Programs' normal
    drew "Ctrl+] in" over the network and volume marks), so the keys give way instead.
    """
    words = sorted(laid_out(desktop)["foot"], key=lambda word: word["starts"])
    return [
        f"{one['text']!r} inks to {one['starts'] + one['inked']}, {after['text']!r} starts at"
        f" {after['starts']}"
        for one, after in itertools.pairwise(words)
        if one["starts"] + one["inked"] > after["starts"]
    ]


def drawn_held_still(desktop: Desktop) -> int:
    """Count the frames the open launcher draws in three seconds left still.

    What the last key moved draws first: its glide, and a drawer it opened, which a loaded guest
    draws slowly, so the count begins once a second passes with no frame, or after ten. A
    launcher that keeps drawing, as a blinking caret would, never settles, and is counted
    anyway: Joel's caret blinks for 15 s after a key, as his kitty's does, so the test machine
    holds it steady (jitsusama.quickshell.caret.fade). The foot's clock may turn its minute in
    the three seconds, once.
    """

    def frames() -> int:
        return int(desktop.as_account("qs ipc call launcher frames"))

    began = frames()
    settling = time.monotonic() + 10
    while time.monotonic() < settling:
        time.sleep(1)
        settled, began = began, frames()
        if began == settled:
            break
    time.sleep(3)
    return frames() - began


# What the launcher's answers areas share: the stand-in battery's charge.
def watch_the_focus(desktop: Desktop) -> None:
    """Keep what niri says from now on, to read what it focused with focus_given."""
    desktop.as_account(
        "setsid sh -c 'niri msg --json event-stream > ~/focus-events' > /dev/null 2>&1 &"
    )
    desktop.machine.wait_until_succeeds(desktop.in_session("[ -s ~/focus-events ]"))


def focus_given(desktop: Desktop) -> tuple[list[int | None], list[int]]:
    """Give each window and each workspace niri focused since watch_the_focus, in order."""
    said = [json.loads(line) for line in desktop.as_account("cat ~/focus-events").splitlines()]
    windows = [one["WindowFocusChanged"]["id"] for one in said if "WindowFocusChanged" in one]
    workspaces = [
        one["WorkspaceActivated"]["id"]
        for one in said
        if one.get("WorkspaceActivated", {}).get("focused")
    ]
    return windows, workspaces


def window_box(desktop: Desktop, window: int) -> tuple[int, int, int, int]:
    """Give where a window shows on its screen, in the screen's pixels: x, y, width, height."""
    shown = next(one for one in desktop.windows() if one["id"] == window)
    layout = shown["layout"]
    output = json.loads(desktop.as_account("niri msg --json focused-output"))
    scale = float(output["logical"]["scale"])
    x, y = layout["tile_pos_in_workspace_view"]
    dx, dy = layout["window_offset_in_tile"]
    width, height = layout["window_size"]
    return (
        round((x + dx) * scale),
        round((y + dy) * scale),
        round(width * scale),
        round(height * scale),
    )


def charged(desktop: Desktop) -> str:
    """Give the stand-in battery's charge, as the kernel's stand-in has it."""
    return desktop.machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()


# What opens-files, which opens what Chrome downloaded and goes to a Neovim's buffer, shares
# with looks-programs, which shoots both programs' pages: Chrome's lists, files the stand-in's
# listers print (tests/desktop.nix), in the form chrome-recent-sites prints them, and Neovims in
# kitty windows, read only from outside, so the account's own configuration is what runs.
CHROME_VISITED = "/run/user/1000/chrome-visited"
CHROME_OPEN = "/run/user/1000/chrome-open"
CHROME_DOWNLOADS = "/run/user/1000/chrome-downloads"
# The sites it visited, an address and a title a line, and the tab it has open: its id, window,
# whether it's in front, when it was looked at, its address and title, a tab between each.
CHROME_SITES = (
    "https://estuary.example.org/\tEstuary Notes\nhttps://reef.example.org/\tReef Survey\n"
)
CHROME_TAB = "7\t1\ttrue\t1794070000000\thttps://tide.example.org/\tTide Tables\n"
# A kitty standing in for Chrome's window, by its app id and its front tab's title.
CHROME_WINDOW = "kitty --class google-chrome --title 'Tide Tables - Google Chrome'"
# The act Chrome's page ends with once its downloads are read.
LATEST_DOWNLOAD = "Open the Latest Download"


def chrome_visited(desktop: Desktop) -> None:
    """Give the stand-in Chrome the sites it visited and the tab it has open."""
    desktop.as_account(f"printf '%s' {shlex.quote(CHROME_SITES)} > {CHROME_VISITED}")
    desktop.as_account(f"printf '%s' {shlex.quote(CHROME_TAB)} > {CHROME_OPEN}")


def chrome_opened(desktop: Desktop) -> None:
    """Open the window standing in for Chrome's, and wait until niri has it focused."""
    desktop.as_account(f"niri msg action spawn -- {CHROME_WINDOW}")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q google-chrome"), timeout=STARTING
    )


def chrome_gone(desktop: Desktop) -> None:
    """Close the window standing in for Chrome's, if it's open, and take its lists away."""
    # The bracket keeps pkill from matching the shell that runs it.
    desktop.as_account(
        f"pkill -f 'kitty --clas[s] google-chrome'; rm -f {CHROME_VISITED} {CHROME_OPEN}"
    )


def chrome_downloaded(desktop: Desktop) -> str:
    """Put a picture in Downloads, listed as Chrome's newest download, and give its path.

    Its line is what chrome-recent-sites downloads prints: its path, its size in bytes, when it
    finished in seconds since 1970 and the page it came from, a tab between each.
    """
    downloaded = f"/home/{desktop.account}/Downloads/tide-chart.png"
    desktop.as_account(
        f"mkdir -p ~/Downloads && cp {desktop.values['picture']} {downloaded} && printf"
        f' \'%s\\t%s\\t%s\\t%s\\n\' {downloaded} "$(stat -c %s {downloaded})" "$(date +%s)"'
        f" https://tide.example.org/charts > {CHROME_DOWNLOADS}"
    )
    return downloaded


def chrome_download_gone(desktop: Desktop, downloaded: str) -> None:
    """Close what opened the download, if anything did, and take it and its listing away."""
    machine = desktop.machine
    # A swayimg left open would be the next subtest's.
    machine.succeed(f"pkill -KILL -u {desktop.account} -x swayimg || true")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -x swayimg", timeout=timedelta(seconds=30)
    )
    machine.succeed(f"rm -f {downloaded} {CHROME_DOWNLOADS}")


def neovim_in_kitty(desktop: Desktop, folder: str, *given: str) -> tuple[int, str]:
    """Open a kitty running Neovim on these, and give niri's window and Neovim's server.

    The server is the one Neovim told its kitty window, read back through kitty's remote
    control as open-now reads it, so a Neovim that never said it is never given.
    """
    before = {window["id"] for window in desktop.windows()}
    command = ["niri", "msg", "action", "spawn", "--", "kitty", "--directory", folder, "nvim"]
    desktop.as_account(shlex.join([*command, *given]))
    told: list[tuple[int, str]] = []

    def said_its_server(_: bool) -> bool:
        """Say whether the new kitty window's Neovim has told kitty where it listens."""
        new = [
            window
            for window in desktop.windows()
            if window["app_id"] == "kitty" and window["id"] not in before
        ]
        if not new:
            return False
        status, listed = desktop.machine.execute(
            desktop.in_session(f"kitty @ --to unix:@kitty-{new[0]['pid']} ls")
        )
        if status != 0:
            return False
        servers = [
            inside["user_vars"]["neovim_server"]
            for kitty_window in json.loads(listed)
            for tab in kitty_window["tabs"]
            for inside in tab["windows"]
            if "neovim_server" in inside["user_vars"]
        ]
        told.extend((new[0]["id"], server) for server in servers)
        return bool(servers)

    retry(said_its_server, timeout=STARTING)
    return told[0]


def asked_of(desktop: Desktop, server: str, *remote: str) -> str:
    """Ask the Neovim at this server from outside, through a client reading no configuration.

    It's headless and reads nothing, since a client given the console started a screen of its
    own there, drawing the answer on the console and writing its terminal's queries in its place.
    """
    client = ["nvim", "--clean", "--headless", "--server", server, *remote]
    return desktop.as_account(f"{shlex.join(client)} < /dev/null")


def neovims_quit(desktop: Desktop, neovims: list[tuple[int, str]]) -> None:
    """Quit each Neovim neovim_in_kitty gave, and wait until every one's window is gone.

    Each quits unsaved, so a change a test made in one is never written.
    """
    for _, server in neovims:
        asked_of(desktop, server, "--remote-send", "<C-\\><C-N>:qa!<CR>")
    gone = {window for window, _ in neovims}
    retry(
        lambda _: not gone & {window["id"] for window in desktop.windows()},
        timeout=timedelta(seconds=30),
    )

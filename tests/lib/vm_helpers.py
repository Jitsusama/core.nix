"""What VM tests' scripts share, so each says only what its own test is about.

A test's .nix gives this to its driver through `extraPythonPackages` (vm-helpers.nix), and
mypy finds it on the path core's shared linters set, as it finds driver_names.pyi. Each helper
takes the machine it acts on, since a module can't see the driver's globals.
"""

import datetime as dt
import operator
import os
import re
import shlex
import time
from typing import TYPE_CHECKING, Any, NamedTuple, Protocol

if TYPE_CHECKING:
    from collections.abc import Callable

    from test_driver.machine import BaseMachine

# Where systemd keeps the first account's runtime folder, its user manager's and PipeWire's
# sockets among them.
FIRST_RUNTIME = "/run/user/1000"


def as_account(  # noqa: PLR0913 # Each keyword is a part of the account's surroundings.
    machine: BaseMachine,
    command: str,
    *,
    account: str = "joel",
    runtime: str = FIRST_RUNTIME,
    bus: bool = False,
    succeed: bool = True,
) -> str:
    """Run a command as an account, in its home, and give what it printed.

    The command gets the account's runtime folder, so it reaches the account's systemd and
    PipeWire, and with bus its session's D-Bus too. Unless succeed, it must fail.
    """
    environment = f"XDG_RUNTIME_DIR={runtime}"
    if bus:
        environment += f" DBUS_SESSION_BUS_ADDRESS=unix:path={runtime}/bus"
    run = machine.succeed if succeed else machine.fail
    return run(f"su - {account} -c " + shlex.quote(f"export {environment}; cd ~; {command}"))


def journal_cursor(said: str) -> str:
    """Give the cursor `journalctl --show-cursor` printed last, to read only what comes after."""
    marker = "-- cursor: "
    if marker not in said:
        message = f"journalctl printed no cursor: {said!r}"
        raise ValueError(message)
    return said.rsplit(marker, 1)[1].strip()


# The most times longer than it says a wait may take on a busy host. Past this the host is so
# oversubscribed that a test should fail, in a time still bounded, rather than wait on.
MOST_PACE = 4.0
# How long the driver waits when it isn't told, as its retry says.
UNSTATED = dt.timedelta(minutes=15)


class Retry(Protocol):
    """The driver's retry: call a function each second until it says yes or the time is up."""

    def __call__(
        self,
        fn: Callable[..., Any],
        timeout_seconds: int | dt.timedelta | None = None,
        timeout: dt.timedelta | None = None,
    ) -> None:
        """Call fn until it's true, then once more, told it's the last try, when time is up."""


def busyness() -> float:
    """Give how many tasks the host has had ready to run each minute, per processor.

    The load average is the host's, a VM test's sandbox not hiding it, so it counts the other
    machines and builds a test's machine shares the processors with.
    """
    return os.getloadavg()[0] / (os.cpu_count() or 1)


def pace(busy: float) -> float:
    """Give how many times longer than it says a wait may take on a host this busy.

    On a host with a processor to spare for each task a wait takes the time it says; past
    that, each task waits its turn behind the others, so its time grows with them, up to
    MOST_PACE.
    """
    return min(MOST_PACE, max(1.0, busy))


def paced(
    retry: Retry,
    busy: Callable[[], float] = busyness,
    describe: Callable[[], None] | None = None,
) -> Retry:
    """Give a retry that allows for how busy the host is as each wait starts.

    A wait that fails, by running out or otherwise, has describe say what the machine shows
    first, so its failure says what was there rather than only that it timed out.
    """

    def paced_retry(
        fn: Callable[..., Any],
        timeout_seconds: int | dt.timedelta | None = None,
        timeout: dt.timedelta | None = None,
    ) -> None:
        if timeout_seconds is not None and timeout is not None:
            # Both, which the driver refuses and says why.
            retry(fn, timeout_seconds, timeout)
            return
        stated = timeout_seconds if timeout is None else timeout
        if stated is None:
            stated = UNSTATED
        elif not isinstance(stated, dt.timedelta):
            stated = dt.timedelta(seconds=stated)
        try:
            retry(fn, timeout=stated * pace(busy()))
        except Exception:
            if describe is not None:
                describe()
            raise

    return paced_retry


def pace_the_driver(script: dict[str, Any], describe: Callable[[], None] | None = None) -> None:
    """Make every wait a script makes allow for how busy the host is.

    Give it the script's globals(): the driver's machines wait through its machine module's
    retry, and the script calls its own copy, which the driver put among its globals before
    the script ran, so both are replaced.
    """
    import test_driver.machine  # noqa: PLC0415 # Only a VM test's driver has it.

    retry = paced(test_driver.machine.retry, describe=describe)
    test_driver.machine.retry = retry
    script["retry"] = retry


# How many spans in a row the screen must stand still for. A guest a busy host gives no time
# through one span draws nothing in it because it runs nothing, so one still span isn't rest;
# a fade that guest draws slowly still draws across the next.
STILL_SPANS = 2


def nowhere(_before: bytes, _now: bytes) -> str:
    """Say nothing of where two pictures differ, for a screen with no picture to read."""
    return ""


# The header of a binary PPM, the kind QEMU's screendump writes.
PPM_HEADER = re.compile(rb"P6\s+(?P<width>\d+)\s+(?P<height>\d+)\s+\d+\s")


class Band(NamedTuple):
    """The rows of a PPM from one up to another: its header, its width and those rows' pixels."""

    header: bytes
    width: int
    pixels: bytes

    def row(self, at: int) -> bytes:
        """Give one of the band's rows, counted from its first."""
        across = self.width * 3
        return self.pixels[at * across : (at + 1) * across]


def band(shot: bytes, top: int, bottom: int) -> Band:
    """Read the rows from top up to bottom of one of QEMU's pictures, a PPM."""
    # A PPM opens with four words, its kind, width, height and greatest level, and one
    # whitespace byte after them; its pixels follow, three bytes each, and may start with a
    # byte that reads as whitespace too.
    header = PPM_HEADER.match(shot)
    if header is None:
        message = f"not a PPM: {shot[:16]!r}"
        raise ValueError(message)
    # A band the picture doesn't hold would read as alike every time, so a wait would pass
    # without looking at anything.
    high = int(header["height"])
    if not 0 <= top < bottom <= high:
        message = f"rows {top} to {bottom} aren't in a picture {high} high"
        raise ValueError(message)
    width = int(header["width"])
    pixels = shot[header.end() :]
    return Band(header[0], width, pixels[top * width * 3 : bottom * width * 3])


def alike_between_rows(top: int, bottom: int) -> Callable[[bytes, bytes], bool]:
    """Say two of QEMU's pictures, PPMs, are alike in the rows from top up to bottom.

    For a screen where something moves by design above or below what a wait is about, as the
    margin cat walks along the windows' tops and sits at the screen's foot.
    """

    def alike(before: bytes, now: bytes) -> bool:
        return band(before, top, bottom) == band(now, top, bottom)

    return alike


def changed_between_rows(top: int, bottom: int) -> Callable[[bytes, bytes], str]:
    """Say where two of QEMU's pictures differ in the rows from top up to bottom, as a box.

    The box is in the screen's own pixels, from its first changed one up to past its last, so
    it reads as the whole picture's would, and names nothing the band leaves out.
    """

    def where(before: bytes, now: bytes) -> str:
        was, now_band = band(before, top, bottom), band(now, top, bottom)
        if was.header != now_band.header:
            return " in its size"
        changed = [at for at in range(bottom - top) if was.row(at) != now_band.row(at)]
        if not changed:
            return ""
        columns = [
            column
            for at in changed
            for column in range(was.width)
            if was.row(at)[column * 3 : column * 3 + 3]
            != now_band.row(at)[column * 3 : column * 3 + 3]
        ]
        return (
            f" from {min(columns)},{top + changed[0]} to {max(columns) + 1},{top + changed[-1] + 1}"
        )

    return where


class Taps:
    """Taps sent at most one a gap, and whether each sent has been heard or had its gap to land.

    A tap sent before the compositor has the device is lost, and one sent after takes a while
    to be heard, so one goes each gap until one is heard; one sent just before it may still be
    on its way, and a count started then would miss it.
    """

    def __init__(self, gap: float, clock: Callable[[], float] = time.monotonic) -> None:
        """Send at most one tap each gap, in seconds, by the clock."""
        self.gap = gap
        self.clock = clock
        self.sent = 0
        self.last: float | None = None

    def due(self) -> bool:
        """Say whether the gap since the last tap sent has passed, as it has before the first."""
        return self.last is None or self.clock() - self.last >= self.gap

    def send(self, tap: Callable[[], object]) -> None:
        """Send a tap through tap, counted as tried before it goes and timed from once it has.

        Counted first, a tap whose write fails still counts as tried; timed after, a write that
        takes seconds on a busy host doesn't eat into the gap it has to land in.
        """
        self.sent += 1
        tap()
        self.last = self.clock()

    def landed(self, heard: int) -> bool:
        """Say whether every tap sent has been heard, or the last has had its gap to land in."""
        return heard >= self.sent or self.due()


def settled(  # noqa: PLR0913 # The clock is the stand-ins' way in; where and alike read pictures.
    screen: Callable[[], bytes],
    drawn: Callable[[], dict[str, int]],
    span: float,
    within: float,
    *,
    clock: Callable[[], float] = time.monotonic,
    where: Callable[[bytes, bytes], str] = nowhere,
    alike: Callable[[bytes, bytes], bool] = operator.eq,
) -> str | None:
    """Wait for the screen to stand still, giving None once it has, or why it never did.

    Still is the screen's picture, looked at over and over, staying the same for STILL_SPANS
    spans, each stretched with how busy the host is, since a busy host draws a fade slower
    than any fixed span (ZJ62K2, 2026-11-09). The picture, not the shell's count of frames,
    says it: an open launcher draws frames that change nothing (every gallery's launcher shots
    on 2026-11-09 counted some), and a fade changes the picture with every frame. The count
    says, when the screen never stands still, whether the shell drew as it changed, and
    where says where the picture last changed. Alike says when two pictures are the same, for
    a screen where something changes by itself by design, as a running clock's figures do.
    """
    deadline = clock() + within
    drawn()
    before = screen()
    since = clock()
    last = before
    while clock() < deadline:
        now = screen()
        if not alike(before, now):
            last, before, since = before, now, clock()
        elif clock() - since >= span * STILL_SPANS:
            return None
    frames = drawn()
    shell = f"the shell drawing {frames}" if frames else "the shell drawing nothing"
    return (
        f"the screen never stood still for {STILL_SPANS} spans of {span:.1f} s"
        f" in {within:.0f} s: it last changed{where(last, before)}, {shell} meanwhile"
    )

"""What VM tests' scripts share, so each says only what its own test is about.

A test's .nix gives this to its driver through `extraPythonPackages` (vm-helpers.nix), and
mypy finds it on the path core's shared linters set, as it finds driver_names.pyi. Each helper
takes the machine it acts on, since a module can't see the driver's globals.
"""

import datetime as dt
import operator
import os
import shlex
import time
from typing import TYPE_CHECKING, Any, Protocol

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

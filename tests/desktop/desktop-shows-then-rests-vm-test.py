"""The desktop test's shows then rests area.

Here the shell shows what changes and goes again, the meters that answer the keys and a
notice, and once nothing shows it draws nothing and barely wakes, behind the lock too.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it, so any test runs alone.
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
    FOLLOWED_THE_ROOM,
    LAUNCHER_OPEN,
    NOTICES,
    for_you_lit,
    for_you_read,
    retry,
)

if TYPE_CHECKING:
    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# How long a meter, a bar or a notice may take to go, once shown.
GOING = timedelta(seconds=10)
# How long a unit may take to say what's wanted after boot, or a picture to be painted.
SAYING = timedelta(seconds=30)


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def gone(desktop: Desktop, namespace: str) -> None:
    """Wait until niri no longer lists one of the shell's surfaces by its namespace."""
    desktop.machine.wait_until_fails(
        desktop.in_session(f"niri msg layers | grep -q {namespace}"), timeout=GOING
    )


def test_the_overview_draws_nothing_of_the_shells_and_the_launcher_says_how_things_stand(
    desktop: Desktop,
) -> None:
    """The overview draws nothing of the shell's, and the launcher says how things stand."""
    machine = desktop.machine
    desktop.as_account("niri msg action open-overview")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json overview-state | grep -q '\"is_open\":true'"),
        timeout=GOING,
    )
    desktop.shoot("overview")
    machine.fail(desktop.in_session("niri msg layers | grep -qw status"))
    desktop.as_account("niri msg action close-overview")
    # The time, the battery's charge and the cable, at least, as drawn once the keys beside
    # them are: each read by its glyph rather than a word, the battery saying its least, its
    # charge, before the network gives way.
    charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            re.match(r"\d\d:\d\d  ", shown["glance"])
            and f"\U000f0079 {charge}%  " in shown["glance"]
            and "\U000f0200" in shown["glance"]
        )
    )
    desktop.shoot("launcher-standing")
    # Typing leaves it as it was, so how things stand is always where Joel looks for it, a
    # row's note going to the field's end instead.
    desktop.type_in("zqxj")
    desktop.launcher_until(
        lambda shown: (
            shown["typed"] == "zqxj"
            and re.match(r"\d\d:\d\d  ", shown["glance"])
            and f"\U000f0079 {charge}%" in shown["glance"]
            and "\U000f0200" in shown["glance"]
        )
    )
    desktop.shoot("launcher-glance-typing")
    desktop.close_launcher()


def test_a_notification_appears(desktop: Desktop) -> None:
    """A notification appears, and leaves on its own."""
    with desktop.told("drawn notifications"):
        desktop.as_account("notify-send 'Build finished' 'the laptop is ready'")
    desktop.shoot("notification")
    # It leaves on its own, before the meters, whose pictures it would be in for as long as the
    # host takes to reach them.
    desktop.machine.wait_until_fails(desktop.in_session(NOTICES), timeout=SAYING)


def test_the_volume_shows_when_it_changes_then_goes(desktop: Desktop) -> None:
    """The volume shows when it changes, then goes."""
    # WirePlumber picks the defaults a moment after Pipewire lists the nodes.
    defaults = "wpctl inspect @DEFAULT_AUDIO_SINK@ && wpctl inspect @DEFAULT_AUDIO_SOURCE@"
    desktop.machine.wait_until_succeeds(desktop.in_session(defaults), timeout=SAYING)
    desktop.meter_shows("volume", "SINK", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 10%-")
    desktop.machine.succeed(
        desktop.in_session("wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q stand-in-speakers")
    )
    gone(desktop, "volume")


def test_the_microphone_shows_when_it_mutes_then_goes(desktop: Desktop) -> None:
    """The microphone shows when it mutes, then goes.

    The microphone's key mutes it through the same command, which niri runs.
    """
    toggle = "niri msg action spawn -- wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"
    desktop.meter_shows("microphone", "SOURCE", toggle)
    gone(desktop, "microphone")
    # Unmuted again, which the margin hears, so it's empty for what's next.
    since = desktop.mark()
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
    print(desktop.as_account("wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
    desktop.reported("holds margin", lambda rest: "muted" not in rest.split(), since)


def test_the_brightness_shows_when_the_keys_command_changes_it_then_goes(desktop: Desktop) -> None:
    """The brightness shows when the keys' command changes it, then goes.

    niri runs the brightness keys' command inside Joel's session, which is the one logind lets
    change a backlight, so the test has niri run it too. Twice, so the bar must say the pick
    just made, as the backlight daemon wrote it, not the one before.
    """

    def as_chosen(rest: str) -> bool:
        """Say whether the bar says the level the backlight daemon chose."""
        level = float(desktop.as_account("cat $XDG_RUNTIME_DIR/backlight-chosen"))
        return rest == f"{round(level * 100)}%"

    for share in [80, 30]:
        since = desktop.mark()
        desktop.as_account(f"niri msg action spawn -- brightnessctl --class=backlight set {share}%")
        desktop.machine.wait_until_succeeds(
            f"grep -qx {share} /sys/class/backlight/stand-in/brightness"
        )
        desktop.reported("level brightness ", as_chosen, since)
    desktop.shoot("brightness")
    gone(desktop, "brightness")


class Room:
    """The stand-in light sensor's room, and the lid it sits in."""

    def __init__(self, desktop: Desktop) -> None:
        """Light the room of that desktop's machine.

        Its sensor is a stand-in whose light is what the test writes to it, which it puts in
        the buffer udev turned on.
        """
        self.desktop = desktop
        named = desktop.machine.succeed("grep -lx als /sys/bus/iio/devices/*/name")
        self.sensor = named.strip().rsplit("/", 1)[0]
        desktop.machine.succeed(f"grep -qx 1 {self.sensor}/buffer/enable")

    def light(self, lux: int) -> None:
        """Light the stand-in sensor's room with so many lux."""
        self.desktop.machine.succeed(f"echo {lux * 1000} > {self.sensor}/in_illuminance_raw")

    def learned(self, _: bool) -> bool:  # As retry calls it.
        """Say whether the backlight has learned the one pick made in a dim room."""
        found = "grep -c '^2 0.100$' ~/.local/state/backlight/learned || true"
        return self.desktop.as_account(found).strip() == "1"

    def lid(self, *, shut: bool, said: str) -> None:
        """Shut or open the lid, and wait until the backlight says this of it."""
        self.desktop.lid(shut=shut)
        self.desktop.machine.wait_until_succeeds(
            self.desktop.in_session(
                f"journalctl --user --unit=backlight --no-pager --output=cat | grep -qF '{said}'"
            ),
            timeout=GOING,
        )


def test_the_screen_follows_the_light_and_the_bar_shows_only_for_picks(desktop: Desktop) -> None:
    """The screen follows the light, and the bar shows only for picks.

    Joel picks a dim screen in a dim room; as the room brightens the screen follows it, and
    the bar, which is for his picks, stays away.
    """
    room = Room(desktop)
    room.light(2)
    with desktop.told("drawn brightness"):
        desktop.as_account("niri msg action spawn -- brightnessctl --class=backlight set 10%")
    retry(room.learned, timeout=GOING)
    gone(desktop, "brightness")
    since = desktop.mark()
    room.light(30)
    retry(lambda _: desktop.brightness() > FOLLOWED_THE_ROOM, timeout=SAYING)
    after = f"--after-cursor={shlex.quote(since)}"
    shell = desktop.as_account(
        f"journalctl --user --unit=quickshell --no-pager --output=cat {after}"
    )
    assert "drawn brightness" not in shell, "the bar showed for the light"

    # The sensor sits in the lid, so logind's word that the lid shut reaches the backlight,
    # and opened, the screen follows the room again. That a shut lid's dark moves nothing is
    # its unit tests' (modules/backlight/src/follow.rs), which need no wait to show it.
    room.lid(shut=True, said="the lid is shut")
    held_at = desktop.brightness()
    room.lid(shut=False, said="the lid is open")
    room.light(1)
    retry(lambda _: desktop.brightness() < held_at, timeout=SAYING)


def test_the_monitor_follows_the_panels_brightness(desktop: Desktop) -> None:
    """The monitor follows the panel's brightness.

    The VM's monitor is a stand-in on a bus named as an MST hub's, which answers DDC/CI for
    its brightness as the desk's does. It follows the panel, at the same share of its own
    brightest, whatever moves the panel: a pick here, the room's light in the test before.
    """
    machine = desktop.machine
    machine.wait_until_succeeds(
        desktop.in_session(
            "journalctl --user --unit=backlight --no-pager --output=cat"
            " | grep -q 'the monitor on i2c-[0-9]* follows'"
        ),
        timeout=GOING,
    )
    panel_brightest = int(machine.succeed("cat /sys/class/backlight/stand-in/max_brightness"))

    def on_monitor(_: bool) -> bool:
        """Say whether the monitor is at the panel's share of its brightest."""
        shown = int(machine.succeed("cat /sys/module/stand_in_monitor/parameters/brightness"))
        return shown == round(desktop.brightness() / panel_brightest * 100)

    retry(on_monitor, timeout=GOING)
    for share in [70, 20]:
        desktop.as_account(f"niri msg action spawn -- brightnessctl --class=backlight set {share}%")
        machine.wait_until_succeeds(f"grep -qx {share} /sys/class/backlight/stand-in/brightness")
        retry(on_monitor, timeout=GOING)
    backlight_said = desktop.as_account(
        "journalctl --user --unit=backlight --no-pager --output=cat"
    )
    assert "can't set the monitor" not in backlight_said, "the monitor refused a level"
    assert "left alone" not in backlight_said, "the monitor didn't answer"


def test_the_charge_shows_when_power_goes_and_comes_back_then_goes(desktop: Desktop) -> None:
    """The charge shows when power goes and comes back, then goes."""
    for state in ["off", "on"]:
        with desktop.told("drawn power"):
            desktop.power(state)
        desktop.shoot(f"power-{state}")
        gone(desktop, "power")


# How long the shell is watched while nothing shows, and the most its main thread may wake in a
# second meanwhile (RSCH-20261029-0E8050's budgets).
QUIET = 30
# Ten of niri's callbacks to a surface it isn't drawing.
QUIET_LOCKED = 10
WAKEUPS_A_SECOND = 5


def woken(desktop: Desktop, pid: str) -> dict[str, int]:
    """Give how often each of the shell's threads has switched off its processor, by its name.

    A thread that ends while they're read, as a render thread does as the screens go off, is
    left out rather than failing the read.
    """
    said = desktop.as_account(
        f"for t in /proc/{pid}/task/*; do"
        " n=$(awk '/ctxt_switches/ { n += $2 } END { print n }' $t/status 2>/dev/null)"
        ' && name=$(cat $t/comm 2>/dev/null) && [ -n "$n" ]'
        ' && printf \'%s %s %s\\n\' "$name" "${t##*/}" "$n"; done; true'
    )
    threads: dict[str, int] = {}
    for line in said.splitlines():
        name, tid, count = line.rsplit(" ", 2)
        threads[f"{name} {tid}"] = int(count)
    return threads


def test_the_shell_draws_nothing_and_barely_wakes_while_nothing_shows(desktop: Desktop) -> None:
    """The shell draws no frame, and its main thread barely wakes, while nothing shows.

    With the launcher closed and nothing arriving, no window has anything new to draw, so
    each draws nothing (RSCH-20261029-0E8050's Motion and Drawing). It's watched from just
    after a minute turns, so the margin's clock has nothing to draw either.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=GOING)
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=GOING)
    pid = desktop.as_account("systemctl --user show quickshell -p MainPID --value").strip()
    machine.wait_until_succeeds("test $(date +%S) -lt 5", timeout=timedelta(seconds=70))
    desktop.as_account("qs ipc call frames start")
    before = woken(desktop, pid)
    # The span measured, not a wait for something to happen: what's counted is what the
    # shell does while nothing does.
    machine.sleep(duration=timedelta(seconds=QUIET))
    after = woken(desktop, pid)
    frames = json.loads(desktop.as_account("qs ipc call frames said"))
    rates = {
        thread: round((count - before.get(thread, 0)) / QUIET, 1)
        for thread, count in after.items()
        if count > before.get(thread, 0)
    }
    print(f"frames in {QUIET} s: {frames}")
    print(f"wakeups a second, by thread: {dict(sorted(rates.items(), key=lambda r: -r[1]))}")
    main = next(thread for thread in rates if thread.endswith(f" {pid}"))
    assert frames == {}, f"windows drew while nothing showed: {frames}"
    assert rates[main] < WAKEUPS_A_SECOND, f"the main thread woke {rates[main]} times a second"
    # None counted could also be a count that never heard a window, so one that draws is
    # counted too: the launcher, opening.
    desktop.as_account("qs ipc call frames start")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.close_launcher()
    opening = json.loads(desktop.as_account("qs ipc call frames said"))
    assert opening.get("launcher", 0) > 0, f"the launcher opened without a frame: {opening}"


# The launcher's closed budget For You's tray is held to, lit: under a hundredth of a core
# (WCJQSG, 2026-11-09), besides no frame and the wakeups above.
CORE_SHARE = 0.01


def spent(desktop: Desktop, pid: str) -> float:
    """Give the processor time the shell has taken so far, in seconds."""
    ticks, hertz = desktop.as_account(
        f"awk '{{ print $14 + $15 }}' /proc/{pid}/stat; getconf CLK_TCK"
    ).split()
    return int(ticks) / int(hertz)


def test_for_yous_tray_lit_costs_no_more_than_the_shell_at_rest(desktop: Desktop) -> None:
    """For You's tray lit, the shell draws nothing, barely wakes, and takes under 1% of a core.

    The tray is lit by what the shell already hears, with no watcher or clock of its own, so
    lit it costs what the margin at rest does. It's watched from just after a minute turns, as
    the shell at rest is, so the margin's clock has nothing to draw.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=GOING)
    for_you_lit(desktop, "Budget")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=GOING)
    pid = desktop.as_account("systemctl --user show quickshell -p MainPID --value").strip()
    machine.wait_until_succeeds("test $(date +%S) -lt 5", timeout=timedelta(seconds=70))
    desktop.as_account("qs ipc call frames start")
    before, began = woken(desktop, pid), spent(desktop, pid)
    # The span measured, not a wait for something to happen: what's counted is what the
    # shell does while the tray stands lit.
    machine.sleep(duration=timedelta(seconds=QUIET))
    after, share = woken(desktop, pid), (spent(desktop, pid) - began) / QUIET
    frames = json.loads(desktop.as_account("qs ipc call frames said"))
    main = next(thread for thread in after if thread.endswith(f" {pid}"))
    rate = (after[main] - before.get(main, 0)) / QUIET
    print(f"the tray lit, in {QUIET} s: {share:.2%} of a core, {rate:.1f} wakeups a second")
    assert frames == {}, f"windows drew while the tray stood lit: {frames}"
    assert share < CORE_SHARE, f"the shell took {share:.2%} of a core with the tray lit"
    assert rate < WAKEUPS_A_SECOND, f"the main thread woke {rate:.1f} times a second"
    for_you_read(desktop)


def test_the_shell_draws_nothing_behind_the_lock(desktop: Desktop) -> None:
    """The shell draws no frame of its own windows while the lock covers them.

    niri draws only the lock then, so it calls each other surface back for its next frame only
    once a second. Qt took a callback that late for a window gone out of sight and drew it
    whole when it came, which asked for the next: the launcher, margin and wallpaper each drew
    once a second for as long as Joel's screen stayed locked (2026-10-31).
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=GOING)
    desktop.as_account("lock-screen")
    try:
        # The frames owed as it locked, each called back within two seconds.
        machine.sleep(duration=timedelta(seconds=3))
        desktop.as_account("qs ipc call frames start")
        # The span measured, not a wait for something to happen.
        machine.sleep(duration=timedelta(seconds=QUIET_LOCKED))
        frames = json.loads(desktop.as_account("qs ipc call frames said"))
    finally:
        desktop.as_account("qs ipc call lock release")
    # The lock is a window of its own, which doesn't count its frames.
    assert frames == {}, f"windows drew in {QUIET_LOCKED} s behind the lock: {frames}"


# The status files a working machine's writers rewrite, and how often one of them is: lag-watch,
# the account's disk guard and spend-watch each every ten seconds, 18 writes a minute in all.
# Joel's machine wrote about 22 a minute to the files the shell reads on 2026-10-31, once the
# presence folder and the disk guard's drifting estimates are left out, as the shell and #1495
# leave them.
REWRITTEN = (
    "$XDG_RUNTIME_DIR/lag-watch/status.json",
    "$XDG_RUNTIME_DIR/disk-guard/status.json",
    "$XDG_RUNTIME_DIR/spend-watch/status.json",
)
REWRITE_EVERY = 10
# The threads that only draw: Qt's render loop, and the software rasteriser the VM draws with.
DRAWING = ("QSGRenderThread ", "llvmpipe-")


def test_the_shell_barely_wakes_behind_the_lock_as_its_files_are_written(
    desktop: Desktop,
) -> None:
    """The shell, every thread of it, wakes under five times a second behind the lock.

    Its files are written meanwhile as a working machine's are. Each read of one logged four
    lines nobody reads, as Quickshell kept every debug line of its own in its log file, each
    waking its logging thread: two of the locked shell's 14 wakes a second (2026-10-31).

    Idle turns Joel's screens off once it has locked, so nothing draws there. The VM's niri
    runs in a window of cage's and can't turn its screen off, so the lock goes on drawing its
    prompt's cursor here. So the threads that only draw are left out of the count: what the
    shell draws is held apart, by the subtests counting its frames.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=GOING)
    pid = desktop.as_account("systemctl --user show quickshell -p MainPID --value").strip()
    files = " ".join(REWRITTEN)
    # The first lock of a shell makes its window, which a loaded host can take past the
    # three seconds lock-screen waits for it.
    machine.wait_until_succeeds(desktop.in_session("lock-screen"), timeout=GOING)
    desktop.as_account(
        f"systemd-run --user --collect --unit=rewrites rewrites {REWRITE_EVERY} {files}"
    )
    try:
        machine.sleep(duration=timedelta(seconds=3))
        before = woken(desktop, pid)
        # The span measured, not a wait for something to happen.
        machine.sleep(duration=timedelta(seconds=QUIET))
        after = woken(desktop, pid)
    finally:
        desktop.as_account("systemctl --user stop rewrites")
        desktop.as_account("qs ipc call lock release")
    rates = {
        thread: round((count - before.get(thread, 0)) / QUIET, 1)
        for thread, count in after.items()
        if count > before.get(thread, 0)
    }
    print(f"wakeups a second behind the lock, by thread: {rates}")
    woke = round(sum(rate for thread, rate in rates.items() if not thread.startswith(DRAWING)), 1)
    assert woke < WAKEUPS_A_SECOND, f"the shell woke {woke} times a second: {rates}"
    # The shell's own log, found by its process, each line counted by its level and category.
    # qs colours what it reads out even into a pipe, which set its colours between a line's
    # level and category, so a search for the two together never found one.
    kept = desktop.as_account(
        f"qs log --no-color -r '*=true' $XDG_RUNTIME_DIR/quickshell/by-pid/{pid}/log.qslog"
        " | sed 's/^ *//' | cut -d: -f1 | sort | uniq -c | sort -rn"
    )
    print(f"what the shell's log kept, by level and category:\n{kept}")
    reads = sum(
        int(line.split()[0])
        for line in kept.splitlines()
        if line.split()[1:] == ["DEBUG", "quickshell.io.fileview"]
    )
    assert reads == 0, f"the shell kept {reads} lines of its files' reads"


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along.

    A binding that throws or assigns the wrong type only warns, and only once a surface draws
    it, which can be long after the start, so this comes last, and the gallery's picture of
    the whole desktop with it.
    """
    # Quickshell colours its levels and categories even in the journal, so a colour code may
    # stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
    desktop.shoot("desktop")

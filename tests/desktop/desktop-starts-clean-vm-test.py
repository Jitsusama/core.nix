"""The desktop test's starts clean area.

Here the session starts clean: its units, its shell, the meters that answer the keys, the theme
programs draw in, the touchscreen's swipes and Bluetooth's agent. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what
the test proves. Every test asks for the desktop, started once a run (desktop_session.py), and
leaves it as it found it, so any test runs alone.
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
    CAP_SYS_NICE,
    FOLLOWED_THE_ROOM,
    LAUNCHER_OPEN,
    NOTICES,
    PICTURE_COLOURS,
    retry,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

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


def test_the_machine_boots_without_an_ordering_cycle(desktop: Desktop) -> None:
    """The machine boots without an ordering cycle.

    A loop in the order units start breaks nothing at a switch, when the targets are already
    reached, so only a boot shows it.
    """
    desktop.machine.fail("journalctl -b -o cat | grep -F 'ordering cycle'")


def caps_of(status: str) -> dict[str, int]:
    """Give each capability set a process's status lists, by its name, as a number."""
    return {
        line.split(":")[0]: int(line.split()[1], 16)
        for line in status.splitlines()
        if line.startswith("Cap")
    }


def test_niri_keeps_cap_sys_nice_to_itself(desktop: Desktop) -> None:
    """Niri keeps CAP_SYS_NICE to itself.

    It holds it so the GPU draws its frames first, from the wrapper it starts through; that
    wrapper puts it in the inheritable and ambient sets too, which niri clears, so what it
    starts holds none of it, nor can take it up.
    """
    machine = desktop.machine
    compositor = machine.succeed(f"pgrep -x -u {desktop.account} niri | head -n 1").strip()
    niri_caps = caps_of(machine.succeed(f"cat /proc/{compositor}/status"))
    assert niri_caps["CapEff"] & (1 << CAP_SYS_NICE), niri_caps
    assert (niri_caps["CapInh"], niri_caps["CapAmb"]) == (0, 0)
    desktop.as_account(
        "niri msg action spawn -- sh -c 'grep ^Cap /proc/self/status > /tmp/child-caps'"
    )
    machine.wait_until_succeeds("test -s /tmp/child-caps", timeout=SAYING)
    offspring = caps_of(machine.succeed("cat /tmp/child-caps"))
    assert [offspring[name] for name in ("CapEff", "CapPrm", "CapInh", "CapAmb")] == [0, 0, 0, 0]


def test_every_user_manager_watches_its_own_pressure(desktop: Desktop) -> None:
    """Every user manager watches its own pressure.

    A user manager handed its whole service's pressure to watch can't set its own threshold
    on it, and says so each time it starts or reloads.
    """
    desktop.as_account("systemctl --user daemon-reload")
    desktop.machine.fail("journalctl -b -o cat | grep -F 'pressure threshold'")


def test_the_power_profile_follows_the_charger_from_boot(desktop: Desktop) -> None:
    """The power profile follows the charger from boot."""
    desktop.machine.wait_until_succeeds(
        "journalctl -b -o cat -u power-profile-follows-the-charger | grep -F 'power profile:'",
        timeout=SAYING,
    )


def test_the_memory_modules_are_written_out_at_boot_for_the_account_to_read(
    desktop: Desktop,
) -> None:
    """The memory modules are written out at boot for the account to read.

    Only root reads the firmware's memory devices, so a unit writes them out at boot for the
    launcher's Memory to read as the account, and QEMU lists the machine's memory as one.
    """
    desktop.machine.wait_until_succeeds("test -e /run/memory-modules.json", timeout=SAYING)
    memory = json.loads(desktop.as_account("cat /run/memory-modules.json"))
    print(memory)
    assert memory["modules"], memory
    assert all(module["size"] for module in memory["modules"]), memory


def test_quickshell_starts_without_a_qml_error(desktop: Desktop) -> None:
    """Quickshell starts without a QML error."""
    log = desktop.as_account("journalctl --user -u quickshell --no-pager -o cat")
    print(log)
    assert "ERROR" not in log, "Quickshell reported an error"
    assert "Error:" not in log, "Quickshell reported an error"


def test_the_wallpaper_stands_behind_the_workspaces(desktop: Desktop) -> None:
    """The wallpaper stands behind the workspaces, painted rather than a flat colour."""
    desktop.wait_for_surface("wallpaper")

    # It fades in once it's read, which takes longer on a slower machine.
    def painted(last: bool) -> bool:  # As retry calls it.
        """Say whether the workspace shows the picture rather than a flat colour."""
        _, colours, _ = desktop.colours_and_light("workspace")
        assert not last or colours > PICTURE_COLOURS, "the workspace shows a flat colour"
        return colours > PICTURE_COLOURS

    retry(painted, timeout=SAYING)
    # The picture to look at, once its fade has finished and the microphone's meter, which can
    # show as the session starts, has gone.
    desktop.machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q microphone"), timeout=SAYING
    )
    desktop.photograph("workspace")


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
    desktop.photograph("overview")
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
    desktop.photograph("launcher-standing")
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
    desktop.photograph("launcher-glance-typing")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=GOING)


def test_a_notification_appears(desktop: Desktop) -> None:
    """A notification appears, and leaves on its own."""
    with desktop.told("drawn notifications"):
        desktop.as_account("notify-send 'Build finished' 'the laptop is ready'")
    desktop.photograph("notification")
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
    desktop.photograph("brightness")
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
        self.desktop.machine.succeed(f"echo {int(shut)} > /sys/module/stand_in_lid/parameters/shut")
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
        desktop.photograph(f"power-{state}")
        gone(desktop, "power")


def test_the_desktop_says_it_prefers_dark(desktop: Desktop) -> None:
    """The desktop says it prefers dark, as Chrome and Slack ask the portal, for websites."""
    reply = desktop.as_account(
        "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
        " org.freedesktop.portal.Settings ReadOne ss org.freedesktop.appearance color-scheme"
    )
    assert "u 1" in reply


def test_programs_in_a_terminal_reach_the_clipboard(desktop: Desktop) -> None:
    """Programs in a terminal reach the clipboard."""
    # wl-copy stays behind to serve what it copied, so it lets go of the output the test reads.
    desktop.as_account("echo -n 'from a terminal' | wl-copy > /dev/null 2>&1")
    assert desktop.as_account("wl-paste --no-newline") == "from a terminal"


def test_monospace_is_the_themes_font(desktop: Desktop) -> None:
    """Monospace is the theme's font."""
    family = desktop.as_account(
        f"{desktop.values['fontconfig_bin']}/bin/fc-match -f '%{{family}}' monospace"
    )
    assert desktop.values["mono_family"] in family


def test_gtk_programs_draw_in_the_themes_colours(desktop: Desktop) -> None:
    """GTK programs, and the file chooser the portal draws, draw in the theme's colours."""
    values = desktop.values
    zenity = f"{values['zenity']}/bin/zenity --text-info --title=libadwaita"
    # Here niri runs in Cage's system service, and the portal refuses the programs it starts
    # ("Unable to open /proc/<pid>/root"), so zenity runs under the account's user manager,
    # where niri's own unit puts them on the laptop.
    desktop.as_account(f"systemd-run --user --collect {zenity} --filename={values['text']}")
    sunken = values["sunken"].lower()
    assert desktop.commonest_colour_in("libadwaita", "libadwaita", sunken) == sunken
    # The file chooser Chrome and Slack open, which the GTK portal draws with GTK 3, asked for
    # the way they ask.
    desktop.as_account(
        "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
        " org.freedesktop.portal.FileChooser OpenFile 'ssa{sv}' \"\" Open 0"
    )
    assert desktop.commonest_colour_in("Open", "file-chooser", sunken) == sunken


class Touchscreen:
    """The stand-in touchscreen, and what niri and the windows under it do with its swipes.

    wev says every touch its window gets.
    """

    def __init__(self, desktop: Desktop) -> None:
        """Swipe on that desktop's machine, as a person seeing its screen would.

        niri nested in Cage says its screen is Flipped180, and touches reach it upside down,
        so the stand-in swipes up and down the way a person seeing it would.
        """
        self.desktop = desktop
        outputs = json.loads(desktop.as_account("niri msg --json outputs"))
        turn = next(iter(outputs.values()))["logical"]["transform"]
        assert turn in {"Normal", "Flipped180"}, f"no swipes for a screen turned {turn}"
        self.over = {"DU": "UD", "UD": "DU"} if turn == "Flipped180" else {}

    def touches(self, kind: str) -> int:
        """Give how many touches of a kind wev has heard."""
        said = self.desktop.machine.succeed(f"grep -c 'wl_touch] {kind}' /tmp/wev.log || true")
        return int(said)

    def focused_workspace(self) -> int:
        """Give the index of the workspace niri has focused."""
        workspaces = json.loads(self.desktop.as_account("niri msg --json workspaces"))
        return cast("int", next(w["idx"] for w in workspaces if w["is_focused"]))

    def overview_open(self) -> bool:
        """Say whether niri's overview is open."""
        state = json.loads(self.desktop.as_account("niri msg --json overview-state"))
        return cast("bool", state["is_open"])

    def focused(self) -> int:
        """Give the id of the window niri has focused."""
        window = json.loads(self.desktop.as_account("niri msg --json focused-window"))
        return cast("int", window["id"])

    def swiped(
        self, fingers: int, way: str, until: Callable[[], object], across: float = 0.5
    ) -> None:
        """Swipe with so many fingers one way, as a person sees it, until something holds."""
        drawn = self.over.get(way, way)
        self.desktop.machine.succeed(f"echo '{fingers} {drawn} {across}' > /run/touchscreen")
        try:
            retry(lambda _: bool(until()), timeout=GOING)
        except Exception:
            print(self.desktop.machine.succeed("cat /tmp/wev.log | grep wl_touch || true"))
            raise


@pytest.fixture
def touchscreen(desktop: Desktop) -> Iterator[Touchscreen]:
    """Give the stand-in touchscreen, with wev's window maximized over a workspace above another.

    The workspace below holds a window, as it does once others have run. The touchscreen goes
    as the test ends.
    """
    machine = desktop.machine
    values = desktop.values
    desktop.opened("Below the swipes")
    desktop.as_account("niri msg action move-window-to-workspace-down")
    desktop.as_account("niri msg action focus-workspace-up")
    machine.succeed(f"systemd-run --unit=touchscreen {values['touchscreen']}")
    machine.wait_for_file("/run/touchscreen", timeout=SAYING)
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=wev -- sh -c "
        f"'{values['coreutils']}/bin/stdbuf -oL {values['wev']}/bin/wev > /tmp/wev.log 2>&1'"
    )
    retry(lambda _: any(window["title"] == "wev" for window in desktop.windows()), timeout=SAYING)
    desktop.as_account("niri msg action maximize-column")
    yield Touchscreen(desktop)
    machine.succeed("systemctl stop touchscreen")


def test_the_screens_warmth_starts_only_when_asked(desktop: Desktop) -> None:
    """What warms the screen starts only when the backlight daemon asks.

    It warms a built-in panel, and this machine names none, so it's never asked, and never
    fails where the compositor can't set gamma, as this nested one can't; the session's bus
    can still start it.
    """
    desktop.machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active backlight"), timeout=SAYING
    )
    desktop.as_account("busctl --user list --activatable | grep -q '^rs.wl-gammarelay '")
    state = desktop.as_account("systemctl --user show -P ActiveState wl-gammarelay").strip()
    assert state == "inactive", state


def test_swipes_on_the_touchscreen_move_through_the_workspaces_not_the_window(
    desktop: Desktop, touchscreen: Touchscreen
) -> None:
    """Swipes on the touchscreen move through the workspaces, not the window.

    Swipes move through the desktop as the touchpad's do, and never reach the window under
    them as a tap.
    """
    screen = touchscreen
    # One finger is the window's: wev hears it touched and let go.
    screen.swiped(1, "tap", lambda: screen.touches("up") == 1)
    first = screen.focused_workspace()
    screen.swiped(3, "DU", lambda: screen.focused_workspace() == first + 1)
    # The two fingers wev already had are cancelled, not let go.
    retry(lambda _: screen.touches("cancel") >= 1, timeout=GOING)
    screen.swiped(3, "UD", lambda: screen.focused_workspace() == first)
    screen.swiped(4, "DU", screen.overview_open)
    screen.swiped(4, "UD", lambda: not screen.overview_open())

    # A second window opens in a column to the right; the view follows the fingers back to the
    # first and on to the second again. Earlier windows share this workspace, so the two go to
    # the empty one at the end, where nothing else is beside them.
    left = screen.focused()
    workspaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    last = max(w["idx"] for w in workspaces)
    desktop.as_account(f"niri msg action move-column-to-workspace {last}")
    desktop.as_account(
        f"systemd-run --user --collect --quiet --unit=wev-right -- {desktop.values['wev']}/bin/wev"
    )
    retry(lambda _: screen.focused() != left, timeout=SAYING)
    right = screen.focused()
    assert desktop.held_still(), "the view never settled"
    screen.swiped(3, "LR", lambda: screen.focused() == left)
    screen.swiped(3, "RL", lambda: screen.focused() == right)
    assert screen.touches("up") == 1, "a swipe reached the window as a tap"

    # Side by side, with the right one focused: a swipe over the left one leaves the focus
    # where it was, as a tap would not. The left one hears the swipe's first two fingers land,
    # which is how the test knows they landed on it.
    desktop.as_account("niri msg action focus-column-left")
    desktop.as_account("niri msg action set-column-width 50%")
    desktop.as_account("niri msg action focus-column-right")
    desktop.as_account("niri msg action set-column-width 50%")
    assert desktop.held_still(), "the view never settled"
    # niri drops the workspace the column left only once its switch has ended, which
    # renumbers those below, so where the view is is read once it holds still.
    first = screen.focused_workspace()
    landed = screen.touches("down")
    screen.swiped(3, "DU", lambda: screen.focused_workspace() == first + 1, across=0.25)
    assert screen.touches("down") == landed + 2, "the swipe missed the window beside"
    screen.swiped(3, "UD", lambda: screen.focused_workspace() == first, across=0.25)
    assert screen.focused() == right, "a swipe focused the window under it"
    desktop.as_account("systemctl --user stop wev wev-right")

    # kitty scrolls by touch: a finger dragged down its whole window brings back what had
    # scrolled off the top, and the last line goes.
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=touched -- "
        "kitty --title touched sh -c 'seq 1 3000; exec sleep infinity'"
    )
    retry(
        lambda _: any(
            window["title"] == "touched" and window["is_focused"] for window in desktop.windows()
        ),
        timeout=timedelta(seconds=60),
    )
    desktop.as_account("niri msg action set-column-width 100%")
    kitty = f"kitty @ --to unix:@kitty-{desktop.window_titled('touched')['pid']} get-text"
    retry(lambda _: "3000" in desktop.as_account(kitty).split(), timeout=SAYING)
    screen.swiped(1, "UD", lambda: "3000" not in desktop.as_account(kitty).split())
    desktop.as_account("systemctl --user stop touched")


@pytest.fixture(scope="module")
def bluetooth(desktop: Desktop) -> Desktop:
    """Give the desktop with BlueZ started, as a radio would start it.

    BlueZ starts when a radio appears, which the VM has none of, so it's started once the
    kernel's Bluetooth is loaded. Its tests come last, since BlueZ starting midway disturbs the
    session's audio for the tests after it.
    """
    desktop.machine.succeed("modprobe bluetooth && systemctl start bluetooth.service")
    return desktop


def test_a_bluetooth_agent_answers_for_paired_devices(bluetooth: Desktop) -> None:
    """A Bluetooth agent answers for paired devices; with none, BlueZ refuses every pairing."""
    bluetooth.machine.wait_until_succeeds(
        "journalctl -u bluetooth-agent | grep -q 'Default agent requested'", timeout=SAYING
    )
    bluetooth.machine.succeed("systemctl is-active bluetooth-agent")


def test_the_bluetooth_agent_stops_at_once_handing_its_agent_back(bluetooth: Desktop) -> None:
    """The Bluetooth agent stops at once, handing its agent back.

    Taking its agent back from BlueZ is a call over the bus, which only an agent still reading
    the bus can make, and stopping it promptly is what keeps it from holding up every shutdown.
    """
    machine = bluetooth.machine
    machine.wait_until_succeeds("systemctl is-active bluetooth-agent", timeout=SAYING)
    machine.succeed("timeout 15 systemctl stop bluetooth-agent")
    machine.succeed("journalctl -u bluetooth-agent | grep -q 'unregistering agent'")
    machine.fail("systemctl is-failed --quiet bluetooth-agent")


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
    desktop.photograph("desktop")

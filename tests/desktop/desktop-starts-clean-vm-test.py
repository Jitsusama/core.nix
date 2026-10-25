"""The desktop test's starts clean area.

Here the session starts clean: its units, its shell, the meters that answer the keys, the theme
programs draw in, the touchscreen's swipes and Bluetooth's agent. tests/desktop.nix runs it
after desktop_prelude.py, the boot and the helpers every area shares, so the names it takes from
there are imported only for type checkers.
"""

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_prelude import (
        ACCOUNT,
        CAP_SYS_NICE,
        COREUTILS,
        FOLLOWED_THE_ROOM,
        FONTCONFIG_BIN,
        LAUNCHER_OPEN,
        MONO_FAMILY,
        PICTURE_COLOURS,
        STARTING,
        SUNKEN,
        TEXT,
        TOUCHSCREEN,
        WEV,
        ZENITY,
        as_account,
        brightness,
        colours_and_light,
        commonest_colour_in,
        held_still,
        in_session,
        launcher_ready,
        launcher_until,
        mark,
        meter_shows,
        photograph,
        power,
        reported,
        told,
        type_in,
        wait_for_surface,
        window_titled,
    )
    from driver_names import machine, retry, subtest, t

# A loop in the order units start breaks nothing at a switch, when the
# targets are already reached, so only a boot shows it.
with subtest("the machine boots without an ordering cycle"):
    machine.fail("journalctl -b -o cat | grep -F 'ordering cycle'")

# niri holds CAP_SYS_NICE, so the GPU draws its frames first, from the
# wrapper it starts through; that wrapper puts it in the inheritable
# and ambient sets too, which niri clears, so what it starts holds
# none of it, nor can take it up.
with subtest("niri keeps CAP_SYS_NICE to itself"):

    def caps_of(status: str) -> dict[str, int]:
        """Give each capability set a process's status lists, by its name, as a number."""
        return {
            line.split(":")[0]: int(line.split()[1], 16)
            for line in status.splitlines()
            if line.startswith("Cap")
        }

    compositor = machine.succeed(f"pgrep -x -u {ACCOUNT} niri | head -n 1").strip()
    niri_caps = caps_of(machine.succeed(f"cat /proc/{compositor}/status"))
    assert niri_caps["CapEff"] & (1 << CAP_SYS_NICE), niri_caps
    t.assertEqual((niri_caps["CapInh"], niri_caps["CapAmb"]), (0, 0))
    as_account("niri msg action spawn -- sh -c 'grep ^Cap /proc/self/status > /tmp/child-caps'")
    machine.wait_until_succeeds("test -s /tmp/child-caps", timeout=timedelta(seconds=30))
    offspring = caps_of(machine.succeed("cat /tmp/child-caps"))
    t.assertEqual(
        [offspring[name] for name in ("CapEff", "CapPrm", "CapInh", "CapAmb")], [0, 0, 0, 0]
    )

# A user manager handed its whole service's pressure to watch can't set
# its own threshold on it, and says so each time it starts or reloads.
with subtest("every user manager watches its own pressure"):
    as_account("systemctl --user daemon-reload")
    machine.fail("journalctl -b -o cat | grep -F 'pressure threshold'")

with subtest("the power profile follows the charger from boot"):
    machine.wait_until_succeeds(
        "journalctl -b -o cat -u power-profile-follows-the-charger | grep -F 'power profile:'",
        timeout=timedelta(seconds=30),
    )

# Only root reads the firmware's memory devices, so a unit writes them out at boot for the
# launcher's Memory to read as the account, and QEMU lists the machine's memory as one.
with subtest("the memory modules are written out at boot for the account to read"):
    machine.wait_until_succeeds("test -e /run/memory-modules.json", timeout=timedelta(seconds=30))
    memory = json.loads(as_account("cat /run/memory-modules.json"))
    print(memory)
    assert memory["modules"], memory
    assert all(module["size"] for module in memory["modules"]), memory

with subtest("Quickshell starts without a QML error"):
    log = as_account("journalctl --user -u quickshell --no-pager -o cat")
    print(log)
    assert "ERROR" not in log, "Quickshell reported an error"
    assert "Error:" not in log, "Quickshell reported an error"


with subtest("the wallpaper stands behind the workspaces"):
    wait_for_surface("wallpaper")

    # It fades in once it's read, which takes longer on a slower machine.
    def painted(last: bool) -> bool:
        """Say whether the workspace shows the picture rather than a flat colour."""
        _, colours, _ = colours_and_light("workspace")
        if last:
            t.assertGreater(
                colours, PICTURE_COLOURS, "the workspace shows a flat colour, not the picture"
            )
        return colours > PICTURE_COLOURS

    retry(painted, timeout=timedelta(seconds=30))
    # The picture to look at, once its fade has finished and the
    # microphone's meter, which can show as the session starts, has
    # gone.
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=30)
    )
    photograph("workspace")

with subtest("the overview draws nothing of the shell's, and the launcher says how things stand"):
    as_account("niri msg action open-overview")
    machine.wait_until_succeeds(
        in_session("niri msg --json overview-state | grep -q '\"is_open\":true'"),
        timeout=timedelta(seconds=10),
    )
    photograph("overview")
    machine.fail(in_session("niri msg layers | grep -qw status"))
    as_account("niri msg action close-overview")
    # The time, the battery's charge and the cable, at least, as
    # drawn once the keys beside them are: each read by its glyph
    # rather than a word, the battery saying its least, its charge,
    # before the network gives way.
    charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    launcher_until(
        lambda shown: (
            re.match(r"\d\d:\d\d  ", shown["glance"])
            and f"\U000f0079 {charge}%  " in shown["glance"]
            and "\U000f0200" in shown["glance"]
        )
    )
    photograph("launcher-standing")
    # Typing leaves it as it was, so how things stand is always where
    # Joel looks for it, a row's note going to the field's end instead.
    type_in("zqxj")
    launcher_until(
        lambda shown: (
            shown["typed"] == "zqxj"
            and re.match(r"\d\d:\d\d  ", shown["glance"])
            and f"\U000f0079 {charge}%" in shown["glance"]
            and "\U000f0200" in shown["glance"]
        )
    )
    photograph("launcher-glance-typing")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

with subtest("a notification appears"):
    with told("drawn notifications"):
        as_account("notify-send 'Build finished' 'the laptop is ready'")
    photograph("notification")
    # It leaves on its own, before the meters, whose pictures it would
    # be in for as long as the host takes to reach them.
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )


with subtest("the volume shows when it changes, then goes"):
    # WirePlumber picks the defaults a moment after Pipewire lists the nodes.
    defaults = "wpctl inspect @DEFAULT_AUDIO_SINK@ && wpctl inspect @DEFAULT_AUDIO_SOURCE@"
    machine.wait_until_succeeds(in_session(defaults), timeout=timedelta(seconds=30))
    meter_shows("volume", "SINK", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 10%-")
    machine.succeed(in_session("wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q stand-in-speakers"))
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q volume"), timeout=timedelta(seconds=10)
    )

# The microphone's key mutes it through the same command, which niri runs.
with subtest("the microphone shows when it mutes, then goes"):
    toggle = "niri msg action spawn -- wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"
    meter_shows("microphone", "SOURCE", toggle)
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=10)
    )
    # Unmuted again, which the margin hears, so it's empty for what's next.
    since = mark()
    as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
    print(as_account("wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
    reported("holds margin", lambda rest: "muted" not in rest.split(), since)

# niri runs the brightness keys' command inside Joel's session, which is
# the one logind lets change a backlight, so the test has niri run it too.
# Twice, so the bar must say the pick just made, as the backlight
# daemon wrote it, not the one before.
with subtest("the brightness shows when the keys' command changes it, then goes"):

    def as_chosen(rest: str) -> bool:
        """Say whether the bar says the level the backlight daemon chose."""
        level = float(as_account("cat $XDG_RUNTIME_DIR/backlight-chosen"))
        return rest == f"{round(level * 100)}%"

    for share in [80, 30]:
        since = mark()
        as_account(f"niri msg action spawn -- brightnessctl --class=backlight set {share}%")
        machine.wait_until_succeeds(f"grep -qx {share} /sys/class/backlight/stand-in/brightness")
        reported("level brightness ", as_chosen, since)
    photograph("brightness")
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q brightness"), timeout=timedelta(seconds=10)
    )

# The VM's light sensor is a stand-in whose light is what the test writes
# to it, which it puts in the buffer udev turned on. Joel picks a dim
# screen in a dim room; as the room brightens the screen follows it, and
# the bar, which is for his picks, stays away.
with subtest("the screen follows the light, and the bar shows only for picks"):
    named = machine.succeed("grep -lx als /sys/bus/iio/devices/*/name")
    sensor = named.strip().rsplit("/", 1)[0]
    machine.succeed(f"grep -qx 1 {sensor}/buffer/enable")

    def light(lux: int) -> None:
        """Light the stand-in sensor's room with so many lux."""
        machine.succeed(f"echo {lux * 1000} > {sensor}/in_illuminance_raw")

    def learned(_: bool) -> bool:
        """Say whether the backlight has learned the one pick made in a dim room."""
        found = "grep -c '^2 0.100$' ~/.local/state/backlight/learned || true"
        return as_account(found).strip() == "1"

    light(2)
    with told("drawn brightness"):
        as_account("niri msg action spawn -- brightnessctl --class=backlight set 10%")
    retry(learned, timeout=timedelta(seconds=10))
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q brightness"), timeout=timedelta(seconds=10)
    )
    since = mark()
    light(30)
    retry(lambda _: brightness() > FOLLOWED_THE_ROOM, timeout=timedelta(seconds=30))
    after = f"--after-cursor={shlex.quote(since)}"
    shell = as_account(f"journalctl --user --unit=quickshell --no-pager --output=cat {after}")
    t.assertNotIn("drawn brightness", shell, "the bar showed for the light")

    # The sensor sits in the lid, so logind's word that the lid shut
    # reaches the backlight, and opened, the screen follows the room
    # again. That a shut lid's dark moves nothing is its unit tests'
    # (modules/backlight/src/follow.rs), which need no wait to show it.
    def lid_says(said: str) -> None:
        """Wait until the backlight says this of the lid."""
        machine.wait_until_succeeds(
            in_session(
                f"journalctl --user --unit=backlight --no-pager --output=cat | grep -qF '{said}'"
            ),
            timeout=timedelta(seconds=10),
        )

    machine.succeed("echo 1 > /sys/module/stand_in_lid/parameters/shut")
    lid_says("the lid is shut")
    held_at = brightness()
    machine.succeed("echo 0 > /sys/module/stand_in_lid/parameters/shut")
    lid_says("the lid is open")
    light(1)
    retry(lambda _: brightness() < held_at, timeout=timedelta(seconds=30))

# The VM's monitor is a stand-in on a bus named as an MST hub's, which
# answers DDC/CI for its brightness as the desk's does. It follows the
# panel, at the same share of its own brightest, whatever moves the
# panel: a pick here, the room's light above.
with subtest("the monitor follows the panel's brightness"):
    machine.wait_until_succeeds(
        in_session(
            "journalctl --user --unit=backlight --no-pager --output=cat"
            " | grep -q 'the monitor on i2c-[0-9]* follows'"
        ),
        timeout=timedelta(seconds=10),
    )
    panel_brightest = int(machine.succeed("cat /sys/class/backlight/stand-in/max_brightness"))

    def on_monitor(_: bool) -> bool:
        """Say whether the monitor is at the panel's share of its brightest."""
        shown = int(machine.succeed("cat /sys/module/stand_in_monitor/parameters/brightness"))
        return shown == round(brightness() / panel_brightest * 100)

    retry(on_monitor, timeout=timedelta(seconds=10))
    for share in [70, 20]:
        as_account(f"niri msg action spawn -- brightnessctl --class=backlight set {share}%")
        machine.wait_until_succeeds(f"grep -qx {share} /sys/class/backlight/stand-in/brightness")
        retry(on_monitor, timeout=timedelta(seconds=10))
    backlight_said = as_account("journalctl --user --unit=backlight --no-pager --output=cat")
    t.assertNotIn("can't set the monitor", backlight_said, "the monitor refused a level")
    t.assertNotIn("left alone", backlight_said, "the monitor didn't answer")

with subtest("the charge shows when power goes and comes back, then goes"):
    for state in ["off", "on"]:
        with told("drawn power"):
            power(state)
        photograph(f"power-{state}")
        machine.wait_until_fails(
            in_session("niri msg layers | grep -q power"), timeout=timedelta(seconds=10)
        )

# Chrome and Slack ask the desktop portal, and websites hear it from them.
with subtest("the desktop says it prefers dark"):
    reply = as_account(
        "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
        " org.freedesktop.portal.Settings ReadOne ss org.freedesktop.appearance color-scheme"
    )
    t.assertIn("u 1", reply)

with subtest("programs in a terminal reach the clipboard"):
    # wl-copy stays behind to serve what it copied, so it lets go of the
    # output the test reads.
    as_account("echo -n 'from a terminal' | wl-copy > /dev/null 2>&1")
    t.assertEqual(as_account("wl-paste --no-newline"), "from a terminal")

with subtest("monospace is the theme's font"):
    family = as_account(f"{FONTCONFIG_BIN}/bin/fc-match -f '%{{family}}' monospace")
    t.assertIn(MONO_FAMILY, family)

with subtest("GTK programs draw in the theme's colours"):
    zenity = f"{ZENITY}/bin/zenity --text-info --title=libadwaita --filename={TEXT}"
    # Here niri runs in Cage's system service, and the portal refuses the
    # programs it starts ("Unable to open /proc/<pid>/root"), so zenity
    # runs under the account's user manager, where niri's own unit puts
    # them on the laptop.
    as_account(f"systemd-run --user --collect {zenity}")
    sunken = SUNKEN.lower()
    t.assertEqual(commonest_colour_in("libadwaita", "libadwaita", sunken), sunken)
    # The file chooser Chrome and Slack open, which the GTK portal draws
    # with GTK 3, asked for the way they ask.
    as_account(
        "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
        " org.freedesktop.portal.FileChooser OpenFile 'ssa{sv}' \"\" Open 0"
    )
    t.assertEqual(commonest_colour_in("Open", "file-chooser", sunken), sunken)

# Swipes on a touchscreen move through the desktop as the touchpad's do,
# and never reach the window under them as a tap. wev says every touch
# its window gets.
with subtest("swipes on the touchscreen move through the workspaces, not the window"):
    # The workspace below holds a window, as it does once others have run.
    as_account("niri msg action spawn -- kitty --title 'Below the swipes'")
    machine.wait_until_succeeds(
        in_session('niri msg --json windows | grep -qF \'"title":"Below the swipes"\''),
        timeout=STARTING,
    )
    as_account("niri msg action move-window-to-workspace-down")
    as_account("niri msg action focus-workspace-up")
    machine.succeed(f"systemd-run --unit=touchscreen {TOUCHSCREEN}")
    machine.wait_for_file("/run/touchscreen", timeout=timedelta(seconds=30))
    as_account(
        "systemd-run --user --collect --quiet --unit=wev -- sh -c "
        f"'{COREUTILS}/bin/stdbuf -oL {WEV}/bin/wev > /tmp/wev.log 2>&1'"
    )

    def wev_open(_: bool) -> bool:
        """Say whether wev's window is open."""
        windows = json.loads(as_account("niri msg --json windows"))
        return any(window["title"] == "wev" for window in windows)

    retry(wev_open, timeout=timedelta(seconds=30))
    as_account("niri msg action maximize-column")

    def touches(kind: str) -> int:
        """Give how many touches of a kind wev has heard."""
        return int(machine.succeed(f"grep -c 'wl_touch] {kind}' /tmp/wev.log || true"))

    def focused_workspace() -> int:
        """Give the index of the workspace niri has focused."""
        workspaces = json.loads(as_account("niri msg --json workspaces"))
        return cast("int", next(w["idx"] for w in workspaces if w["is_focused"]))

    def overview_open() -> bool:
        """Say whether niri's overview is open."""
        return cast("bool", json.loads(as_account("niri msg --json overview-state"))["is_open"])

    # niri nested in Cage says its screen is Flipped180, and touches
    # reach it upside down, so the stand-in swipes up and down the way a
    # person seeing it would.
    outputs = json.loads(as_account("niri msg --json outputs"))
    turn = next(iter(outputs.values()))["logical"]["transform"]
    assert turn in ("Normal", "Flipped180"), f"no swipes for a screen turned {turn}"
    over = {"DU": "UD", "UD": "DU"} if turn == "Flipped180" else {}

    def swiped(fingers: int, way: str, until: Callable[[], object], across: float = 0.5) -> None:
        """Swipe with so many fingers one way, as a person sees it, until something holds."""
        drawn = over.get(way, way)
        machine.succeed(f"echo '{fingers} {drawn} {across}' > /run/touchscreen")
        try:
            retry(lambda _: until(), timeout=timedelta(seconds=10))
        except Exception:
            print(machine.succeed("cat /tmp/wev.log | grep wl_touch || true"))
            raise

    # One finger is the window's: wev hears it touched and let go.
    swiped(1, "tap", lambda: touches("up") == 1)
    first = focused_workspace()
    swiped(3, "DU", lambda: focused_workspace() == first + 1)
    # The two fingers wev already had are cancelled, not let go.
    retry(lambda _: touches("cancel") >= 1, timeout=timedelta(seconds=10))
    swiped(3, "UD", lambda: focused_workspace() == first)
    swiped(4, "DU", overview_open)
    swiped(4, "UD", lambda: not overview_open())

    # A second window opens in a column to the right; the view follows
    # the fingers back to the first and on to the second again. Earlier
    # windows share this workspace, so the two go to the empty one at
    # the end, where nothing else is beside them.
    def focused() -> int:
        """Give the id of the window niri has focused."""
        return cast("int", json.loads(as_account("niri msg --json focused-window"))["id"])

    left = focused()
    workspaces = json.loads(as_account("niri msg --json workspaces"))
    last = max(w["idx"] for w in workspaces)
    as_account(f"niri msg action move-column-to-workspace {last}")
    as_account(f"systemd-run --user --collect --quiet --unit=wev-right -- {WEV}/bin/wev")
    retry(lambda _: focused() != left, timeout=timedelta(seconds=30))
    right = focused()
    t.assertTrue(held_still(), "the view never settled")
    swiped(3, "LR", lambda: focused() == left)
    swiped(3, "RL", lambda: focused() == right)
    t.assertEqual(touches("up"), 1, "a swipe reached the window as a tap")

    # Side by side, with the right one focused: a swipe over the left
    # one leaves the focus where it was, as a tap would not. The left one
    # hears the swipe's first two fingers land, which is how the test
    # knows they landed on it.
    as_account("niri msg action focus-column-left")
    as_account("niri msg action set-column-width 50%")
    as_account("niri msg action focus-column-right")
    as_account("niri msg action set-column-width 50%")
    t.assertTrue(held_still(), "the view never settled")
    # niri drops the workspace the column left only once its switch has
    # ended, which renumbers those below, so where the view is is read
    # once it holds still.
    first = focused_workspace()
    landed = touches("down")
    swiped(3, "DU", lambda: focused_workspace() == first + 1, across=0.25)
    t.assertEqual(touches("down"), landed + 2, "the swipe missed the window beside")
    swiped(3, "UD", lambda: focused_workspace() == first, across=0.25)
    t.assertEqual(focused(), right, "a swipe focused the window under it")
    as_account("systemctl --user stop wev wev-right")

    # kitty scrolls by touch: a finger dragged down its whole window
    # brings back what had scrolled off the top, and the last line goes.
    as_account(
        "systemd-run --user --collect --quiet --unit=touched -- "
        "kitty --title touched sh -c 'seq 1 3000; exec sleep infinity'"
    )

    def touched_focused(_: bool) -> bool:
        """Say whether the touched terminal is open and focused."""
        windows = json.loads(as_account("niri msg --json windows"))
        return any(w["title"] == "touched" and w["is_focused"] for w in windows)

    retry(touched_focused, timeout=timedelta(seconds=60))
    as_account("niri msg action set-column-width 100%")
    kitty = f"kitty @ --to unix:@kitty-{window_titled('touched')['pid']} get-text"
    retry(lambda _: "3000" in as_account(kitty).split(), timeout=timedelta(seconds=30))
    swiped(1, "UD", lambda: "3000" not in as_account(kitty).split())
    as_account("systemctl --user stop touched")
    machine.succeed("systemctl stop touchscreen")

# With no agent BlueZ refuses every pairing, even one asked for here.
# BlueZ starts when a radio appears, which the VM has none of, so it's
# started as a radio would, once the kernel's Bluetooth is loaded. It
# comes last, since BlueZ starting midway disturbs the session's audio
# for the subtests after it.
with subtest("a Bluetooth agent answers for paired devices"):
    machine.succeed("modprobe bluetooth && systemctl start bluetooth.service")
    machine.wait_until_succeeds(
        "journalctl -u bluetooth-agent | grep -q 'Default agent requested'",
        timeout=timedelta(seconds=30),
    )
    machine.succeed("systemctl is-active bluetooth-agent")

# Taking its agent back from BlueZ is a call over the bus, which only
# an agent still reading the bus can make, and stopping it promptly
# is what keeps it from holding up every shutdown.
with subtest("the Bluetooth agent stops at once, handing its agent back"):
    machine.succeed("timeout 15 systemctl stop bluetooth-agent")
    machine.succeed("journalctl -u bluetooth-agent | grep -q 'unregistering agent'")
    machine.fail("systemctl is-failed --quiet bluetooth-agent")

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

photograph("desktop")

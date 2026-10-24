"""The shell's surfaces through a screen going and coming back, in a virtual machine.

tests/screens.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the helpers and the values,
runs whichever subtests are chosen. Each subtest takes the screen as the one before left it,
so they're meant to run in order.
"""

import json
import shlex
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

# The account greetd signs in, as screens.nix declares it.
values = json.loads(Path("@values@").read_text())
account: str = values["account"]


def in_niri(command: str) -> str:
    """Give a command, run as the account inside its niri session, to run on the machine."""
    surroundings = (
        "export XDG_RUNTIME_DIR=/run/user/1000"
        " DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus;"
        " export $(systemctl --user show-environment | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)=');"
    )
    return f"su - {account} -c " + shlex.quote(f"{surroundings} {command}")


def surfaces() -> set[str]:
    """Give the shell's surfaces niri holds now, by namespace."""
    held = json.loads(machine.succeed(in_niri("niri msg --json layers")))
    return {layer["namespace"] for layer in held}


def wait_for_surfaces(*namespaces: str, timeout: int = 60) -> None:
    """Wait until niri holds a surface for each namespace, saying which never came."""
    wanted = " && ".join(
        f"niri msg --json layers | grep -qF {shlex.quote(json.dumps(name))}" for name in namespaces
    )
    status, _ = machine.execute(
        f"timeout {timeout} sh -c "
        + shlex.quote(f"until {in_niri(wanted)} > /dev/null 2>&1; do sleep 1; done"),
        timeout=timeout + 30,
    )
    if status != 0:
        machine.screenshot("missing-surfaces")
    missing = set(namespaces) - surfaces()
    assert not missing, f"niri holds no surface for {sorted(missing)}"


def plug(status: str) -> None:
    """Force the screen's connector to a status and tell niri, as a cable coming or going does.

    The VM's display driver doesn't watch its connectors, so a forced status alone announces
    nothing; niri looks at the card's screens again on any change to it.
    """
    machine.succeed(
        f"echo {status} > {connector}/status"
        " && udevadm trigger --action=change --settle /sys/class/drm/card0"
    )


def displays_say(*, named: bool) -> None:
    """Wait until desk-facts' displays.json names the screen, or no longer names it."""
    facts = "/run/user/1000/desk/displays.json"
    machine.wait_until_succeeds(
        f"test -s {facts} && {'' if named else '! '}grep -qF '\"Virtual-1\"' {facts}", timeout=30
    )
    machine.log(f"displays.json: {machine.succeed(f'cat {facts}')}")


machine.wait_for_unit("multi-user.target")
wait_for_surfaces("wallpaper", "launcher", timeout=300)
connector = machine.succeed("ls -d /sys/class/drm/card0-Virtual-1").strip()

with subtest("a critical notice is up before the screen goes"):
    machine.succeed(
        in_niri("notify-send --urgency=critical 'Still here' 'A notice that never goes by itself'")
    )
    wait_for_surfaces("notifications")

with subtest("the margin shows a mark, and dictation listens, before the screen goes"):
    # Keep Awake is a mark the margin keeps for as long as it's on.
    machine.succeed(in_niri("systemctl --user start caffeinate.service"))
    # The stand-in listens from the first press until the next.
    machine.succeed(in_niri("dictation toggle"))
    wait_for_surfaces("margin", "dictation")

with subtest("desk-facts says what the screen is before it goes"):
    displays_say(named=True)

with subtest("the screen goes, and niri lets it go"):
    plug("off")
    machine.wait_until_succeeds(in_niri("niri msg --json outputs | grep -qx '{}'"), timeout=60)

with subtest("desk-facts no longer names the screen once it's gone"):
    displays_say(named=False)

with subtest("the screen comes back, and the launcher and the notice with it"):
    plug("detect")
    machine.wait_until_succeeds(in_niri("niri msg outputs | grep -qF Virtual-1"), timeout=60)
    wait_for_surfaces("wallpaper", "launcher", "notifications")

with subtest("the margin's mark and dictation's bar come back with the screen"):
    wait_for_surfaces("margin", "dictation")

# desk-facts naming the screen again isn't held here: the VM's display driver reads the
# screen's EDID only when QEMU says its screens changed, so a forced detect brings the
# connector back with an empty EDID, and desk-facts rightly says nothing of a screen with
# none. A real DisplayPort replug reads the EDID afresh.


def niri_answers(after: str) -> None:
    """Fail, saying where each of niri's threads waits, unless niri answers."""
    answered, _ = machine.execute(in_niri("timeout 10 niri msg --json layers"), timeout=30)
    waits = ""
    if answered != 0:
        _, waits = machine.execute(
            "for t in /proc/$(pgrep -o niri)/task/*; do"
            ' echo "$(cat $t/comm): $(cat $t/wchan)"; cat $t/stack; done',
            timeout=60,
        )
    assert answered == 0, f"niri stopped answering after {after}:\n{waits}"


with subtest("a console switch away and back keeps them all"):
    console = machine.succeed("cat /sys/class/tty/tty0/active").strip().removeprefix("tty")
    machine.succeed(f"chvt 2 && sleep 2 && chvt {console}")
    niri_answers("a console switch")
    wait_for_surfaces("wallpaper", "launcher", "notifications", "margin", "dictation")

with subtest("the machine sleeps and wakes at a key, and keeps them all"):
    sleeps = machine.succeed("cat /sys/power/mem_sleep").strip()
    assert "[s2idle]" in sleeps, f"the machine sleeps by {sleeps}, not as a laptop does"
    # A key wakes a laptop, so the keyboard may wake the machine.
    machine.succeed("for w in /sys/bus/serio/devices/*/power/wakeup; do echo enabled > $w; done")
    machine.succeed("systemctl suspend")
    machine.wait_for_console_text("PM: suspend entry", timeout=60)
    # A key while it's still going to sleep would only stop it going, and
    # the wait is the driver's, since the sleeping machine runs nothing.
    time.sleep(5)
    machine.send_key("shift")
    machine.wait_for_console_text("PM: suspend exit", timeout=60)
    slept = machine.succeed("journalctl --boot --identifier=systemd-sleep --output=cat")
    assert "System returned from sleep" in slept, f"the machine never slept: {slept}"
    niri_answers("a sleep")
    wait_for_surfaces("wallpaper", "launcher", "notifications", "margin", "dictation")


def wait_for_no_surface(namespace: str, timeout: int = 60) -> None:
    """Wait until niri holds no surface for a namespace, saying it stayed."""
    deadline = time.monotonic() + timeout
    while namespace in surfaces():
        assert time.monotonic() < deadline, f"niri still holds a surface for {namespace!r}"
        time.sleep(1)


# A surface drawn again says nothing of what it hears. Each of these goes
# by what the shell is told, over the bus, a socket or a unit's state, so
# a connection that died in the sleep and wasn't made again would leave it
# standing.

with subtest("after the wake, a notice still arrives"):
    machine.succeed(in_niri("notify-send 'After the wake' 'Heard over the bus'"))
    machine.wait_until_succeeds(
        in_niri(
            "journalctl --user --unit=quickshell --output=cat"
            " | grep -qF 'report[shows card After the wake]'"
        ),
        timeout=30,
    )

with subtest("after the wake, dictation still hears its keys, and its bar goes when it's done"):
    # The second press types, then says it heard nothing, and the bar goes.
    machine.succeed(in_niri("dictation toggle"))
    wait_for_no_surface("dictation")

with subtest("after the wake, Keep Awake let go takes its mark away"):
    # With nothing else on, the margin goes with its last mark.
    machine.succeed(in_niri("systemctl --user stop caffeinate.service"))
    wait_for_no_surface("margin")

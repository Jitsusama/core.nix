"""The shell's surfaces through a screen going and coming back, in a virtual machine.

tests/screens.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the helpers and the values,
runs whichever subtests are chosen. Each subtest takes the screen as the one before left it,
so they're meant to run in order.
"""

import json
import shlex
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


machine.wait_for_unit("multi-user.target")
wait_for_surfaces("wallpaper", "launcher", timeout=300)
connector = machine.succeed("ls -d /sys/class/drm/card0-Virtual-1").strip()

with subtest("a critical notice is up before the screen goes"):
    machine.succeed(
        in_niri("notify-send --urgency=critical 'Still here' 'A notice that never goes by itself'")
    )
    wait_for_surfaces("notifications")

with subtest("the screen goes, and niri lets it go"):
    plug("off")
    machine.wait_until_succeeds(in_niri("niri msg --json outputs | grep -qx '{}'"), timeout=60)

with subtest("the screen comes back, and the launcher and the notice with it"):
    plug("detect")
    machine.wait_until_succeeds(in_niri("niri msg outputs | grep -qF Virtual-1"), timeout=60)
    wait_for_surfaces("wallpaper", "launcher", "notifications")

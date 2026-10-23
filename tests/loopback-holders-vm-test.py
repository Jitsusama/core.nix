"""Which programs have a loopback camera open, as the file naming them says.

tests/loopback-holders.nix runs this, and its opening comment says what has to hold. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the boot and the helpers, runs
whichever subtests are chosen. The values it needs from Nix, the file naming the holders and
the programs that hold the camera, come in one JSON file, its path put in place of the marker.
"""

import json
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

values = json.loads(Path("@values@").read_text())
# The file naming who holds the stand-in camera.
file: str = values["file"]
bash: str = values["bash"]
sleep: str = values["sleep"]
# A program as Nix wraps one, which waits until it's stopped.
doze: str = values["doze"]
# Takes the name it's given, then holds the camera it's given open.
renamed: str = values["renamed"]

machine.wait_for_unit("stand-in-holders.service")
machine.wait_for_file(file)
device = machine.succeed("cat /run/stand-in/device").strip()
machine.succeed(f"udevadm wait --timeout 10 {device}")


def holders() -> list[str]:
    """Give the names the file holds, one a line."""
    return machine.succeed(f"cat {file}").split("\n")[:-1]


def hold(unit: str, command: str, extra: str = "") -> None:
    """Run a command as someone else, in a unit of its own, while the test goes on."""
    machine.succeed(f"systemd-run --unit={unit} --uid=someone -p Type=exec {extra} -- {command}")


def holding(program: str, camera: str) -> str:
    """Give how to run a program with the camera it's given held open, for hold to run."""
    return f"{bash} -c 'exec 3< {camera}; exec {program} 300'"


with subtest("the relay holding the camera isn't named"):
    relay = machine.succeed("systemctl show -P MainPID stand-in.service").strip()
    machine.wait_until_succeeds(f"readlink /proc/{relay}/fd/* | grep -q -x -F {device}", timeout=10)
    machine.sleep(1)
    t.assertEqual(holders(), [])

with subtest("each program another account has it open with is named"):
    hold("held", holding(sleep, device))
    hold("dozing", holding(doze, device))
    machine.wait_until_succeeds(f"grep -q -x doze {file}", timeout=10)
    t.assertEqual(holders(), ["doze", "sleep"])
    mode = machine.succeed(f"stat -c %a {file}").strip()
    t.assertEqual(mode, "644")

with subtest("it empties once they close it"):
    machine.succeed("systemctl stop held.service dozing.service")
    machine.wait_until_succeeds(f"! test -s {file}", timeout=10)

with subtest("a program named as the relay is named, and a name is one line"):
    hold("posing", f"{renamed} stand-in-relay {device}")
    hold("odd", f"'{renamed}' 'a\\nb\\xff' {device}")
    machine.wait_until_succeeds(f"grep -q -x stand-in-relay {file}", timeout=10)
    machine.wait_until_succeeds(f"grep -q '^a?b' {file}", timeout=10)
    named = holders()
    t.assertEqual(len(named), 2, named)
    t.assertIn("stand-in-relay", named)
    machine.succeed("systemctl stop posing.service odd.service")
    machine.wait_until_succeeds(f"! test -s {file}", timeout=10)

with subtest("a holder that leaves the camera to its child is replaced by it"):
    hold(
        "forking",
        f"{bash} -c 'exec 3< {device}; {doze} & {sleep} 1'",
        # So the child outlives the parent's unit, as it would its shell.
        "-p KillMode=process",
    )
    machine.wait_until_succeeds(f"grep -q -x doze {file}", timeout=10)
    machine.wait_until_succeeds(f"! grep -q -x bash {file}", timeout=10)
    t.assertEqual(holders(), ["doze"])
    machine.succeed("pkill -u someone -x .doze-wrapped")
    machine.wait_until_succeeds(f"! test -s {file}", timeout=10)

with subtest("its folder outlives the camera's service, and says nobody while it's down"):
    folder = machine.succeed("stat -c %i /run/camera-holders").strip()
    hold("stopping", holding(sleep, device))
    machine.wait_until_succeeds(f"grep -q -x sleep {file}", timeout=10)
    # The camera stops while the program still holds it, so only the
    # service's stopping can empty the file.
    machine.succeed("systemctl stop stand-in.service")
    machine.wait_until_fails("systemctl is-active stand-in-holders.service", timeout=10)
    t.assertEqual(machine.succeed(f"cat {file}"), "")
    machine.succeed("systemctl stop stopping.service")
    machine.succeed("systemctl start stand-in.service")
    machine.wait_for_unit("stand-in-holders.service")
    t.assertEqual(machine.succeed("stat -c %i /run/camera-holders").strip(), folder)

with subtest("it follows the camera to the next loopback"):
    machine.succeed("systemctl restart stand-in.service")
    machine.wait_for_unit("stand-in-holders.service")
    device = machine.succeed("cat /run/stand-in/device").strip()
    machine.succeed(f"udevadm wait --timeout 10 {device}")
    hold("again", holding(sleep, device))
    machine.wait_until_succeeds(f"grep -q -x sleep {file}", timeout=10)
    machine.succeed("systemctl stop again.service")

with subtest("another camera's loopback isn't taken for it"):
    other = machine.succeed("v4l2loopback-ctl add -n 'Other'").strip()
    machine.succeed(f"udevadm wait --timeout 10 {other}")
    machine.succeed("systemctl stop stand-in-holders.service")
    machine.succeed(f"echo {other} > /run/stand-in/device")
    machine.succeed("systemctl start stand-in-holders.service")
    hold("elsewhere", holding(sleep, other))
    machine.sleep(4)
    t.assertEqual(holders(), [])
    machine.succeed("systemctl stop elsewhere.service")
    machine.succeed(f"echo {device} > /run/stand-in/device")

with subtest("it waits while the camera isn't there, and finds it when it is"):
    machine.succeed("systemctl stop stand-in-holders.service")
    machine.succeed("echo /dev/video-not-yet > /run/stand-in/device")
    machine.succeed("systemctl start stand-in-holders.service")
    machine.sleep(4)
    machine.succeed("systemctl is-active stand-in-holders.service")
    restarts = machine.succeed("systemctl show -P NRestarts stand-in-holders.service")
    t.assertEqual(restarts.strip(), "0")
    t.assertEqual(holders(), [])
    machine.succeed(f"echo {device} > /run/stand-in/device")
    hold("found", holding(sleep, device))
    machine.wait_until_succeeds(f"grep -q -x sleep {file}", timeout=10)
    machine.succeed("systemctl stop found.service")

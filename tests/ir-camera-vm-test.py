"""A stand-in IR camera relayed to the programs that read it, and to no others.

tests/ir-camera.nix runs this, and its opening comment says what has to hold. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the boot and the stand-in camera's
start, runs whichever subtests are chosen. The frames' size, the one value it needs from Nix,
comes in a JSON file, its path put in place of the marker.
"""

import datetime as dt
import json
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

values = json.loads(Path("@values@").read_text())
width: int = values["width"]
height: int = values["height"]

machine.wait_for_unit("ir-camera.service")
machine.wait_for_file("/dev/ir-camera")

with subtest("the relay waits for udev to give it the loopback"):
    restarts = machine.succeed("systemctl show -P NRestarts ir-camera.service")
    t.assertEqual(restarts.strip(), "0")
    machine.fail("journalctl -b -o cat -u ir-camera.service | grep -q 'Permission denied'")

source = machine.succeed("v4l2loopback-ctl add -n 'Stand-in IR' -x 1 -b 2").strip()
machine.succeed(f"udevadm wait --timeout 10 {source}")
machine.succeed(f"systemd-run --unit=stand-in -p Type=exec stand-in-camera {source}")
machine.wait_for_file("/run/stand-in-camera")
relay = machine.succeed("systemctl show -P MainPID ir-camera.service").strip()
holds_camera = f"readlink /proc/{relay}/fd/* | grep -q -x -F {source}"

with subtest("the camera stays closed until a program reads"):
    machine.fail(holds_camera)

with subtest("programs get grey frames, and browsers don't take it for a camera"):
    video_format = machine.succeed("v4l2-ctl -d /dev/ir-camera --get-fmt-video")
    t.assertIn("'GREY'", video_format)
    t.assertIn(f"{width}/{height}", video_format)
    capabilities = machine.succeed("v4l2-ctl -d /dev/ir-camera --info")
    t.assertIn("Video Output", capabilities.split("Device Caps")[1])

with subtest("a reader gets the camera's frames in grey"):
    machine.succeed(
        "v4l2-ctl -d /dev/ir-camera --stream-mmap --stream-count=10 --stream-to=/tmp/read"
    )
    names = machine.succeed("frames /tmp/read").split()
    print(f"The reader got: {' '.join(names)}")
    t.assertNotIn("other", names)
    t.assertEqual(names[-1], "pattern", " ".join(names))

with subtest("a paused camera's next reader gets no frame from before it opened"):
    machine.succeed("touch /run/stand-in-paused; sleep 1")
    machine.execute(
        f"timeout 3 v4l2-ctl -d {source} --stream-mmap --stream-count=1 --stream-to=/tmp/stale"
    )
    stale = int(machine.succeed("stat -c %s /tmp/stale 2>/dev/null || echo 0"))
    t.assertEqual(stale, 0, "A reader opening a paused camera got the frame from before it")
    machine.succeed("rm /run/stand-in-paused")

with subtest("the camera closes when the reader stops"):
    machine.wait_until_succeeds(f"! {holds_camera}", timeout=30)
    machine.wait_until_succeeds(
        "journalctl -b -o cat -u ir-camera.service | grep -q 'The reader stopped'", timeout=30
    )

with subtest("the next reader never gets the last one's frames"):
    machine.succeed("systemctl stop stand-in.service")
    machine.execute(
        ": > /tmp/next; timeout 5 v4l2-ctl -d /dev/ir-camera --stream-mmap --stream-count=1"
        " --stream-to=/tmp/next"
    )
    names = machine.succeed("frames /tmp/next").split()
    t.assertNotIn("pattern", names, "The next reader got the last one's frames")

with subtest("a relay started with its loopback gone waits for it rather than retrying"):
    machine.succeed("systemctl stop ir-camera.service")
    machine.succeed("touch /tmp/before-gone")
    machine.succeed("v4l2loopback-ctl delete $(readlink /run/ir-camera-loopback/sink)")
    machine.succeed("systemctl start --no-block ir-camera.service")
    # Bound to the loopback's device, the relay's start waits for it, so the
    # relay itself never runs to fail.
    machine.wait_until_succeeds(
        "systemctl list-jobs --no-legend ir-camera.service | grep -q waiting",
        timeout=dt.timedelta(seconds=30),
    )
    # The holders watcher says nobody as the node goes, then looks for it
    # once a second without a word, so two looks follow the saying.
    machine.wait_until_succeeds(
        "test /run/camera-holders/ir-camera -nt /tmp/before-gone",
        timeout=dt.timedelta(seconds=30),
    )
    machine.sleep(dt.timedelta(seconds=2))
    restarts = machine.succeed("systemctl show -P NRestarts ir-camera.service").strip()
    t.assertEqual(restarts, "0", "The relay kept retrying a loopback that had gone")
    holders = machine.succeed("systemctl show -P NRestarts ir-camera-holders.service")
    t.assertEqual(holders.strip(), "0", "Saying who holds the camera failed while it was gone")
    machine.succeed("systemctl restart ir-camera-loopback.service ir-camera.service")
    machine.wait_for_unit("ir-camera.service")
    machine.wait_for_unit("ir-camera-holders.service")
    t.assertEqual(machine.succeed("cat /run/camera-holders/ir-camera"), "")

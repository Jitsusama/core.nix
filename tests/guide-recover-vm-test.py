"""A machine recovered from a repair stick in a virtual machine, its data left in place.

tests/guide-recover.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every subtest
stays at the top level under its own name, and what sits outside them, the overlays, the keys'
state files, the boot and the helpers, runs whichever subtests are chosen. What the script needs
from Nix, the machine's disk image, the repair stick, the keys' state files, the PIN and the
recovery key, comes in one JSON file, whose path guide-recover.nix writes in place of the marker
at the top.
"""

import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest, t

values = json.loads(Path("@values@").read_text())
qemu_img: str = values["qemuImg"]
image: str = values["image"]
written: str = values["stick"]
# The state files of the kept, lost and new keys, made before QEMU starts.
states: str = values["keys"]
pin: str = values["pin"]
recovery: str = values["recovery"]

# The machine writes to an overlay, since the image is in the store. Each overlay is deleted once
# it's closed, so it's held open for the whole script rather than in a with block, which would
# take every subtest off the top level.
disk = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
# qemu-img and the images are Nix's own store paths, so nothing untrusted reaches the call.
subprocess.run(  # noqa: S603
    [qemu_img, "create", "-f", "qcow2", "-b", image, "-F", "qcow2", disk.name],
    check=True,
)
os.environ["NIX_DISK_IMAGE"] = disk.name
# The stick writes to an overlay too, since its image is in the store. The name is the stick's
# node's, which the script never starts: the machine boots the stick's image from USB. Held open
# and called as the disk's overlay is, for the same reasons.
stick = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
subprocess.run(  # noqa: S603
    [qemu_img, "create", "-f", "qcow2", "-b", written, "-F", "raw", stick.name],
    check=True,
)
os.environ["STICK_IMAGE"] = stick.name
keys = tempfile.mkdtemp()
for name in ["kept", "lost", "new"]:
    shutil.copy(f"{states}/{name}", f"{keys}/{name}")
    Path(f"{keys}/{name}").chmod(0o600)
os.environ["KEYS_DIR"] = keys


def screen() -> str:
    """Give the text on the guide's console."""
    return machine.get_tty_text("1")


def on_screen(pattern: str, timeout: int = 120) -> None:
    """Wait until the guide's screen shows the pattern, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the screen shows the pattern, raising if it shows the guide stopped."""
        text = screen()
        if re.search(r"(?i)(try again|look again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, text) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout)
        except Exception:
            print("SCREEN\n" + screen())
            print("LOG\n" + machine.execute("tail -n 40 /run/guide/log")[1])
            raise


def enter(text: str) -> None:
    """Type the text at the guide, and Return after it."""
    machine.send_chars(text + "\n")


def plug(name: str) -> None:
    """Swap the key in the machine for the one of that name, waiting for each to come and go."""
    machine.send_monitor_command("device_del key")
    machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=30)
    machine.send_monitor_command(f"device_add canokey,id=key,bus=xhci.0,file={keys}/{name}")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=30)


volume = "/dev/disk/by-partlabel/system"
plugin = "LD_LIBRARY_PATH=$(dirname $(readlink -f $(which systemd-cryptsetup)))/../lib/cryptsetup"


def opens(name: str) -> bool:
    """Plug in the key of that name, and say whether it opens the disk with the PIN."""
    plug(name)
    return (
        machine.execute(
            f"echo {pin} | {plugin} cryptsetup open --test-passphrase --token-only"
            f" --token-type systemd-fido2 {volume}"
        )[0]
        == 0
    )


machine.start()

with subtest("the machine's disk opens with two YubiKeys, as a machine's does"):
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=60)
    for name in ["kept", "lost"]:
        if name != "kept":
            plug(name)
        machine.succeed(f"set-pin {pin}")
        machine.succeed(
            f"install -d -m 0700 /run/pins && printf {pin} > /run/pins/cryptenroll.fido2-pin"
        )
        machine.succeed(
            f"PASSWORD='{recovery}' CREDENTIALS_DIRECTORY=/run/pins"
            " systemd-cryptenroll --fido2-device=auto"
            f" --fido2-with-client-pin=yes --fido2-with-user-presence=yes {volume} >&2"
        )
    plug("kept")

with subtest("a repair stick boots into the guide's recovery"):
    on_screen("Recover a machine")
    on_screen("Check the keyboard")
    enter("keyboard")

with subtest("the machine's disk is found by its layout and opened with a YubiKey's PIN"):
    on_screen("holds the machine")
    t.assertIn("named system", screen())
    enter("")
    on_screen("Or plug in a YubiKey")
    enter(pin)

with subtest("everything the machine mounts is mounted under /mnt"):
    # The menu, which comes only once every mount is done: "disk is
    # open" is shown as soon as the disk opens, before they are.
    on_screen("What should be done")
    for at in ["/mnt", "/mnt/nix", "/mnt/home", "/mnt/var/log", "/mnt/boot"]:
        machine.succeed(f"findmnt {at}")
    machine.succeed("test -e /mnt/etc/NIXOS")
    # What bootctl lists from outside the machine, for the guide's
    # fixtures (qemu-canokey).
    listed = machine.succeed("bootctl --esp-path=/mnt/boot list --json=short")
    print("FIXTURE bootctl-list-from-the-stick.json " + listed.replace("\n", "\\n"))

with subtest("a generation chosen is set to boot next, and the menu comes back"):
    on_screen("What should be done")
    machine.send_key("down")
    enter("")
    on_screen("Which should it boot next")
    t.assertIn("Generation 1", screen())
    enter("")
    on_screen("What should be done")
    t.assertIn("boots generation 1 next", screen())
    # The firmware's own default, which systemd-boot reads first.
    machine.succeed(
        "test -e /sys/firmware/efi/efivars/LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"
    )

with subtest(
    "a lost YubiKey is replaced: the new one enrolled and proven, then the lost one's slot wiped"
):
    machine.send_key("down")
    machine.send_key("down")
    enter("")
    on_screen("the YubiKey you still have")
    enter("")
    on_screen("plug in the new YubiKey")
    plug("new")
    machine.succeed(f"set-pin {pin}")
    enter("")
    on_screen("the new YubiKey's PIN")
    enter(pin)
    on_screen("the recovery key")
    enter(recovery)
    on_screen("What should be done", timeout=180)
    t.assertIn("the lost one no longer does", screen())
    header = json.loads(machine.succeed(f"cryptsetup luksDump --dump-json-metadata {volume}"))
    kinds = sorted(token["type"] for token in header["tokens"].values())
    t.assertEqual(kinds, ["systemd-fido2", "systemd-fido2"])
    t.assertTrue(opens("new"), "the new key doesn't open the disk")
    t.assertTrue(opens("kept"), "the kept key no longer opens the disk")
    t.assertFalse(opens("lost"), "the lost key still opens the disk")

with subtest("an ESP that lost its boot loader gets it again from the installed system"):
    machine.succeed("rm -r /mnt/boot/EFI/systemd /mnt/boot/loader/entries")
    for _ in range(3):
        machine.send_key("down")
    # What the menu looks like on a console, its keys drawn by its font.
    machine.screenshot("menu")
    enter("")
    on_screen("installed again", timeout=180)
    machine.succeed("test -s /mnt/boot/EFI/systemd/systemd-bootx64.efi")
    machine.succeed("ls /mnt/boot/loader/entries/*.conf")

with subtest("Return gives a root shell inside the installed system"):
    enter("")
    machine.sleep(3)
    enter("cat /etc/hostname > /recovered && nixos-version >> /recovered")
    machine.wait_until_succeeds("test -s /mnt/recovered", timeout=30)
    said = machine.succeed("cat /mnt/recovered")
    print(said)
    t.assertIn("machine", said.splitlines()[0])

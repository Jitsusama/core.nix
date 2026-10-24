"""A stick written on a Linux host by the guide, as the person signed in there and not root.

tests/guide-host.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the stick's overlay, the boot and the
helpers, runs whichever subtests are chosen. What the script needs from Nix, the used stick and
the tool to put an overlay on it, the old stick's bundle and the passphrase of the host's own,
comes in one JSON file, whose path guide-host.nix writes in place of the marker at the top.
"""

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import host, retry, subtest

values = json.loads(Path("@values@").read_text())
qemu_img: str = values["qemuImg"]
used: str = values["used"]
desktop_bundle: str = values["desktopBundle"]
bundle_passphrase: str = values["bundlePassphrase"]

# How many words the guide makes the stick's passphrase of.
PASSPHRASE_WORDS = 7

# The host writes to an overlay, since the used stick is in the store. The overlay is deleted
# once it's closed, so it's held open for the whole script rather than in a with block, which
# would take every subtest off the top level.
stick = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
# qemu-img and the used stick are Nix's own store paths, so nothing untrusted reaches the call.
subprocess.run(  # noqa: S603
    [qemu_img, "create", "-f", "qcow2", "-b", used, "-F", "raw", stick.name, "6G"],
    check=True,
)
os.environ["STICK_IMAGE"] = stick.name


def screen() -> str:
    """Give what the guide shows on the first console."""
    return host.get_tty_text("1")


def on_screen(pattern: str, timeout: int = 120) -> None:
    """Wait until the guide's screen matches the pattern, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the screen matches the pattern, or fail if the guide stopped."""
        text = screen()
        if re.search(r"(?i)(try again|look again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, text) is not None

    with host.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout)
        except Exception:
            # What the host was at when the wait ran out: the screen, the
            # guide's own log, and the journal's last.
            print("SCREEN\n" + screen())
            print("LOG\n" + host.execute("tail -n 60 /home/alice/.local/state/guide/log")[1])
            print("JOURNAL\n" + host.execute("journalctl -b -n 80 -o cat")[1])
            raise


def type_in(text: str) -> None:
    """Type the text at the guide, and Return."""
    host.send_chars(text + "\n")


host.wait_for_unit("multi-user.target")

with subtest("the stick, once used, has a partition the desktop mounted"):
    host.succeed("udevadm settle")
    # udisks lets only someone signed in at the machine mount a stick, and
    # takes alice for someone at another seat until her sign-in on the
    # first console is open, which multi-user.target doesn't wait for.
    host.wait_until_succeeds('test "$(loginctl show-user alice --property=State --value)" = active')
    host.succeed("su alice -c 'udisksctl mount --block-device /dev/sda1'")
    host.succeed("findmnt /dev/sda1")

with subtest("the bundle the host keeps for the machine is opened with its passphrase"):
    on_screen("Write laptop's stick")
    # Sealed to no YubiKey, since the payload carries no recipients.
    on_screen("which opens")
    type_in(bundle_passphrase)

with subtest("the one stick is named, by what it is"):
    on_screen("is the stick to write")
    type_in("")

with subtest("the passphrase is seven words shown alone for a phone, then typed back"):
    on_screen("Copy All")
    type_in("")
    retry(lambda _: len(screen().split()) == PASSPHRASE_WORDS, 30)
    words = " ".join(screen().split())
    type_in("")
    on_screen("reading it off your phone")
    # Another's words are refused and left to be put right, and
    # nothing is written.
    other = "abacus abdomen abdominal abide abiding ability ablaze"
    type_in(other)
    on_screen("that isn't it")
    assert other in screen(), screen()
    host.fail("sfdisk --json /dev/sda | grep -q store-verity")
    for _ in other:
        host.send_key("backspace")
    # The words are read whatever their case.
    type_in(words.upper())

with subtest("the stick is written as the person, and read back"):
    on_screen("The stick is written", timeout=900)
    host.fail("findmnt /dev/sda1")
    # Written by alice's guide, never as root.
    host.succeed("pgrep -u alice -f 'guide host'")
    host.fail("pgrep -u root -f 'guide host'")
    verified = host.succeed("sfdisk --verify /dev/sda")
    assert "No errors detected" in verified, verified
    # From the disk itself, not udev's record of what was there before.
    listed = json.loads(host.succeed("sfdisk --json /dev/sda"))
    names = [p.get("name") for p in listed["partitiontable"]["partitions"]]
    assert names == ["esp", "store-verity", "store", "vault"], names

with subtest("the old stick's bundles and the host's own are kept, and carried on"):
    host.succeed("udevadm settle && mkdir -p /run/vault && mount -o ro /dev/sda4 /run/vault")
    kept = "/home/alice/.local/state/guide/rescue"
    for owner, bundle in [
        ("desktop", desktop_bundle),
        ("laptop", "/var/lib/guide/rescue-bundle.json"),
    ]:
        host.succeed(f"cmp {bundle} /run/vault/{owner}-rescue-bundle.json")
        host.succeed(f"cmp {bundle} {kept}/{owner}-rescue-bundle.json")
    host.succeed(f"test $(stat -c %U {kept}) = alice")
    host.succeed("umount /run/vault")
    # The old vault was mounted for alice to be read, not its disk
    # opened, and unmounted again: UDisks2 says so only as it takes
    # the mount point away.
    host.succeed(
        "journalctl -u udisks2 --grep"
        " 'Cleaning up mount point /run/media/alice/.* \\(device 8:2 is not mounted\\)'"
    )

with subtest("the stick's boot program is signed with the machine's db key, from its bundle"):
    host.succeed("mkdir -p /run/esp && mount -o ro /dev/sda1 /run/esp")
    host.succeed("sbverify --cert /var/lib/sbctl/keys/db/db.pem /run/esp/EFI/BOOT/BOOTX64.EFI")
    host.succeed("umount /run/esp")

"""A stick written on Linux by the guide, as the person signed in there and not root.

guide-writer.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The guide's screens
come one after another, so each state a test starts from is a fixture that walks the guide one
screen further than the fixture it asks for: the used stick mounted, the writer's bundle opened,
the stick named, its passphrase typed back, the stick written, then written again keeping it. A
fixture's work is done once a run, so any test runs alone with `-k`, walking only as far as it
needs. What the tests need from Nix, the used stick and the tool to put an overlay on it, the
old stick's bundle and the passphrase of the writer's own, are guide-writer.nix's, given as the
`values` fixture.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import BaseMachine, QemuMachine

# How many words the guide makes the stick's passphrase of.
PASSPHRASE_WORDS = 7
# How long a screen of the guide may take to come, and the write with its read back, which
# took 14 minutes on a machine busy with other machines' disks.
SCREEN = timedelta(seconds=120)
WRITTEN = timedelta(minutes=30)
# Another's words, refused when typed back.
OTHER = "abacus abdomen abdominal abide abiding ability ablaze"


def screen(writer: QemuMachine) -> str:
    """Give what the guide shows on the first console."""
    return writer.get_tty_text("1")


def on_screen(writer: QemuMachine, pattern: str, timeout: timedelta = SCREEN) -> None:
    """Wait until the guide's screen matches the pattern, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the screen matches the pattern, or fail if the guide stopped."""
        text = screen(writer)
        if re.search(r"(?i)(try again|look again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, text) is not None

    with writer.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout=timeout)
        except Exception:
            # What the writer was at when the wait ran out: the screen, the
            # guide's own log, and the journal's last.
            print("SCREEN\n" + screen(writer))
            print("LOG\n" + writer.execute("tail -n 60 /home/alice/.local/state/guide/log")[1])
            print("JOURNAL\n" + writer.execute("journalctl -b -n 80 -o cat")[1])
            raise


def type_in(writer: QemuMachine, text: str) -> None:
    """Type the text at the guide, and Return."""
    writer.send_chars(text + "\n")


@pytest.fixture(scope="module")
def overlay(values: dict[str, Any]) -> Generator[str]:
    """Give the overlay the writer writes to, the used stick in the store beneath it.

    guide-writer.nix plugs it in as the writer's stick, so it's made before the writer starts.
    """
    with tempfile.NamedTemporaryFile(suffix=".qcow2") as stick:
        # qemu-img and the used stick are Nix's own store paths, so nothing untrusted
        # reaches the call.
        backing = ["-b", values["used"], "-F", "raw"]
        subprocess.run(  # noqa: S603
            [values["qemuImg"], "create", "-f", "qcow2", *backing, stick.name, "6G"],
            check=True,
        )
        os.environ["STICK_IMAGE"] = stick.name
        yield stick.name


@pytest.fixture(scope="module")
def writer(machines: dict[str, BaseMachine], overlay: str) -> QemuMachine:
    """Give the writer booted, its stick plugged in."""
    del overlay
    booted = cast("QemuMachine", machines["host"])
    booted.wait_for_unit("multi-user.target")
    return booted


@pytest.fixture(scope="module")
def mounted(writer: QemuMachine) -> QemuMachine:
    """Mount the used stick's first partition as the desktop does, for alice."""
    writer.succeed("udevadm settle")
    # udisks lets only someone signed in at the machine mount a stick, and
    # takes alice for someone at another seat until her sign-in on the
    # first console is open, which multi-user.target doesn't wait for.
    writer.wait_until_succeeds(
        'test "$(loginctl show-user alice --property=State --value)" = active'
    )
    writer.succeed("su alice -c 'udisksctl mount --block-device /dev/sda1'")
    return writer


@pytest.fixture(scope="module")
def bundle_opened(mounted: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Open the bundle the writer keeps for the machine with its passphrase."""
    on_screen(mounted, "Write laptop's stick")
    # Sealed to the key too, so its PIN opens it, the key touched by
    # itself as a canokey is.
    on_screen(mounted, r"type\s+its\s+PIN")
    type_in(mounted, values["pin"])
    return mounted


@pytest.fixture(scope="module")
def named(bundle_opened: QemuMachine) -> QemuMachine:
    """Take the one stick the guide names."""
    on_screen(bundle_opened, "is the stick to write")
    type_in(bundle_opened, "")
    return bundle_opened


@pytest.fixture(scope="module")
def passphrase(named: QemuMachine) -> str:
    """Read the passphrase off its own screen, and go on to where it's typed back."""
    on_screen(named, "Copy All")
    type_in(named, "")
    retry(lambda _: len(screen(named).split()) == PASSPHRASE_WORDS, timeout=timedelta(seconds=30))
    words = " ".join(screen(named).split())
    type_in(named, "")
    on_screen(named, "reading it off your phone")
    return words


@pytest.fixture(scope="module")
def refused(named: QemuMachine, passphrase: str) -> QemuMachine:
    """Type back another's words, which the guide refuses and leaves to be put right."""
    del passphrase
    type_in(named, OTHER)
    on_screen(named, "that isn't it")
    return named


@pytest.fixture(scope="module")
def written(refused: QemuMachine, passphrase: str) -> QemuMachine:
    """Put the words right, whatever their case, and wait until the stick's written."""
    for _ in OTHER:
        refused.send_key("backspace")
    type_in(refused, passphrase.upper())
    on_screen(refused, "The stick is written", timeout=WRITTEN)
    return refused


def test_used_stick_has_a_partition_the_desktop_mounted(mounted: QemuMachine) -> None:
    """The stick, once used, has a partition the desktop mounted."""
    mounted.succeed("findmnt /dev/sda1")


def test_writers_bundle_opens_with_its_keys_pin(bundle_opened: QemuMachine) -> None:
    """The bundle the writer keeps for the machine, sealed to a real key, opens with its PIN."""
    on_screen(bundle_opened, "is the stick to write")


def test_one_stick_is_named_by_what_it_is(named: QemuMachine) -> None:
    """The one stick is named, by what it is."""
    on_screen(named, "Copy All")


def test_passphrase_is_seven_words_typed_back(refused: QemuMachine) -> None:
    """The passphrase is seven words shown alone for a phone, then typed back."""
    # Another's words stay to be put right, and nothing is written.
    assert OTHER in screen(refused), screen(refused)
    refused.fail("sfdisk --json /dev/sda | grep -q store-verity")


def test_stick_is_written_as_the_person_and_read_back(written: QemuMachine) -> None:
    """The stick is written as the person, and read back."""
    written.fail("findmnt /dev/sda1")
    # Held awake through the write, by alice's guide, against sleep,
    # idling and the lid, for as long as the guide runs.
    held = written.succeed("systemd-inhibit --list --no-legend --no-pager")
    assert re.search(
        r"guide +1000 +alice .*sleep:idle:handle-lid-switch +writing a stick +block", held
    ), held
    # Written by alice's guide, never as root.
    written.succeed("pgrep -u alice -f 'guide host'")
    written.fail("pgrep -u root -f 'guide host'")
    verified = written.succeed("sfdisk --verify /dev/sda")
    assert "No errors detected" in verified, verified
    # From the disk itself, not udev's record of what was there before.
    listed = json.loads(written.succeed("sfdisk --json /dev/sda"))
    names = [p.get("name") for p in listed["partitiontable"]["partitions"]]
    assert names == ["esp", "store-verity", "store", "vault"], names


def test_bundles_are_kept_and_carried_on(written: QemuMachine, values: dict[str, Any]) -> None:
    """The old stick's bundles and the writer's own are kept, and carried on."""
    written.succeed("udevadm settle && mkdir -p /run/vault && mount -o ro /dev/sda4 /run/vault")
    kept = "/home/alice/.local/state/guide/rescue"
    for owner, bundle in [
        ("desktop", values["desktopBundle"]),
        ("laptop", "/var/lib/guide/rescue-bundle.json"),
    ]:
        written.succeed(f"cmp {bundle} /run/vault/{owner}-rescue-bundle.json")
        written.succeed(f"cmp {bundle} {kept}/{owner}-rescue-bundle.json")
    written.succeed(f"test $(stat -c %U {kept}) = alice")
    written.succeed("umount /run/vault")
    # The old vault was mounted for alice to be read, not its disk
    # opened, and unmounted again: UDisks2 says so only as it takes
    # the mount point away.
    written.succeed(
        "journalctl -u udisks2 --grep"
        " 'Cleaning up mount point /run/media/alice/.* \\(device 8:2 is not mounted\\)'"
    )


def test_boot_program_is_signed_with_the_machines_db_key(written: QemuMachine) -> None:
    """The stick's boot program is signed with the machine's db key, from its bundle."""
    written.succeed("mkdir -p /run/boot && mount -o ro /dev/sda1 /run/boot")
    written.succeed("sbverify --cert /var/lib/sbctl/keys/db/db.pem /run/boot/EFI/BOOT/BOOTX64.EFI")
    written.succeed("umount /run/boot")


@pytest.fixture(scope="module")
def rewritten(written: QemuMachine, passphrase: str, values: dict[str, Any]) -> QemuMachine:
    """Write the stick over again, keeping the passphrase its vault has."""
    type_in(written, "")
    written.wait_until_fails("pgrep -u alice -f 'guide host'")
    type_in(written, "write-the-stick-again")
    # Its passphrase opens it as well, tried before the key that's still
    # plugged in, so none of the key's PIN tries is spent.
    on_screen(written, r"type\s+its\s+PIN")
    type_in(written, values["bundlePassphrase"])
    # Whether to keep it comes first, as a step of its own, then the stick.
    on_screen(written, "has its passphrase already")
    # Keeping it is the first choice, and taken with Return.
    type_in(written, "")
    on_screen(written, "is the stick to write")
    type_in(written, "")
    on_screen(written, r"which\s+it\s+keeps")
    type_in(written, passphrase)
    on_screen(written, "The stick is written", timeout=WRITTEN)
    return written


def test_a_stick_written_again_keeps_its_passphrase(rewritten: QemuMachine) -> None:
    """Written again, the stick keeps its passphrase, the old words proven against its vault."""
    # The rewrite asked for the words it keeps rather than showing new
    # ones, and wrote only once they opened the old vault's key.
    verified = rewritten.succeed("sfdisk --verify /dev/sda")
    assert "No errors detected" in verified, verified

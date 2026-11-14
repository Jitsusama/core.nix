"""A machine installed by the guide beside another system's partitions, in a virtual machine.

guide-beside.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Each state a test
starts from is a fixture that walks the machine one stage further than the fixture it asks for:
the stick booted, its vault opened, the partition confirmed, the Nano enrolled, the recovery key
kept, the system installed, the machine's first boot by the Nano's PIN, and a boot by the
recovery words with no key plugged in. What the tests need from Nix, the key's state, the
stick's image, the Asahi-shaped disk and what it held, are guide-beside.nix's, given as the
`values` fixture.
"""

from __future__ import annotations

import contextlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import QemuMachine

# systemd saying the disk opened, perhaps coloured.
OPENED = r"Finished (?:\x1b\[[0-9;]*m)*Cryptography Setup for system"
RECOVERY_KEY_WORDS = 7
DISK = "/dev/vda"
LUKS = "CA7D7CCB-63ED-4C53-861C-1742536059CC"
# As guide-install-vm-test.py paces its keys, for the same reason.
KEY_PACE = timedelta(milliseconds=50)
COPIED = timedelta(minutes=15)
SCREEN = timedelta(seconds=120)
STEP = timedelta(seconds=300)
PLUGGED = timedelta(seconds=30)
INSTALLED = timedelta(minutes=30)


@dataclass(frozen=True)
class Rig:
    """What QEMU's devices read, made before the machine starts.

    The key's state, the stick's overlay and a blank stick for once it's out, and the
    machine's disk over Asahi's layout.
    """

    keys: str
    stick: str
    blank: str
    disk: str


def screen(machine: QemuMachine) -> str:
    """Give what the guide's console shows."""
    return machine.get_tty_text("1")


def on_screen(
    machine: QemuMachine,
    pattern: str,
    timeout: timedelta = SCREEN,
    stopped: str = r"try again|look again",
) -> None:
    """Wait until the pattern is on the guide's screen, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the pattern is on the screen, failing if the guide says it stopped."""
        text = screen(machine)
        if re.search(f"(?i)({stopped})", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, " ".join(text.split())) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout=timeout)
        except Exception:
            print("SCREEN\n" + screen(machine))
            print("LOG\n" + machine.execute("tail -n 60 /run/guide/log")[1])
            raise


def enter(machine: QemuMachine, text: str) -> None:
    """Type the text at the guide and press Return."""
    machine.send_chars(text + "\n", delay=KEY_PACE)


def unlock(machine: QemuMachine, secret: str) -> None:
    """Answer the disk's prompts with the secret until it opens."""
    machine.wait_for_console_text(r"Please enter|Starting password query on /dev/ttyS0")
    for _ in range(4):
        machine.send_console(secret + "\n")
        with contextlib.suppress(Exception):
            machine.wait_for_console_text(OPENED, timeout=timedelta(seconds=30))
            return
    message = "the disk never opened"
    raise RuntimeError(message)


def overlay(qemu_img: str, file: str, *backing: str, size: str | None = None) -> None:
    """Make a qcow2 overlay over an image, or a blank one of this size."""
    # qemu-img and the images are Nix's own store paths or the test's own files.
    subprocess.run(
        [qemu_img, "create", "-f", "qcow2", *backing, file, *([size] if size else [])],
        check=True,
    )


def table(json_text: str) -> dict[int, dict[str, Any]]:
    """Give a table's entries by partition number, from what sfdisk printed."""
    entries = json.loads(json_text)["partitiontable"]["partitions"]
    numbered = {}
    for entry in entries:
        number = re.search(r"(\d+)$", entry["node"])
        if number is None:
            message = f"sfdisk named a partition {entry['node']}, with no number"
            raise ValueError(message)
        numbered[int(number.group(1))] = entry
    return numbered


@pytest.fixture(scope="module")
def rig(values: dict[str, Any]) -> Generator[Rig]:
    """Give the key's state, the sticks' overlays and the machine's disk, made before QEMU."""
    with (
        tempfile.TemporaryDirectory() as keys,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as stick,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as blank,
        tempfile.TemporaryDirectory() as disks,
    ):
        shutil.copy(f"{values['canokeys']}/nano", f"{keys}/nano")
        Path(f"{keys}/nano").chmod(0o600)
        os.environ["KEYS_DIR"] = keys
        overlay(values["qemuImg"], stick.name, "-b", values["written"], "-F", "raw")
        os.environ["STICK_IMAGE"] = stick.name
        overlay(values["qemuImg"], blank.name, size="64M")
        # The machine's disk is an overlay over Asahi's layout, which the
        # VM's start script takes in place of making a blank one.
        disk = f"{disks}/machine.qcow2"
        overlay(values["qemuImg"], disk, "-b", f"{values['asahi']}/disk.qcow2", "-F", "qcow2")
        os.environ["NIX_DISK_IMAGE"] = disk
        yield Rig(keys, stick.name, blank.name, disk)


@pytest.fixture(scope="module")
def vault_opened(machine: QemuMachine, rig: Rig, values: dict[str, Any]) -> QemuMachine:
    """Boot the stick, type the keyboard's word, then the stick's passphrase."""
    del rig
    machine.start(allow_reboot=True)
    on_screen(machine, "Check the keyboard types what you mean", timeout=COPIED)
    enter(machine, "keyboard")
    on_screen(machine, "the stick's passphrase")
    enter(machine, values["passphrase"])
    return machine


@pytest.fixture(scope="module")
def confirmed(vault_opened: QemuMachine) -> str:
    """Type the machine's name back at the partition it names, and give what it said."""
    machine = vault_opened
    on_screen(machine, "Confirm the disk to wipe")
    on_screen(machine, "leaves its other partitions as they are")
    said = " ".join(screen(machine).split())
    enter(machine, "machine")
    return said


@pytest.fixture(scope="module")
def recovery_key(machine: QemuMachine, confirmed: str, values: dict[str, Any]) -> str:
    """Enrol the Nano with the stick out, keep the recovery key, and give its words."""
    del confirmed
    on_screen(machine, "Plug in the Nano", timeout=STEP)
    machine.send_monitor_command("device_del stickdev")
    machine.wait_until_succeeds("test ! -e /dev/disk/by-partlabel/vault", timeout=PLUGGED)
    machine.succeed(f"set-pin {values['pin']}")
    enter(machine, "")
    on_screen(machine, "the Nano's PIN")
    enter(machine, values["pin"])
    on_screen(machine, "Keep the recovery key", timeout=STEP)
    on_screen(machine, "Copy All")
    enter(machine, "")
    retry(
        lambda _: len(screen(machine).split()) == RECOVERY_KEY_WORDS,
        timeout=timedelta(seconds=30),
    )
    words = " ".join(screen(machine).split())
    enter(machine, "")
    on_screen(machine, "reading it off your phone")
    enter(machine, words)
    return words


@pytest.fixture(scope="module")
def installed(machine: QemuMachine, recovery_key: str, rig: Rig, values: dict[str, Any]) -> str:
    """Put the stick back, let the system install, give alice her password, and give the card."""
    del recovery_key
    # The card asking for the stick offers "look again" until the guide
    # next looks, up to two seconds after the stick is back.
    on_screen(machine, "Put the stick back in", timeout=STEP, stopped="try again")
    machine.send_monitor_command(f"drive_add 0 if=none,id=stick,format=qcow2,file={rig.stick}")
    machine.send_monitor_command("device_add usb-storage,bus=xhci.0,drive=stick,id=stickdev")
    machine.wait_until_succeeds("test -e /dev/disk/by-partlabel/vault", timeout=PLUGGED)
    on_screen(machine, "a password for alice", timeout=INSTALLED, stopped="try again")
    enter(machine, values["password"])
    on_screen(machine, "the same password again")
    enter(machine, values["password"])
    on_screen(machine, "shut down", timeout=STEP)
    on_screen(machine, "at every start")
    return " ".join(screen(machine).split())


@pytest.fixture(scope="module")
def guide_log(machine: QemuMachine, installed: str) -> str:
    """Give the guide's log of the install, read before the stick's memory goes."""
    del installed
    return machine.succeed("cat /run/guide/log")


@pytest.fixture(scope="module")
def first_boot(
    machine: QemuMachine, guide_log: str, rig: Rig, values: dict[str, Any]
) -> QemuMachine:
    """Shut down, then boot the machine's own disk with the stick out, by the Nano's PIN."""
    del guide_log
    enter(machine, "")
    machine.wait_for_shutdown()
    os.environ["STICK_IMAGE"] = rig.blank
    machine.start(allow_reboot=True)
    unlock(machine, values["pin"])
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    return machine


def test_the_partition_is_confirmed_rather_than_the_whole_disk(confirmed: str) -> None:
    """The confirmation says only the machine's partition is wiped, the rest left alone."""
    assert "wipes machine's partition on" in confirmed, confirmed
    assert "everything on it" not in confirmed, confirmed


def test_no_secure_boot_step_runs_on_a_machine_without_it(guide_log: str) -> None:
    """The install makes no Secure Boot keys and signs nothing, since the firmware has none."""
    for step in ["check-firmware", "secure-boot-keys", "seal-rescue-bundle", "keep-on-stick"]:
        assert step not in guide_log, step


def test_the_card_says_the_disk_asks_at_every_start(installed: str) -> None:
    """The reboot card promises no TPM, since there's none to seal to."""
    assert "TPM" not in installed, installed


def test_every_other_partition_is_as_it_was(
    first_boot: QemuMachine, values: dict[str, Any]
) -> None:
    """Apple's partitions and Asahi's stub are the same byte for byte."""
    for number in [1, 2, 3, 4]:
        expected = Path(f"{values['asahi']}/partition-{number}.sha256").read_text().strip()
        found = first_boot.succeed(f"sha256sum {DISK}{number}").split()[0]
        assert found == expected, number


def test_the_table_gained_only_the_machines_partition(
    first_boot: QemuMachine, values: dict[str, Any]
) -> None:
    """Each entry is where it was, the boot partition only renamed, and one more in the space."""
    before = table(Path(f"{values['asahi']}/table.json").read_text())
    after = table(first_boot.succeed(f"sfdisk --json {DISK}"))
    assert sorted(after) == [1, 2, 3, 4, 5, 6], after
    for number, entry in before.items():
        expected = {**entry, "node": after[number]["node"]}
        if number == 5:  # noqa: PLR2004 # Asahi's boot partition, by the Mac's numbering.
            expected["name"] = "system-boot"
        assert after[number] == expected, number
    assert after[6]["name"] == "system", after[6]
    assert after[6]["type"] == LUKS, after[6]


def test_the_boot_partition_keeps_asahis_files(
    first_boot: QemuMachine, values: dict[str, Any]
) -> None:
    """m1n1's stage and the firmware's copy are there beside systemd-boot."""
    for name, path in [("boot.bin", "m1n1/boot.bin"), ("firmware.cpio", "vendorfw/firmware.cpio")]:
        expected = Path(f"{values['asahi']}/{name}.sha256").read_text().strip()
        assert first_boot.succeed(f"sha256sum /boot/{path}").split()[0] == expected, path
    first_boot.succeed("test -e /boot/EFI/systemd/systemd-bootx64.efi")


def test_the_installed_guide_has_nothing_to_seal(first_boot: QemuMachine) -> None:
    """With no TPM, the guide at boot finishes without taking the screen."""
    first_boot.wait_until_succeeds(
        "systemctl show -P ActiveState guide-installed | grep -qvx activating"
    )
    assert first_boot.succeed("systemctl show -P Result guide-installed").strip() == "success"


def test_the_recovery_words_open_the_disk(first_boot: QemuMachine, recovery_key: str) -> None:
    """Without the Nano, the disk opens with the words the person keeps."""
    # Tested on the running machine, since QEMU plugs the Nano back in at
    # every start, and with the tokens left out, so only the words count.
    first_boot.succeed(
        f"echo '{recovery_key}' | cryptsetup open --test-passphrase"
        " --disable-external-tokens /dev/disk/by-partlabel/system"
    )

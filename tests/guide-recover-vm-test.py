"""A machine recovered from a repair stick in a virtual machine, its data left in place.

tests/guide-recover.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The guide's screens come one after another,
and its menu keeps where it was, so each state a test starts from is a fixture that walks the
guide one step further than the fixture it asks for: the disk's keys enrolled, the recovery
started, the disk opened, a generation chosen, a lost key replaced and the boot loader put back.
A fixture's work is done once a run, so any test runs alone with `-k`, walking only as far as
it needs. What the tests need from Nix, the machine's disk image, the repair stick, the keys'
state files, the PIN and the recovery key, are guide-recover.nix's, given as the `values`
fixture.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import QemuMachine

VOLUME = "/dev/disk/by-partlabel/system"
PLUGIN = "LD_LIBRARY_PATH=$(dirname $(readlink -f $(which systemd-cryptsetup)))/../lib/cryptsetup"
# How long a screen of the guide may take to come, a key to come or go, and a step that
# writes to the disk.
SCREEN = timedelta(seconds=120)
KEY = timedelta(seconds=30)
WRITING = timedelta(seconds=180)


def screen(machine: QemuMachine) -> str:
    """Give the text on the guide's console."""
    return machine.get_tty_text("1")


def on_screen(machine: QemuMachine, pattern: str, timeout: timedelta = SCREEN) -> None:
    """Wait until the guide's screen shows the pattern, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the screen shows the pattern, raising if it shows the guide stopped."""
        text = screen(machine)
        if re.search(r"(?i)(try again|look again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, text) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout=timeout)
        except Exception:
            print("SCREEN\n" + screen(machine))
            print("LOG\n" + machine.execute("tail -n 40 /run/guide/log")[1])
            raise


def enter(machine: QemuMachine, text: str) -> None:
    """Type the text at the guide, and Return after it."""
    machine.send_chars(text + "\n")


def plug(machine: QemuMachine, keys: str, name: str) -> None:
    """Swap the key in the machine for the one of that name, waiting for each to come and go."""
    machine.send_monitor_command("device_del key")
    machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=KEY)
    machine.send_monitor_command(f"device_add canokey,id=key,bus=xhci.0,file={keys}/{name}")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=KEY)


def opens(machine: QemuMachine, keys: str, name: str, pin: str) -> bool:
    """Plug in the key of that name, and say whether it opens the disk with the PIN."""
    plug(machine, keys, name)
    status, _ = machine.execute(
        f"echo {pin} | {PLUGIN} cryptsetup open --test-passphrase --token-only"
        f" --token-type systemd-fido2 {VOLUME}"
    )
    return status == 0


def overlay(qemu_img: str, backing: str, kind: str, file: str) -> None:
    """Put an overlay on a store image, which the machine writes to in its place."""
    # qemu-img and the images are Nix's own store paths, so nothing untrusted reaches the call.
    subprocess.run(  # noqa: S603
        [qemu_img, "create", "-f", "qcow2", "-b", backing, "-F", kind, file],
        check=True,
    )


@pytest.fixture(scope="module")
def keys(values: dict[str, Any]) -> Generator[str]:
    """Give the folder of the kept, lost and new keys' state files, the overlays made beside.

    guide-recover.nix reads each from the environment as QEMU starts, so all of it comes
    before the machine does. The stick's overlay is named as the stick's node's, which never
    starts: the machine boots the stick's image from USB.
    """
    with (
        tempfile.NamedTemporaryFile(suffix=".qcow2") as disk,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as stick,
        tempfile.TemporaryDirectory() as folder,
    ):
        overlay(values["qemuImg"], values["image"], "qcow2", disk.name)
        os.environ["NIX_DISK_IMAGE"] = disk.name
        overlay(values["qemuImg"], values["stick"], "raw", stick.name)
        os.environ["STICK_IMAGE"] = stick.name
        for name in ["kept", "lost", "new"]:
            shutil.copy(f"{values['keys']}/{name}", f"{folder}/{name}")
            Path(f"{folder}/{name}").chmod(0o600)
        os.environ["KEYS_DIR"] = folder
        yield folder


@pytest.fixture(scope="module")
def enrolled(machine: QemuMachine, keys: str, values: dict[str, Any]) -> QemuMachine:
    """Start the machine, and enrol the kept and lost keys on its disk as a machine's are."""
    pin, recovery = values["pin"], values["recovery"]
    machine.start()
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=timedelta(seconds=60))
    for name in ["kept", "lost"]:
        if name != "kept":
            plug(machine, keys, name)
        machine.succeed(f"set-pin {pin}")
        machine.succeed(
            f"install -d -m 0700 /run/pins && printf {pin} > /run/pins/cryptenroll.fido2-pin"
        )
        machine.succeed(
            f"PASSWORD='{recovery}' CREDENTIALS_DIRECTORY=/run/pins"
            " systemd-cryptenroll --fido2-device=auto"
            f" --fido2-with-client-pin=yes --fido2-with-user-presence=yes {VOLUME} >&2"
        )
    plug(machine, keys, "kept")
    return machine


@pytest.fixture(scope="module")
def recovering(enrolled: QemuMachine) -> QemuMachine:
    """Let the repair stick's guide check the keyboard, and go on to the machine's disk."""
    on_screen(enrolled, "Recover a machine")
    on_screen(enrolled, "Check the keyboard")
    enter(enrolled, "keyboard")
    on_screen(enrolled, "holds the machine")
    return enrolled


@pytest.fixture(scope="module")
def opened(recovering: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Open the machine's disk with a YubiKey's PIN, and wait for the menu."""
    enter(recovering, "")
    on_screen(recovering, "Or plug in a YubiKey")
    enter(recovering, values["pin"])
    # The menu, which comes only once every mount is done: "disk is open"
    # is shown as soon as the disk opens, before they are.
    on_screen(recovering, "What should be done")
    return recovering


@pytest.fixture(scope="module")
def generation_chosen(opened: QemuMachine) -> QemuMachine:
    """Choose generation 1 to boot next, and come back to the menu."""
    opened.send_key("down")
    enter(opened, "")
    on_screen(opened, "Which should it boot next")
    assert "Generation 1" in screen(opened), screen(opened)
    enter(opened, "")
    on_screen(opened, "What should be done")
    return opened


@pytest.fixture(scope="module")
def replaced(generation_chosen: QemuMachine, keys: str, values: dict[str, Any]) -> QemuMachine:
    """Replace the lost key with the new one, opened by the kept key and the recovery key."""
    machine, pin = generation_chosen, values["pin"]
    machine.send_key("down")
    machine.send_key("down")
    enter(machine, "")
    on_screen(machine, "the YubiKey you still have")
    enter(machine, "")
    on_screen(machine, "plug in the new YubiKey")
    plug(machine, keys, "new")
    machine.succeed(f"set-pin {pin}")
    enter(machine, "")
    on_screen(machine, "the new YubiKey's PIN")
    enter(machine, pin)
    on_screen(machine, "the recovery key")
    enter(machine, values["recovery"])
    on_screen(machine, "What should be done", timeout=WRITING)
    return machine


@pytest.fixture(scope="module")
def boot_loader_back(replaced: QemuMachine) -> QemuMachine:
    """Take the boot loader off the ESP, and have the guide install it again."""
    replaced.succeed("rm -r /mnt/boot/EFI/systemd /mnt/boot/loader/entries")
    for _ in range(3):
        replaced.send_key("down")
    # What the menu looks like on a console, its keys drawn by its font.
    replaced.screenshot("menu")
    enter(replaced, "")
    on_screen(replaced, "installed again", timeout=WRITING)
    return replaced


def test_disk_opens_with_two_yubikeys(enrolled: QemuMachine) -> None:
    """The machine's disk opens with two YubiKeys, as a machine's does."""
    header = json.loads(enrolled.succeed(f"cryptsetup luksDump --dump-json-metadata {VOLUME}"))
    kinds = sorted(token["type"] for token in header["tokens"].values())
    assert kinds == ["systemd-fido2", "systemd-fido2"], kinds


def test_repair_stick_boots_into_recovery(recovering: QemuMachine) -> None:
    """A repair stick boots into the guide's recovery."""
    assert "named system" in screen(recovering), screen(recovering)


def test_everything_the_machine_mounts_is_under_mnt(opened: QemuMachine) -> None:
    """Everything the machine mounts is mounted under /mnt, its disk opened with a key's PIN."""
    for at in ["/mnt", "/mnt/nix", "/mnt/home", "/mnt/var/log", "/mnt/boot"]:
        opened.succeed(f"findmnt {at}")
    opened.succeed("test -e /mnt/etc/NIXOS")
    # What bootctl lists from outside the machine, for the guide's fixtures (qemu-canokey).
    listed = opened.succeed("bootctl --esp-path=/mnt/boot list --json=short")
    print("FIXTURE bootctl-list-from-the-stick.json " + listed.replace("\n", "\\n"))


def test_generation_chosen_boots_next(generation_chosen: QemuMachine) -> None:
    """A generation chosen is set to boot next, and the menu comes back."""
    assert "boots generation 1 next" in screen(generation_chosen), screen(generation_chosen)
    # The firmware's own default, which systemd-boot reads first.
    generation_chosen.succeed(
        "test -e /sys/firmware/efi/efivars/LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"
    )


def test_lost_yubikey_is_replaced(replaced: QemuMachine, keys: str, values: dict[str, Any]) -> None:
    """A lost YubiKey is replaced: the new one enrolled and proven, the lost one's slot wiped."""
    assert "the lost one no longer does" in screen(replaced), screen(replaced)
    header = json.loads(replaced.succeed(f"cryptsetup luksDump --dump-json-metadata {VOLUME}"))
    kinds = sorted(token["type"] for token in header["tokens"].values())
    assert kinds == ["systemd-fido2", "systemd-fido2"], kinds
    pin = values["pin"]
    assert opens(replaced, keys, "new", pin), "the new key doesn't open the disk"
    assert opens(replaced, keys, "kept", pin), "the kept key no longer opens the disk"
    assert not opens(replaced, keys, "lost", pin), "the lost key still opens the disk"


def test_esp_gets_its_boot_loader_again(boot_loader_back: QemuMachine) -> None:
    """An ESP that lost its boot loader gets it again from the installed system."""
    boot_loader_back.succeed("test -s /mnt/boot/EFI/systemd/systemd-bootx64.efi")
    boot_loader_back.succeed("ls /mnt/boot/loader/entries/*.conf")


def test_return_gives_a_root_shell_inside(boot_loader_back: QemuMachine) -> None:
    """Return gives a root shell inside the installed system."""
    enter(boot_loader_back, "")
    boot_loader_back.sleep(timedelta(seconds=3))
    enter(boot_loader_back, "cat /etc/hostname > /recovered && nixos-version >> /recovered")
    boot_loader_back.wait_until_succeeds("test -s /mnt/recovered", timeout=KEY)
    said = boot_loader_back.succeed("cat /mnt/recovered")
    print(said)
    assert "machine" in said.splitlines()[0], said

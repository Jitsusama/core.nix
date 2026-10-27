"""A paired Bluetooth keyboard types at the disk prompt of a laptop with Secure Boot.

bluetooth-disk-prompt.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Each state a test
starts from is a fixture, in the order the boots come: btvirt and the keyboard up, the laptop's
keys enrolled, its real system booted with measured boot, the keyboard paired, the pairing
sealed, the restart the keyboard types at, and a restart past a seal that won't open. What the
tests need from Nix are the nix file's, given as the `values` fixture.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import BaseMachine, QemuMachine

CREDENTIAL = "/boot/loader/credentials/bluetooth-pairing.cred"
# How long a controller, a pairing or a connection may take to come up.
SETTLE = timedelta(seconds=60)
# How long a keyboard may keep calling its host before the host takes it, each call
# waiting out L2CAP's own time limit.
CALLING = timedelta(minutes=3)


def unlock(machine: QemuMachine, secret: str) -> None:
    """Type the secret at the disk's prompt on the serial console, and wait for the disk."""
    machine.wait_for_console_text("Please enter")
    machine.send_console(secret + "\n")
    opened(machine)


def opened(machine: QemuMachine) -> None:
    """Wait for the disk to open, and for the system's shell."""
    # Perhaps coloured; the driver matches across lines joined with nothing
    # between them, so the pattern mustn't span one.
    machine.wait_for_console_text(r"Finished (?:\x1b\[[0-9;]*m)*Cryptography Setup for system")
    # The driver can't see a restart the machine started itself, so every
    # boot's shell is connected in turn.
    machine.connected = False
    machine.connect()


def address(machine: QemuMachine) -> str:
    """Give the machine's controller's address, once BlueZ has it."""
    listed = machine.wait_until_succeeds("bluetoothctl list | grep -m1 Controller", timeout=SETTLE)
    return listed.split()[1]


@pytest.fixture(scope="module")
def btvirt(values: dict[str, Any]) -> Generator[None]:
    """Run BlueZ's emulator, whose TCP server each machine's controller connects to."""
    # btvirt is a Nix store path, so nothing untrusted reaches the call.
    emulator = subprocess.Popen(  # noqa: S603
        [values["btvirt"], f"--tcp={values['port']}"],
        stderr=subprocess.PIPE,
        text=True,
    )
    assert emulator.stderr is not None
    for line in emulator.stderr:
        print(line, end="", flush=True)
        if "Listening TCP" in line:
            break
    yield
    emulator.terminate()
    emulator.wait()


@pytest.fixture(scope="module")
def keyboard(machines: dict[str, BaseMachine], btvirt: None) -> QemuMachine:
    """Start the keyboard's machine, its stand-in ready to pair."""
    del btvirt
    machine = cast("QemuMachine", machines["keyboard"])
    machine.start()
    machine.wait_until_succeeds(
        "journalctl -u stand-in-keyboard.service -o cat | grep -qx ready", timeout=SETTLE
    )
    return machine


@pytest.fixture(scope="module")
def overlay(values: dict[str, Any]) -> Generator[str]:
    """Give the overlay the laptop writes to, the store's disk image beneath it.

    QEMU reads it from the environment as the laptop starts, so it's made before the laptop is.
    """
    with tempfile.NamedTemporaryFile(suffix=".qcow2") as disk:
        # qemu-img and the image are Nix's own store paths, so nothing untrusted reaches
        # the call.
        backing = ["-b", values["image"], "-F", "qcow2"]
        subprocess.run(  # noqa: S603
            [values["qemuImg"], "create", "-f", "qcow2", *backing, disk.name],
            check=True,
        )
        os.environ["NIX_DISK_IMAGE"] = disk.name
        yield disk.name


@pytest.fixture(scope="module")
def enrolled(
    machines: dict[str, BaseMachine], keyboard: QemuMachine, overlay: str, values: dict[str, Any]
) -> QemuMachine:
    """Boot the laptop for the first time, through the restart that enrols its keys."""
    del keyboard, overlay
    machine = cast("QemuMachine", machines["laptop"])
    machine.start(allow_reboot=True)
    unlock(machine, values["passphrase"])
    machine.wait_for_console_text("Exporting as auth files")
    machine.wait_for_console_text("Linux version")
    unlock(machine, values["passphrase"])
    machine.wait_for_unit("multi-user.target")
    return machine


@pytest.fixture(scope="module")
def real(enrolled: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Install the real system, with measured boot, and boot it."""
    machine, real = enrolled, values["real"]
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {real}")
    machine.succeed(f"{real}/bin/switch-to-configuration boot")
    machine.reboot()
    unlock(machine, values["passphrase"])
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("systemd-pcrlock-make-policy.service")
    return machine


@pytest.fixture(scope="module")
def paired(real: QemuMachine, keyboard: QemuMachine) -> str:
    """Pair the laptop with the keyboard and connect it, and give the keyboard's address."""
    keys = address(keyboard)
    address(real)
    real.wait_until_succeeds(
        f"bluetoothctl --timeout 10 scan on >/dev/null; bluetoothctl devices | grep -q {keys}",
        timeout=SETTLE,
    )
    real.succeed(f"bluetoothctl pair {keys}")
    real.succeed(f"bluetoothctl trust {keys}")
    real.succeed(f"bluetoothctl connect {keys}")
    real.wait_until_succeeds("grep -q 'Stand-in Keyboard' /proc/bus/input/devices", timeout=SETTLE)
    return keys


@pytest.fixture(scope="module")
def sealed(real: QemuMachine, paired: str) -> QemuMachine:
    """Seal the pairing, as the timer would within two minutes."""
    del paired
    print(
        real.succeed(
            "systemctl start bluetooth-pairing-seal.service; journalctl -o cat"
            " -u bluetooth-pairing-seal.service"
        )
    )
    real.succeed(f"test -s {CREDENTIAL}")
    return real


@pytest.fixture(scope="module")
def typed(
    sealed: QemuMachine, keyboard: QemuMachine, values: dict[str, Any]
) -> tuple[QemuMachine, str]:
    """Restart the laptop and have the keyboard type the passphrase at its prompt.

    Gives the laptop and the address it had before the restart.
    """
    laptop = sealed
    before = address(laptop)
    laptop.reboot()
    laptop.wait_for_console_text("Please enter")
    # As a keyboard does at a key press after its host restarted: it calls
    # the host, which takes it only once the sealed pairing is BlueZ's.
    keyboard.wait_until_succeeds(
        f"bluetooth-keyboard-stand-in type {before} '{values['passphrase']}\\n'",
        timeout=CALLING,
    )
    opened(laptop)
    laptop.wait_for_unit("multi-user.target")
    return laptop, before


@pytest.fixture(scope="module")
def unopenable(typed: tuple[QemuMachine, str], values: dict[str, Any]) -> QemuMachine:
    """Restart the laptop past a seal that won't open, its passphrase typed on the console."""
    laptop, _ = typed
    # The pairing is unchanged, so the seal at shutdown leaves this be.
    laptop.succeed(f"echo not-a-credential > {CREDENTIAL}")
    laptop.reboot()
    unlock(laptop, values["passphrase"])
    laptop.wait_for_unit("multi-user.target")
    return laptop


def test_with_nothing_sealed_the_prompt_waits_for_nothing(real: QemuMachine) -> None:
    """With nothing sealed, the pairings are skipped and the prompt asks as ever."""
    skipped = real.succeed(
        "journalctl -b -o cat | grep 'Opens the sealed Bluetooth pairings' || true"
    )
    print(skipped)
    assert "skipped" in skipped, skipped


def test_the_keyboard_pairs_and_types_in_the_running_system(real: QemuMachine, paired: str) -> None:
    """The keyboard pairs, and the running system takes it as a keyboard."""
    info = real.succeed(f"cat /var/lib/bluetooth/*/{paired}/info")
    assert "[LinkKey]" in info, info


def test_the_seal_opens_only_with_the_tpm_and_holds_no_key_in_the_clear(
    sealed: QemuMachine, paired: str
) -> None:
    """The ESP holds the pairing only sealed, and the TPM opens it under the disk's policy."""
    info = sealed.succeed(f"cat /var/lib/bluetooth/*/{paired}/info")
    key = info.split("[LinkKey]", 1)[1].split("Key=", 1)[1].split()[0]
    sealed.fail(f"grep -r -i -q {key} /boot")
    listed = sealed.succeed(
        f"systemd-creds decrypt --name=bluetooth-pairing {CREDENTIAL} - | tar -t"
    )
    print(listed)
    assert f"/{paired}/info" in listed, listed


def test_the_keyboard_types_at_the_disk_prompt(typed: tuple[QemuMachine, str]) -> None:
    """The keyboard typed the passphrase at the prompt, through the initrd's BlueZ."""
    laptop, _ = typed
    kernel = laptop.succeed("journalctl -b -k -o cat")
    assert "Stand-in Keyboard" in kernel, kernel


def test_the_laptop_keeps_the_address_it_paired_with(typed: tuple[QemuMachine, str]) -> None:
    """The laptop's address after the restart is the one the keyboard was paired with."""
    laptop, before = typed
    assert address(laptop) == before


# Last, since the seal it spoils stays spoiled until the pairing changes.
def test_a_seal_that_wont_open_leaves_the_prompt_asking(unopenable: QemuMachine) -> None:
    """A seal that won't open leaves the prompt asking, and the passphrase opens the disk."""
    print(unopenable.succeed("journalctl -b -o cat | grep -i 'bluetooth' || true"))

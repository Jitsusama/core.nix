"""One stick, signed for two machines, booted under each one's firmware and refused by a third's.

guide-one-stick.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The same machine
boots under each machine's firmware variables in turn, a fresh copy of them each time where the
driver has QEMU keep its variables, with an overlay on the stick for it to write to. What the
tests need from Nix, the stick, each machine's variables and the tool to make overlays, are
guide-one-stick.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import QemuMachine

# How long the firmware may take to boot the stick, or to say it won't.
BOOT = timedelta(minutes=5)
# What the firmware says on its console when Secure Boot refuses the stick's boot program.
REFUSED = (
    r'failed to load Boot[0-9A-F]+ "UEFI QEMU QEMU USB HARDDRIVE[^"]*"'
    r" from \S+: Access Denied"
)


def overlay(qemu_img: str, backing: str, file: str) -> None:
    """Put an overlay on the stick's image, which the machine writes to in its place."""
    # qemu-img and the image are Nix's own store paths, so nothing untrusted reaches the call.
    subprocess.run(  # noqa: S603
        [qemu_img, "create", "-f", "qcow2", "-b", backing, "-F", "raw", file],
        check=True,
    )


@pytest.fixture
def under(values: dict[str, Any]) -> Generator[None]:
    """Give a fresh overlay on the stick, gone after."""
    with tempfile.NamedTemporaryFile(suffix=".qcow2") as stick:
        overlay(values["qemuImg"], values["stick"], stick.name)
        os.environ["STICK_IMAGE"] = stick.name
        yield


def start_under(machine: QemuMachine, values: dict[str, Any], name: str) -> None:
    """Start the machine with a writable copy of that machine's firmware variables."""
    variables = Path(machine.efi_vars_path)
    shutil.copy(f"{values['firmware']}/{name}.fd", variables)
    variables.chmod(0o644)
    machine.start()


def booted_under(machine: QemuMachine, values: dict[str, Any], name: str) -> str:
    """Boot the stick under that machine's firmware, and give what bootctl says of it."""
    start_under(machine, values, name)
    machine.wait_for_unit("multi-user.target", timeout=BOOT)
    status = machine.succeed("bootctl status")
    machine.shutdown()
    return status


@pytest.mark.usefixtures("under")
def test_the_stick_boots_enforcing_under_the_first_machines_keys(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """The first machine's firmware, holding only its own keys, boots the stick enforcing."""
    status = booted_under(machine, values, "first")
    assert "Secure Boot: enabled (user)" in status, status


@pytest.mark.usefixtures("under")
def test_the_stick_boots_enforcing_under_the_second_machines_keys(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """The second machine's firmware, holding only its own keys, boots the stick enforcing."""
    status = booted_under(machine, values, "second")
    assert "Secure Boot: enabled (user)" in status, status


@pytest.mark.usefixtures("under")
def test_a_machine_whose_keys_signed_neither_refuses_the_stick(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """A third machine's firmware, holding neither signer's key, won't start the stick."""
    start_under(machine, values, "neither")
    machine.wait_for_console_text(REFUSED, timeout=BOOT)
    machine.crash()

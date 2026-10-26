"""A stick's vault sealed to a YubiKey, shut to its PIN at the install, opened by passphrase.

tests/guide-vault.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: the machine booted with its key plugged in, the key given a PIN and a recipient, and a
stick written and looped. A fixture's work is done once a run, however many tests ask for it,
so any test runs alone with `-k`. The key's PIN and the stick's recovery passphrase are
guide-vault.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import QemuMachine

# How long a screen of the guide may take to come, and the key to come or go.
SCREEN = timedelta(seconds=60)
KEY = timedelta(seconds=30)


def screen(machine: QemuMachine) -> str:
    """Give what the guide shows on the first console."""
    return machine.get_tty_text("1")


def on_screen(machine: QemuMachine, pattern: str, timeout: timedelta = SCREEN) -> None:
    """Wait until the guide's screen matches the pattern, saying what was running if it never does.

    It fails at once if the guide stopped, unless the stop says the pattern.
    """

    def shown(_: bool) -> bool:
        """Say whether the screen matches the pattern, or fail if the guide stopped."""
        text = screen(machine)
        if re.search("(?i)" + pattern, text) is not None:
            return True
        if re.search(r"(?i)(try again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return False

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout=timeout)
        except Exception:
            print("SCREEN\n" + screen(machine))
            _, processes = machine.execute(
                "ps -eo pid,stat,wchan:20,args | grep -i -e guide -e age-plugin"
            )
            print("PROCESSES\n" + processes)
            _, log = machine.execute("tail -n 40 /run/guide/log")
            print("LOG\n" + log)
            raise


def type_in(machine: QemuMachine, text: str) -> None:
    """Type the text at the guide, and Return."""
    machine.send_chars(text + "\n")


@pytest.fixture(scope="module")
def booted(machine: QemuMachine) -> QemuMachine:
    """Give the machine booted, its key plugged in."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=KEY)
    return machine


@pytest.fixture(scope="module")
def recipient(booted: QemuMachine, values: dict[str, Any]) -> str:
    """Give the key a PIN, then the recipient made for it as a person makes one."""
    booted.succeed(f"set-pin {shlex.quote(values['pin'])}")
    return booted.succeed(f"make-recipient {shlex.quote(values['pin'])}").strip()


@pytest.fixture(scope="module")
def stick(booted: QemuMachine, recipient: str, values: dict[str, Any]) -> QemuMachine:
    """Write a stick sealed to the key into a file, and loop it so its vault is a partition."""
    # A small image with an ESP and one more partition, as core's has.
    booted.succeed(
        "truncate -s 16M /root/image.raw && "
        "printf 'label: gpt\\nsize=4M, type=uefi, name=esp\\nsize=4M, name=store\\n'"
        " | sfdisk /root/image.raw"
    )
    booted.succeed(
        "mkdir -p /root/payload/cache && touch /root/payload/layout.sh /root/payload/mount.sh"
    )
    target = {
        "machine": "laptop",
        "disk": "/dev/disk/by-id/nvme-the-laptops",
        "disk_name": "system",
        "install_key": "/run/guide/install-key",
        "layout_packages": [],
        "system": "/nix/store/abc",
        "account": "alice",
        "yubikeys": ["the Nano"],
        "recipients": [recipient],
    }
    booted.succeed(f"echo {shlex.quote(json.dumps(target))} > /root/payload/target.json")
    booted.succeed("truncate -s 120M /root/stick.raw")
    print(
        booted.succeed(
            f"echo {shlex.quote(values['passphrase'])}"
            " | guide write-stick /root/image.raw /root/stick.raw --install /root/payload"
        )
    )
    booted.succeed("losetup --find --partscan --show /root/stick.raw && udevadm settle")
    return booted


def test_key_gets_a_pin_and_a_recipient(recipient: str) -> None:
    """The key gets a PIN and a recipient, as a person makes one."""
    assert recipient.startswith("age1fido2-hmac1"), recipient


def test_guide_seals_a_sticks_vault(stick: QemuMachine) -> None:
    """The guide seals a stick's vault to it, needing no touch."""
    stick.succeed("test -e /dev/disk/by-partlabel/vault")


def test_a_key_and_its_pin_leave_the_install_vault_shut(
    stick: QemuMachine, values: dict[str, Any]
) -> None:
    """On the stick, the install's vault takes its passphrase only."""
    stick.wait_until_succeeds("pgrep -u root -t tty1 bash")
    type_in(stick, "guide install")
    on_screen(stick, "Check the keyboard")
    type_in(stick, "keyboard")
    on_screen(stick, "the stick's passphrase")
    type_in(stick, values["pin"])
    on_screen(stick, "wouldn't open")


def test_recovery_passphrase_opens_it(stick: QemuMachine, values: dict[str, Any]) -> None:
    """With the key taken out, the recovery passphrase opens it."""
    # The guide left as a person leaves it, if it's running, and the key
    # it opened the vault with forgotten.
    stick.send_key("ctrl-c")
    stick.succeed("rm -f /run/guide/vault-key")
    stick.send_monitor_command("device_del key")
    stick.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=KEY)
    stick.wait_until_succeeds("pgrep -u root -t tty1 bash")
    type_in(stick, "guide install")
    on_screen(stick, "the stick's passphrase")
    type_in(stick, values["passphrase"])
    on_screen(stick, "vault is open")

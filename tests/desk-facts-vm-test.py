"""desk-facts follows a USB sound card plugged in, pulled out and plugged in again.

tests/desk-facts.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), on a
machine with an xHCI controller named desk and nothing plugged into it. QEMU's monitor plugs its
USB audio device in and takes it out, and each time sound.json, which desk-facts rewrites only
on what it hears, must name the card or no longer name it. Each state, booted, plugged in,
pulled out and plugged in again, is a fixture made from the one before, so any test runs alone
with `-k`. The tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
import json
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import QemuMachine
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

FACTS = "/run/user/1000/desk/sound.json"
PLUG = "device_add usb-audio,id=speakers,bus=desk.0,audiodev=nowhere"
# How long desk-facts may take to say what it heard, or QEMU to let a device go.
HEARING = dt.timedelta(seconds=30)


class Desk:
    """The VM's desk controller, and what desk-facts says of what's plugged into it."""

    def __init__(self, machine: QemuMachine) -> None:
        """Act on the desk of that machine."""
        self.machine = machine

    def said(self) -> dict[str, list[dict[str, str]]]:
        """Give sound.json's cards, by ALSA id and direction, each with its facts."""
        parsed: dict[str, list[dict[str, str]]] = json.loads(
            as_account(self.machine, f"cat {FACTS}")
        )
        return parsed

    def until(self, *, named: bool) -> list[str]:
        """Wait until sound.json names a card, or names none, and give what it names then."""
        self.machine.wait_until_succeeds(
            f"test -s {FACTS} && {'' if named else '! '}grep -q ':out' {FACTS}",
            timeout=HEARING,
        )
        return sorted(self.said())


@pytest.fixture(scope="module")
def desk(machine: BaseMachine) -> Desk:
    """Boot the machine with nothing at its desk, and desk-facts running for Joel."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("desk-facts.service", "joel")
    # Only QEMU's monitor plugs devices in and takes them out.
    assert isinstance(machine, QemuMachine)
    return Desk(machine)


@pytest.fixture(scope="module")
def plugged(desk: Desk) -> list[str]:
    """Plug the USB sound card in, and give the cards sound.json names then."""
    desk.machine.send_monitor_command(PLUG)
    return desk.until(named=True)


@pytest.fixture(scope="module")
def pulled(desk: Desk, plugged: list[str]) -> list[str]:
    """Pull the card out once it's been named, and give the cards sound.json names then."""
    assert plugged
    desk.machine.send_monitor_command("device_del speakers")
    return desk.until(named=False)


@pytest.fixture(scope="module")
def replugged(desk: Desk, pulled: list[str]) -> list[str]:
    """Plug the card in again once it's gone, and give the cards sound.json names then."""
    assert not pulled
    # QEMU frees the id only once the guest has let the device go.
    desk.machine.wait_until_succeeds("! ls /proc/asound/card[0-9]", timeout=HEARING)
    desk.machine.send_monitor_command(PLUG)
    return desk.until(named=True)


def test_nothing_plugged_in_names_no_card(desk: Desk) -> None:
    """Nothing plugged in, sound.json names no card."""
    assert desk.until(named=False) == []


def test_a_card_plugged_in_is_named_with_what_it_offers(desk: Desk, plugged: list[str]) -> None:
    """A USB sound card plugged in is named, with what it offers."""
    assert len(plugged) == 1
    facts = {fact["name"]: fact["value"] for fact in desk.said()[plugged[0]]}
    assert "offers" in facts


def test_pulled_out_it_s_no_longer_named(pulled: list[str]) -> None:
    """Pulled out, it's no longer named."""
    assert pulled == []


def test_plugged_in_again_it_s_named_again(desk: Desk, replugged: list[str]) -> None:
    """Plugged in again, it's named again."""
    assert len(replugged) == 1
    as_account(desk.machine, "systemctl --user is-active desk-facts.service", bus=True)

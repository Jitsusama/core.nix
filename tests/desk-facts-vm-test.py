"""desk-facts follows a USB sound card plugged in, pulled out and plugged in again.

tests/desk-facts.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), on a
machine with an xHCI controller named desk and nothing plugged into it. QEMU's monitor plugs its
USB audio device in and takes it out, and each time sound.json, which desk-facts rewrites only
on what it hears, must name the card or no longer name it. snd-dummy's card, which isn't on USB,
stands in for a laptop's own, named by each of its devices, and QEMU's USB keyboard must be named
in keyboards.json while it's plugged in. Each state, booted, plugged in,
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
# A USB card's key, its ALSA id and direction, which a built-in device's comma never is in.
USB_OUT = '"[^",]+:out"'
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
        """Wait until sound.json names a USB card, or names none, and give those it names then."""
        self.machine.wait_until_succeeds(
            f"test -s {FACTS} && {'' if named else '! '}grep -qE '{USB_OUT}' {FACTS}",
            timeout=HEARING,
        )
        return sorted(key for key in self.said() if "," not in key)


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
    desk.machine.wait_until_succeeds("! ls /proc/asound/card*/usbid", timeout=HEARING)
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


def test_a_card_not_on_usb_is_named_by_each_of_its_devices(desk: Desk) -> None:
    """A card not on USB is named by each of its devices."""
    desk.machine.wait_until_succeeds(f"grep -q '\"Dummy,0:in\"' {FACTS}", timeout=HEARING)
    assert {"Dummy,0:out", "Dummy,0:in"} <= set(desk.said())


def test_a_device_not_on_usb_says_what_it_plays_at_while_it_plays(desk: Desk) -> None:
    """A device not on USB says what it plays at while it plays, and no longer once it stops."""
    desk.machine.succeed(
        "systemd-run --unit=playing aplay -q -D hw:Dummy,0 -t raw -f S16_LE -r 48000 -c 2 /dev/zero"
    )
    desk.machine.wait_until_succeeds(f"grep -q '\"playing\"' {FACTS}", timeout=HEARING)
    facts = {fact["name"]: fact["value"] for fact in desk.said()["Dummy,0:out"]}
    assert facts["playing"].startswith("48 kHz, 16 bit, period ")
    desk.machine.succeed("systemctl stop playing")
    desk.machine.wait_until_fails(f"grep -q '\"playing\"' {FACTS}", timeout=HEARING)


def test_a_usb_keyboard_is_named_with_its_polling_rate_until_it_s_pulled_out(desk: Desk) -> None:
    """A USB keyboard plugged in is named with its polling rate, and no longer once pulled out."""
    keyboards = "/run/user/1000/desk/keyboards.json"
    named = f"grep -q '\"QEMU QEMU USB Keyboard\"' {keyboards}"
    desk.machine.send_monitor_command("device_add usb-kbd,id=keys,bus=desk.0")
    desk.machine.wait_until_succeeds(named, timeout=HEARING)
    said = json.loads(as_account(desk.machine, f"cat {keyboards}"))
    facts = {fact["name"]: fact["value"] for fact in said["QEMU QEMU USB Keyboard"]}
    assert facts["polling rate"].endswith(" a second")
    assert int(facts["keys"]) > 0
    desk.machine.send_monitor_command("device_del keys")
    desk.machine.wait_until_fails(named, timeout=HEARING)


def test_plugged_in_again_it_s_named_again(desk: Desk, replugged: list[str]) -> None:
    """Plugged in again, it's named again."""
    assert len(replugged) == 1
    as_account(desk.machine, "systemctl --user is-active desk-facts.service", bus=True)

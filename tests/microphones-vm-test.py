"""A webcam's microphone ranks below a USB microphone's, and others keep their own rank.

tests/microphones.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Two dummy sound cards stand in for microphones,
the second marked a webcam by udev, and each test reads the rank WirePlumber gave them from
pw-dump. Each asks for the session with both cards heard as a fixture, so any test runs alone
with `-k`. The cards are named in the machine microphones.nix builds, so the tests need no
value from Nix.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The rank WirePlumber gives a webcam's microphone, under a USB microphone's 2100.
YIELDED = "1900"
# How long PipeWire may take to hear a card come or go.
HEARING = dt.timedelta(seconds=30)
# Where the dummy cards' driver is bound, and the second card's device: unbinding it is the
# kernel's hot-unplug, which a USB sound card's removal takes too.
DRIVER = "/sys/bus/platform/drivers/snd_dummy"
WEBCAM = "snd_dummy.1"


class Session:
    """Joel's session, its PipeWire hearing the dummy cards."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on Joel's session on that machine."""
        self.machine = machine

    def rank(self, card: int) -> str:
        """Give the rank WirePlumber gave the microphone on a card, by the card's number."""
        return as_account(
            self.machine,
            "pw-dump | jq -r '.[] | .info.props? // empty"
            ' | select(."media.class" == "Audio/Source"'
            f' and ."alsa.card" == {card}) | ."priority.session"'
            "'",
        ).strip()

    def sources_are(self, count: int) -> None:
        """Wait until PipeWire has as many microphones as given."""
        self.machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-dump'"
            ' | jq -e \'[.[] | select(.info.props?."media.class" == "Audio/Source")]'
            f" | length == {count}'",
            timeout=HEARING,
        )


@pytest.fixture(scope="module")
def session(machine: BaseMachine) -> Session:
    """Give Joel's session, its PipeWire running and both microphones heard."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_account(machine, "systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    heard = Session(machine)
    heard.sources_are(2)
    return heard


def test_a_webcam_s_microphone_yields_to_a_usb_microphone(session: Session) -> None:
    """A webcam's microphone yields to a USB microphone."""
    assert session.rank(1) == YIELDED


def test_other_microphones_keep_their_rank(session: Session) -> None:
    """Other microphones keep their rank."""
    assert session.rank(0) not in ("", YIELDED)


def test_a_webcam_plugged_back_in_still_yields(session: Session) -> None:
    """Unplugged and plugged back in, the webcam's microphone still yields."""
    session.machine.succeed(f"echo {WEBCAM} > {DRIVER}/unbind")
    session.sources_are(1)
    # The card's number is free again only once everything holding it has let go.
    session.machine.wait_until_fails("test -e /proc/asound/card1", timeout=HEARING)
    session.machine.succeed(f"echo {WEBCAM} > {DRIVER}/bind")
    session.sources_are(2)
    assert session.rank(1) == YIELDED

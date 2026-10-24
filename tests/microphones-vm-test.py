"""A webcam's microphone ranks below a USB microphone's, and others keep their own rank.

tests/microphones.nix runs this, and its opening comment says what the test proves. Two dummy
sound cards stand in for microphones, the second marked a webcam by udev, and the script reads
the rank WirePlumber gave each from pw-dump. It needs no value from Nix.
"""

import datetime as dt
from typing import TYPE_CHECKING

from vm_helpers import as_account

if TYPE_CHECKING:
    from driver_names import machine, subtest


def rank(card: int) -> str:
    """Give the rank WirePlumber gave the microphone on a card, by the card's number."""
    return as_account(
        machine,
        "pw-dump | jq -r '.[] | .info.props? // empty"
        ' | select(."media.class" == "Audio/Source"'
        f' and ."alsa.card" == {card}) | ."priority.session"'
        "'",
    ).strip()


def sources_are(count: int) -> None:
    """Wait until PipeWire has as many microphones as given."""
    machine.wait_until_succeeds(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-dump'"
        ' | jq -e \'[.[] | select(.info.props?."media.class" == "Audio/Source")]'
        f" | length == {count}'",
        timeout=dt.timedelta(seconds=30),
    )


# Where the dummy cards' driver is bound, and the second card's device: unbinding it is the
# kernel's hot-unplug, which a USB sound card's removal takes too.
DRIVER = "/sys/bus/platform/drivers/snd_dummy"
WEBCAM = "snd_dummy.1"

machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
as_account(machine, "systemctl --user start pipewire.service")
machine.wait_for_unit("wireplumber.service", "joel")
sources_are(2)

with subtest("a webcam's microphone yields to a USB microphone"):
    webcam = rank(1)
    machine.log(f"the webcam's microphone ranks {webcam}")
    assert webcam == "1900", webcam

with subtest("other microphones keep their rank"):
    other = rank(0)
    machine.log(f"the other microphone ranks {other}")
    assert other not in ("", "1900"), other

with subtest("unplugged and plugged back in, the webcam's microphone still yields"):
    machine.succeed(f"echo {WEBCAM} > {DRIVER}/unbind")
    sources_are(1)
    # The card's number is free again only once everything holding it has let go.
    machine.wait_until_fails("test -e /proc/asound/card1", timeout=dt.timedelta(seconds=30))
    machine.succeed(f"echo {WEBCAM} > {DRIVER}/bind")
    sources_are(2)
    webcam = rank(1)
    machine.log(f"plugged back in, the webcam's microphone ranks {webcam}")
    assert webcam == "1900", webcam

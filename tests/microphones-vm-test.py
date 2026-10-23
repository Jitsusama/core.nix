"""A webcam's microphone ranks below a USB microphone's, and others keep their own rank.

tests/microphones.nix runs this, and its opening comment says what the test proves. Two dummy
sound cards stand in for microphones, the second marked a webcam by udev, and the script reads
the rank WirePlumber gave each from pw-dump. It needs no value from Nix.
"""

import shlex
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest


def as_joel(command: str) -> str:
    """Run a command in Joel's account, with his PipeWire's runtime folder, and give its output."""
    environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
    return machine.succeed("su - joel -c " + shlex.quote(environment + command))


def rank(card: int) -> str:
    """Give the rank WirePlumber gave the microphone on a card, by the card's number."""
    return as_joel(
        "pw-dump | jq -r '.[] | .info.props? // empty"
        ' | select(."media.class" == "Audio/Source"'
        f' and ."alsa.card" == {card}) | ."priority.session"'
        "'"
    ).strip()


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
as_joel("systemctl --user start pipewire.service")
machine.wait_for_unit("wireplumber.service", "joel")
machine.wait_until_succeeds(
    "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-dump'"
    ' | jq -e \'[.[] | select(.info.props?."media.class" == "Audio/Source")] | length == 2\''
)

with subtest("a webcam's microphone yields to a USB microphone"):
    webcam = rank(1)
    machine.log(f"the webcam's microphone ranks {webcam}")
    assert webcam == "1900", webcam

with subtest("other microphones keep their rank"):
    other = rank(0)
    machine.log(f"the other microphone ranks {other}")
    assert other not in ("", "1900"), other

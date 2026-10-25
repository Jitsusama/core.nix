"""Dictation tells its watchers it's listening only once the microphone sounds.

tests/dictation-listening.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the tests prove. The service runs in alice's account beside her
PipeWire, with nothing to transcribe with. Each test watches the service afresh, so any test runs
alone with `-k`; the test with a microphone makes its own and takes it away again.
"""

from __future__ import annotations

import datetime as dt
import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# How long a press with no microphone must go without its watchers hearing "listening": several
# times the 0.7 s a laptop's idle microphone took to start.
UNHEARD = dt.timedelta(seconds=3)
# How long a press with a microphone may take before its watchers hear "listening".
HEARD = dt.timedelta(seconds=10)
# The stand-in microphone, a silent source PipeWire keeps once made.
MICROPHONE = (
    "{ factory.name = support.null-audio-sink node.name = stand-in-microphone"
    " media.class = Audio/Source/Virtual audio.position = [ MONO ] object.linger = true }"
)


class Alice:
    """Alice's account, with her PipeWire and dictation's service running."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act in alice's account on that machine."""
        self.machine = machine

    def run(self, command: str) -> str:
        """Run a command as alice, with her runtime folder, giving its output."""
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return self.machine.succeed("su - alice -c " + shlex.quote(environment + command))

    def press(self) -> str:
        """Press dictation as the shell does, giving the state the service answers with."""
        return self.run("dictation toggle").strip()

    def said(self) -> list[str]:
        """Give each line the service has told the watcher so far."""
        return self.run("cat watched").split()


@pytest.fixture(scope="module")
def alice(machine: BaseMachine) -> Alice:
    """Boot the machine with alice's PipeWire and dictation's service running."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger alice")
    machine.wait_for_unit("user@1000.service")
    account = Alice(machine)
    account.run("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "alice")
    account.run(
        "systemd-run --user --unit dictation -E PATH=$PATH -E DICTATION_DEVICE=CPU"
        " $(command -v dictation) serve /var/empty"
    )
    machine.wait_until_succeeds("test -S /run/user/1000/dictation.sock")
    return account


@pytest.fixture
def watched(alice: Alice) -> Iterator[Alice]:
    """Watch the service from idle, as the shell does, into the file `watched`."""
    alice.run(
        "systemd-run --user --unit watch --same-dir -E PATH=$PATH sh -c"
        " '(echo watch; sleep infinity) | socat - UNIX-CONNECT:/run/user/1000/dictation.sock"
        " > watched'"
    )
    alice.machine.wait_until_succeeds("su - alice -c 'grep -qx idle watched'")
    yield alice
    alice.run("systemctl --user stop watch")


def test_a_press_with_no_microphone_isnt_said_to_listen(watched: Alice) -> None:
    """A press with no microphone isn't said to listen."""
    assert watched.press() == "listening"
    try:
        watched.machine.sleep(UNHEARD)
        assert "listening" not in watched.said(), watched.said()
    finally:
        watched.press()
    watched.machine.wait_until_succeeds("su - alice -c 'grep -q heard-nothing watched'")
    assert "listening" not in watched.said(), watched.said()


def test_a_press_is_said_to_listen_once_the_microphone_sounds(watched: Alice) -> None:
    """A press is said to listen once the microphone sounds."""
    watched.run(f"pw-cli create-node adapter {shlex.quote(MICROPHONE)}")
    watched.machine.wait_until_succeeds(
        "su - alice -c 'XDG_RUNTIME_DIR=/run/user/1000 wpctl inspect @DEFAULT_AUDIO_SOURCE@'"
        " | grep -q stand-in-microphone"
    )
    try:
        assert watched.press() == "listening"
        try:
            watched.machine.wait_until_succeeds(
                "su - alice -c 'grep -qx listening watched'", timeout=HEARD
            )
        finally:
            watched.press()
        watched.machine.wait_until_succeeds("su - alice -c 'tail -1 watched | grep -qx idle'")
    finally:
        watched.run(
            "pw-cli destroy $(pw-cli ls Node | awk '/^\\tid [0-9]+,/ {id = $2}"
            ' /node.name = "stand-in-microphone"/ {sub(",", "", id); print id}\')'
        )

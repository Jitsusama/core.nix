"""A microphone in Joel's PipeWire is offered noise-cancelled, heard only while it's recorded.

tests/noise-cancelled.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the tests prove. The noise-cancelled program is started once
for the module, and each test records from what it offers.
"""

from __future__ import annotations

import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# Where Joel's PipeWire is.
RUNTIME = "/run/user/1000"
# The stand-in microphone, and what the program offers for it.
MICROPHONE = "microphone"
OFFERED = f"noise-cancelled.{MICROPHONE}"
CAPTURE = f"{OFFERED}.capture"
# The program's capture of the microphone, by its state.
RUNNING = f'.[] | select(.info.props["node.name"] == "{CAPTURE}") | .info.state'
DESCRIPTION = "Desk Microphone"
# How long PipeWire and the program have to answer, and the model to load on the VM's CPU.
ANSWERS_WITHIN = timedelta(minutes=1)
LOADS_WITHIN = timedelta(minutes=3)
# The clips, made as the test builds.
CLIPS = "/etc/noise-cancelled-clips"
# How much quieter the noise must come out than the microphone hears it, and how near its
# voice must stay, in dB. Steady pink noise comes out of the denoiser nearly 40 dB quieter
# in the build's own check; here two recordings start as PipeWire starts them, so the bar
# is set well short of that.
QUIETER_DB = 10.0
NEAR_DB = 10.0


class Joel:
    """Joel's account on the machine."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act as Joel on that machine."""
        self.machine = machine

    @staticmethod
    def as_joel(command: str) -> str:
        """Give a command run as Joel, with his runtime folder."""
        return "su - joel -c " + shlex.quote(f"export XDG_RUNTIME_DIR={RUNTIME}; cd ~; {command}")

    def run(self, command: str) -> str:
        """Run a command as Joel, giving its output."""
        return self.machine.succeed(self.as_joel(command))

    def wait_for(self, command: str, timeout: timedelta) -> None:
        """Wait until a command run as Joel succeeds."""
        self.machine.wait_until_succeeds(self.as_joel(command), timeout=timeout)

    def said(self) -> str:
        """Give what the program has logged."""
        return self.run("journalctl --user -u noise-cancelled --no-pager -o cat")

    def wait_until_said(self, words: str, timeout: timedelta) -> None:
        """Wait until the program logs these words."""
        self.wait_for(
            f"journalctl --user -u noise-cancelled -o cat | grep -qF {shlex.quote(words)}",
            timeout,
        )

    def loudness(self, recording: str) -> float:
        """Give a recording's loudness, in dB below full scale."""
        stats = self.run(f"sox {recording} -n stats 2>&1")
        line = next(line for line in stats.splitlines() if line.startswith("RMS lev dB"))
        return float(line.split()[3])

    def recorded(self, clip: str) -> tuple[float, float]:
        """Play a clip into the microphone, recording it raw and noise-cancelled at once."""
        for name, target in (("raw", MICROPHONE), ("cleaned", OFFERED)):
            self.run(
                f"systemd-run --user --collect --unit=record-{name} pw-record --target {target}"
                f" --rate 48000 --channels 1 --format s16 {name}.wav"
            )
        self.wait_for(f'test "$(pw-dump | jq -r {shlex.quote(RUNNING)})" = running', ANSWERS_WITHIN)
        self.run(f"pw-play --target mic-in {CLIPS}/{clip}")
        self.run("systemctl --user stop record-raw record-cleaned")
        return self.loudness("raw.wav"), self.loudness("cleaned.wav")

    def explain(self) -> None:
        """Print what PipeWire and the program said, for a test that failed."""
        print(self.run("pw-link -l || true"))
        print(self.said())


@pytest.fixture(scope="module")
def joel(machine: BaseMachine, values: dict[str, Any]) -> Iterator[Joel]:
    """Boot with Joel's sound, a microphone, and the noise-cancelled program running."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Joel(machine)
    joel.run("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    # A microphone that hears what's played into mic-in, carrying a device's id, as the node of
    # one plugged in does.
    joel.run(
        "systemd-run --user --unit=microphone pw-loopback"
        " --capture-props='node.name=mic-in media.class=Audio/Sink audio.position=[MONO]'"
        f" --playback-props='node.name={MICROPHONE} media.class=Audio/Source"
        f' audio.position=[MONO] device.id=0 node.description="{DESCRIPTION}"\''
    )
    joel.wait_for(f"pw-cli ls Node | grep -q 'node.name = \"{MICROPHONE}\"'", ANSWERS_WITHIN)
    joel.run(f"systemd-run --user --unit=noise-cancelled noise-cancelled serve {values['model']}")
    try:
        joel.wait_until_said(f"offering {DESCRIPTION}", ANSWERS_WITHIN)
        yield joel
    except Exception:
        joel.explain()
        raise


def test_the_microphone_is_offered_named_for_it(joel: Joel) -> None:
    """The program offers the microphone a second time, named for it, as a microphone."""
    query = f'.[] | select(.info.props["node.name"] == "{OFFERED}") | .info.props'
    props = joel.run(
        f"pw-dump | jq -r {shlex.quote(query)}"
        ' | jq -r \'[."node.description", ."media.class"] | join("|")\''
    ).strip()
    assert props == f"{DESCRIPTION}, Noise Cancelled|Audio/Source"


def test_the_microphone_isnt_heard_while_nothing_records(joel: Joel) -> None:
    """Offered and not recorded from, the microphone isn't heard and the model isn't loaded."""
    assert joel.run(f"pw-dump | jq -r {shlex.quote(RUNNING)}").strip() != "running"
    assert "cleaning" not in joel.said()
    assert "the model loaded" not in joel.said()


@pytest.fixture(scope="module")
def noise(joel: Joel) -> Iterator[tuple[float, float]]:
    """Noise alone, played in once the model has loaded, raw and noise-cancelled."""
    try:
        # The first recording loads the model, and a frame before it passes as heard, so one
        # short recording warms it before anything is measured.
        joel.recorded("noise.wav")
        joel.wait_until_said("the model loaded", LOADS_WITHIN)
        yield joel.recorded("noise.wav")
    except Exception:
        joel.explain()
        raise


def test_the_noise_comes_out_quieter(noise: tuple[float, float]) -> None:
    """Noise alone comes out of the noise-cancelled microphone much quieter than it's heard."""
    raw, cleaned = noise
    print(f"noise: {raw} dB raw, {cleaned} dB noise-cancelled")
    assert raw - cleaned >= QUIETER_DB


def test_a_voice_comes_through(joel: Joel, noise: tuple[float, float]) -> None:
    """A sentence spoken over the noise comes through about as loud as the microphone hears it."""
    assert noise
    raw, cleaned = joel.recorded("spoken.wav")
    print(f"voice: {raw} dB raw, {cleaned} dB noise-cancelled")
    assert abs(raw - cleaned) <= NEAR_DB


def test_the_microphone_rests_once_nothing_records(joel: Joel, noise: tuple[float, float]) -> None:
    """Once the recording stops, the program says it rests and stops hearing the microphone."""
    assert noise
    joel.wait_until_said(f"resting {DESCRIPTION}", ANSWERS_WITHIN)
    joel.wait_for(f'test "$(pw-dump | jq -r {shlex.quote(RUNNING)})" != running', ANSWERS_WITHIN)
    assert f"cleaning {DESCRIPTION}" in joel.said()

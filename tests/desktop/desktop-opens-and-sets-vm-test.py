"""The desktop test's opens and sets area.

Here the launcher opens and sets what the machine has: sound and its outputs' parts, the
power profile, displays, Wi-Fi, the addresses, Joel's timers and the VMs virsh lists.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import KEY_PACE, LAUNCHER_OPEN, SIGNING_IN, retry
from PIL import Image, ImageChops

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def test_the_launchers_sound_moves_sound_to_another_output_and_back(desktop: Desktop) -> None:
    """The launcher's Sound moves sound to another output, and back."""
    machine = desktop.machine
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        desktop.as_account("qs ipc call launcher collection sound")
        desktop.launcher_ready()
        desktop.shoot(f"launcher-sound-{output}")
        kind = output.removeprefix("stand-in-")
        desktop.type_in(kind, selecting=kind)
        # Return steps in, as on every device, its use first, and in normal,
        # where the step ends, Return plays sound through it.
        desktop.step_in("ret")
        desktop.launcher_until(sound_selected("Play Sound Through It"))
        machine.send_key("ret")
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(desktop.in_session(used), timeout=timedelta(seconds=10))
        # It changed something, so the launcher stays until it's closed.
        desktop.close_launcher()
    sets_the_output(desktop)
    sends_a_program(desktop)


def sends_a_program(desktop: Desktop) -> None:
    """Send a program playing on the speakers to the headphones, then back to the default.

    Its Plays On steps in to the default and each output, each said by its value alone. Picking
    one sets its stream's target.object, which WirePlumber moves it by, and the launcher stays
    with the program, on its new device's path, on Plays On, which says the device it's sent
    to, or the default and the device that is (VKCF3Z, 2026-11-09). pw-cat names no target of
    its own, so back on the default it follows the default where it moves next.
    """
    machine = desktop.machine
    # Silence, as fast as the node plays it, which pw-cat paces, on the
    # default, the speakers.
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=sent -- sh -c "
        + shlex.quote(
            "pw-cat --playback --raw --format s16 --rate 48000 --channels 2 - < /dev/zero"
        )
    )
    sound_heard_on(desktop, "stand-in-speakers")
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(lambda shown: "Playing" in shown["rows"])
    desktop.down_to("Playing")
    desktop.follow()
    desktop.launcher_until(lambda shown: "pw-cat" in shown["rows"])
    desktop.down_to("pw-cat")
    desktop.follow()
    # What WirePlumber keeps of where the stream goes: the headphones, then nothing.
    target = "pw-metadata -n default | grep -qF \"key:'target.object' value:'{}\""
    for device, choice, reads, kept in [
        ("stand-in-headphones", "stand-in-headphones", "stand-in-headphones", True),
        ("stand-in-speakers", "the default", "the default \u00b7 stand-in-speakers", False),
    ]:
        desktop.launcher_until(lambda shown: "Plays On" in shown["rows"])
        desktop.down_to("Plays On")
        desktop.step_in("ret")

        # As launcher_until calls them, for this choice and the device it sends to.
        def offered(shown: Json, choice: str = choice) -> bool:
            return choice in shown["rows"]

        def landed(shown: Json, device: str = device, reads: str = reads) -> bool:
            return bool(
                shown["path"][-3:] == [device, "Playing", "pw-cat"]
                and sound_selected("Plays On")(shown)
                and sound_reads("Plays On", reads)(shown)
            )

        desktop.launcher_until(offered)
        desktop.down_to(choice)
        machine.send_key("ret")
        desktop.launcher_until(landed)
        sent = desktop.in_session(target.format("stand-in-headphones"))
        if kept:
            machine.wait_until_succeeds(sent, timeout=WAIT)
        else:
            machine.wait_until_fails(sent, timeout=WAIT)
    desktop.close_launcher()
    # Following the default again, it goes where the default goes, and the
    # speakers are the default again after.
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        named = {
            node["info"]["props"].get("node.name"): int(node["id"])
            for node in sound_nodes(desktop)
            if node.get("type") == "PipeWire:Interface:Node"
        }
        desktop.as_account(f"wpctl set-default {named[output]}")
        sound_heard_on(desktop, output)
    desktop.as_account("systemctl --user stop sent")


def sound_heard_on(desktop: Desktop, output: str) -> None:
    """Wait until pw-cat's stream is linked to this output, as the sound server lists it."""

    def linked(_: bool) -> bool:  # As retry calls it.
        nodes = sound_nodes(desktop)
        props = {
            int(node["id"]): node["info"]["props"]
            for node in nodes
            if node.get("type") == "PipeWire:Interface:Node"
        }
        return any(
            link.get("type") == "PipeWire:Interface:Link"
            and props.get(link["info"]["output-node-id"], {}).get("application.name") == "pw-cat"
            and props.get(link["info"]["input-node-id"], {}).get("node.name") == output
            for link in nodes
        )

    retry(linked, timeout=WAIT)


def sets_the_output(desktop: Desktop) -> None:
    """Choose the output by :set output=, as Play Sound Through It does, and ask which it is.

    After output= the line lists the outputs by name, as each row names one; :set output?
    says the one in use, and :set input? the microphone listened through.
    """
    machine = desktop.machine

    def menu(shown: Json) -> list[str]:
        return [str(entry).strip() for entry in (shown["command"] or {}).get("menu", [])]

    def said(shown: Json) -> str:
        return str(shown["commandSaid"]) if shown["command"] is None else ""

    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        kind = output.removeprefix("stand-in-")

        def listed(shown: Json, kind: str = kind) -> bool:
            return any(kind in entry for entry in menu(shown))

        def told(shown: Json, kind: str = kind) -> bool:
            return kind in said(shown)

        machine.send_chars(":set output=", delay=KEY_PACE)
        desktop.launcher_until(listed)
        chosen = next(entry for entry in menu(desktop.showing()) if kind in entry)
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: shown["command"] is None)
        machine.send_chars(f":set {chosen}\n", delay=KEY_PACE)
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(desktop.in_session(used), timeout=timedelta(seconds=10))
        machine.send_chars(":set output?\n", delay=KEY_PACE)
        desktop.launcher_until(told)
    machine.send_chars(":set input?\n", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: said(shown) == "input=Stand-in microphone")
    desktop.close_launcher()


def sound_reads(name: str, said: str) -> Callable[[Json], bool]:
    """Make a test of whether the launcher shows a row by this name reading this."""
    return lambda shown: any(
        thing["reads"].startswith(name) and said in thing["reads"] for thing in shown["things"]
    )


def sound_selected(name: str) -> Callable[[Json], bool]:
    """Make a test of whether the launcher's selected row is this one."""
    return lambda shown: (shown["selected"] or {}).get("name") == name


def sound_device(desktop: Desktop, typed: str, named: str) -> None:
    """Open the launcher's Sound, and step into a device's parts by Return."""
    desktop.as_account("qs ipc call launcher collection sound")
    desktop.launcher_ready()
    desktop.type_in(typed, selecting=named)
    desktop.follow()


def sound_nodes(desktop: Desktop) -> list[Json]:
    """Give everything the sound server lists, from pw-dump.

    A dump taken as pw-record's stream started came back as JSON that didn't parse, once, so a
    dump that doesn't read is taken again.
    """
    dumps: list[list[Json]] = []

    def dumped(_: bool) -> bool:
        try:
            dumps.append(json.loads(desktop.as_account("pw-dump")))
        except json.JSONDecodeError:
            return False
        return True

    retry(dumped, timeout=timedelta(seconds=10))
    return dumps[-1]


def sound_recording(desktop: Desktop) -> int | None:
    """Give the id of pw-record's stream, as the sound server lists it, if it's there."""
    nodes = sound_nodes(desktop)
    return next(
        (
            int(node["id"])
            for node in nodes
            if node.get("type") == "PipeWire:Interface:Node"
            and node["info"]["props"].get("media.class") == "Stream/Input/Audio"
            and "pw-record"
            in {
                node["info"]["props"].get("application.name"),
                node["info"]["props"].get("node.name"),
            }
        ),
        None,
    )


# One walk through Sound, as its subtest was: each step starts where the last left it.
def test_the_launchers_sound_opens_an_outputs_and_a_microphones_parts(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """The launcher's Sound opens an output's and a microphone's parts."""
    machine = desktop.machine
    # The speakers, chosen, hold whether they're Muted first, one switch,
    # then their Level, Volume and Channels, named first; the null sink
    # has no card, so nothing more.
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Muted", "Level", "Volume"])
    assert "Mute" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.launcher_until(lambda shown: "Channels" in shown["rows"])
    desktop.launcher_until(sound_reads("Channels", "stereo"))
    desktop.launcher_until(sound_reads("Muted", "no"))
    # Nothing plays, so the Level reads silent, and no program is listed.
    desktop.launcher_until(sound_reads("Level", "silent"))
    assert "Playing" not in desktop.showing()["rows"]
    desktop.shoot("launcher-sound-output-parts")
    # Return on Volume, past Muted and the Level, steps into its choices,
    # each level saying its decibels in the footer.
    desktop.next_row()
    desktop.launcher_until(sound_selected("Level"))
    desktop.next_row()
    desktop.launcher_until(sound_selected("Volume"))
    desktop.launcher_until(lambda shown: shown["footer"].endswith(" dB"))
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Louder", "Quieter"])
    desktop.shoot("launcher-sound-volume")
    desktop.type_in("50", selecting="Volume to 50%")
    desktop.launcher_until(lambda shown: "-18.1 dB" in shown["footer"])
    desktop.land()
    machine.send_key("ret")
    sink = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.50'"
    machine.wait_until_succeeds(desktop.in_session(sink), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
    # The part says what it was set to.
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(sound_reads("Volume", "50%"))
    # Muted, first, is one switch keeping its name: Return mutes the
    # speakers and it says yes, the launcher staying; Return again unmutes.
    desktop.launcher_until(sound_selected("Muted"))
    machine.send_key("ret")
    hushed_sink = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED"
    machine.wait_until_succeeds(desktop.in_session(hushed_sink), timeout=timedelta(seconds=10))
    desktop.launcher_until(
        lambda shown: (
            shown["open"] and sound_selected("Muted")(shown) and sound_reads("Muted", "yes")(shown)
        )
    )
    desktop.shoot("launcher-sound-muted")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(hushed_sink), timeout=timedelta(seconds=10))
    desktop.launcher_until(sound_reads("Muted", "no"))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()

    # The microphone holds its own: whether it's Muted, a live Level, its
    # Gain, and who's Recording, which is nobody, though the Level's own
    # stream is reading it while it shows.
    sound_device(desktop, "microphone", "stand-in microphone")
    desktop.launcher_until(
        lambda shown: shown["rows"][:4] == ["Muted", "Level", "Gain", "Recording"],
    )
    desktop.launcher_until(sound_reads("Level", "%"))
    desktop.launcher_until(sound_reads("Muted", "no"))
    desktop.launcher_until(sound_reads("Recording", "nothing"))
    desktop.shoot("launcher-sound-input-parts")
    desktop.close_launcher()
    listening = f"/home/{desktop.account}/listening.wav"
    desktop.as_account(
        f"systemd-run --user --unit=listening pw-record --target stand-in-microphone {listening}"
    )

    def records(last: bool) -> bool:
        """Say whether pw-record's stream is there, and on the last try, what is."""
        if last:
            print(desktop.as_account("systemctl --user status listening; pw-cli ls Node"))
        return sound_recording(desktop) is not None

    retry(records, timeout=timedelta(seconds=30))
    recording = sound_recording(desktop)
    sound_device(desktop, "microphone", "stand-in microphone")
    desktop.launcher_until(lambda shown: "Recording" in shown["rows"])
    desktop.launcher_until(lambda shown: not sound_reads("Recording", "nothing")(shown))
    # l on Recording, then on the program, and Return on
    # its Muted mutes what it records. Recording is counted to.
    for _down in range(desktop.showing()["rows"].index("Recording")):
        desktop.next_row()
    desktop.launcher_until(sound_selected("Recording"))
    desktop.follow()
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Muted", "Louder", "Quieter"])
    desktop.shoot("launcher-sound-recording")
    desktop.launcher_until(sound_selected("Muted"))
    machine.send_key("ret")
    hushed = f"wpctl get-volume {recording} | grep -q MUTED"
    machine.wait_until_succeeds(desktop.in_session(hushed), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
    desktop.as_account("systemctl --user stop listening")
    desktop.as_account(f"rm -f {listening}")


def test_the_launchers_sound_chooses_noise_cancelling_by_ear(desktop: Desktop) -> None:
    """A microphone's Choose by Ear records, plays a clip and picks another, by its daemon.

    The noise-cancelled daemon stands in (stand-in-noise-cancelled.py), its clips in a known
    order, so B is DeepFilterNet gentle. One path through the connection: the record held from
    its socket, each act asked over it, and the record's change drawn; every case of each step
    is tests/by-ear.qml's.
    """
    desktop.as_account("systemctl --user start noise-cancelled-stand-in.service")
    sound_device(desktop, "microphone", "stand-in microphone")
    desktop.launcher_until(sound_reads("Noise Cancelling", "no noise cancelling"))
    desktop.down_to("Choose by Ear")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Record"])
    desktop.machine.send_key("ret")
    # Record becomes its count, then Record Again once the clips come after it.
    clips = ["Clip A", "Clip B", "Clip C", "Clip D"]
    desktop.launcher_until(lambda shown: shown["rows"] == ["Record Again", *clips])
    desktop.down_to("Clip A")
    desktop.machine.send_key("ret")
    desktop.launcher_until(sound_reads("Clip A", "playing"))
    desktop.down_to("Clip B")
    desktop.follow()
    # A step in leads with Return's own act, as every row's does, then Pick.
    desktop.launcher_until(lambda shown: shown["rows"] == ["Play", "Pick"])
    desktop.down_to("Pick")
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Picked", *clips])
    desktop.launcher_until(sound_reads("Picked", "B \u00b7 DeepFilterNet, gentle"))
    desktop.launcher_until(sound_reads("Clip D", "no noise cancelling"))
    # Leaving discards it, so the next opening starts afresh.
    desktop.close_launcher()
    sound_device(desktop, "microphone", "stand-in microphone")
    desktop.launcher_until(sound_reads("Noise Cancelling", "DeepFilterNet, gentle"))
    desktop.down_to("Choose by Ear")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Record"])
    desktop.close_launcher()
    desktop.as_account("systemctl --user stop noise-cancelled-stand-in.service")


def test_for_you_offers_choose_by_ear_after_a_call(desktop: Desktop) -> None:
    """An offer of Choose by Ear is For You's to-do, and Return goes to its microphone's.

    The service leaves the offer after a call on a microphone not chosen for where it is; the
    stand-in daemon tells it as the trial does. One path through the connection, Sound's to-do
    reaching For You and its trail reaching Choose by Ear, and its name typed from the top
    reaching it too; what the offer says, case by case, is tests/by-ear.qml's.
    """
    offer = json.dumps(
        {
            "microphone": "Stand-in microphone",
            "node": "stand-in-microphone",
            "spot": "the desk",
            "key": "home=6366013",
            "new": "spot",
        }
    )
    left = "~/.local/state/noise-cancelled/offer"
    desktop.as_account("mkdir -p ~/.local/state/noise-cancelled")
    desktop.as_account(f"echo {shlex.quote(offer)} > {left}")
    desktop.as_account("systemctl --user start noise-cancelled-stand-in.service")
    offered = "Choose by Ear for Stand-in microphone"
    desktop.as_account("qs ipc call launcher collection on")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "For You")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: offered in shown["rows"])
    desktop.down_to(offered)
    desktop.machine.send_key("ret")
    # Its microphone's Choose by Ear, waiting to record, and the offer taken.
    desktop.launcher_until(lambda shown: shown["rows"] == ["Record"])
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"test ! -e {left}"), timeout=timedelta(seconds=10)
    )
    desktop.close_launcher()
    # Its name typed from the top finds it again, offered or not.
    desktop.answer("choose by ear")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == offered)
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Record"])
    desktop.close_launcher()
    desktop.as_account("systemctl --user stop noise-cancelled-stand-in.service")


# A program's own Media and Sound, against a real player playing real
# sound: mpv, through its MPRIS script, playing two tones in turn through
# PipeWire to the speakers. What each act does is read back from the
# player over D-Bus and from the stream in PipeWire, never from the
# launcher, which is only asked what it shows. The tones are long and the
# playlist loops, so nothing moves on by itself, and Next and Previous
# always apply, as they do in a music player set to repeat.
#
# How long the player, PipeWire or the launcher may take to show a change.
WAIT = timedelta(seconds=15)
# How far a pixel's shade moves, of 255, to count as drawn anew rather than
# as noise.
CHANGED = 24
# How far above and below the line that changed most a row's dots reach,
# in pixels, which is less than the gap to the next row's.
ROW = 16


# One walk through a program's player and stream, as its subtest was: each step starts where
# the last left it.
def test_a_programs_media_and_sound_work_its_real_player_and_its_stream(  # noqa: C901, PLR0915
    desktop: Desktop,
) -> None:
    """A program's Media and Sound work its real player and its stream."""
    machine = desktop.machine
    tones = f"/home/{desktop.account}/tones"
    desktop.as_account(f"mkdir -p {tones}")
    for title, pitch in [("First tone", 440), ("Second tone", 660)]:
        desktop.as_account(
            f"{desktop.values['ffmpeg_headless']}/bin/ffmpeg -loglevel error -f lavfi"
            f" -i sine=frequency={pitch}:sample_rate=8000:duration=600"
            f" -metadata title='{title}' -metadata artist='Test tones' {tones}/{pitch}.flac"
        )
    desktop.as_account(
        "systemd-run --user --unit=tones mpv --ao=pipewire --no-video --no-terminal"
        f" --loop-playlist=inf {tones}/440.flac {tones}/660.flac"
    )
    # Its name on the bus ends in an instance of its own making.
    named = "busctl --user list --acquired | grep -o '^org\\.mpris\\.MediaPlayer2\\.mpv\\S*'"
    bus = machine.wait_until_succeeds(desktop.in_session(named), timeout=WAIT).strip()
    mpris = (
        f"busctl --user --json=short get-property {bus}"
        " /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player"
    )

    def player(field: str) -> Any:  # noqa: ANN401, since D-Bus says what type it is.
        """Give one of the player's own properties, as it tells D-Bus."""
        return json.loads(desktop.as_account(f"{mpris} {field}"))["data"]

    def playing(status: str, title: str) -> None:
        """Wait until the player says it's in this state, playing this tone."""

        def says(_: bool) -> bool:
            return bool(
                player("PlaybackStatus") == status
                and player("Metadata")["xesam:title"]["data"] == title
            )

        retry(says, timeout=timedelta(seconds=15))

    def streams() -> list[int]:
        """Give the ids of the streams mpv plays through, as PipeWire lists them."""
        return [
            int(node["id"])
            for node in sound_nodes(desktop)
            if node.get("type") == "PipeWire:Interface:Node"
            and node["info"]["props"].get("media.class") == "Stream/Output/Audio"
            and node["info"]["props"].get("application.name") == "mpv"
        ]

    playing("Playing", "First tone")
    retry(lambda _: streams() != [], timeout=WAIT)

    def heard(volume: str) -> None:
        """Wait until PipeWire says mpv's stream is this loud, as wpctl says it.

        Its stream is looked up again on every try: mpv can open a new one as it skips,
        pauses and plays, and the launcher turns the one open now, so an id read before
        then names a stream that's gone, and the wait never ends.
        """

        def loud(_: bool) -> bool:
            said = f"wpctl get-volume {{}} | grep -qxF 'Volume: {volume}'"
            return any(
                machine.execute(desktop.in_session(said.format(tone)))[0] == 0 for tone in streams()
            )

        retry(loud, timeout=WAIT)

    def reads(name: str, said: str) -> Callable[[Json], bool]:
        """Make a test of whether the launcher shows a row by this name reading this."""
        return lambda shown: any(
            thing["reads"].startswith(name) and said in thing["reads"] for thing in shown["things"]
        )

    def controls(part: str = "") -> None:
        """Open the launcher on mpv's controls, and on one of its parts when one is named."""
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("mpv", selecting="mpv Media Player")
        desktop.into_program()
        desktop.launcher_until(lambda shown: {"Media", "Sound"} <= set(shown["rows"]))
        if part:
            desktop.type_in(part.lower(), selecting=part)
            desktop.follow()

    def press(name: str, then: str = "", becomes: str = "", *, lands: bool = True) -> None:
        """Press Return on a row by its name: it stays, saying so or becoming its opposite.

        Each changes how the program plays, so the launcher stays. One that says where it is
        now has what was typed cleared for the next; one that becomes its opposite is closed.
        Each but a skip is landed on first; a skip is pressed again and again, and the key
        language says it changes nothing, so one Return from the find does it.
        """
        desktop.type_in(name.lower(), selecting=name)
        if lands:
            desktop.land()
        machine.send_key("ret")
        if then:
            desktop.launcher_until(lambda shown: shown["footer"] == then)
            desktop.normal()
            machine.send_chars("cc", delay=KEY_PACE)
            desktop.launcher_until(lambda shown: shown["typed"] == "")
            # cc leaves it on the find; back in normal, a changing row's
            # Return does it rather than landing.
            desktop.normal()
        else:
            desktop.launcher_until(lambda shown: becomes in shown["rows"])
            desktop.close_launcher()

    # l on the program holds Media and Sound, each saying
    # how it stands.
    controls()
    desktop.shoot("launcher-program-parts")
    desktop.launcher_until(reads("Media", "playing"))
    desktop.launcher_until(reads("Sound", "100%"))
    desktop.type_in("media", selecting="Media")
    desktop.follow()
    desktop.shoot("launcher-program-media")
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Pause", "Next", "Previous", "First tone"]
    )
    desktop.launcher_until(reads("First tone", "Test tones"))
    # Next and Previous keep the launcher, saying what's on now, and what's
    # playing follows.
    press("Next", then="Second tone \u00b7 Test tones", lands=False)
    playing("Playing", "Second tone")
    desktop.launcher_until(lambda shown: shown["rows"][-1] == "Second tone")
    press("Previous", then="First tone \u00b7 Test tones", lands=False)
    playing("Playing", "First tone")
    desktop.launcher_until(lambda shown: shown["rows"][-1] == "First tone")
    # Paused from elsewhere, as a media key does, Pause says Play.
    desktop.as_account("qs ipc call music toggle")
    playing("Paused", "First tone")
    desktop.launcher_until(lambda shown: shown["rows"][0] == "Play")
    press("Play", becomes="Pause")
    playing("Playing", "First tone")
    controls("Media")
    press("Pause", becomes="Play")
    playing("Paused", "First tone")
    controls()
    desktop.launcher_until(reads("Media", "paused"))
    desktop.close_launcher()

    # Sound's Louder and Quieter step the stream by a twentieth, keeping
    # the launcher, and its volume follows. The stream is only ever set
    # from the launcher here: set from elsewhere, as by wpctl, Quickshell
    # now and then reads the stream's volume back from before the change
    # and keeps it until the next one, which is PipeWire's and Quickshell's
    # to mend, not the launcher's.
    controls("Sound")
    desktop.shoot("launcher-program-sound")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Muted", "Louder", "Quieter", "Volume"])
    desktop.launcher_until(reads("Volume", "100%"))
    press("Quieter", then="now 95%")
    heard("0.95")
    desktop.launcher_until(reads("Volume", "95%"))
    press("Louder", then="now 100%")
    heard("1.00")
    desktop.launcher_until(reads("Volume", "100%"))
    # Muted is one switch keeping its name: Return mutes the stream, the
    # launcher staying, and it says yes; the program's Sound says muted.
    # Typed on the find, Return lands on it first.
    desktop.launcher_until(reads("Muted", "no"))
    desktop.type_in("muted", selecting="Muted")
    desktop.land()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["open"] and reads("Muted", "yes")(shown))
    desktop.close_launcher()
    heard("1.00 [MUTED]")
    controls()
    desktop.launcher_until(reads("Sound", "muted"))
    desktop.type_in("sound", selecting="Sound")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][0] == "Muted")
    desktop.launcher_until(reads("Muted", "yes"))
    desktop.launcher_until(reads("Volume", "muted"))
    # Louder is meant to be heard, so it unmutes.
    press("Louder", then="now 100%")
    heard("1.00")
    # :setl sets the option of what it's in, the program's Sound, as Vim's
    # :setlocal sets the buffer's: it mutes the stream alone, never the
    # speakers, and says how it stands.
    machine.send_chars(":setl mute\n", delay=KEY_PACE)
    heard("1.00 [MUTED]")
    machine.fail(desktop.in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED"))
    machine.send_chars(":setl mute?\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "mute"
    )
    machine.send_chars(":setl nomute\n", delay=KEY_PACE)
    heard("1.00")

    # The volume's bar follows too, filling from the left by the volume:
    # twenty places in ten characters, so a twentieth more lights one
    # place just past where half ends. Each is measured with the volume
    # row selected and only two figures beside it, so nothing else on the
    # screen moves and the graphs' column stays put. Page Down stops on
    # the last row, the volume; Louder is two rows up, Quieter one.
    loudness = {"now": 100}

    def foot(said: str) -> Callable[[Json], bool]:
        """Make a test of whether the launcher's foot says this."""
        return lambda shown: bool(shown["footer"] == said)

    def shown_at(volume: int) -> Image.Image:
        """Turn the stream to this loud from the launcher, and give the screen at its volume."""
        steps = (volume - loudness["now"]) // 5
        machine.send_key("pgdn")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Volume")
        for _up in range(2 if steps > 0 else 1):
            desktop.previous_row()
        turning = "Louder" if steps > 0 else "Quieter"
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == turning)
        for step in range(1, abs(steps) + 1):
            now = loudness["now"] + (5 if steps > 0 else -5) * step
            machine.send_key("ret")
            desktop.launcher_until(foot(f"now {now}%"))
        loudness["now"] = volume
        heard(f"0.{volume}")
        machine.send_key("pgdn")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Volume")
        desktop.launcher_until(reads("Volume", f"{volume}%"))
        return desktop.settled_screen(f"launcher-program-volume-{volume}")

    def changed(before: Image.Image, after: Image.Image) -> int:
        """Give the leftmost of what changed in the row that changed most."""
        difference = ImageChops.difference(before, after).convert("L")
        mask = difference.point(lambda shade: 255 if shade > CHANGED else 0)
        width, height = mask.size
        data = mask.tobytes()
        counts = [width - data[y * width : (y + 1) * width].count(0) for y in range(height)]
        busiest = max(range(height), key=counts.__getitem__)
        if counts[busiest] == 0:
            before.save(machine.out_dir / "launcher-program-unchanged-before.png")
            after.save(machine.out_dir / "launcher-program-unchanged-after.png")
        assert counts[busiest] > 0, "nothing changed, as the pictures kept show"
        box = mask.crop((0, busiest - ROW, width, busiest + ROW)).getbbox()
        assert box is not None
        return box[0]

    most, more, half, tenth = (shown_at(volume) for volume in [90, 55, 50, 10])
    filling = changed(tenth, half)
    past_half = changed(half, more)
    to_most = changed(half, most)
    assert filling < past_half, (filling, past_half)
    assert abs(past_half - to_most) <= 1, (past_half, to_most)
    desktop.close_launcher()
    desktop.as_account("systemctl --user stop tones")
    machine.succeed(f"rm -r {tones}")


def test_the_media_place_lists_each_player_and_its_acts_reach_it(desktop: Desktop) -> None:
    """The Media place lists each player, and its acts and :set reach the player itself.

    Two players, one playing and one paused, each a row saying how it stands; Return on the
    paused one plays it, a step in holds its acts, and :set shuffle reaches the one that
    matters now, as MPRIS tells D-Bus.
    """
    machine = desktop.machine
    # One track under two names, so the rows, alike but for their files, are told apart.
    track = desktop.values["track"]
    copy = "~/second-take.mp3"
    desktop.as_account(f"install -m 644 {track} {copy}")
    for unit, paused, played in [("first-player", "", track), ("second-player", " --pause", copy)]:
        desktop.as_account(
            f"systemd-run --user --collect --unit={unit} mpv --ao=null --no-video"
            f" --no-terminal --loop{paused} {played}"
        )
    try:
        named = "busctl --user list --acquired | grep -c '^org\\.mpris\\.MediaPlayer2\\.mpv'"
        machine.wait_until_succeeds(desktop.in_session(f"test $({named}) -ge 2"), timeout=WAIT)

        def stands(shown: Json) -> bool:
            sides = [thing["reads"] for thing in shown["things"]]
            return (
                any("paused \u00b7 " in side for side in sides)
                and any(
                    re.search(r"\d:\d\d of \d:\d\d", side) and "paused" not in side
                    for side in sides
                )
                and any("\u00b7 second-take.mp3 \u00b7" in side for side in sides)
            )

        # Shot at each of Joel's scales, then back at 1 for what follows.
        for scale in ("1.5", "2", "1"):
            desktop.as_account(f"niri msg output winit scale {scale}")
            at = "" if scale == "1" else f"-at-{scale.replace('.', '-')}"
            desktop.as_account("qs ipc call launcher toggle")
            desktop.launcher_ready()
            desktop.type_in("media", selecting="Media")
            desktop.step_in("ret")
            desktop.launcher_until(stands)
            desktop.shoot(f"launcher-media{at}")
            # Tab into a player holds its acts, the first named for what it will do.
            desktop.into_program()
            desktop.launcher_until(lambda shown: shown["rows"][:1] in (["Pause"], ["Play"]))
            desktop.shoot(f"launcher-media-player{at}")
            desktop.close_launcher()
        playing = "busctl --user --json=short get-property {bus} /org/mpris/MediaPlayer2"
        buses = desktop.as_account(
            "busctl --user list --acquired | grep -o '^org\\.mpris\\.MediaPlayer2\\.mpv\\S*'"
        ).split()
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.normal()
        machine.send_chars(":set shuffle\n", delay=KEY_PACE)

        def shuffled(_: bool) -> bool:
            return any(
                json.loads(
                    desktop.as_account(
                        f"{playing.format(bus=bus)} org.mpris.MediaPlayer2.Player Shuffle"
                    )
                )["data"]
                for bus in buses
            )

        retry(shuffled, timeout=WAIT)
        desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user stop first-player second-player || true")
        desktop.as_account(f"rm -f {copy}")


def test_the_launcher_sets_the_power_profile(desktop: Desktop) -> None:
    """The launcher sets the power profile, staying to say so, from a step in too."""
    machine = desktop.machine
    # By its name, power saver last, where what follows starts from. Setting
    # it changes something, so Return keeps the launcher, the row saying
    # it's chosen, and Ctrl+] then Return does the same, never closing
    # (Joel, 2026-11-08: a pick that closed the launcher was lost), with
    # no other chord with Ctrl said anywhere. Each pick is said in the
    # journal, and so is the daemon, read back, agreeing it's taken.
    journal = "journalctl --user --unit=quickshell --no-pager --output=cat --since=-30s"
    for typed, profile, name, stepping in [
        ("balanced power", "balanced", "Balanced Power", False),
        ("power saver", "power-saver", "Power Saver", True),
    ]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in(typed, selecting=name)
        # In normal, where Ctrl+] goes in; on the find letters type, so it names none.
        desktop.normal()
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("told", [])[1:2] == ["Tab in"]
        )
        told = desktop.showing()["selected"]["told"]
        assert not any(key.startswith("Ctrl") for key in told[2:]), told
        if stepping:
            # Its own act leads the step, selected.
            desktop.follow()
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == shown["rows"][0]
            )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )

        # In the step, its act selected says nothing of its own, and stays.
        def stays_open(shown: Json, stepped: bool = stepping) -> bool:
            chosen = (shown["selected"] or {}).get("side") == "chosen"
            return bool(shown["open"]) and (stepped or chosen)

        desktop.launcher_until(stays_open)
        option = profile.removeprefix("power-")
        for line in [f"set power profile {option}", f"power profile {option} taken"]:
            machine.wait_until_succeeds(
                desktop.in_session(f"{journal} | grep -aqF 'launcher: {line}'"),
                timeout=timedelta(seconds=10),
            )
        desktop.close_launcher()
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # A profile that isn't balanced is a mode left on, so Now lists it,
    # saying which.
    machine.send_key("meta_l-o")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["now"]
            and any(
                thing["reads"].startswith("Power Profile \u00b7 Power Saver")
                for thing in shown["things"]
            )
        )
    )
    desktop.close_launcher()
    # And as the setting it is: one part saying which is chosen, its
    # choices a step in by Return or Ctrl+], each named there as
    # the part reads it. Balanced is reached by the arrows, as Joel does,
    # and Return on it stays.
    for follows, profile, name, was in [
        (False, "balanced", "Balanced", "Power Saver"),
        (True, "power-saver", "Power Saver", "Balanced"),
    ]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("power profile", selecting="Power Profile")
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == was)  # noqa: B023
        if follows:
            desktop.follow()
        else:
            desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Power Saver", "Balanced"])
        # Typed, Return on the find lands first; by the arrows, in normal,
        # it does it at once.
        if follows:
            desktop.type_in(name.split()[0][:3].lower(), selecting=name)
            desktop.land()
        else:
            desktop.down_to(name)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "chosen")
        desktop.close_launcher()


# Ctrl+] on one of a collection's members goes into its own
# actions, as on anything at the top.


# What the machine chose for the charger and the battery, where the shell
# reads it (modules/power-profiles-daemon/follow.sh).
WANTED = "/run/power-profile/wanted"


def test_now_lists_the_power_profile_only_when_picked_by_hand(desktop: Desktop) -> None:
    """Now lists the power profile only when it isn't the machine's own choice."""
    machine = desktop.machine

    def listed(shown: Json) -> str:
        """Give what Now's power profile row reads, or nothing while it isn't there."""
        return next(
            (
                str(thing["reads"])
                for thing in shown["things"]
                if str(thing["reads"]).startswith("Power Profile") and not thing.get("gone")
            ),
            "",
        )

    def now_reads(reads: str) -> None:
        """Open Now and wait until it holds the keys and its power profile row reads so.

        Or is gone, for nothing. Now's rows can read so a frame before niri hands the launcher
        the keys, and an Escape to close it, sent then, went to the window underneath
        (2026-10-31, at 3a350c6c: Escape 300 ms before the keys).
        """
        machine.send_key("meta_l-o")
        desktop.launcher_until(
            lambda shown: (
                shown["keys"]
                and shown["held"] == ["now"]
                and (listed(shown).startswith(reads) if reads else listed(shown) == "")
            )
        )

    def close() -> None:
        desktop.close_launcher()

    # The VM runs on mains without a performance profile, so the machine
    # chose balanced, and says so; in use, Now has nothing to say of it.
    machine.wait_until_succeeds(f"grep -qx balanced {WANTED}", timeout=timedelta(seconds=10))
    machine.succeed("powerprofilesctl set balanced")
    now_reads("")
    close()
    # Power saver picked by hand is a mode left on, and its Return uses the
    # machine's choice again, keeping the launcher.
    machine.succeed("powerprofilesctl set power-saver")
    now_reads("Power Profile \u00b7 Power Saver")
    desktop.down_to("Power Profile")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "use")
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "powerprofilesctl get | grep -qx balanced", timeout=timedelta(seconds=10)
    )
    desktop.launcher_until(lambda shown: shown["open"] and listed(shown) == "")
    close()
    # The machine's own power saver, as it chooses on a low battery, says
    # nothing; the VM has no battery, so its choice is written as follow.sh
    # writes it. Balanced, then, is the one picked by hand.
    machine.succeed(f"echo power-saver > {WANTED}")
    now_reads("Power Profile \u00b7 Balanced")
    close()
    machine.succeed("powerprofilesctl set power-saver")
    now_reads("")
    close()
    machine.succeed(f"echo balanced > {WANTED}", "powerprofilesctl set balanced")


def test_the_launchers_displays_scales_a_display_by_keys_and_back(desktop: Desktop) -> None:
    """The launcher's Screens scales a screen by keys, and back."""
    machine = desktop.machine

    def scale() -> float:
        """Give the scale niri draws the display at."""
        outputs = json.loads(desktop.as_account("niri msg --json outputs"))
        return cast("float", next(iter(outputs.values()))["logical"]["scale"])

    # What desk-facts would say of the display, which this machine doesn't
    # run: each fact reads by the vocabulary's name for it, those asked most
    # first, in desk-facts' order: the depth, then the white point and
    # primaries, as the colours would be, each as the figures desk-facts
    # gives for the launcher to say.
    shown_as = next(iter(json.loads(desktop.as_account("niri msg --json outputs"))))
    said = {
        shown_as: [
            {"name": "EDID", "value": "1.4"},
            {"name": "depth", "value": "8 bit"},
            {"name": "white", "value": "6462 0.3134765625 0.3291015625"},
            {
                "name": "primaries",
                "value": "0.6396484375 0.330078125 0.2998046875 0.599609375 "
                "0.150390625 0.0595703125",
            },
        ]
    }
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/desk && echo '{json.dumps(said)}' > $XDG_RUNTIME_DIR/desk/.d"
        " && mv $XDG_RUNTIME_DIR/desk/.d $XDG_RUNTIME_DIR/desk/displays.json"
    )

    # Opened the first time by the launcher's own call, and the second by
    # the laptop's projection key, as Win+P: Super with what keyd's Colemak
    # makes a semicolon, which this machine takes as it comes.
    for wanted, inside, by_key in [(1.5, True, False), (scale(), False, True)]:
        if by_key:
            machine.send_key("meta_l-semicolon")
        else:
            desktop.as_account("qs ipc call launcher collection displays")
        desktop.launcher_ready()
        desktop.follow()
        # Its settings lead, each saying how it's set, their choices a step
        # in, there by Return on the setting, or found from here by what's
        # typed. The backlight lights the one screen there is, so its
        # Brightness comes first after Return's own Focus, and the Room
        # light once the sensor's said.
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:2] == ["Focus", "Brightness"]
                and [row for row in shown["rows"] if row != "Room Light"][2:10]
                == [
                    "Scale",
                    "Refresh",
                    "Resolution",
                    "Rotation",
                    "Colour Depth",
                    "White Point",
                    "Primaries",
                    "EDID",
                ]
            )
        )
        desktop.shoot("launcher-displays")
        # A scale is said as a multiple, "1.5\u00d7", by its value alone, and
        # typed as one inside, or found from outside by its setting and its
        # share, "scale 150", as niri says it.
        # A whole one starts every other too, and what was picked last leads
        # them, so the row is gone down to.
        multiple = f"{wanted:g}"
        if inside:
            desktop.type_in("scale", selecting="Scale")
            desktop.step_in("ret")
            desktop.launcher_until(lambda shown: "1.5\u00d7" in shown["rows"])
            desktop.type_in(multiple)
        else:
            desktop.type_in(f"scale {round(wanted * 100)}")
        desktop.down_to(f"{multiple}\u00d7")
        desktop.land()
        machine.send_key("ret")

        def scaled(_: bool, wanted: float = wanted) -> bool:  # As retry calls it.
            """Say whether the display is at the scale wanted."""
            return scale() == wanted

        retry(scaled, timeout=timedelta(seconds=15))
        # Scaling changes something, so the launcher stays until it's closed.
        desktop.close_launcher()

    # :setl scales the screen the cursor is in, as Scale's choice does; a
    # scale it doesn't offer says what it does, the screen named as its row
    # is, and sets nothing (VKCF3Z, 2026-11-09).
    was = scale()
    desktop.as_account("qs ipc call launcher collection displays")
    desktop.launcher_ready()
    desktop.follow()
    desktop.launcher_until(lambda shown: "Scale" in shown["rows"])
    desktop.normal()
    for typed, wanted in [("1.25", 1.25), (f"{was:g}", was)]:
        machine.send_chars(f":setl scale={typed}\n", delay=KEY_PACE)

        def scaled_by_line(_: bool, wanted: float = wanted) -> bool:  # As retry calls it.
            """Say whether the display is at the scale the line set."""
            return scale() == wanted

        retry(scaled_by_line, timeout=timedelta(seconds=15))
    machine.send_chars(":setl scale=1.3\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: (
            shown["command"] is None
            and str(shown["commandSaid"]).startswith("1.3\u00d7 not set \u00b7 ")
            and str(shown["commandSaid"]).endswith(" offers 1\u00d7 to 3\u00d7")
        )
    )
    assert scale() == was, f"a scale not offered was set: {scale()}"
    desktop.close_launcher()

    # A part is asked by its words from the top: the refresh rate.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("refresh rate")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and str((shown["selected"] or {}).get("name", "")).endswith(" Hz")
        )
    )
    desktop.close_launcher()


# One walk through joining networks, as its subtest was: each step starts where the last left
# it.
# How many times wifid has left Lantern as asked, so a wait sees a new one.
LEFT_LANTERN = "journalctl -u wifid | grep -c 'left Lantern, as asked' || true"


def left_lantern(desktop: Desktop) -> int:
    """Say how many times wifid has left Lantern as asked."""
    return int(desktop.machine.succeed(LEFT_LANTERN).strip() or "0")


def stands(thing: Json, side: str) -> bool:
    """Say whether a row is Lantern's, standing as `side` says, whatever bars it has.

    How a network stands leads its side, and its bars follow, as they move (VKCF3Z,
    2026-11-09), so the row reads as "Lantern \u00b7 connected \u00b7 4 of 4 bars \u00b7 network".
    """
    reads = str(thing["reads"])
    return reads.startswith(f"Lantern \u00b7 {side}") and reads.endswith(" \u00b7 network")


def disconnects_and_takes_back(desktop: Desktop) -> None:
    """End Lantern by dd, unasked; u joins it again, Ctrl+R leaves it again, and u once more.

    dd on a network disconnects it, never forgets it, and doesn't ask, since u joins it again;
    the foot says so until the next key. Disconnected, it has no end, since d never forgets a
    network, so . doing dd again says so. The last u leaves it joined for the steps after.
    """
    machine = desktop.machine

    def lantern_reads(side: str) -> Callable[[Json], bool]:
        return lambda shown: any(stands(thing, side) for thing in shown["things"])

    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.normal()
    # Normal's foot names d by the row's ending verb, as docs/design.md's foot does.
    told = desktop.showing()["selected"]["told"]
    assert "d disconnect" in told, told
    machine.send_chars("d")
    desktop.launcher_until(lambda shown: shown["modeWord"].startswith("d \u00b7 disconnect"))
    before = left_lantern(desktop)
    with desktop.told("ended 1"):
        machine.send_chars("d")
    desktop.launcher_until(
        lambda shown: shown["operated"].endswith("1 done \u00b7 u brings it back")
    )
    machine.wait_until_succeeds(
        f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
    )
    machine.send_key("ctrl-r")
    desktop.launcher_until(lambda shown: shown["operated"].endswith("Already at newest change"))
    desktop.launcher_until(lantern_reads("saved"))
    joined = "networkctl status wlan1 | grep -q 10.42.0."
    with desktop.told("undid 1"):
        machine.send_chars("u")
    desktop.launcher_until(lambda shown: shown["operated"].endswith("1 brought back"))
    machine.wait_until_succeeds(joined, timeout=timedelta(seconds=60))
    desktop.launcher_until(lantern_reads("connected"))
    before = left_lantern(desktop)
    with desktop.told("redid 1"):
        machine.send_key("ctrl-r")
    desktop.launcher_until(lambda shown: shown["operated"].endswith("1 done again"))
    machine.wait_until_succeeds(
        f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
    )
    # Once the row says it's only saved, . does dd again on it, which
    # finds nothing to end, and u then takes back the Ctrl+R.
    desktop.launcher_until(lantern_reads("saved"))
    machine.send_chars(".")
    desktop.launcher_until(lambda shown: shown["operated"].endswith("nothing to end"))
    with desktop.told("undid 1"):
        machine.send_chars("u")
    machine.wait_until_succeeds(joined, timeout=timedelta(seconds=60))
    desktop.close_launcher()


def commands_a_network(desktop: Desktop) -> None:
    """Press every key the command line takes on Lantern's row, then bind a range.

    : in normal opens it with Lantern's acts, Return's first; typing narrows it; Tab, Shift+Tab,
    the arrows, Ctrl+N and Ctrl+P choose a line and write it in, round to what was typed, the
    drawer saying what it acts on; Ctrl+E puts the menu away and what was typed back, Ctrl+Y
    takes what's chosen; Ctrl+U, Ctrl+W and Ctrl+H delete; a word that's no command says E492
    and does nothing; Escape, Ctrl+C and Backspace on an empty line let it go.
    """
    machine = desktop.machine

    def line(shown: Json) -> Json:
        return shown["command"] or {}

    def holds(typed: str, chosen: int = -1, menu: list[str] | None = None) -> None:
        desktop.launcher_until(
            lambda shown: (
                line(shown).get("line") == typed
                and line(shown).get("chosen") == chosen
                and (menu is None or line(shown).get("menu") == menu)
            )
        )

    disconnect = "disconnect Disconnect It"
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.normal()
    machine.send_chars(":")
    desktop.launcher_until(lambda shown: disconnect in line(shown).get("menu", []))
    machine.send_chars("disc", delay=KEY_PACE)
    holds("disc", menu=[disconnect])
    machine.send_key("tab")
    holds("disconnect", 0)
    desktop.launcher_until(lambda shown: shown["drawer"]["body"] == "on Lantern")
    for key, typed, chosen in [
        ("tab", "disc", -1),
        ("down", "disconnect", 0),
        ("up", "disc", -1),
        ("ctrl-n", "disconnect", 0),
        ("ctrl-p", "disc", -1),
        ("shift-tab", "disconnect", 0),
        ("ctrl-e", "disc", -1),
    ]:
        machine.send_key(key)
        holds(typed, chosen)
    desktop.launcher_until(lambda shown: line(shown)["menu"] == [])
    machine.send_chars("o")
    holds("disco", menu=[disconnect])
    machine.send_key("ctrl-n")
    machine.send_key("ctrl-y")
    holds("disconnect", menu=[])
    machine.send_key("ctrl-u")
    machine.send_chars("wifi on", delay=KEY_PACE)
    machine.send_key("ctrl-w")
    machine.send_key("ctrl-h")
    holds("wifi", menu=[])
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["command"] is None
            and shown["commandSaid"] == "E492: Not an editor command: wifi"
            and shown["mode"] == "normal"
        )
    )
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # :setl trust trusts the network the cursor is on, as its Trust It row
    # does, which turns on mDNS there, and notrust stops.
    for typed, mdns in [("trust", "+mDNS"), ("notrust", "-mDNS")]:
        machine.send_chars(f":setl {typed}\n", delay=KEY_PACE)
        machine.wait_until_succeeds(
            f"resolvectl status wlan1 | grep -qF -- {mdns}", timeout=timedelta(seconds=30)
        )
    for key in ["esc", "ctrl-c", "backspace"]:
        machine.send_chars(":")
        desktop.launcher_until(lambda shown: line(shown).get("line") == "")
        machine.send_key(key)
        desktop.launcher_until(
            lambda shown: shown["open"] and shown["command"] is None and shown["mode"] == "normal"
        )
    binds_a_range(desktop)


def binds_a_range(desktop: Desktop) -> None:
    """Over every network, say E481 for :%q, ask how many can for :%disc, then :g/Lant/disc.

    Lantern alone can disconnect, so :%disc asks once, naming how many can, and n leaves every
    network as it was; :g finds Lantern by name and disconnects it at once, as d would, since u
    joins it again, as it does here before Ctrl+R leaves it again.
    """
    machine = desktop.machine

    def says(typed: str, said: str) -> None:
        machine.send_chars(typed, delay=KEY_PACE)
        desktop.launcher_until(
            lambda shown: shown["command"] is None and said in (shown["commandSaid"] or "")
        )

    # Every network listed, not Lantern alone as "lant" found it, so % is them all.
    if desktop.showing()["open"]:
        desktop.close_launcher()
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: len(shown["rows"]) > 1 and "Lantern" in shown["rows"])
    desktop.normal()
    says(":%q\n", "E481: No range allowed")
    machine.send_chars(":%disc\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: (
            shown["question"].endswith("can disconnect; disconnect them? (y/n)")
            and shown["takesAnswer"]
        )
    )
    machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["question"] == "")
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    before = left_lantern(desktop)
    machine.send_chars(":g/Lant/disc\n", delay=KEY_PACE)
    machine.wait_until_succeeds(
        f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
    )
    desktop.launcher_until(lambda shown: shown["command"] is None and shown["question"] == "")
    with desktop.told("undid 1"):
        machine.send_chars("u")
    machine.wait_until_succeeds(
        "networkctl status wlan1 | grep -q 10.42.0.", timeout=timedelta(seconds=60)
    )
    before = left_lantern(desktop)
    with desktop.told("redid 1"):
        machine.send_key("ctrl-r")
    machine.wait_until_succeeds(
        f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
    )
    lists_earlier_lines(desktop)


def lists_earlier_lines(desktop: Desktop) -> None:
    """List the lines the command line ran by :his, newest first, and again by @:."""
    machine = desktop.machine
    if not desktop.showing()["open"]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.normal()
    for typed in [":his\n", "@:"]:
        machine.send_chars(typed, delay=KEY_PACE)
        desktop.launcher_until(
            lambda shown: (
                [one.strip() for one in (shown["command"] or {}).get("menu", [])[:2]]
                == ["his", "g/Lant/disc"]
            )
        )
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: shown["command"] is None)
    if desktop.showing()["open"]:
        desktop.close_launcher()


def says_how_wi_fi_is_set(desktop: Desktop) -> None:
    """List :set's options after a space, Tab writing one in, and ask how Wi-Fi is set.

    A space after :set lists every option with where it stands; typing narrows them and Tab
    writes the one chosen in; :set wifi? says the network joined, and a word no option names
    says E518. Then the session's: dnd=60 holds notices an hour and nodnd ends it, and
    keepawake keeps the machine awake until nokeepawake lets it sleep.
    """
    machine = desktop.machine

    def line(shown: Json) -> Json:
        return shown["command"] or {}

    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    machine.send_chars(":set ", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: (
            {"wifi wifi=Office", "volume"}
            <= {
                entry if entry.startswith("wifi ") else entry.split(" ")[0]
                for entry in line(shown).get("menu", [])
            }
        )
    )
    machine.send_chars("wi", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: line(shown).get("menu") == ["wifi wifi=Office"])
    machine.send_key("tab")
    desktop.launcher_until(
        lambda shown: line(shown).get("line") == "set wifi" and line(shown).get("chosen") == 0
    )
    machine.send_chars("?\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "wifi=Office"
    )
    machine.send_chars(":set wfii\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: shown["commandSaid"] == "E518: Unknown option: wfii" and shown["open"]
    )
    sets_the_session(desktop)
    desktop.close_launcher()


def sets_the_session(desktop: Desktop) -> None:
    """Hold notices an hour by :set dnd=60, end it by nodnd, and keep awake by keepawake."""
    machine = desktop.machine

    def says(typed: str, said: str) -> None:
        machine.send_chars(typed, delay=KEY_PACE)
        desktop.launcher_until(
            lambda shown: shown["command"] is None and shown["commandSaid"] == said
        )

    machine.send_chars(":set dnd=60\n", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: shown["command"] is None)
    says(":set dnd?\n", "dnd=60")
    machine.send_chars(":set nodnd\n", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: shown["command"] is None)
    says(":set dnd?\n", "nodnd")
    machine.send_chars(":set keepawake\n", delay=KEY_PACE)
    machine.wait_until_succeeds("ls /run/user/*/kept-awake", timeout=timedelta(seconds=30))
    says(":set keepawake?\n", "keepawake")
    machine.send_chars(":set nokeepawake\n", delay=KEY_PACE)
    machine.wait_until_fails("ls /run/user/*/kept-awake", timeout=timedelta(seconds=30))


def sets_wi_fi(desktop: Desktop) -> None:
    """Turn Wi-Fi on by :set wifi, off by :set nowifi, and on again, as the rfkill switch says."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    for typed, blocked in [("wifi", "no"), ("nowifi", "yes"), ("wifi", "no")]:
        machine.send_chars(f":set {typed}\n", delay=KEY_PACE)
        machine.wait_until_succeeds(
            f"rfkill list wifi | grep -q 'Soft blocked: {blocked}'", timeout=timedelta(seconds=15)
        )
    desktop.launcher_until(lambda shown: shown["open"] and shown["command"] is None)
    desktop.close_launcher()


def test_a_wi_fi_network_is_joined_from_the_launcher_a_new_one_asking_its_password(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """A Wi-Fi network is joined from the launcher, a new one asking its password."""
    machine = desktop.machine
    machine.wait_for_unit("lantern.service")
    machine.wait_for_unit("wifid.service")
    wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Lantern\"'",
        timeout=timedelta(seconds=60),
    )
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.shoot("launcher-wifi")
    desktop.type_in("lant", selecting="Lantern")
    # Never joined, it asks for its password, which a wrong one doesn't
    # satisfy; the launcher stays to say so.
    desktop.step_in("ret")
    # A password is a secret, as Vim's inputsecret() is: one line with no
    # modes and no mode word, where Ctrl+H deletes the letter before, Ctrl+W
    # the word before, the arrows only move the caret, Ctrl+U deletes back
    # to the start, and Escape gives it up.
    desktop.launcher_until(lambda shown: shown["asking"] and shown["modal"] is False)
    foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    assert not any(word["text"] == "insert" for word in foot), foot
    # Its padlock leads the line, never the find's /.
    assert desktop.showing()["sigil"] != "/", desktop.showing()
    machine.send_chars("not the pass", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: shown["caret"] == len("not the pass"))
    machine.send_key("ctrl-h")
    desktop.launcher_until(lambda shown: shown["caret"] == len("not the pas"))
    machine.send_key("ctrl-w")
    desktop.launcher_until(lambda shown: shown["caret"] == len("not the "))
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["caret"] == len("not the"))
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["caret"] == 0)
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["open"] and not shown["asking"])
    desktop.step_in("ret")
    desktop.type_in("not the password")
    desktop.shoot("launcher-wifi-password")
    with desktop.told("refused wifi Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    # wifid writes its journal after it answers, so the shell can hear the
    # refusal first: a run read the journal 91 ms before wifid's line was in it.
    machine.wait_until_succeeds(
        'journalctl -u wifid | grep -q "couldn\'t join Lantern: WrongPassword"',
        timeout=timedelta(seconds=10),
    )
    desktop.shoot("launcher-wifi-refused")
    machine.succeed(desktop.in_session(LAUNCHER_OPEN))
    desktop.type_in("glowing in the dark")
    machine.send_key("ret")
    # Joined means an address too, and wifid says so once it's checked
    # how far the network reaches.
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Lantern on wlan1'", timeout=timedelta(seconds=60)
    )
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # Once it's connected, the password is done with: the launcher steps
    # back to Lantern, which says so, and stays.
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and not shown["asking"]
            and (shown["selected"] or {}).get("name") == "Lantern"
            and str((shown["selected"] or {}).get("side")).startswith("connected")
        )
    )
    desktop.close_launcher()
    # Lantern wants a sign-in, so Sign in to it is found by typing, and
    # Return opens the window on its front desk.
    described = "varlinkctl --json=short call /run/wifid/io.jitsusama.Wifi"
    described += " io.jitsusama.Wifi.Describe '{}'"
    machine.wait_until_succeeds(
        f'{described} | grep -qF \'"reach":"signIn"\'', timeout=timedelta(seconds=60)
    )
    # Return's act, signing in to it, comes first, as it's all that's left
    # to do there, then Disconnect it.
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Sign in to It", "Disconnect It"])
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.shoot("launcher-sign-in")
    desktop.type_in("sign in", selecting="Sign in to Lantern")
    machine.send_key("ret")
    # The window is GTK and WebKit started cold, drawn in software,
    # which on a machine running other tests too has taken 42 seconds
    # to show and more than 60, so it's given three minutes for both.
    window = 'niri msg --json windows | grep -qF \'"app_id":"io.jitsusama.WifiSignIn"\''
    machine.wait_until_succeeds(desktop.in_session(window), timeout=SIGNING_IN)
    # Its button can only be pressed once the front desk has served
    # the page, which a window that has to try again does later.
    machine.wait_until_succeeds(
        "journalctl -u lantern-front-desk | grep -q 'GET /sign-in'", timeout=SIGNING_IN
    )
    # Return reaches its button only once the window holds the keyboard and
    # has drawn the page, and a window slow to start gets both later: a run
    # that pressed it as soon as the page was served lost the key.
    focused = 'niri msg --json focused-window | grep -qF \'"app_id":"io.jitsusama.WifiSignIn"\''
    machine.wait_until_succeeds(desktop.in_session(focused), timeout=SIGNING_IN)
    desktop.held_still(timedelta(seconds=20))
    desktop.shoot("sign-in", timeout=timedelta(seconds=20))
    # Its one button signs in, and the window closes as Lantern lets
    # the machine through, its notice taken back with it.
    with desktop.told("withdrew sign-in Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(window), timeout=timedelta(seconds=60))
    machine.succeed(f'{described} | grep -qF \'"reach":"online"\'')
    # Its name typed and Return, on the find, never disconnects it: Return
    # steps in, to what can be done to it, onto a row that changes nothing,
    # and it stays connected (Joel, 2026-11-04: "it is quite dangerous").
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.launcher_until(lambda shown: shown["mode"] == "find")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["open"] and "Disconnect It" in shown["rows"])
    first = desktop.showing()["selected"]["name"]
    assert first not in ("Disconnect It", "Forget It"), desktop.showing()["rows"]
    # The network's glyph leads the path, after the find's / where the line finds.
    shown = desktop.showing()
    mark = "/" if shown["mode"] == "find" else ""
    assert (shown["sigil"], shown["lead"] != "") == (mark, True), shown
    desktop.held_still(timedelta(seconds=3))
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # Disconnect It found never disconnects on Return, from the find or from normal: Return
    # puts :disconnect on the command line, naming it at the foot's end, and a Return
    # doubled with it does nothing more there (VKCF3Z's re-spec, 2026-11-05).
    desktop.type_in("disc", selecting="Disconnect It")
    told = desktop.showing()["selected"]["told"]
    assert told[:1] == ["\u21b5 :disconnect"], told
    desktop.press_at_once("ret ret")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "disconnect")
    desktop.held_still(timedelta(seconds=2))
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    shown = desktop.showing()
    assert (shown["returned"]["act"], shown["command"]["line"]) == ("lined", "disconnect"), shown
    assert "disconnect Disconnect It" in shown["command"]["menu"], shown["command"]
    desktop.shoot("launcher-lined")
    # Let go of, back on the find, and from normal too Return only writes the line, while
    # the row and the foot name d, which carries it at once.
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["command"] is None)
    desktop.normal()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("carried") == "d disconnect")
    told = desktop.showing()["selected"]["told"]
    assert told[0] == "\u21b5 :disconnect", told
    assert "d disconnect" in told, told
    assert not any(key.startswith(": ") for key in told), told
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "disconnect")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["command"] is None)
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # Shift, Ctrl and Alt with Return do nothing at all.
    for chord in ("shift-ret", "ctrl-ret", "alt-ret"):
        machine.send_key(chord)
    desktop.held_still(timedelta(seconds=2))
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    shown = desktop.showing()
    assert (shown["open"], shown["landed"]) == (True, ""), shown
    # Let go of, it's still joined, for the command line to disconnect it; Wi-Fi Off below
    # goes to the line too, and a fresh Return there does it.
    desktop.close_launcher()
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    disconnects_and_takes_back(desktop)
    commands_a_network(desktop)
    # The Office asks for an account: met for the first time, it asks
    # the account's name, its server's domain and its password in
    # turn, and joins.
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Office\"'",
        timeout=timedelta(seconds=60),
    )
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    desktop.step_in("ret")
    desktop.type_in("joel")
    desktop.step_in("ret")
    desktop.type_in("radius.office.test")
    desktop.step_in("ret")
    desktop.type_in("hunter22")
    desktop.shoot("launcher-wifi-account")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Office on wlan1'", timeout=timedelta(seconds=60)
    )
    desktop.launcher_until(lambda shown: shown["open"] and not shown["asking"])
    desktop.close_launcher()
    # Found from the root, Return goes straight inside the Office,
    # the networks' crumb above it saying where it lives, and -
    # comes back to the search with what was typed (VKCF3Z,
    # 2026-10-29: "\u2190 is where you came from; the crumbs are where it
    # lives").
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    # Joined, its Return typed on the find steps in too, rather than
    # disconnecting it, so the foot says "\u21b5 open", as stepping in does
    # wherever it's done; the find's arrows move the caret, so no arrow is
    # named there.
    told = desktop.showing()["selected"]["told"]
    assert told[:1] == ["\u21b5 open"], told
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) > 1 and shown["path"][-1:] == ["Office"] and shown["typed"] == ""
        )
    )
    desktop.step_out()
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "office")
    desktop.close_launcher()
    # Stepped into, a network says after what can be done to it what's
    # heard of it, each a part named first, the selected row's footer
    # saying more: the card's Signal and Speed once the card has read
    # them, each holding more, and its access points a step further in.
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    desktop.follow()
    office = len(desktop.showing()["held"])
    # The Signal says its dBm, always read, and how far over the noise
    # below; Return opens it, where the kernel, read by wifid while the
    # network shows, says what the card sends at.
    desktop.type_in("signal", selecting="Signal")
    desktop.launcher_until(lambda shown: str(shown["selected"]["side"]).endswith(" dBm"))
    desktop.launcher_until(lambda shown: "over the noise" in shown["footer"])
    desktop.follow()
    desktop.launcher_until(lambda shown: "Transmit Power" in shown["rows"])
    # - steps out with the word put back, and cc takes the word, onto the
    # find.
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == office and shown["typed"] == "signal"
    )
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    desktop.shoot("launcher-wifi-detail")
    # The Speed holds each way and the retries, Return into it.
    desktop.type_in("speed", selecting="Speed")
    desktop.follow()
    desktop.launcher_until(lambda shown: {"In", "Retries"} <= set(shown["rows"]))
    desktop.step_out()
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "speed")
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    # Its traffic each way since it connected, as wifid reads it, each
    # way's packets a step in.
    desktop.type_in("traffic", selecting="Traffic")
    desktop.follow()
    desktop.launcher_until(lambda shown: {"In", "Out"} <= set(shown["rows"]))
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == office and shown["typed"] == "traffic"
    )
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    # The access points, each named by its band and channel, and each
    # holding its own parts, its hardware address among them; the figure's
    # own Copy last (docs/design.md, "Copy ... is always the last").
    desktop.type_in("access", selecting="Access Points")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["2.4 GHz, Channel 1", "Copy"])
    desktop.shoot("launcher-wifi-points")
    desktop.step_in("ret")
    desktop.type_in("hardware", selecting="Hardware Address")
    assert desktop.showing()["selected"]["side"] == "02:00:00:00:02:00"
    desktop.close_launcher()
    reads_a_network_in_a_terminal(desktop)
    # Wi-Fi holds its own off, last, so stepping in never lands on it;
    # Network lists Wi-Fi alone.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and "Wi-Fi" in shown["rows"])
    assert "Wi-Fi Off" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.type_in("wi-fi", selecting="Wi-Fi")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            len(shown["rows"]) > 1
            and shown["rows"][-1] == "Wi-Fi Off"
            and (shown["selected"] or {}).get("name") != "Wi-Fi Off"
        )
    )
    desktop.close_launcher()
    says_how_wi_fi_is_set(desktop)
    # Turning Wi-Fi off is an action like any, found by name from the top. It ends what
    # Wi-Fi holds, so Return puts :set nowifi on the command line, the row naming the
    # line rather than d, which carries none of it; two Returns closer together than
    # Joel presses two, as a key bounced, are one press doubled, so they only write it.
    # A fresh Return there turns it off.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("wifi off", selecting="Wi-Fi Off")
    desktop.normal()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("carried") == ":set nowifi")
    told = desktop.showing()["selected"]["told"]
    assert told[0] == "\u21b5 :set nowifi", told
    assert ": commands" in told, told
    desktop.press_at_once("ret ret")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "set nowifi")
    desktop.held_still(timedelta(seconds=2))
    machine.succeed("rfkill list wifi | grep -q 'Soft blocked: no'")
    assert desktop.showing()["returned"]["act"] == "lined", desktop.showing()["returned"]
    # A fresh Return runs the line, which turns it off without asking, as :set never
    # asks: :set wifi takes it back.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=timedelta(seconds=15)
    )
    # It changed something, so the launcher stays, its row now Wi-Fi On.
    desktop.launcher_until(
        lambda shown: shown["open"] and (shown["selected"] or {}).get("name") == "Wi-Fi On"
    )
    desktop.close_launcher()
    # And it's an option, :set wifi, as Vim's are.
    sets_wi_fi(desktop)


def reads_a_network_in_a_terminal(desktop: Desktop) -> None:
    """With the launcher closed, a terminal steps into the Office's Speed.

    The launcher closed holding the Office, which asks wifid nothing now; the terminal's own
    walk has wifid say the Office's detail, which Speed is made from, so the step into it is
    asked again until it's there. How long it waits is in-a-terminal's own tests'.
    """
    assert not desktop.showing()["open"], desktop.showing()
    speed = json.loads(desktop.as_account("launcher show --json network wi-fi office speed"))
    assert {"In", "Retries"} <= {row["name"] for row in speed["rows"]}, speed


def test_a_network_left_reads_so_on_its_key_and_wi_fi_off_unanswered_is_put_back(
    desktop: Desktop,
) -> None:
    """A network left reads so before wifid answers, and Wi-Fi Off unanswered is put back.

    Joel, 2026-11-07: "when i modify something in the launcher ... it should show right away.
    the launcher needs to be as live as possible for good ux". wifid is stopped in its tracks,
    so nothing it says can have changed the row: Lantern reads saved on the key alone, and stays
    so once wifid wakes and leaves it. Wi-Fi Off that wifid doesn't answer within five seconds
    is put back, the foot saying so, and shows what wifid did once it does.
    """
    machine = desktop.machine
    machine.wait_for_unit("lantern.service")
    machine.wait_for_unit("wifid.service")
    joins_lantern(machine)

    def lantern_reads(side: str) -> Callable[[Json], bool]:
        return lambda shown: any(stands(thing, side) for thing in shown["things"])

    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.normal()
    desktop.launcher_until(lantern_reads("connected"))
    before = left_lantern(desktop)
    machine.succeed(WIFID_STOPPED)
    try:
        machine.send_chars("d")
        desktop.launcher_until(lambda shown: shown["modeWord"].startswith("d \u00b7 disconnect"))
        with desktop.told("ended 1"):
            machine.send_chars("d")
        # Stopped, wifid can't have said so: only the act can.
        desktop.launcher_until(lantern_reads("saved"))
        assert left_lantern(desktop) == before, "wifid left Lantern while stopped"
        # Woken, wifid leaves it, and the launcher lets go of what it drew.
        with desktop.told("became wifi:Lantern"):
            machine.succeed(WIFID_WOKEN)
            machine.wait_until_succeeds(
                f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
            )
    finally:
        machine.succeed(WIFID_WOKEN)
    desktop.launcher_until(lantern_reads("saved"))
    desktop.close_launcher()
    puts_wi_fi_off_back_unanswered(desktop)
    # Left as it was found, joined, for the tests after it.
    joins_lantern(machine)


def joins_lantern(machine: BaseMachine) -> None:
    """Join Lantern through wifid unless the card already has, once wifid can see it."""
    wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Lantern\"'",
        timeout=timedelta(seconds=60),
    )
    # The card's own address, which it holds only while joined: networkctl's
    # status still named Lantern's 10.42.0 once the card had left it.
    joined = "ip -4 address show wlan1 | grep -q 'inet 10.42.0.'"
    if machine.execute(joined)[0] != 0:
        join = '{"name":"Lantern","password":"glowing in the dark"}'
        machine.succeed(f"varlinkctl call --more {wifi.split(' ', 2)[2]}.Join '{join}'")
    machine.wait_until_succeeds(joined, timeout=timedelta(seconds=60))


WIFID_STOPPED = "systemctl kill -s SIGSTOP wifid.service"
WIFID_WOKEN = "systemctl kill -s SIGCONT wifid.service"


def names(shown: Json) -> list[str]:
    """Say the names of the rows shown."""
    return cast("list[str]", shown["rows"])


def puts_wi_fi_off_back_unanswered(desktop: Desktop) -> None:
    """Turn Wi-Fi off with wifid stopped: it reads on at once, and is put back unanswered."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("wi-fi off", selecting="Wi-Fi Off")
    desktop.normal()
    machine.succeed(WIFID_STOPPED)
    try:
        machine.send_chars(":set nowifi\n", delay=KEY_PACE)
        desktop.launcher_until(lambda shown: "Wi-Fi On" in names(shown))
        with desktop.told("not become wifi:radio", timeout=timedelta(seconds=15)):
            pass
        desktop.launcher_until(
            lambda shown: (
                "Wi-Fi Off" in names(shown)
                and any(
                    hint.replace("\u00a0", " ").endswith("Wi-Fi not turned off yet")
                    for hint in shown["hints"]
                )
                and "no answer " in [hint.replace("\u00a0", " ") for hint in shown["hints"]]
                # Its why stands by what it's the why of, not a Return's
                # room away, as Return's place held for keys would put it.
                and shown["keysRoom"]["place"] == shown["keysRoom"]["placeSays"]
            )
        )
        desktop.shoot("launcher-wifi-unanswered")
    finally:
        machine.succeed(WIFID_WOKEN)
    # wifid, woken, does what it was asked, and the row says what it did.
    machine.wait_until_succeeds(
        "rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=timedelta(seconds=15)
    )
    desktop.launcher_until(lambda shown: "Wi-Fi On" in names(shown))
    machine.send_chars(":set wifi\n", delay=KEY_PACE)
    machine.wait_until_succeeds(
        "rfkill list wifi | grep -q 'Soft blocked: no'", timeout=timedelta(seconds=15)
    )
    desktop.close_launcher()


def test_the_addresses_name_each_connection_its_parts_a_step_in(desktop: Desktop) -> None:
    """The addresses name each connection, its parts a step in."""
    machine = desktop.machine
    # A tunnel beside the Wi-Fi, as the laptop has.
    machine.succeed(
        "ip tuntap add dev tailscale0 mode tun"
        " && ip address add 100.100.1.2/32 dev tailscale0 && ip link set tailscale0 up"
    )
    # Wi-Fi was turned off and on again, so the card joins again first.
    machine.wait_until_succeeds(
        "ip -4 address show wlan1 | grep -q 'inet 10.4'", timeout=timedelta(seconds=90)
    )
    desktop.answer("address")
    desktop.follow()
    addresses = len(desktop.showing()["held"])
    # Each is named by what it is, never by the kernel's name for it.
    desktop.launcher_until(lambda shown: {"Wi-Fi", "VPN"} <= set(shown["rows"]))
    assert not {"wlan1", "tailscale0"} & set(desktop.showing()["rows"]), desktop.showing()
    # Return opens the Wi-Fi's parts, and the kernel's name is none of them.
    desktop.type_in("wi-fi", selecting="Wi-Fi")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: {"Address", "Hardware Address", "MTU", "State"} <= set(shown["rows"])
    )
    assert "Device name" not in desktop.showing()["rows"], desktop.showing()
    desktop.type_in("mtu", selecting="MTU")
    side = desktop.showing()["selected"]["side"]
    assert re.fullmatch(r"[\d,]+ bytes", side), side
    desktop.shoot("launcher-addresses-wifi")
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == addresses and shown["typed"] == "wi-fi"
    )
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    # l opens the VPN's, which has no hardware address, and
    # the old word for it finds nothing.
    desktop.type_in("tunnel")
    desktop.launcher_until(lambda shown: "VPN" not in shown["rows"])
    desktop.normal()
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    desktop.type_in("vpn", selecting="VPN")
    desktop.follow()
    desktop.type_in("address", selecting="Address")
    assert desktop.showing()["selected"]["side"] == "100.100.1.2/32"
    assert "Hardware Address" not in desktop.showing()["rows"], desktop.showing()
    desktop.close_launcher()
    machine.succeed("ip link delete tailscale0")


def test_a_cable_says_how_its_link_is_doing(desktop: Desktop) -> None:
    """A cable says how its link is doing, as it changes."""
    machine = desktop.machine
    # tests/desktop.nix gives this area a cable, cable0, at a gigabit.
    machine.wait_until_succeeds(
        "ip -4 address show cable0 | grep -q 'inet 10.9.0'", timeout=timedelta(seconds=60)
    )
    desktop.answer("address")
    desktop.follow()
    desktop.launcher_until(lambda shown: any(row.startswith("Ethernet") for row in shown["rows"]))
    # The kernel's name finds it, never shown, and Return opens its parts,
    # its link among them, as its card reads it.
    desktop.type_in("cable0", selecting="Ethernet")
    desktop.step_in("ret")

    def reads(shown: Json) -> list[str]:
        return [thing["reads"] for thing in shown["things"]]

    def lost(times: int) -> str:
        return f"Link Lost \u00b7 {({0: 'never', 1: 'once'}).get(times, f'{times} times')}"

    # However often the card's link went at its start, as the kernel counts.
    before = int(machine.succeed("cat /sys/class/net/cable0/carrier_down_count"))
    link = ["Speed \u00b7 1 Gb/s", "Duplex \u00b7 full", lost(before), "Errors \u00b7 none"]
    desktop.launcher_until(lambda shown: set(link) <= set(reads(shown)))
    desktop.shoot("launcher-addresses-ethernet")
    # Unplugged and plugged in again, it says so while it's shown, as its
    # card counts it.
    machine.send_monitor_command("set_link cable off")
    desktop.launcher_until(
        lambda shown: "State \u00b7 nothing at the other end" in reads(shown),
        timeout=timedelta(seconds=10),
    )
    machine.send_monitor_command("set_link cable on")
    desktop.launcher_until(
        lambda shown: {lost(before + 1), "State \u00b7 up"} <= set(reads(shown)),
        timeout=timedelta(seconds=10),
    )
    # Its errors a step in: none either way, and what was set aside.
    desktop.type_in("errors", selecting="Errors")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["rows"][:4] == ["In", "Out", "Dropped In", "Dropped Out"]
    )
    assert reads(desktop.showing())[:2] == ["In \u00b7 none", "Out \u00b7 none"]
    desktop.close_launcher()


# Where the timers daemon keeps its timers, in the account's state.
TIMERS_KEPT = "~/.local/state/timers/held.json"
# The timers the gallery shows: one left running, one paused, one rung.
TIMERS_SHOWN = ("timer 25m bread", "timer 10m tea", "timer 1s eggs")


def test_a_timer_started_in_the_launcher_rings_as_a_notice_with_it_closed(
    desktop: Desktop,
) -> None:
    """A timer typed in the launcher rings as a notice with it closed, and putting it away ends it.

    The timers daemon keeps it, so it rings without the launcher, and the notice is its own over
    D-Bus; the notice put away tells the daemon, which lets the timer go. Its gallery holds
    Timers with one running, one paused and one rung, and the stopwatch lapped, at both of
    Joel's scales.
    """
    machine = desktop.machine
    desktop.answer("timer 3s tea")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Start Tea, a 3 s Timer"
    )
    desktop.shoot("launcher-timer-typed")
    # Return from the find lands on it, since it starts something, and a second does it, the
    # answer becoming the timer it started, counting.
    desktop.land()
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: str((shown["selected"] or {}).get("name")).startswith("Tea \u00b7 ")
    )
    desktop.close_launcher()
    rang = 'qs ipc call notices front | grep -qF \'"signed":"Timers"\''
    machine.wait_until_succeeds(desktop.in_session(rang), timeout=timedelta(seconds=30))
    desktop.shoot("notice-timer-rung")
    kept = desktop.as_account(f"cat {TIMERS_KEPT}")
    assert '"state":"rung"' in kept, kept
    desktop.as_account("qs ipc call notices putAway")
    gone = f"! grep -q timer: {TIMERS_KEPT}"
    machine.wait_until_succeeds(desktop.in_session(gone), timeout=timedelta(seconds=15))
    desktop.as_account("qs ipc call notices seen")
    # Only the gallery sets more going, for its shots.
    if desktop.values["shots"]:
        shoot_the_timers(desktop)


def shoot_the_timers(desktop: Desktop) -> None:
    """Shoot Timers holding one running, one paused and one rung, at Joel's two scales.

    The stopwatch, listed last, is set going and lapped twice first, and its laps are shot from
    inside it. Then Now is shot with one rung, with none rung, and with only paused ones.
    """
    machine = desktop.machine
    listed = len(TIMERS_SHOWN) + 1
    for typed in TIMERS_SHOWN:
        desktop.answer(typed)
        desktop.land()
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: " \u00b7 " in str((shown["selected"] or {}).get("name"))
        )
        desktop.close_launcher()
    # The rung one's notice is taken back as its sender would take it, which
    # leaves the timer rung, since putting it away would end it; the test
    # doesn't know its number, and only the daemon's notices are up.
    desktop.as_account(
        "for id in $(seq 1 30); do busctl --user call org.freedesktop.Notifications"
        " /org/freedesktop/Notifications org.freedesktop.Notifications"
        " CloseNotification u $id 2>/dev/null || true; done"
    )
    desktop.as_account("qs ipc call launcher collection timers")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: len(shown["rows"]) == listed)
    desktop.type_in("tea", selecting="Tea")
    desktop.normal()
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: str((shown["selected"] or {}).get("side")).startswith("paused")
    )
    # Afresh, since typing on from normal would add to "tea".
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher collection timers")
    desktop.launcher_ready()
    desktop.type_in("stopwatch", selecting="Stopwatch")
    desktop.normal()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("note") == "running")
    for note in ("running \u00b7 1 lap", "running \u00b7 2 laps"):

        def lapped(shown: Json, note: str = note) -> bool:
            return bool((shown["selected"] or {}).get("note") == note)

        machine.send_chars(":lap\n")
        desktop.launcher_until(lapped)
    held = len(desktop.showing()["held"])
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: len(shown["held"]) > held)
    desktop.launcher_until(lambda shown: "Lap 2" in shown["rows"])
    desktop.shoot("launcher-stopwatch-laps")
    desktop.close_launcher()
    for scale in ("1.5", "2"):
        desktop.as_account(f"niri msg output winit scale {scale}")
        desktop.as_account("qs ipc call launcher collection timers")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: len(shown["rows"]) == listed)
        desktop.shoot(f"launcher-timers-at-{scale.replace('.', '-')}", ticking="Bread's clock")
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")
    shoot_now_through_the_timers(desktop)
    desktop.as_account("systemctl --user stop timers.service")
    desktop.as_account(f"rm -f {TIMERS_KEPT}")
    desktop.as_account("qs ipc call notices putAway || true")
    desktop.as_account("qs ipc call notices seen")


def test_ethernet_in_network_holds_each_cable_its_lease_and_its_neighbour(
    desktop: Desktop,
) -> None:
    """Ethernet in Network holds each cable, its lease and its neighbour."""
    machine = desktop.machine
    machine.wait_until_succeeds(
        "ip -4 address show cable0 | grep -q 'inet 10.9.0'", timeout=timedelta(seconds=60)
    )
    # tests/desktop.nix puts a switch at the cable's other end, its own
    # second card, which says what it is over LLDP every few seconds.
    machine.wait_until_succeeds(
        "networkctl lldp cable0 | grep -q switch0", timeout=timedelta(seconds=60)
    )

    def reads(shown: Json) -> list[str]:
        return [thing["reads"] for thing in shown["things"]]

    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: shown["held"] == ["network"] and "Ethernet" in shown["rows"]
    )
    # Beside Wi-Fi, before what either is addressed by.
    rows = desktop.showing()["rows"]
    assert rows.index("Wi-Fi") < rows.index("Ethernet") < rows.index("Address"), rows
    # The whole word asks the answer itself, so the start of it finds the
    # row, an answer that Tab goes into.
    desktop.type_in("eth", selecting="Ethernet")
    desktop.step_in("tab")
    # Every card on a cable, whatever its address: the way out, the cable
    # and the switch's.
    desktop.launcher_until(lambda shown: "Ethernet 3" in shown["rows"])
    desktop.type_in("cable0", selecting="Ethernet")
    desktop.step_in("ret")
    # Its lease, from QEMU's DHCP server, which gives a day, and what's at
    # its other end, by the name it gives.
    desktop.launcher_until(
        lambda shown: (
            any(read.startswith("Lease \u00b7 ends in 23 h") for read in reads(shown))
            and any(read.startswith("Neighbour \u00b7 machine \u00b7 ") for read in reads(shown))
        )
    )
    desktop.type_in("lease", selecting="Lease")
    desktop.step_in("ret")
    # No name servers: the cable asks for none (tests/desktop.nix), and
    # QEMU's server offers only what's asked.
    lease = ["Server \u00b7 10.9.0.2", "Given For \u00b7 1 d"]
    desktop.launcher_until(lambda shown: set(lease) <= set(reads(shown)))
    times = dict(
        read.split(" \u00b7 ", 1) for read in reads(desktop.showing()) if " \u00b7 " in read
    )
    assert times["Got"].endswith(" ago"), times
    assert times["Renews"].startswith("in 11 h"), times
    assert times["Ends"].startswith("in 23 h"), times
    desktop.shoot("launcher-ethernet-lease")
    held = len(desktop.showing()["held"])
    desktop.step_out()
    machine.send_chars("cc", delay=KEY_PACE)
    desktop.launcher_until(lambda shown: len(shown["held"]) == held - 1 and shown["typed"] == "")
    desktop.type_in("neighbour", selecting="Neighbour")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: {"System \u00b7 machine", "Port \u00b7 switch0"} <= set(reads(shown))
    )
    desktop.close_launcher()


def shoot_now_through_the_timers(desktop: Desktop) -> None:
    """Shoot Now with one timer rung, then with that one ended, then with only paused ones."""
    machine = desktop.machine
    shoot_now_holding_timers(desktop, "rung", "Eggs \u00b7 Bread \u00b7 Tea \u00b7 Stopwatch")
    # Eggs ended, by the keys that carry End, leaves them running. End can't be undone, so it
    # asks once, and takes y only once the ask is ready.
    pick_a_timer(desktop, "Eggs")
    with desktop.told("ask ready"), desktop.told("asked End Eggs? (y/n)"):
        machine.send_chars("dd")
    with desktop.told("answered yes"):
        machine.send_chars("y")
    # An ended row stays, drawn as gone, until the launcher closes.
    desktop.launcher_until(lambda shown: all(thing.get("gone") for thing in shown["things"]))
    desktop.close_launcher()
    shoot_now_holding_timers(desktop, "running", "Bread \u00b7 Tea \u00b7 Stopwatch")
    # Bread and the stopwatch paused leave only what's paused.
    pick_a_timer(desktop, "Bread")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: str((shown["selected"] or {}).get("side")).startswith("paused")
    )
    desktop.close_launcher()
    pick_a_timer(desktop, "Stopwatch")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: str((shown["selected"] or {}).get("note")).startswith("paused")
    )
    desktop.close_launcher()
    # The stopwatch paused is named no more, and Tea, with less left, comes first.
    shoot_now_holding_timers(desktop, "paused", "Tea \u00b7 Bread")


def pick_a_timer(desktop: Desktop, name: str) -> None:
    """Open Timers afresh and stand on the one named, in normal, for a key to act on it."""
    desktop.as_account("qs ipc call launcher collection timers")
    desktop.launcher_ready()
    desktop.type_in(name.lower(), selecting=name)
    desktop.normal()


def shoot_now_holding_timers(desktop: Desktop, standing: str, held: str) -> None:
    """Shoot Now with its Timers row, and the margin with its mark, at Joel's two scales.

    Its drawer names what it holds, `held`, in Timers' order, the stopwatch last.
    """
    machine = desktop.machine
    for scale in ("1.5", "2"):
        # Each scale is set with the launcher closed, so it opens laid out for it, as
        # desktop-looks sets its own.
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
        desktop.as_account(f"niri msg output winit scale {scale}")
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: "Timers" in shown["rows"])
        # Picked, by G as it's listed last, so it's in view at 2 and says its names and Return.
        desktop.normal()
        # The screen shows the pick some frames after the shell says it, and at 2 the list
        # scrolls to it first, so the shot waits for the screen itself to move from before G:
        # a count of the card's frames moves with the piece's own clock too.
        desktop.held_still()
        before = desktop.screen_now()
        machine.send_chars("G")
        # A place, it says nothing after its name, its timers in its drawer (VKCF3Z, 5fa2cbda).
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Timers"
                and not shown["selected"].get("count")
            )
        )
        desktop.launcher_until(
            lambda shown: (
                (shown["drawer"]["title"], shown["drawer"]["eyebrow"], shown["drawer"]["body"])
                == ("Timers", "place", held)
            )
        )

        def moved(_: bool, before: Image.Image = before) -> bool:
            return ImageChops.difference(before, desktop.screen_now()).getbbox() is not None

        retry(moved, timeout=timedelta(seconds=30))
        desktop.shoot(
            f"launcher-now-timers-{standing}-at-{scale.replace('.', '-')}", ticking="Bread's clock"
        )
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def test_a_vm_virsh_lists_is_in_programs_with_its_figures_and_starts_from_its_acts(
    desktop: Desktop,
) -> None:
    """A VM virsh lists is in Programs with its state and figures, and Start It starts it."""
    machine = desktop.machine
    # tests/desktop.nix gives the launcher a stand-in virsh, whose session
    # holds web, which libvirt says is shut off, and which keeps each run
    # in virsh-asked.
    asked = "/run/user/1000/virsh-asked"
    uuid = "3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10"
    machine.succeed(f"rm -f {asked} /run/user/1000/virsh-web")
    desktop.answer("vms")
    # Programs lists VMs only once virsh has said a machine exists, which
    # it's asked as the launcher opens: the system's read-only.
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "VMs")
    lines = machine.succeed(f"cat {asked}").splitlines()
    assert "-c qemu:///session list --all --name" in lines, lines
    assert "-r -c qemu:///system list --all --name" in lines, lines
    assert "-c qemu:///session dominfo --domain web" in lines, lines
    desktop.into_program()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "web"
            and shown["selected"]["side"] == "off"
            and shown["drawer"]["out"] is True
        )
    )
    drawer = desktop.showing()["drawer"]
    assert drawer["title"] == "web", drawer
    assert {"Most Memory", "Processors", "UUID"} <= set(drawer["facts"]), drawer
    assert "Memory in Use" not in drawer["facts"], drawer
    # What a machine that's off can't say is named, never left to look forgotten.
    known = "Known only while it runs: Processor Time, Memory in Use and ID."
    assert known in drawer["body"], drawer
    desktop.shoot("launcher-virtual-machines")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: (shown["rows"] or [""])[0] == "Start It")
    desktop.down_to("Start It")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF -- '-c qemu:///session start --domain {uuid}' {asked}",
        timeout=timedelta(seconds=30),
    )
    # Read again once it's done, so a running machine's acts take its place.
    desktop.launcher_until(lambda shown: "Power It Off" in shown["rows"])
    desktop.close_launcher()


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

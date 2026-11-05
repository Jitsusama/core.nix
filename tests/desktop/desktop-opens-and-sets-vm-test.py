"""The desktop test's opens and sets area.

Here the launcher opens and sets what the machine has: sound and its outputs' parts, the
power profile, displays, Wi-Fi and the addresses.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, SIGNING_IN, retry
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
        desktop.photograph(f"launcher-sound-{output}")
        kind = output.removeprefix("stand-in-")
        desktop.type_in(kind, selecting=kind)
        # Return steps in, as on every device, its use first, so Return
        # twice plays sound through it.
        desktop.step_in("ret")
        desktop.launcher_until(sound_selected("Play Sound Through It"))
        machine.send_key("ret")
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(desktop.in_session(used), timeout=timedelta(seconds=10))
        # It changed something, so the launcher stays until it's closed.
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
    """Open the launcher's Sound, and step into a device's parts by the right arrow."""
    desktop.as_account("qs ipc call launcher collection sound")
    desktop.launcher_ready()
    desktop.type_in(typed, selecting=named)
    desktop.step_in()


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
    desktop.photograph("launcher-sound-output-parts")
    # Return on Volume, past Muted and the Level, steps into its choices,
    # each level saying its decibels in the footer.
    machine.send_key("down")
    desktop.launcher_until(sound_selected("Level"))
    machine.send_key("down")
    desktop.launcher_until(sound_selected("Volume"))
    desktop.launcher_until(lambda shown: shown["footer"].endswith(" dB"))
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Louder", "Quieter"])
    desktop.photograph("launcher-sound-volume")
    desktop.type_in("50", selecting="Volume to 50%")
    desktop.launcher_until(lambda shown: "-18.1 dB" in shown["footer"])
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
    desktop.photograph("launcher-sound-muted")
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
    desktop.photograph("launcher-sound-input-parts")
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
    # The right arrow on Recording, then on the program, and Return on
    # its Muted mutes what it records. Recording is counted to.
    for _down in range(desktop.showing()["rows"].index("Recording")):
        machine.send_key("down")
    desktop.launcher_until(sound_selected("Recording"))
    desktop.step_in()
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Muted", "Louder", "Quieter"])
    desktop.photograph("launcher-sound-recording")
    desktop.launcher_until(sound_selected("Muted"))
    machine.send_key("ret")
    hushed = f"wpctl get-volume {recording} | grep -q MUTED"
    machine.wait_until_succeeds(desktop.in_session(hushed), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
    desktop.as_account("systemctl --user stop listening")
    desktop.as_account(f"rm -f {listening}")


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

    def stream() -> int:
        """Give the id of the stream mpv plays through, as PipeWire lists it."""
        nodes = sound_nodes(desktop)
        return next(
            int(node["id"])
            for node in nodes
            if node.get("type") == "PipeWire:Interface:Node"
            and node["info"]["props"].get("media.class") == "Stream/Output/Audio"
            and node["info"]["props"].get("application.name") == "mpv"
        )

    playing("Playing", "First tone")
    machine.wait_until_succeeds(
        desktop.in_session("pw-dump | grep -q Stream/Output/Audio"), timeout=WAIT
    )
    tone = stream()

    def heard(volume: str) -> None:
        """Wait until PipeWire says the stream is this loud, as wpctl says it."""
        loud = f"wpctl get-volume {tone} | grep -qxF 'Volume: {volume}'"
        machine.wait_until_succeeds(desktop.in_session(loud), timeout=WAIT)

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
        desktop.step_in()
        desktop.launcher_until(lambda shown: {"Media", "Sound"} <= set(shown["rows"]))
        if part:
            desktop.type_in(part.lower(), selecting=part)
            desktop.step_in()

    def press(name: str, then: str = "", becomes: str = "") -> None:
        """Press Return on a row by its name: it stays, saying so or becoming its opposite.

        Each changes how the program plays, so the launcher stays. One that says where it is
        now has what was typed cleared for the next; one that becomes its opposite is closed.
        """
        desktop.type_in(name.lower(), selecting=name)
        machine.send_key("ret")
        if then:
            desktop.launcher_until(lambda shown: shown["footer"] == then)
            desktop.normal()
            machine.send_key("left")
            desktop.launcher_until(lambda shown: shown["typed"] == "")
        else:
            desktop.launcher_until(lambda shown: becomes in shown["rows"])
            desktop.close_launcher()

    # The right arrow on the program holds Media and Sound, each saying
    # how it stands.
    controls()
    desktop.photograph("launcher-program-parts")
    desktop.launcher_until(reads("Media", "playing"))
    desktop.launcher_until(reads("Sound", "100%"))
    desktop.type_in("media", selecting="Media")
    desktop.step_in()
    desktop.photograph("launcher-program-media")
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Pause", "Next", "Previous", "First tone"]
    )
    desktop.launcher_until(reads("First tone", "Test tones"))
    # Next and Previous keep the launcher, saying what's on now, and what's
    # playing follows.
    press("Next", then="Second tone, Test tones")
    playing("Playing", "Second tone")
    desktop.launcher_until(lambda shown: shown["rows"][-1] == "Second tone")
    press("Previous", then="First tone, Test tones")
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
    desktop.photograph("launcher-program-sound")
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
    desktop.launcher_until(reads("Muted", "no"))
    desktop.type_in("muted", selecting="Muted")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["open"] and reads("Muted", "yes")(shown))
    desktop.close_launcher()
    heard("1.00 [MUTED]")
    controls()
    desktop.launcher_until(reads("Sound", "muted"))
    desktop.type_in("sound", selecting="Sound")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][0] == "Muted")
    desktop.launcher_until(reads("Muted", "yes"))
    desktop.launcher_until(reads("Volume", "muted"))
    # Louder is meant to be heard, so it unmutes.
    press("Louder", then="now 100%")
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
            machine.send_key("up")
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


def test_the_launcher_sets_the_power_profile(desktop: Desktop) -> None:
    """The launcher sets the power profile, staying to say so, or closing from a step in."""
    machine = desktop.machine
    # By its name, power saver last, where what follows starts from. Setting
    # it changes something, so Return keeps the launcher, the row saying
    # it's chosen, and the right arrow then Return sets it and closes, with
    # no chord with Ctrl said anywhere.
    for typed, profile, name, stepping in [
        ("balanced power", "balanced", "Balanced Power", False),
        ("power saver", "power-saver", "Power Saver", True),
    ]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in(typed, selecting=name)
        # In normal, where the arrows step; insert's move the caret, so it names none.
        desktop.normal()
        desktop.launcher_until(lambda shown: "\u2192" in (shown["selected"] or {}).get("told", []))
        told = desktop.showing()["selected"]["told"]
        assert not any(key.startswith("Ctrl") for key in told), told
        if stepping:
            # Its own act leads the step, selected.
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == shown["rows"][0]
            )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        if not stepping:
            desktop.launcher_until(
                lambda shown: shown["open"] and (shown["selected"] or {}).get("side") == "chosen"
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
    # choices a step in by Return or the right arrow, each named there as
    # the part reads it.
    for key, profile, name, was in [
        ("ret", "balanced", "Balanced", "Power Saver"),
        ("right", "power-saver", "Power Saver", "Balanced"),
    ]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("power profile", selecting="Power Profile")
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == was)  # noqa: B023
        desktop.step_in(key)
        desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Power Saver", "Balanced"])
        desktop.type_in(name.split()[0][:3].lower(), selecting=name)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "chosen")
        desktop.close_launcher()


# The right arrow on one of a collection's members offers its own
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
    # run: each fact reads by the vocabulary's name for it, the depth first.
    shown_as = next(iter(json.loads(desktop.as_account("niri msg --json outputs"))))
    said = {shown_as: [{"name": "EDID", "value": "1.4"}, {"name": "depth", "value": "8 bit"}]}
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
        desktop.step_in()
        # Its settings lead, each saying how it's set, their choices a step
        # in, there by Return on the setting, or found from here by what's
        # typed. The backlight lights the one screen there is, so its
        # Brightness comes first after Return's own Focus, and the Room
        # light once the sensor's said.
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:2] == ["Focus", "Brightness"]
                and [row for row in shown["rows"] if row != "Room Light"][2:8]
                == ["Scale", "Refresh", "Resolution", "Rotation", "Colour Depth", "EDID"]
            )
        )
        desktop.photograph("launcher-displays")
        percent = round(wanted * 100)
        if inside:
            desktop.type_in("scale", selecting="Scale")
            desktop.step_in("ret")
            desktop.launcher_until(lambda shown: "Scale to 150%" in shown["rows"])
            desktop.type_in(str(percent))
        else:
            desktop.type_in(f"scale {percent}")
        machine.send_key("ret")

        def scaled(_: bool, wanted: float = wanted) -> bool:  # As retry calls it.
            """Say whether the display is at the scale wanted."""
            return scale() == wanted

        retry(scaled, timeout=timedelta(seconds=15))
        # Scaling changes something, so the launcher stays until it's closed.
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


def disconnects_and_takes_back(desktop: Desktop) -> None:
    """End Lantern by dd, unasked; Ctrl+R finds nothing to redo, and . asks to forget it.

    dd on a network disconnects it, never forgets it, and doesn't ask, since u joins it again;
    the foot says so until the next key. Disconnected, its row's end is Forget, which can't be
    undone, so . asks, and Escape keeps it. wifid joins it again for the steps after.
    """
    machine = desktop.machine

    def lantern_reads(side: str) -> Callable[[Json], bool]:
        return lambda shown: (
            f"Lantern \u00b7 {side} \u00b7 network" in [thing["reads"] for thing in shown["things"]]
        )

    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.normal()
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
    # Once the row says it's only saved, . ends it as that row ends.
    desktop.launcher_until(lantern_reads("saved"))
    with desktop.told("ask ready"), desktop.told("asked Forget Lantern? (y/n)"):
        machine.send_chars(".")
    with desktop.told("answered no"):
        machine.send_key("esc")
    desktop.close_launcher()
    wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.succeed(f'{wifi}.Join \'{{"name": "Lantern"}}\'')
    machine.wait_until_succeeds(
        "networkctl status wlan1 | grep -q 10.42.0.", timeout=timedelta(seconds=60)
    )


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
    machine.send_chars("disc")
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
    machine.send_chars("wifi on")
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
    network as it was; :g finds Lantern by name and disconnects it by its own act, unasked, as
    it can be undone.
    """
    machine = desktop.machine

    def says(typed: str, said: str) -> None:
        machine.send_chars(typed)
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
    machine.send_chars(":%disc\n")
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
    machine.send_chars(":g/Lant/disc\n")
    machine.wait_until_succeeds(
        f"test $({LEFT_LANTERN}) -gt {before}", timeout=timedelta(seconds=30)
    )
    desktop.launcher_until(lambda shown: shown["command"] is None)
    lists_earlier_lines(desktop)


def lists_earlier_lines(desktop: Desktop) -> None:
    """List the lines the command line ran by :his, newest first, and again by @:."""
    machine = desktop.machine
    if not desktop.showing()["open"]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.normal()
    for typed in [":his\n", "@:"]:
        machine.send_chars(typed)
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
    says E518.
    """
    machine = desktop.machine

    def line(shown: Json) -> Json:
        return shown["command"] or {}

    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    machine.send_chars(":set ")
    desktop.launcher_until(
        lambda shown: (
            {"wifi wifi=Office", "volume"}
            <= {
                entry if entry.startswith("wifi ") else entry.split(" ")[0]
                for entry in line(shown).get("menu", [])
            }
        )
    )
    machine.send_chars("wi")
    desktop.launcher_until(lambda shown: line(shown).get("menu") == ["wifi wifi=Office"])
    machine.send_key("tab")
    desktop.launcher_until(
        lambda shown: line(shown).get("line") == "set wifi" and line(shown).get("chosen") == 0
    )
    machine.send_chars("?\n")
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "wifi=Office"
    )
    machine.send_chars(":set wfii\n")
    desktop.launcher_until(
        lambda shown: shown["commandSaid"] == "E518: Unknown option: wfii" and shown["open"]
    )
    desktop.close_launcher()


def sets_wi_fi(desktop: Desktop) -> None:
    """Turn Wi-Fi on by :set wifi, off by :set nowifi, and on again, as the rfkill switch says."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    for typed, blocked in [("wifi", "no"), ("nowifi", "yes"), ("wifi", "no")]:
        machine.send_chars(f":set {typed}\n")
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
    desktop.photograph("launcher-wifi")
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
    machine.send_chars("not the pass")
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
    desktop.photograph("launcher-wifi-password")
    with desktop.told("refused wifi Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    machine.succeed('journalctl -u wifid | grep -q "couldn\'t join Lantern: WrongPassword"')
    desktop.photograph("launcher-wifi-refused")
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
            and (shown["selected"] or {}).get("side") == "connected"
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
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Sign in to It", "Disconnect It"])
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.photograph("launcher-sign-in")
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
    desktop.photograph("sign-in", timeout=timedelta(seconds=20))
    # Its one button signs in, and the window closes as Lantern lets
    # the machine through, its notice taken back with it.
    with desktop.told("withdrew sign-in Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(window), timeout=timedelta(seconds=60))
    machine.succeed(f'{described} | grep -qF \'"reach":"online"\'')
    # Its name typed and Return, in insert, never disconnects it: Return
    # steps in, to what can be done to it, and it stays connected (Joel,
    # 2026-11-04: "it is quite dangerous").
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.launcher_until(lambda shown: shown["mode"] == "insert")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["open"] and "Disconnect It" in shown["rows"])
    desktop.held_still(timedelta(seconds=3))
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    desktop.close_launcher()
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
    desktop.photograph("launcher-wifi-account")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Office on wlan1'", timeout=timedelta(seconds=60)
    )
    desktop.launcher_until(lambda shown: shown["open"] and not shown["asking"])
    desktop.close_launcher()
    # Found from the root, the right arrow goes straight inside the Office,
    # the networks' crumb above it saying where it lives, and the left
    # arrow comes back to the search with what was typed (VKCF3Z,
    # 2026-10-29: "\u2190 is where you came from; the crumbs are where it
    # lives").
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    # Joined, its Return typed in insert steps in too, rather than
    # disconnecting it, so the foot says Return alone: insert's arrows
    # move the caret, so no arrow is named there.
    told = desktop.showing()["selected"]["told"]
    assert told[:1] == ["\u21b5"], told
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) > 1 and shown["path"][-1:] == ["Office"] and shown["typed"] == ""
        )
    )
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "office")
    desktop.close_launcher()
    # Stepped into, a network says after what can be done to it what's
    # heard of it, each a part named first, the selected row's footer
    # saying more: the card's Signal and Speed once the card has read
    # them, each holding more, and its access points a step further in.
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    desktop.step_in()
    office = len(desktop.showing()["held"])
    # The Signal says its dBm, always read, and how far over the noise
    # below; Return opens it, where iw, read while the network shows,
    # says what the card sends at.
    desktop.type_in("signal", selecting="Signal")
    desktop.launcher_until(lambda shown: str(shown["selected"]["side"]).endswith(" dBm"))
    desktop.launcher_until(lambda shown: "over the noise" in shown["footer"])
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Transmit Power" in shown["rows"])
    # The left arrow steps out with the word put back, and again takes
    # the word.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == office and shown["typed"] == "signal"
    )
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    desktop.photograph("launcher-wifi-detail")
    # The Speed holds each way and the retries, the right arrow into it.
    desktop.type_in("speed", selecting="Speed")
    desktop.step_in()
    desktop.launcher_until(lambda shown: {"In", "Retries"} <= set(shown["rows"]))
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "speed")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    # The access points, each named by its band and channel, and each
    # holding its own parts, its hardware address among them; the figure's
    # own Copy last (docs/design.md, "Copy ... is always the last").
    desktop.type_in("access", selecting="Access Points")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["2.4 GHz, Channel 1", "Copy"])
    desktop.photograph("launcher-wifi-points")
    desktop.step_in("ret")
    desktop.type_in("hardware", selecting="Hardware Address")
    assert desktop.showing()["selected"]["side"] == "02:00:00:00:02:00"
    desktop.close_launcher()
    # Wi-Fi holds its own off, last, so stepping in never lands on it;
    # Network lists Wi-Fi alone.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ")
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and "Wi-Fi" in shown["rows"])
    assert "Wi-Fi Off" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.type_in("wi-fi", selecting="Wi-Fi")
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            len(shown["rows"]) > 1
            and shown["rows"][-1] == "Wi-Fi Off"
            and (shown["selected"] or {}).get("name") != "Wi-Fi Off"
        )
    )
    desktop.close_launcher()
    says_how_wi_fi_is_set(desktop)
    # Turning Wi-Fi off is an action like any, found by name from the top.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("wifi off")
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
    desktop.step_in()
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
    desktop.photograph("launcher-addresses-wifi")
    desktop.normal()
    machine.send_key("left")
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == addresses and shown["typed"] == "wi-fi"
    )
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    # The right arrow opens the VPN's, which has no hardware address, and
    # the old word for it finds nothing.
    desktop.type_in("tunnel")
    desktop.launcher_until(lambda shown: "VPN" not in shown["rows"])
    desktop.normal()
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    desktop.type_in("vpn", selecting="VPN")
    desktop.step_in()
    desktop.type_in("address", selecting="Address")
    assert desktop.showing()["selected"]["side"] == "100.100.1.2/32"
    assert "Hardware Address" not in desktop.showing()["rows"], desktop.showing()
    desktop.close_launcher()
    machine.succeed("ip link delete tailscale0")


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

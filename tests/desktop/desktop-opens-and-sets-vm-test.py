"""The desktop test's opens and sets area.

Here the desktop opens and sets things: the clipboard, pictures, documents, sound, the power
profile, displays and Wi-Fi. tests/desktop.nix runs it after desktop_prelude.py, the boot and
the helpers every area shares, so the names it takes from there are imported only for type
checkers.
"""

import datetime
import json
import time
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

from PIL import Image, ImageChops

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_prelude import (
        ACCOUNT,
        ASK_PORTAL,
        FFMPEG_HEADLESS,
        LAUNCHER_OPEN,
        PAGE,
        PICTURE,
        SIGNING_IN,
        STARTING,
        ZENITY,
        Json,
        answer,
        as_account,
        copied_then_closed,
        drawn_at,
        in_session,
        launcher_ready,
        launcher_until,
        mark,
        opens_in,
        pasted_the_picture,
        photograph,
        reported,
        settled_screen,
        showing,
        step_in,
        told,
        type_in,
        window_titled,
    )
    from driver_names import machine, retry, subtest, t

with subtest("Super+V finds what was copied earlier, and copies it again"):
    keeping = as_account("systemctl --user is-active clipboard-history")
    t.assertEqual(keeping.strip(), "active")
    # Each copy is waited on until the shell has heard it, so they're
    # kept in the order made.
    with told("heard copy Notes for Tuesday"):
        as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")
    with told("heard copy the second copy"):
        as_account("printf 'the second copy\\nover two lines' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    launcher_ready()
    # The newest is selected, its two lines run together in its row and
    # read whole below the rows.
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("reads") == "the second copy\nover two lines"
    )
    photograph("launcher-copies-all")
    type_in("tues", selecting="Notes for Tuesday")
    photograph("launcher-copies")
    machine.send_key("ret")

    def copied_again(_: bool) -> bool:
        """Say whether the clipboard holds the notes' title again."""
        return as_account("wl-paste --no-newline") == "Notes for Tuesday"

    retry(copied_again, timeout=timedelta(seconds=10))


# Return pastes into the window Super+V opened over, with the keys it
# pastes by: a terminal's Ctrl+Shift+V, as a terminal takes Ctrl+V as a key
# of its own, and Ctrl+V in a GTK program's field.
with subtest("Super+V pastes into the window it opened over, a terminal or not"):
    as_account(
        "niri msg action spawn -- kitty --title paste-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/pasted'"
    )
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-here"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-v")
    launcher_ready()
    type_in("tues", selecting="Notes for Tuesday")
    # Paste first, and Copy beside it for only copying.
    step_in()
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Paste")
    with told("pastes into kitty"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("grep -qx 'Notes for Tuesday' /tmp/pasted"), timeout=timedelta(seconds=10)
    )
    as_account("pkill -f 'cat > /tmp/pasted'")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q paste-here"), timeout=timedelta(seconds=30)
    )

    # zenity says what its field holds once Return closes it.
    entry = f"{ZENITY}/bin/zenity --entry --title=paste-gtk"
    as_account(f"systemd-run --user --collect sh -c '{entry} > /tmp/entered'")
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-gtk"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-v")
    launcher_ready()
    type_in("tues", selecting="Notes for Tuesday")
    before_paste = mark()
    machine.send_key("ret")
    reported("pastes into ", lambda program: program != "", before_paste)
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q paste-gtk"), timeout=timedelta(seconds=10)
    )
    # The shell says so as it starts wtype, which then presses the keys;
    # Return any sooner would close the field still empty.
    time.sleep(1)
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("grep -qx 'Notes for Tuesday' /tmp/entered"), timeout=timedelta(seconds=10)
    )


with subtest("a copy outlives the program that made it, unless it wasn't kept"):
    with told("heard copy gone with its program"):
        copied_then_closed("gone with its program")

    # Empty for a moment between its program going and the helper's
    # offer, which the retry waits out.
    def still_there(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return as_account("wl-paste --no-newline 2>&1 || true") == "gone with its program"

    retry(still_there, timeout=timedelta(seconds=10))
    # One the history didn't keep, such as a password manager's, goes
    # with its program, leaving the clipboard empty rather than handing
    # back an older copy. The helper answers the moment it empties, so
    # empty twice over is empty for good.
    copied_then_closed(
        "a password",
        flags="--type x-kde-passwordManagerHint",
        served="wl-paste --list-types 2>/dev/null | grep -qx x-kde-passwordManagerHint",
    )
    machine.wait_until_fails(
        in_session("wl-paste --list-types > /dev/null 2>&1"), timeout=timedelta(seconds=10)
    )
    machine.fail(in_session("wl-paste --list-types > /dev/null 2>&1"))

with subtest("a copy made as another's program closes is the one kept"):
    # The helper reads one event at a time, so the clipboard emptying
    # can wait behind a newer copy, as it does while a slow program
    # hands over what it copied. Paused, it hears both late.
    with told("heard copy closing as another copies"):
        as_account(
            "systemd-run --user --unit=closing sh -c "
            "\"printf %s 'closing as another copies' | wl-copy --foreground\""
        )
    as_account("systemctl --user kill --signal=SIGSTOP clipboard-history")
    as_account("systemctl --user stop closing")
    as_account("echo -n 'copied meanwhile' | wl-copy > /dev/null 2>&1")
    with told("heard copy copied meanwhile"):
        as_account("systemctl --user kill --signal=SIGCONT clipboard-history")
    # Nothing takes it back a moment later.
    machine.sleep(timedelta(seconds=1))
    t.assertEqual(as_account("wl-paste --no-newline"), "copied meanwhile")


with subtest("a picture copied is kept, shown as it is, and copied again"):
    with told("heard copy Image 256 \u00d7 256"):
        as_account(f"wl-copy --type image/png < {PICTURE} > /dev/null 2>&1")
    with told("heard copy something after the picture"):
        as_account("echo -n 'something after the picture' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    launcher_ready()
    with told("showed Image 256 \u00d7 256"):
        type_in("image", selecting="Image 256 \u00d7 256")
    photograph("launcher-copies-picture")
    machine.send_key("ret")
    retry(pasted_the_picture, timeout=timedelta(seconds=10))

with subtest("a picture outlives the program that copied it"):
    # The helper holds the same picture already, so wl-copy serves it
    # once the clipboard's types no longer include the helper's own.
    with told("heard copy Image 256 \u00d7 256"):
        as_account(
            f"wl-copy --foreground --type image/png < {PICTURE} > /dev/null 2>&1 & p=$!; "
            "for i in $(seq 100); do t=$(wl-paste --list-types 2>/dev/null); "
            'echo "$t" | grep -qx image/png && '
            '! echo "$t" | grep -qx application/x-clipboard-history-kept && break; '
            "sleep 0.1; done; kill $p"
        )
    retry(pasted_the_picture, timeout=timedelta(seconds=10))

with subtest("Forget the Clipboard forgets every copy, and what it held"):
    with told("heard copy kept until forgotten"):
        copied_then_closed("kept until forgotten")

    def still_kept(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return as_account("wl-paste --no-newline 2>&1 || true") == "kept until forgotten"

    retry(still_kept, timeout=timedelta(seconds=10))
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("forget the clip", selecting="Forget the clipboard")
    photograph("launcher-forget-copies")
    with told("forgot the copies"):
        machine.send_key("ret")
    t.assertEqual(as_account("wl-paste --no-newline 2>&1 || true"), "")
    # The pictures' files go with them.
    t.assertEqual(as_account("ls -A $XDG_RUNTIME_DIR/clipboard-history"), "")

# Home is a shelf that keeps its shape: what's in front and the likely
# picks, then the same three collections, in the same order, then the
# categories to browse. The rest are a word away.
with subtest("the launcher's home keeps its shelf, and a collection opens by name"):
    windows = len(json.loads(as_account("niri msg --json windows")))
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    photograph("launcher-home")
    rows = showing()["rows"]
    shelf = rows.index("Windows")
    t.assertEqual(rows[shelf : shelf + 4], ["Windows", "Clipboard", "Notifications", "Computer"])
    # A page down stops on the last of the categories.
    for _page in range(6):
        machine.send_key("pgdn")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Commands")
    # Opening a collection keeps the launcher open, where a program
    # picked instead would close it.
    type_in("shortcuts", selecting="Shortcuts")
    step_in("ret")
    type_in("open a terminal")
    machine.send_key("ret")

    def another_opened(_: bool) -> bool:
        """Say whether a window has opened since the launcher did."""
        return len(json.loads(as_account("niri msg --json windows"))) > windows

    retry(another_opened, timeout=timedelta(seconds=15))

with subtest("a screenshot just taken is at the launcher's home, and opens in GIMP"):
    pictures = f"/home/{ACCOUNT}/Pictures/Screenshots"
    # Made at login, since niri makes it only with its first screenshot
    # and the launcher watches it from the start.
    machine.succeed(f"test -d {pictures}")
    as_account("niri msg action screenshot-screen")
    machine.wait_until_succeeds(f"ls {pictures}/*.png", timeout=timedelta(seconds=15))
    picture = machine.succeed(f"ls {pictures}/*.png").strip()
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    # Just below the window in front, which home always puts first.
    machine.send_key("down")
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Edit the screenshot in GIMP"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(f"pgrep -u {ACCOUNT} -f '[g]imp.*{picture}'", timeout=STARTING)
    # GIMP goes before it draws, so no window of its own takes the keys
    # meant for what follows, and the picture too, since the region
    # screenshot's subtest waits for one of its own. GIMP takes any other
    # signal as a crash, and writes its backtrace to the log.
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[g]imp'")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[g]imp'", timeout=timedelta(seconds=30))
    machine.succeed(f"rm {pictures}/*.png")

# A program asks the portal what to open a picture with, as one does when
# more than one app could, and the launcher asks rather than GNOME's
# dialog, holding the apps the portal offers. Return opens the picture
# with the one taken, and the portal says it opened; asked again, Escape
# opens nothing, and the portal says it was refused; and each ask's
# socket goes once it's answered. Each time the apps offered change
# while it waits, as the portal says when apps are installed, so a
# pick and a refusal still answer the ask as it was first held.
with subtest("the launcher asks what to open a picture with, for the portal"):
    portal_png = f"/home/{ACCOUNT}/portal-picked.png"
    machine.succeed(
        "echo iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk"
        "+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        f" | base64 -d > {portal_png} && chown {ACCOUNT} {portal_png}"
    )
    opened_with = f"pgrep -u {ACCOUNT} -af '[p]ortal-picked.png' | grep -v ask-portal"
    sockets_gone = "! ls /run/user/1000/open-with/*.sock"
    portal_said = "/run/user/1000/portal-said"

    def portal_told() -> str:
        """Give what the shell, the chooser, the portal and the caller made of an ask.

        It's said when the launcher never holds the apps.
        """
        _, said = machine.execute(
            in_session(
                "qs ipc show | grep -A4 open-with; ls -la /run/user/1000/open-with; "
                f"cat {portal_said}; pgrep -af ask-portal; "
                "busctl --user list | grep -i -e portal -e ask; "
                "journalctl --user --no-pager --output=cat --lines=80 "
                "--unit=ask-portal --unit=open-with --unit=xdg-desktop-portal"
            )
        )
        return said

    def ask_portal() -> None:
        """Ask the portal to open the picture, and wait until the launcher holds its apps."""
        machine.succeed(f"rm -f {portal_said}")
        # Under the account's manager, on the bus the portal is on.
        as_account(f"systemd-run --user --collect --unit=ask-portal {ASK_PORTAL} {portal_png}")
        try:
            launcher_until(
                lambda shown: (
                    shown["keys"] and shown["held"] == ["apps"] and shown["selected"] is not None
                ),
                timeout=timedelta(seconds=60),
            )
        except AssertionError as unheard:
            unanswered = f"{unheard}; what was said:\n{portal_told()}"
            raise AssertionError(unanswered) from None

    def offer_only_swayimg() -> None:
        """Offer only swayimg now, one of the apps first offered."""
        reply = machine.succeed("ls /run/user/1000/open-with/*.sock").strip()
        as_account(f"qs ipc call open-with update {reply} swayimg")
        launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Swayimg")

    ask_portal()
    photograph("open-with")
    offer_only_swayimg()
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 0 {portal_said}", timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(opened_with, timeout=timedelta(seconds=60))
    launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[p]ortal-picked.png'")
    machine.wait_until_fails(
        f"pgrep -u {ACCOUNT} -f '[p]ortal-picked.png'", timeout=timedelta(seconds=30)
    )

    ask_portal()
    offer_only_swayimg()
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(f"grep -qx 1 {portal_said}", timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))
    machine.fail(opened_with)
    machine.succeed(f"rm {portal_png}")


with subtest("a picture opens floating in swayimg, and e takes it to GIMP"):
    # Beside the screenshots, never among them, since the region
    # screenshot's subtest waits for a picture there of its own.
    looked = f"/home/{ACCOUNT}/snowflake.png"
    as_account(f"cp {PICTURE} {looked}")
    opens_in(looked, "image/png", "swayimg")
    swayimg = window_titled("Swayimg: snowflake.png")
    assert swayimg["is_floating"], swayimg
    # Four fifths of the screen, centred, at the size niri gave it,
    # less the gaps and border niri takes out of it. Without swayimg's
    # size given again, it's twice that, off the screen.
    left, top, width, height = drawn_at(swayimg)
    wide = json.loads(as_account("niri msg --json focused-output"))["logical"]["width"]
    assert abs(width - wide * 0.8) <= wide * 0.02, swayimg
    assert abs(left + width / 2 - wide / 2) <= wide * 0.02, swayimg
    # The steps before leave notices, which go on their own.
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    photograph("swayimg")
    machine.send_key("e")
    machine.wait_until_succeeds(f"pgrep -u {ACCOUNT} -f '[g]imp.*{looked}'", timeout=STARTING)
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -x swayimg", timeout=timedelta(seconds=10))
    # As in the screenshot's subtest: GIMP goes before it draws.
    machine.succeed(f"pkill -KILL -u {ACCOUNT} -f '[g]imp'")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[g]imp'", timeout=timedelta(seconds=30))
    machine.succeed(f"rm {looked}")

with subtest("a document opens in zathura, beside the rest, and q closes it"):
    opens_in(PAGE, "application/pdf", "org.pwmt.zathura")
    windows = json.loads(as_account("niri msg --json windows"))
    zathura = next(window for window in windows if window["app_id"] == "org.pwmt.zathura")
    assert not zathura["is_floating"], zathura
    photograph("zathura")
    machine.send_key("q")
    machine.wait_until_fails(f"pgrep -u {ACCOUNT} -f '[z]athura'", timeout=timedelta(seconds=10))

with subtest("the launcher's Sound moves sound to another output, and back"):
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        as_account("qs ipc call launcher collection sound")
        launcher_ready()
        photograph(f"launcher-sound-{output}")
        kind = output.removeprefix("stand-in-")
        type_in(kind, selecting=kind)
        machine.send_key("ret")
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(in_session(used), timeout=timedelta(seconds=10))
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


def sound_reads(name: str, said: str) -> Callable[[Json], bool]:
    """Make a test of whether the launcher shows a row by this name reading this."""
    return lambda shown: any(
        thing["reads"].startswith(name) and said in thing["reads"] for thing in shown["things"]
    )


def sound_selected(name: str) -> Callable[[Json], bool]:
    """Make a test of whether the launcher's selected row is this one."""
    return lambda shown: (shown["selected"] or {}).get("name") == name


def sound_device(typed: str, named: str) -> None:
    """Open the launcher's Sound, and step into a device's parts by the right arrow."""
    as_account("qs ipc call launcher collection sound")
    launcher_ready()
    type_in(typed, selecting=named)
    step_in()


def sound_recording() -> int | None:
    """Give the id of pw-record's stream, as the sound server lists it, if it's there."""
    nodes: list[Json] = json.loads(as_account("pw-dump"))
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


with subtest("the launcher's Sound opens an output's and a microphone's parts"):
    # The speakers, in use, hold their Level, Volume, whether they're
    # Muted, and Channels, named first; the null sink has no card, so
    # nothing more.
    sound_device("speakers", "speakers")
    launcher_until(lambda shown: shown["rows"][:3] == ["Level", "Volume", "Muted"])
    launcher_until(lambda shown: "Channels" in shown["rows"])
    launcher_until(sound_reads("Channels", "stereo"))
    launcher_until(sound_reads("Muted", "no"))
    # Nothing plays, so the Level reads silent, and no program is listed.
    launcher_until(sound_reads("Level", "silent"))
    t.assertNotIn("Playing", showing()["rows"])
    photograph("launcher-sound-output-parts")
    # Return on Volume steps into its choices, each level saying its
    # decibels in the footer.
    machine.send_key("down")
    launcher_until(sound_selected("Volume"))
    launcher_until(lambda shown: shown["footer"].endswith(" dB"))
    step_in("ret")
    launcher_until(lambda shown: shown["rows"][:2] == ["Louder", "Quieter"])
    photograph("launcher-sound-volume")
    type_in("50", selecting="Volume to 50%")
    launcher_until(lambda shown: "-18.1 dB" in shown["footer"])
    machine.send_key("ret")
    sink = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.50'"
    machine.wait_until_succeeds(in_session(sink), timeout=timedelta(seconds=10))
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # The part says what it was set to.
    sound_device("speakers", "speakers")
    launcher_until(sound_reads("Volume", "50%"))
    # The right arrow on Muted holds Mute and Unmute, Unmute in use, and
    # Return on Mute mutes the speakers; then Unmute, its Return, unmutes.
    for _down in range(2):
        machine.send_key("down")
    launcher_until(sound_selected("Muted"))
    step_in()
    launcher_until(lambda shown: shown["rows"] == ["Mute", "Unmute"])
    launcher_until(sound_reads("Unmute", "in use"))
    photograph("launcher-sound-muted")
    launcher_until(sound_selected("Mute"))
    machine.send_key("ret")
    hushed_sink = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED"
    machine.wait_until_succeeds(in_session(hushed_sink), timeout=timedelta(seconds=10))
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    sound_device("speakers", "speakers")
    launcher_until(sound_reads("Muted", "yes"))
    type_in("unmute", selecting="Unmute")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(hushed_sink), timeout=timedelta(seconds=10))
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

    # The microphone holds its own: a live Level, its Gain, whether it's
    # Muted, and who's Recording, which is nobody, though the Level's own
    # stream is reading it while it shows.
    sound_device("microphone", "stand-in microphone")
    launcher_until(
        lambda shown: shown["rows"][:4] == ["Level", "Gain", "Muted", "Recording"],
    )
    launcher_until(sound_reads("Level", "%"))
    launcher_until(sound_reads("Muted", "no"))
    launcher_until(sound_reads("Recording", "nothing"))
    photograph("launcher-sound-input-parts")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    listening = f"/home/{ACCOUNT}/listening.wav"
    as_account(
        f"systemd-run --user --unit=listening pw-record --target stand-in-microphone {listening}"
    )

    def records(last: bool) -> bool:
        """Say whether pw-record's stream is there, and on the last try, what is."""
        if last:
            print(as_account("systemctl --user status listening; pw-cli ls Node"))
        return sound_recording() is not None

    retry(records, timeout=datetime.timedelta(seconds=30))
    recording = sound_recording()
    sound_device("microphone", "stand-in microphone")
    launcher_until(lambda shown: "Recording" in shown["rows"])
    launcher_until(lambda shown: not sound_reads("Recording", "nothing")(shown))
    # The right arrow on Recording, then on the program, and Return mutes
    # what it records.
    for _down in range(3):
        machine.send_key("down")
    launcher_until(sound_selected("Recording"))
    step_in()
    step_in()
    launcher_until(lambda shown: shown["rows"][:3] == ["Mute", "Louder", "Quieter"])
    photograph("launcher-sound-recording")
    launcher_until(sound_selected("Mute"))
    machine.send_key("ret")
    hushed = f"wpctl get-volume {recording} | grep -q MUTED"
    machine.wait_until_succeeds(in_session(hushed), timeout=timedelta(seconds=10))
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    as_account("systemctl --user stop listening")
    as_account(f"rm -f {listening}")

# A program's own Media and Sound, against a real player playing real
# sound: mpv, through its MPRIS script, playing two tones in turn through
# PipeWire to the speakers. What each act does is read back from the
# player over D-Bus and from the stream in PipeWire, never from the
# launcher, which is only asked what it shows. The tones are long and the
# playlist loops, so nothing moves on by itself, and Next and Previous
# always apply, as they do in a music player set to repeat.
#
# How long the player, PipeWire or the launcher may take to show a change.
WAIT = datetime.timedelta(seconds=15)
# How far a pixel's shade moves, of 255, to count as drawn anew rather than
# as noise.
CHANGED = 24
# How far above and below the line that changed most a row's dots reach,
# in pixels, which is less than the gap to the next row's.
ROW = 16


with subtest("a program's Media and Sound work its real player and its stream"):
    tones = f"/home/{ACCOUNT}/tones"
    as_account(f"mkdir -p {tones}")
    for title, pitch in [("First tone", 440), ("Second tone", 660)]:
        as_account(
            f"{FFMPEG_HEADLESS}/bin/ffmpeg -loglevel error -f lavfi"
            f" -i sine=frequency={pitch}:sample_rate=8000:duration=600"
            f" -metadata title='{title}' -metadata artist='Test tones' {tones}/{pitch}.flac"
        )
    as_account(
        "systemd-run --user --unit=tones mpv --ao=pipewire --no-video --no-terminal"
        f" --loop-playlist=inf {tones}/440.flac {tones}/660.flac"
    )
    # Its name on the bus ends in an instance of its own making.
    named = "busctl --user list --acquired | grep -o '^org\\.mpris\\.MediaPlayer2\\.mpv\\S*'"
    bus = machine.wait_until_succeeds(in_session(named), timeout=WAIT).strip()
    mpris = (
        f"busctl --user --json=short get-property {bus}"
        " /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player"
    )

    def player(field: str) -> Any:  # noqa: ANN401, since D-Bus says what type it is.
        """Give one of the player's own properties, as it tells D-Bus."""
        return json.loads(as_account(f"{mpris} {field}"))["data"]

    def playing(status: str, title: str) -> None:
        """Wait until the player says it's in this state, playing this tone."""

        def says(_: bool) -> bool:
            return bool(
                player("PlaybackStatus") == status
                and player("Metadata")["xesam:title"]["data"] == title
            )

        retry(says, timeout=datetime.timedelta(seconds=15))

    def stream() -> int:
        """Give the id of the stream mpv plays through, as PipeWire lists it."""
        nodes: list[Json] = json.loads(as_account("pw-dump"))
        return next(
            int(node["id"])
            for node in nodes
            if node.get("type") == "PipeWire:Interface:Node"
            and node["info"]["props"].get("media.class") == "Stream/Output/Audio"
            and node["info"]["props"].get("application.name") == "mpv"
        )

    playing("Playing", "First tone")
    machine.wait_until_succeeds(in_session("pw-dump | grep -q Stream/Output/Audio"), timeout=WAIT)
    tone = stream()

    def heard(volume: str) -> None:
        """Wait until PipeWire says the stream is this loud, as wpctl says it."""
        loud = f"wpctl get-volume {tone} | grep -qxF 'Volume: {volume}'"
        machine.wait_until_succeeds(in_session(loud), timeout=WAIT)

    def reads(name: str, said: str) -> Callable[[Json], bool]:
        """Make a test of whether the launcher shows a row by this name reading this."""
        return lambda shown: any(
            thing["reads"].startswith(name) and said in thing["reads"] for thing in shown["things"]
        )

    def controls(part: str = "") -> None:
        """Open the launcher on mpv's controls, and on one of its parts when one is named."""
        as_account("qs ipc call launcher toggle")
        launcher_ready()
        type_in("mpv", selecting="mpv Media Player")
        step_in()
        launcher_until(lambda shown: {"Media", "Sound"} <= set(shown["rows"]))
        if part:
            type_in(part.lower(), selecting=part)
            step_in()

    def press(name: str, then: str = "") -> None:
        """Type a row's name and press Return on it, then clear what was typed if it stays."""
        type_in(name.lower(), selecting=name)
        machine.send_key("ret")
        if then:
            launcher_until(lambda shown: shown["footer"] == then)
            machine.send_key("left")
            launcher_until(lambda shown: shown["typed"] == "")
        else:
            machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=WAIT)

    # The right arrow on the program holds Media and Sound, each saying
    # how it stands.
    controls()
    photograph("launcher-program-parts")
    launcher_until(reads("Media", "playing"))
    launcher_until(reads("Sound", "100%"))
    type_in("media", selecting="Media")
    step_in()
    photograph("launcher-program-media")
    launcher_until(lambda shown: shown["rows"] == ["Pause", "Next", "Previous", "First tone"])
    launcher_until(reads("First tone", "Test tones"))
    # Next and Previous keep the launcher, saying what's on now, and what's
    # playing follows.
    press("Next", then="Second tone, Test tones")
    playing("Playing", "Second tone")
    launcher_until(lambda shown: shown["rows"][-1] == "Second tone")
    press("Previous", then="First tone, Test tones")
    playing("Playing", "First tone")
    launcher_until(lambda shown: shown["rows"][-1] == "First tone")
    # Paused from elsewhere, as a media key does, Pause says Play.
    as_account("qs ipc call music toggle")
    playing("Paused", "First tone")
    launcher_until(lambda shown: shown["rows"][0] == "Play")
    press("Play")
    playing("Playing", "First tone")
    controls("Media")
    press("Pause")
    playing("Paused", "First tone")
    controls()
    launcher_until(reads("Media", "paused"))
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=WAIT)

    # Sound's Louder and Quieter step the stream by a twentieth, keeping
    # the launcher, and its volume follows. The stream is only ever set
    # from the launcher here: set from elsewhere, as by wpctl, Quickshell
    # now and then reads the stream's volume back from before the change
    # and keeps it until the next one, which is PipeWire's and Quickshell's
    # to mend, not the launcher's.
    controls("Sound")
    photograph("launcher-program-sound")
    launcher_until(lambda shown: shown["rows"] == ["Mute", "Louder", "Quieter", "Volume"])
    launcher_until(reads("Volume", "100%"))
    press("Quieter", then="now 95%")
    heard("0.95")
    launcher_until(reads("Volume", "95%"))
    press("Louder", then="now 100%")
    heard("1.00")
    launcher_until(reads("Volume", "100%"))
    # Mute closes the launcher, and the stream is muted; it says Unmute
    # when next opened, and the program's Sound says it's muted.
    press("Mute")
    heard("1.00 [MUTED]")
    controls()
    launcher_until(reads("Sound", "muted"))
    type_in("sound", selecting="Sound")
    step_in()
    launcher_until(lambda shown: shown["rows"][0] == "Unmute")
    launcher_until(reads("Volume", "muted"))
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
        launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Volume")
        for _up in range(2 if steps > 0 else 1):
            machine.send_key("up")
        turning = "Louder" if steps > 0 else "Quieter"
        launcher_until(lambda shown: (shown["selected"] or {}).get("name") == turning)
        for step in range(1, abs(steps) + 1):
            now = loudness["now"] + (5 if steps > 0 else -5) * step
            machine.send_key("ret")
            launcher_until(foot(f"now {now}%"))
        loudness["now"] = volume
        heard(f"0.{volume}")
        machine.send_key("pgdn")
        launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Volume")
        launcher_until(reads("Volume", f"{volume}%"))
        return settled_screen(f"launcher-program-volume-{volume}")

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
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=WAIT)
    as_account("systemctl --user stop tones")
    machine.succeed(f"rm -r {tones}")

with subtest("the launcher sets the power profile"):
    # By its name, or a word a person would call it.
    for typed, profile, name in [
        ("eco", "power-saver", "Power saver"),
        ("balanced power", "balanced", "Balanced power"),
        # Its old name, a hidden word, still finds it.
        ("use power saver", "power-saver", "Power saver"),
    ]:
        as_account("qs ipc call launcher toggle")
        launcher_ready()
        type_in(typed, selecting=name)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # And as the setting it is: one part saying which is in use, its
    # choices a step in by Return or the right arrow, each named there as
    # the part reads it.
    for key, profile, name, was in [
        ("ret", "balanced", "Balanced", "Power saver"),
        ("right", "power-saver", "Power saver", "Balanced"),
    ]:
        as_account("qs ipc call launcher toggle")
        launcher_ready()
        type_in("power profile", selecting="Power profile")
        # The wait ends before the loop moves on, so it reads this pass's name.
        launcher_until(lambda shown: (shown["selected"] or {}).get("side") == was)  # noqa: B023
        step_in(key)
        launcher_until(lambda shown: shown["rows"][:2] == ["Power saver", "Balanced"])
        type_in(name.split()[0][:3].lower(), selecting=name)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

# The right arrow on one of a collection's members offers its own
# actions, as on anything at the top.
with subtest("the launcher's Displays scales a display by keys, and back"):

    def scale() -> float:
        """Give the scale niri draws the display at."""
        outputs = json.loads(as_account("niri msg --json outputs"))
        return cast("float", next(iter(outputs.values()))["logical"]["scale"])

    # What desk-facts would say of the display, which this machine doesn't
    # run: each fact reads by the vocabulary's name for it, the depth first.
    shown_as = next(iter(json.loads(as_account("niri msg --json outputs"))))
    said = {shown_as: [{"name": "EDID", "value": "1.4"}, {"name": "depth", "value": "8 bit"}]}
    as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/desk && echo '{json.dumps(said)}' > $XDG_RUNTIME_DIR/desk/.d"
        " && mv $XDG_RUNTIME_DIR/desk/.d $XDG_RUNTIME_DIR/desk/displays.json"
    )

    for wanted, inside in [(1.5, True), (scale(), False)]:
        as_account("qs ipc call launcher collection displays")
        launcher_ready()
        step_in()
        # Its settings lead, each saying how it's set, their choices a step
        # in, there by Return on the setting, or found from here by what's
        # typed.
        launcher_until(
            lambda shown: (
                shown["rows"][:6]
                == ["Scale", "Refresh", "Resolution", "Rotation", "Colour depth", "EDID"]
            )
        )
        photograph("launcher-displays")
        percent = round(wanted * 100)
        if inside:
            step_in("ret")
            launcher_until(lambda shown: "Scale to 150%" in shown["rows"])
            type_in(str(percent))
        else:
            type_in(f"scale {percent}")
        machine.send_key("ret")
        retry(lambda _, wanted=wanted: scale() == wanted, timeout=timedelta(seconds=15))
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

    # A part is asked by its words from the top: the refresh rate.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("refresh rate")
    launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and str((shown["selected"] or {}).get("name", "")).endswith(" Hz")
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

with subtest("a Wi-Fi network is joined from the launcher, a new one asking its password"):
    machine.wait_for_unit("lantern.service")
    machine.wait_for_unit("wifid.service")
    wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Lantern\"'",
        timeout=timedelta(seconds=60),
    )
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    photograph("launcher-wifi")
    type_in("lant", selecting="Lantern")
    # Never joined, it asks for its password, which a wrong one doesn't
    # satisfy; the launcher stays to say so.
    step_in("ret")
    type_in("not the password")
    photograph("launcher-wifi-password")
    with told("refused wifi Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    machine.succeed('journalctl -u wifid | grep -q "couldn\'t join Lantern: WrongPassword"')
    photograph("launcher-wifi-refused")
    machine.succeed(in_session(LAUNCHER_OPEN))
    type_in("glowing in the dark")
    machine.send_key("ret")
    # Joined means an address too, and wifid says so once it's checked
    # how far the network reaches.
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Lantern on wlan1'", timeout=timedelta(seconds=60)
    )
    machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
    # Once it's joined, the launcher's done.
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # Lantern wants a sign-in, so Sign in to it comes first at home, and
    # Return opens the window on its front desk.
    described = "varlinkctl --json=short call /run/wifid/io.jitsusama.Wifi"
    described += " io.jitsusama.Wifi.Describe '{}'"
    machine.wait_until_succeeds(
        f'{described} | grep -qF \'"reach":"signIn"\'', timeout=timedelta(seconds=60)
    )
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    photograph("launcher-sign-in")
    type_in("sign in", selecting="Sign in to Lantern")
    machine.send_key("ret")
    # The window is GTK and WebKit started cold, drawn in software,
    # which on a machine running other tests too has taken 42 seconds
    # to show and more than 60, so it's given three minutes for both.
    window = 'niri msg --json windows | grep -qF \'"app_id":"io.jitsusama.WifiSignIn"\''
    machine.wait_until_succeeds(in_session(window), timeout=SIGNING_IN)
    # Its button can only be pressed once the front desk has served
    # the page, which a window that has to try again does later.
    machine.wait_until_succeeds(
        "journalctl -u lantern-front-desk | grep -q 'GET /sign-in'", timeout=SIGNING_IN
    )
    photograph("sign-in", timeout=timedelta(seconds=20))
    # Its one button signs in, and the window closes as Lantern lets
    # the machine through, its notice taken back with it.
    with told("withdrew sign-in Lantern", timeout=timedelta(seconds=60)):
        machine.send_key("ret")
    machine.wait_until_fails(in_session(window), timeout=timedelta(seconds=60))
    machine.succeed(f'{described} | grep -qF \'"reach":"online"\'')
    # The Office asks for an account: met for the first time, it asks
    # the account's name, its server's domain and its password in
    # turn, and joins.
    machine.wait_until_succeeds(
        f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Office\"'",
        timeout=timedelta(seconds=60),
    )
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    type_in("office", selecting="Office")
    step_in("ret")
    type_in("joel")
    step_in("ret")
    type_in("radius.office.test")
    step_in("ret")
    type_in("hunter22")
    photograph("launcher-wifi-account")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "journalctl -u wifid | grep -q 'joined Office on wlan1'", timeout=timedelta(seconds=60)
    )
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # Stepped into, a network says after what can be done to it what's
    # heard of it, each a part named first, the selected row's footer
    # saying more: the card's Signal and Speed once the card has read
    # them, each holding more, and its access points a step further in.
    as_account("qs ipc call launcher collection wifi")
    launcher_ready()
    type_in("office", selecting="Office")
    step_in()
    office = len(showing()["held"])
    # The Signal says its dBm, always read, and how far over the noise
    # below; Return opens it, where iw, read while the network shows,
    # says what the card sends at.
    type_in("signal", selecting="Signal")
    launcher_until(lambda shown: str(shown["selected"]["side"]).endswith(" dBm"))
    launcher_until(lambda shown: "over the noise" in shown["footer"])
    step_in("ret")
    launcher_until(lambda shown: "Transmit power" in shown["rows"])
    # The left arrow steps out with the word put back, and again takes
    # the word.
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "signal")
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    photograph("launcher-wifi-detail")
    # The Speed holds each way and the retries, the right arrow into it.
    type_in("speed", selecting="Speed")
    step_in()
    launcher_until(lambda shown: {"In", "Retries"} <= set(shown["rows"]))
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "speed")
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == office and shown["typed"] == "")
    # The access points, each named by its band and channel, and each
    # holding its own parts, its hardware address among them.
    type_in("access", selecting="Access points")
    step_in()
    launcher_until(lambda shown: shown["rows"] == ["2.4 GHz, channel 1"])
    photograph("launcher-wifi-points")
    step_in("ret")
    type_in("hardware", selecting="Hardware address")
    t.assertEqual(showing()["selected"]["side"], "02:00:00:00:02:00")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # Turning Wi-Fi off is an action like any.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("wifi off")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=timedelta(seconds=15)
    )
    machine.succeed("rfkill unblock wifi")

with subtest("the addresses name each connection, its parts a step in"):
    # A tunnel beside the Wi-Fi, as the laptop has.
    machine.succeed(
        "ip tuntap add dev tailscale0 mode tun"
        " && ip address add 100.100.1.2/32 dev tailscale0 && ip link set tailscale0 up"
    )
    # Wi-Fi was turned off and on again, so the card joins again first.
    machine.wait_until_succeeds(
        "ip -4 address show wlan1 | grep -q 'inet 10.4'", timeout=timedelta(seconds=90)
    )
    answer("ip")
    step_in()
    addresses = len(showing()["held"])
    # Each is named by what it is, never by the kernel's name for it.
    launcher_until(lambda shown: {"Wi-Fi", "Tunnel"} <= set(shown["rows"]))
    assert not {"wlan1", "tailscale0"} & set(showing()["rows"]), showing()
    # Return opens the Wi-Fi's parts, and the kernel's name is none of them.
    type_in("wi-fi", selecting="Wi-Fi")
    step_in("ret")
    launcher_until(
        lambda shown: {"Address", "Hardware address", "MTU", "State"} <= set(shown["rows"])
    )
    assert "Device name" not in showing()["rows"], showing()
    type_in("mtu", selecting="MTU")
    t.assertRegex(showing()["selected"]["side"], r"^[\d,]+ bytes$")
    photograph("launcher-addresses-wifi")
    machine.send_key("left")
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "wi-fi")
    machine.send_key("left")
    launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    # The right arrow opens the tunnel's, which has no hardware address.
    type_in("tunnel", selecting="Tunnel")
    step_in()
    type_in("address", selecting="Address")
    t.assertEqual(showing()["selected"]["side"], "100.100.1.2/32")
    assert "Hardware address" not in showing()["rows"], showing()
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.succeed("ip link delete tailscale0")

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

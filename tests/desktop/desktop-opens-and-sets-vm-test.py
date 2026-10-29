"""The desktop test's opens and sets area.

Here the desktop opens and sets things: the clipboard, pictures, documents, sound, the power
profile, displays and Wi-Fi. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for
the desktop, started once a run (desktop_session.py), and waits at its start for what it needs,
so any test runs alone.
"""

from __future__ import annotations

import json
import re
import time
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, SIGNING_IN, STARTING, drawn_at, retry
from PIL import Image, ImageChops

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def test_super_v_finds_what_was_copied_earlier_and_copies_it_again(desktop: Desktop) -> None:
    """Super+V finds what was copied earlier, and copies it again."""
    machine = desktop.machine
    keeping = desktop.as_account("systemctl --user is-active clipboard-history")
    assert keeping.strip() == "active"
    # Each copy is waited on until the shell has heard it, so they're
    # kept in the order made.
    with desktop.told("heard copy Notes for Tuesday"):
        desktop.as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")
    with desktop.told("heard copy the second copy"):
        desktop.as_account("printf 'the second copy\\nover two lines' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    desktop.launcher_ready()
    # The newest is selected, its two lines run together in its row and
    # read whole below the rows.
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("reads") == "the second copy\nover two lines"
    )
    desktop.photograph("launcher-copies-all")
    desktop.type_in("tues", selecting="Notes for Tuesday")
    desktop.photograph("launcher-copies")
    machine.send_key("ret")

    def copied_again(_: bool) -> bool:
        """Say whether the clipboard holds the notes' title again."""
        return desktop.as_account("wl-paste --no-newline") == "Notes for Tuesday"

    retry(copied_again, timeout=timedelta(seconds=10))


# Return pastes into the window Super+V opened over, with the keys it
# pastes by: a terminal's Ctrl+Shift+V, as a terminal takes Ctrl+V as a key
# of its own, and Ctrl+V in a GTK program's field.


def test_super_v_pastes_into_the_window_it_opened_over_a_terminal_or_not(desktop: Desktop) -> None:
    """Super+V pastes into the window it opened over, a terminal or not."""
    machine = desktop.machine
    desktop.as_account(
        "niri msg action spawn -- kitty --title paste-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/pasted'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q paste-here"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-v")
    desktop.launcher_ready()
    desktop.type_in("tues", selecting="Notes for Tuesday")
    # Paste first, and Copy beside it for only copying.
    desktop.step_in()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Paste")
    with desktop.told("pastes into kitty"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("grep -qx 'Notes for Tuesday' /tmp/pasted"),
        timeout=timedelta(seconds=10),
    )
    desktop.as_account("pkill -f 'cat > /tmp/pasted'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q paste-here"), timeout=timedelta(seconds=30)
    )

    # zenity says what its field holds once Return closes it.
    entry = f"{desktop.values['zenity']}/bin/zenity --entry --title=paste-gtk"
    desktop.as_account(f"systemd-run --user --collect sh -c '{entry} > /tmp/entered'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q paste-gtk"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-v")
    desktop.launcher_ready()
    desktop.type_in("tues", selecting="Notes for Tuesday")
    before_paste = desktop.mark()
    machine.send_key("ret")
    desktop.reported("pastes into ", lambda program: program != "", before_paste)
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q paste-gtk"),
        timeout=timedelta(seconds=10),
    )
    # The shell says so as it starts wtype, which then presses the keys;
    # Return any sooner would close the field still empty.
    time.sleep(1)
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("grep -qx 'Notes for Tuesday' /tmp/entered"),
        timeout=timedelta(seconds=10),
    )


def test_a_copy_outlives_the_program_that_made_it_unless_it_wasnt_kept(desktop: Desktop) -> None:
    """A copy outlives the program that made it, unless it wasn't kept."""
    machine = desktop.machine
    with desktop.told("heard copy gone with its program"):
        desktop.copied_then_closed("gone with its program")

    # Empty for a moment between its program going and the helper's
    # offer, which the retry waits out.
    def still_there(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return desktop.as_account("wl-paste --no-newline 2>&1 || true") == "gone with its program"

    retry(still_there, timeout=timedelta(seconds=10))
    # One the history didn't keep, such as a password manager's, goes
    # with its program, leaving the clipboard empty rather than handing
    # back an older copy. The helper answers the moment it empties, so
    # empty twice over is empty for good.
    desktop.copied_then_closed(
        "a password",
        flags="--type x-kde-passwordManagerHint",
        served="wl-paste --list-types 2>/dev/null | grep -qx x-kde-passwordManagerHint",
    )
    machine.wait_until_fails(
        desktop.in_session("wl-paste --list-types > /dev/null 2>&1"), timeout=timedelta(seconds=10)
    )
    machine.fail(desktop.in_session("wl-paste --list-types > /dev/null 2>&1"))


def test_a_copy_made_as_anothers_program_closes_is_the_one_kept(desktop: Desktop) -> None:
    """A copy made as another's program closes is the one kept."""
    machine = desktop.machine
    # The helper reads one event at a time, so the clipboard emptying
    # can wait behind a newer copy, as it does while a slow program
    # hands over what it copied. Paused, it hears both late.
    with desktop.told("heard copy closing as another copies"):
        desktop.as_account(
            "systemd-run --user --unit=closing sh -c "
            "\"printf %s 'closing as another copies' | wl-copy --foreground\""
        )
    desktop.as_account("systemctl --user kill --signal=SIGSTOP clipboard-history")
    desktop.as_account("systemctl --user stop closing")
    desktop.as_account("echo -n 'copied meanwhile' | wl-copy > /dev/null 2>&1")
    with desktop.told("heard copy copied meanwhile"):
        desktop.as_account("systemctl --user kill --signal=SIGCONT clipboard-history")
    # Nothing takes it back a moment later.
    machine.sleep(timedelta(seconds=1))
    assert desktop.as_account("wl-paste --no-newline") == "copied meanwhile"


def test_a_picture_copied_is_kept_shown_as_it_is_and_copied_again(desktop: Desktop) -> None:
    """A picture copied is kept, shown as it is, and copied again."""
    machine = desktop.machine
    with desktop.told("heard copy Image 256 \u00d7 256"):
        desktop.as_account(
            f"wl-copy --type image/png < {desktop.values['picture']} > /dev/null 2>&1"
        )
    with desktop.told("heard copy something after the picture"):
        desktop.as_account("echo -n 'something after the picture' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    desktop.launcher_ready()
    with desktop.told("showed Image 256 \u00d7 256"):
        desktop.type_in("image", selecting="Image 256 \u00d7 256")
    desktop.photograph("launcher-copies-picture")
    machine.send_key("ret")
    retry(desktop.pasted_the_picture, timeout=timedelta(seconds=10))


def test_a_picture_outlives_the_program_that_copied_it(desktop: Desktop) -> None:
    """A picture outlives the program that copied it."""
    # The helper holds the same picture already, so wl-copy serves it
    # once the clipboard's types no longer include the helper's own.
    with desktop.told("heard copy Image 256 \u00d7 256"):
        desktop.as_account(
            f"wl-copy --foreground --type image/png < {desktop.values['picture']}"
            " > /dev/null 2>&1 & p=$!; "
            "for i in $(seq 100); do t=$(wl-paste --list-types 2>/dev/null); "
            'echo "$t" | grep -qx image/png && '
            '! echo "$t" | grep -qx application/x-clipboard-history-kept && break; '
            "sleep 0.1; done; kill $p"
        )
    retry(desktop.pasted_the_picture, timeout=timedelta(seconds=10))


def test_forget_the_clipboard_forgets_every_copy_and_what_it_held(desktop: Desktop) -> None:
    """Forget the Clipboard forgets every copy, and what it held."""
    machine = desktop.machine
    with desktop.told("heard copy kept until forgotten"):
        desktop.copied_then_closed("kept until forgotten")

    def still_kept(_: bool) -> bool:
        """Say whether the clipboard still holds the copy its program made."""
        return desktop.as_account("wl-paste --no-newline 2>&1 || true") == "kept until forgotten"

    retry(still_kept, timeout=timedelta(seconds=10))
    # The clipboard holds its own Forget, after the copies, so it opens on
    # the newest copy, and Desktop lists the clipboard alone.
    desktop.as_account("qs ipc call launcher collection clipboard")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][-1:] == ["Forget the Clipboard"]
            and (shown["selected"] or {}).get("name") != "Forget the Clipboard"
        ),
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("desktop ")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["desktop"] and "Clipboard" in shown["rows"],
    )
    assert "Forget the Clipboard" not in desktop.showing()["rows"]
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("forget the clip", selecting="Forget the Clipboard")
    desktop.photograph("launcher-forget-copies")
    with desktop.told("forgot the copies"):
        machine.send_key("ret")
    assert desktop.as_account("wl-paste --no-newline 2>&1 || true") == ""
    # The pictures' files go with them.
    assert desktop.as_account("ls -A $XDG_RUNTIME_DIR/clipboard-history") == ""


# Home keeps one shape: Now, then the four places by name. The rest are
# a word away.


def test_the_launchers_home_keeps_its_shape_and_a_collection_opens_by_name(
    desktop: Desktop,
) -> None:
    """The launcher's home keeps its shape, and a collection opens by name."""
    machine = desktop.machine
    windows = len(json.loads(desktop.as_account("niri msg --json windows")))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.photograph("launcher-home")
    rows = desktop.showing()["rows"]
    home = ["Now", "Agents", "Programs", "People", "Desktop", "Machine", "Network", "Keyboard"]
    assert rows == home, rows
    # A page down stops on the last.
    machine.send_key("pgdn")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Keyboard")
    # Opening a collection keeps the launcher open, where a program
    # picked instead would close it.
    desktop.type_in("shortcuts", selecting="Shortcuts")
    desktop.step_in("ret")
    desktop.type_in("open a terminal")
    machine.send_key("ret")

    def another_opened(_: bool) -> bool:
        """Say whether a window has opened since the launcher did."""
        return len(json.loads(desktop.as_account("niri msg --json windows"))) > windows

    retry(another_opened, timeout=timedelta(seconds=15))


def test_a_screenshot_just_taken_is_found_by_typing_and_opens_in_gimp(
    desktop: Desktop,
) -> None:
    """A screenshot just taken is found by typing, and opens in GIMP."""
    machine = desktop.machine
    desktop.pictures = f"/home/{desktop.account}/Pictures/Screenshots"
    # Made at login, since niri makes it only with its first screenshot
    # and the launcher watches it from the start.
    machine.succeed(f"test -d {desktop.pictures}")
    desktop.as_account("niri msg action screenshot-screen")
    machine.wait_until_succeeds(f"ls {desktop.pictures}/*.png", timeout=timedelta(seconds=15))
    picture = machine.succeed(f"ls {desktop.pictures}/*.png").strip()
    # Capture holds what's done with a screenshot once it's taken, after
    # what it captures, so it still opens on the whole screen.
    desktop.as_account("qs ipc call launcher collection capture")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][-2:] == ["Edit the Newest Screenshot", "Look Through the Screenshots"]
            and (shown["selected"] or {}).get("name") == "The Whole Screen"
        ),
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("edit the newest", selecting="Edit the Newest Screenshot")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"pgrep -u {desktop.account} -f '[g]imp.*{picture}'", timeout=STARTING
    )
    # GIMP goes before it draws, so no window of its own takes the keys
    # meant for what follows, and the picture too, since the region
    # screenshot's subtest waits for one of its own. GIMP takes any other
    # signal as a crash, and writes its backtrace to the log.
    machine.succeed(f"pkill -KILL -u {desktop.account} -f '[g]imp'")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[g]imp'", timeout=timedelta(seconds=30)
    )
    machine.succeed(f"rm {desktop.pictures}/*.png")


# A program asks the portal what to open a picture with, as one does when
# more than one app could, and the launcher asks rather than GNOME's
# dialog, holding the apps the portal offers. Return opens the picture
# with the one taken, and the portal says it opened; asked again, Escape
# opens nothing, and the portal says it was refused; and each ask's
# socket goes once it's answered. Each time the apps offered change
# while it waits, as the portal says when apps are installed, so a
# pick and a refusal still answer the ask as it was first held.


def test_the_launcher_asks_what_to_open_a_picture_with_for_the_portal(desktop: Desktop) -> None:
    """The launcher asks what to open a picture with, for the portal."""
    machine = desktop.machine
    portal_png = f"/home/{desktop.account}/portal-picked.png"
    machine.succeed(
        "echo iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk"
        "+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        f" | base64 -d > {portal_png} && chown {desktop.account} {portal_png}"
    )
    opened_with = f"pgrep -u {desktop.account} -af '[p]ortal-picked.png' | grep -v ask-portal"
    sockets_gone = "! ls /run/user/1000/open-with/*.sock"
    portal_said = "/run/user/1000/portal-said"

    def portal_told() -> str:
        """Give what the shell, the chooser, the portal and the caller made of an ask.

        It's said when the launcher never holds the apps.
        """
        _, said = machine.execute(
            desktop.in_session(
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
        desktop.as_account(
            "systemd-run --user --collect --unit=ask-portal"
            f" {desktop.values['ask_portal']} {portal_png}"
        )
        try:
            desktop.launcher_until(
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
        desktop.as_account(f"qs ipc call open-with update {reply} swayimg")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Swayimg")

    ask_portal()
    desktop.photograph("open-with")
    offer_only_swayimg()
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 0 {portal_said}", timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(opened_with, timeout=timedelta(seconds=60))
    desktop.launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))
    machine.succeed(f"pkill -KILL -u {desktop.account} -f '[p]ortal-picked.png'")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[p]ortal-picked.png'", timeout=timedelta(seconds=30)
    )

    ask_portal()
    offer_only_swayimg()
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(f"grep -qx 1 {portal_said}", timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))
    machine.fail(opened_with)
    machine.succeed(f"rm {portal_png}")


def test_a_picture_opens_floating_in_swayimg_and_e_takes_it_to_gimp(desktop: Desktop) -> None:
    """A picture opens floating in swayimg, and e takes it to GIMP."""
    machine = desktop.machine
    # Beside the screenshots, never among them, since the region
    # screenshot's subtest waits for a picture there of its own.
    looked = f"/home/{desktop.account}/snowflake.png"
    desktop.as_account(f"cp {desktop.values['picture']} {looked}")
    desktop.opens_in(looked, "image/png", "swayimg")
    swayimg = desktop.window_titled("Swayimg: snowflake.png")
    assert swayimg["is_floating"], swayimg
    # Four fifths of the screen, centred, at the size niri gave it,
    # less the gaps and border niri takes out of it. Without swayimg's
    # size given again, it's twice that, off the screen.
    left, _, width, _ = drawn_at(swayimg)
    wide = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]["width"]
    assert abs(width - wide * 0.8) <= wide * 0.02, swayimg
    assert abs(left + width / 2 - wide / 2) <= wide * 0.02, swayimg
    # The steps before leave notices, which go on their own.
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    desktop.photograph("swayimg")
    machine.send_key("e")
    machine.wait_until_succeeds(
        f"pgrep -u {desktop.account} -f '[g]imp.*{looked}'", timeout=STARTING
    )
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -x swayimg", timeout=timedelta(seconds=10)
    )
    # As in the screenshot's subtest: GIMP goes before it draws.
    machine.succeed(f"pkill -KILL -u {desktop.account} -f '[g]imp'")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[g]imp'", timeout=timedelta(seconds=30)
    )
    machine.succeed(f"rm {looked}")


def test_a_document_opens_in_zathura_beside_the_rest_and_q_closes_it(desktop: Desktop) -> None:
    """A document opens in zathura, beside the rest, and q closes it."""
    machine = desktop.machine
    desktop.opens_in(desktop.values["page"], "application/pdf", "org.pwmt.zathura")
    windows = json.loads(desktop.as_account("niri msg --json windows"))
    zathura = next(window for window in windows if window["app_id"] == "org.pwmt.zathura")
    assert not zathura["is_floating"], zathura
    desktop.photograph("zathura")
    machine.send_key("q")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[z]athura'", timeout=timedelta(seconds=10)
    )


def test_the_launchers_sound_moves_sound_to_another_output_and_back(desktop: Desktop) -> None:
    """The launcher's Sound moves sound to another output, and back."""
    machine = desktop.machine
    for output in ["stand-in-headphones", "stand-in-speakers"]:
        desktop.as_account("qs ipc call launcher collection sound")
        desktop.launcher_ready()
        desktop.photograph(f"launcher-sound-{output}")
        kind = output.removeprefix("stand-in-")
        desktop.type_in(kind, selecting=kind)
        machine.send_key("ret")
        used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
        machine.wait_until_succeeds(desktop.in_session(used), timeout=timedelta(seconds=10))
        # It changed something, so the launcher stays until it's closed.
        machine.send_key("esc")
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


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
    # The speakers, in use, hold Return's own Mute, then their Level,
    # Volume, whether they're Muted, and Channels, named first; the null
    # sink has no card, so nothing more.
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(lambda shown: shown["rows"][:4] == ["Mute", "Level", "Volume", "Muted"])
    desktop.launcher_until(lambda shown: "Channels" in shown["rows"])
    desktop.launcher_until(sound_reads("Channels", "stereo"))
    desktop.launcher_until(sound_reads("Muted", "no"))
    # Nothing plays, so the Level reads silent, and no program is listed.
    desktop.launcher_until(sound_reads("Level", "silent"))
    assert "Playing" not in desktop.showing()["rows"]
    desktop.photograph("launcher-sound-output-parts")
    # Return on Volume, past Mute and the Level, steps into its choices,
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # The part says what it was set to.
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(sound_reads("Volume", "50%"))
    # The right arrow on Muted holds Mute and Unmute, Unmute in use, and
    # Return on Mute mutes the speakers; then Unmute, its Return, unmutes.
    # Muted is the fourth row, after Return's own Mute, Level and Volume.
    for _down in range(3):
        machine.send_key("down")
    desktop.launcher_until(sound_selected("Muted"))
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Mute", "Unmute"])
    desktop.launcher_until(sound_reads("Unmute", "in use"))
    desktop.photograph("launcher-sound-muted")
    desktop.launcher_until(sound_selected("Mute"))
    machine.send_key("ret")
    hushed_sink = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED"
    machine.wait_until_succeeds(desktop.in_session(hushed_sink), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    sound_device(desktop, "speakers", "speakers")
    desktop.launcher_until(sound_reads("Muted", "yes"))
    desktop.type_in("unmute", selecting="Unmute")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(hushed_sink), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

    # The microphone holds its own after Return's Mute: a live Level, its
    # Gain, whether it's Muted, and who's Recording, which is nobody,
    # though the Level's own stream is reading it while it shows.
    sound_device(desktop, "microphone", "stand-in microphone")
    desktop.launcher_until(
        lambda shown: shown["rows"][:5] == ["Mute", "Level", "Gain", "Muted", "Recording"],
    )
    desktop.launcher_until(sound_reads("Level", "%"))
    desktop.launcher_until(sound_reads("Muted", "no"))
    desktop.launcher_until(sound_reads("Recording", "nothing"))
    desktop.photograph("launcher-sound-input-parts")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    # The right arrow on Recording, then on the program, and Return mutes
    # what it records. Return's own Mute leads, so Recording is counted to.
    for _down in range(desktop.showing()["rows"].index("Recording")):
        machine.send_key("down")
    desktop.launcher_until(sound_selected("Recording"))
    desktop.step_in()
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Mute", "Louder", "Quieter"])
    desktop.photograph("launcher-sound-recording")
    desktop.launcher_until(sound_selected("Mute"))
    machine.send_key("ret")
    hushed = f"wpctl get-volume {recording} | grep -q MUTED"
    machine.wait_until_succeeds(desktop.in_session(hushed), timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
            machine.send_key("left")
            desktop.launcher_until(lambda shown: shown["typed"] == "")
        else:
            desktop.launcher_until(lambda shown: becomes in shown["rows"])
            machine.send_key("esc")
            machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=WAIT)

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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=WAIT)

    # Sound's Louder and Quieter step the stream by a twentieth, keeping
    # the launcher, and its volume follows. The stream is only ever set
    # from the launcher here: set from elsewhere, as by wpctl, Quickshell
    # now and then reads the stream's volume back from before the change
    # and keeps it until the next one, which is PipeWire's and Quickshell's
    # to mend, not the launcher's.
    controls("Sound")
    desktop.photograph("launcher-program-sound")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Mute", "Louder", "Quieter", "Volume"])
    desktop.launcher_until(reads("Volume", "100%"))
    press("Quieter", then="now 95%")
    heard("0.95")
    desktop.launcher_until(reads("Volume", "95%"))
    press("Louder", then="now 100%")
    heard("1.00")
    desktop.launcher_until(reads("Volume", "100%"))
    # Mute keeps the launcher, and the stream is muted; it says Unmute at
    # once, and the program's Sound says it's muted.
    press("Mute", becomes="Unmute")
    heard("1.00 [MUTED]")
    controls()
    desktop.launcher_until(reads("Sound", "muted"))
    desktop.type_in("sound", selecting="Sound")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][0] == "Unmute")
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=WAIT)
    desktop.as_account("systemctl --user stop tones")
    machine.succeed(f"rm -r {tones}")


# kitty's Recent holds the folders zoxide has learned, read again as
# zoxide's database changes, even one it made after the shell started.
# Return on one opens a kitty window there through the kitty already open,
# over its socket, rather than starting another.


def test_kittys_recent_opens_a_window_in_a_folder_it_visited(desktop: Desktop) -> None:
    """Recent in kitty lists a folder zoxide learned, and Return opens a window there."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/src/tide"
    desktop.as_account(f"mkdir -p {folder} && zoxide add {folder}")
    desktop.as_account("niri msg action spawn -- kitty --title recent-from")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q recent-from"), timeout=STARTING
    )

    def kittys() -> list[int]:
        """Name the process behind each of kitty's windows, one a window."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        return [window["pid"] for window in windows if window["app_id"] == "kitty"]

    before = kittys()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.step_in()
    desktop.type_in("recent", selecting="Recent")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "tide" in shown["rows"])
    desktop.photograph("launcher-kitty-recent")
    desktop.type_in("tide", selecting="tide")
    machine.send_key("ret")
    there = (
        f"for p in $(pgrep -u {desktop.account}); do"
        f' [ "$(readlink /proc/$p/cwd)" = {folder} ] && exit 0; done; exit 1'
    )
    machine.wait_until_succeeds(desktop.in_session(there), timeout=STARTING)
    retry(lambda _: len(kittys()) > len(before), timeout=timedelta(seconds=30))
    assert set(kittys()) == set(before), "it started another kitty"
    desktop.as_account("pkill -f 'kitty --title recent-from'")
    machine.wait_until_fails(desktop.in_session(there), timeout=timedelta(seconds=30))


def test_the_launcher_sets_the_power_profile(desktop: Desktop) -> None:
    """The launcher sets the power profile, staying to say so, or closing on Ctrl+Return."""
    machine = desktop.machine
    # By its name, power saver last, where what follows starts from. Setting
    # it changes something, so Return keeps the launcher, the row saying
    # it's in use, and Ctrl+Return sets it and closes.
    for typed, profile, name, key in [
        ("balanced power", "balanced", "Balanced Power", "ret"),
        ("power saver", "power-saver", "Power Saver", "ctrl-ret"),
    ]:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in(typed, selecting=name)
        desktop.launcher_until(
            lambda shown: "Ctrl+\u21b5 close" in (shown["selected"] or {}).get("told", [])
        )
        machine.send_key(key)
        machine.wait_until_succeeds(
            f"powerprofilesctl get | grep -qx {profile}", timeout=timedelta(seconds=10)
        )
        if key == "ret":
            desktop.launcher_until(
                lambda shown: shown["open"] and (shown["selected"] or {}).get("side") == "in use"
            )
            machine.send_key("esc")
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # And as the setting it is: one part saying which is in use, its
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
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "in use")
        machine.send_key("esc")
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


# The right arrow on one of a collection's members offers its own
# actions, as on anything at the top.


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
        machine.send_key("esc")
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))

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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))


# One walk through joining networks, as its subtest was: each step starts where the last left
# it.
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    # Found from the root, the right arrow steps into every network with
    # the Office still selected among them (Joel, 2026-10-26: stepping in
    # keeps the match), and the left arrow puts back what was typed.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("office", selecting="Office")
    told = desktop.showing()["selected"]["told"]
    assert "\u2192" in told, told
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) == 1
            and shown["typed"] == ""
            and len(shown["rows"]) > 1
            and (shown["selected"] or {}).get("name") == "Office"
        )
    )
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "office")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    # holding its own parts, its hardware address among them.
    desktop.type_in("access", selecting="Access Points")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["2.4 GHz, Channel 1"])
    desktop.photograph("launcher-wifi-points")
    desktop.step_in("ret")
    desktop.type_in("hardware", selecting="Hardware Address")
    assert desktop.showing()["selected"]["side"] == "02:00:00:00:02:00"
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.succeed("rfkill unblock wifi")


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
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == addresses and shown["typed"] == "")
    desktop.type_in("vpn", selecting="VPN")
    desktop.step_in()
    desktop.type_in("address", selecting="Address")
    assert desktop.showing()["selected"]["side"] == "100.100.1.2/32"
    assert "Hardware Address" not in desktop.showing()["rows"], desktop.showing()
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
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

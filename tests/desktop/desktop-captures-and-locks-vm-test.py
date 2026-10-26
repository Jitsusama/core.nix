"""The desktop test's captures and locks area.

Here the desktop captures and locks: screenshots, recordings, the camera, the lock screen,
polkit's prompt and the askpass. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks
for the desktop, started once a run (desktop_session.py), and leaves it as it found it:
unlocked, the shell running, nothing of its own still open, so any test runs alone.
"""

from __future__ import annotations

import json
import re
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, NOTICES, PICTURE_COLOURS, SIGNING, STARTING, retry

if TYPE_CHECKING:
    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# The camera's window, which the viewfinder is.
CAMERA = 'niri msg --json windows | grep -qF \'"app_id":"camera"\''
# Where the shell keeps the notices it holds.
KEPT = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
# Where lock-screen and the shell say whether the screens are locked.
LOCKED = "$XDG_RUNTIME_DIR/quickshell-locked"
# How long a camera may take to wake, or a recording to start, stop or be saved.
WAKING = timedelta(seconds=60)


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def test_print_opens_capture_where_return_takes_the_whole_screen(desktop: Desktop) -> None:
    """Print opens Capture, where Return takes the whole screen."""
    desktop.capture()
    screen = desktop.showing()["selected"]
    assert screen["name"] == "The whole screen"
    assert screen["chord"] == "\u21e7 Print", "the whole screen's row shows its chord"
    desktop.photograph("capture")
    with desktop.told("shows card Screenshot captured"), desktop.captured():
        desktop.machine.send_key("ret")


def test_capture_takes_a_window_found_by_its_title_on_any_workspace(desktop: Desktop) -> None:
    """Capture takes a window found by its title, on any workspace."""
    desktop.opened("Seashells")
    # The window the keys were in, though niri says none has focus
    # while the launcher has the keys.
    desktop.capture()
    desktop.type_in("this window", selecting="This window \u00b7 Seashells")
    assert desktop.showing()["selected"]["chord"] == "Alt Print"
    with desktop.captured():
        desktop.machine.send_key("ret")
    seashells = desktop.window_titled("Seashells")
    assert desktop.newest_size() == tuple(seashells["layout"]["window_size"])
    # Moved to a workspace of its own and left there.
    desktop.as_account("niri msg action move-window-to-workspace-down")
    desktop.as_account("niri msg action focus-workspace-up")
    desktop.capture()
    desktop.type_in("seashells", selecting="Seashells")
    with desktop.captured():
        desktop.machine.send_key("ret")
    seashells = desktop.window_titled("Seashells")
    assert desktop.newest_size() == tuple(seashells["layout"]["window_size"])
    desktop.close(seashells)


def test_capture_records_the_window_and_says_why_when_niri_cant_share_it(
    desktop: Desktop,
) -> None:
    """Capture records the window, and says why when niri can't share it."""
    pool = desktop.opened("Tidepool")
    desktop.capture()
    desktop.type_in("record this window", selecting="Record this window \u00b7 Tidepool")
    assert desktop.showing()["selected"]["chord"] == "Ctrl Alt Print"
    # niri here offers no screencast on the session bus, so the
    # recording can't start, and the notice says why rather than it
    # failing unseen.
    with desktop.told("shows card The window could not be recorded"):
        desktop.machine.send_key("ret")
    print(
        desktop.machine.wait_until_succeeds(
            desktop.in_session(KEPT + ' | grep -o \'"summary":"The window could not[^}]*\''),
            timeout=timedelta(seconds=10),
        )
    )
    desktop.close(pool)


def test_a_region_is_picked_by_the_keys_and_escape_goes_back_to_capture(
    desktop: Desktop,
) -> None:
    """A region is picked by the keys, and Escape goes back to Capture."""
    machine = desktop.machine
    # The picker covers the screen niri gives it, whatever cage leaves.
    logical = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]
    wide, high = logical["width"], logical["height"]
    # It opens on the middle quarter; an arrow moves it a step, and
    # Shift and an arrow size it from its bottom right corner.
    width, height = round(wide / 2), round(high / 2)
    x, y = round((wide - width) / 2), round((high - height) / 2)
    pictures = desktop.pictures
    pictures_before = int(machine.succeed(f"ls {pictures}/*.png | wc -l"))

    def picker() -> None:
        """Open Capture's region picker, ready for the keys."""
        desktop.capture()
        desktop.type_in("region", selecting="A region")
        with desktop.told("keys picker"):
            machine.send_key("ret")

    def pictures_taken(count: int) -> None:
        """Wait until there are so many more pictures than there were."""
        machine.wait_until_succeeds(
            f"ls {pictures}/*.png | wc -l | grep -qx {pictures_before + count}",
            timeout=timedelta(seconds=15),
        )

    # Return at once takes the box it opened on, which has to be there
    # from its first frame, though the window learns its size later.
    picker()
    with desktop.told(f"picked {x},{y} {width}x{height}"):
        machine.send_key("ret")
    pictures_taken(1)
    assert desktop.newest_size() == (width, height)

    picker()
    for key in ("left", "up", "shift-right", "shift-right"):
        machine.send_key(key)
    # The pictures before leave their notices, which go on their own.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.photograph("picker")
    with desktop.told(f"picked {x - 16},{y - 16} {width + 32}x{height}"):
        machine.send_key("ret")
    pictures_taken(2)
    assert desktop.newest_size() == (width + 32, height)

    # One cell's letters and another's make the box between them.
    picker()
    with desktop.told("picker typed b"):
        machine.send_chars("aab")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.photograph("picker-cells")
    rows = round(16 * high / wide)
    with desktop.told(f"picked 0,0 {round(2 * wide / 16)}x{round(2 * high / rows)}"):
        machine.send_chars("b\n")

    # Escape clears the letters typed first, then goes back to Capture,
    # taking nothing.
    picker()
    machine.send_chars("c")
    machine.send_key("esc")
    with desktop.told("gone picker"):
        machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["keys"] and shown["held"] == ["captures"])
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


class Camera:
    """The machine's stand-in camera, its viewfinder, and what it keeps."""

    def __init__(self, desktop: Desktop) -> None:
        """Use the camera on that desktop."""
        self.desktop = desktop
        self.machine = desktop.machine
        self.ffmpeg = f"{desktop.values['ffmpeg_headless']}/bin"
        self.pictures = f"/home/{desktop.account}/Pictures/Camera"
        self.videos = f"/home/{desktop.account}/Videos/Camera"

    def viewfinder(self, opened_for: str) -> None:
        """Open the camera from Capture's row for it."""
        self.desktop.capture()
        self.desktop.type_in(opened_for, selecting=f"A {opened_for}")
        self.machine.send_key("ret")
        self.machine.wait_until_succeeds(self.desktop.in_session(CAMERA), timeout=WAKING)

    def gone(self) -> None:
        """Wait until the viewfinder has gone."""
        self.machine.wait_until_fails(
            self.desktop.in_session(CAMERA), timeout=timedelta(seconds=30)
        )

    def probe(self, entries: str, files: str) -> str:
        """Give what ffprobe says of each file there, which it reads one at a time."""
        ffprobe = f"{self.ffmpeg}/ffprobe -v error -of csv=p=0"
        each = f'for f in {files}; do {ffprobe} -show_entries {entries} "$f" || exit; done'
        return self.machine.succeed(each)

    def recording(self, count: int, last: bool) -> bool:  # As retry calls it.
        """Say whether there are so many videos, the newest grown, so recording."""
        kept = self.desktop.in_session(
            "find ~/.local/state/quickshell -name notices.json -exec cat {} + 2>/dev/null; true"
        )
        failed = re.search(
            r'"summary":"The video could not be recorded",[^}]*', self.machine.succeed(kept)
        )
        if failed is not None:
            # What the recording said, and who had the camera then.
            print(self.machine.execute("cat /run/user/1000/camera-recording.log")[1])
            holders = (
                "for p in /proc/[0-9]*; do"
                " ls -l $p/fd 2>/dev/null | grep -q video9 && echo open $p $(cat $p/comm);"
                " grep -qs video9 $p/maps && echo mapped $p $(cat $p/comm); done; true"
            )
            print(self.machine.succeed(holders))
            raise AssertionError(failed.group(0))
        grown = (
            f"[ $(ls {self.videos}/*.mp4 | wc -l) = {count} ]"
            f' && [ $(stat -c %s "$(ls -t {self.videos}/*.mp4 | head -1)") -gt 50000 ]'
        )
        status, _ = self.machine.execute(grown)
        if last and status != 0:
            self.said()
        assert not last or status == 0, f"video {count} never grew"
        return status == 0

    def saved(self, key: str) -> None:
        """Press a key that stops the recording, and wait for the notice that it's saved.

        A run has gone a minute without it, so what the recording was doing then is printed.
        """
        try:
            with self.desktop.told("shows card Video saved", timeout=WAKING):
                self.machine.send_key(key)
        except AssertionError:
            self.said()
            raise

    def said(self) -> None:
        """Print what the recording and its programs were doing, for a wait that ran out."""
        print(self.machine.execute("ps -eww -o pid,stat,wchan:32,args | grep -aE 'ffmpeg|mpv'")[1])
        print(self.machine.execute(f"ls -la /run/user/1000 {self.videos}")[1])
        print(self.machine.execute("cat /run/user/1000/camera-recording.log")[1])
        print(self.desktop.as_account("pw-top -b -n 2 || true"))
        print(self.desktop.as_account("journalctl --user --no-pager -n 60 -o cat"))

    def likeness(self, picture: str, turn: str) -> float:
        """Give how alike a picture kept and the camera's own are, the camera's turned so."""
        compared = f"[1]{turn}format=yuv420p[given];[0]format=yuv420p[kept];[kept][given]psnr"
        said = self.machine.succeed(
            f'{self.ffmpeg}/ffmpeg -i "{picture}" -i /tmp/given.png -lavfi "{compared}"'
            " -f null - 2>&1"
        )
        found = re.search(r"average:([0-9.]+)", said)
        assert found is not None, said
        return float(found.group(1))


def test_the_camera_takes_a_picture_and_a_video_by_the_keys(desktop: Desktop) -> None:
    """The camera takes a picture and a video by the keys, and keeps a picture unmirrored."""
    camera = Camera(desktop)
    machine = desktop.machine
    camera.viewfinder("picture from the camera")
    # Return before the camera wakes takes the picture once it has, and then the viewfinder
    # goes, as a screenshot's would.
    with desktop.told("shows card Picture copied", timeout=WAKING):
        machine.send_key("ret")
    camera.gone()
    sizes = camera.probe("stream=width,height", f"{camera.pictures}/*.jpg")
    assert sizes.split() == ["640,480"]

    # Tab makes Return record, and Return again stops it, the viewfinder going once the video
    # is saved.
    camera.viewfinder("picture from the camera")
    machine.send_key("tab")
    machine.send_key("ret")
    retry(lambda last: camera.recording(1, last), timeout=WAKING)
    desktop.photograph("camera")
    camera.saved("ret")
    camera.gone()
    # Escape while recording saves the video, then closes. The arrows choose that way, so Right
    # after Left is still a video.
    camera.viewfinder("video from the camera")
    machine.send_key("left")
    machine.send_key("right")
    machine.send_key("ret")
    retry(lambda last: camera.recording(2, last), timeout=WAKING)
    camera.saved("esc")
    camera.gone()

    streams = camera.probe("stream=codec_name", f"{camera.videos}/*.mp4")
    assert streams.split() == ["h264", "aac"] * 2

    # Left is a picture, even in a viewfinder opened for a video.
    camera.viewfinder("video from the camera")
    machine.send_key("left")
    with desktop.told("shows card Picture copied", timeout=WAKING):
        machine.send_key("ret")
    camera.gone()

    # A picture is kept as others see Joel, not mirrored as the viewfinder
    # shows him: it matches what the camera gives better than its mirror.
    machine.succeed(
        f"{camera.ffmpeg}/ffmpeg -v error -f v4l2 -i /dev/video9 -frames:v 1 -y /tmp/given.png"
    )
    picture = machine.succeed(f"ls {camera.pictures}/*.jpg | head -1").strip()
    assert camera.likeness(picture, "") > camera.likeness(picture, "hflip,") + 3


def test_a_camera_video_shows_as_recording_and_captures_stop_recording_stops_it(
    desktop: Desktop,
) -> None:
    """A camera video shows as recording, and Capture's Stop recording stops it."""
    camera = Camera(desktop)
    machine = desktop.machine
    state = "/run/user/1000/camera-recording.json"
    videos = int(machine.succeed(f"ls {camera.videos}/*.mp4 2>/dev/null | wc -l"))

    def marked(rest: str) -> bool:
        return any(glyph["mark"] == "recording" for glyph in json.loads(rest))

    camera.viewfinder("video from the camera")
    since = desktop.mark()
    machine.send_key("ret")
    retry(lambda last: camera.recording(videos + 1, last), timeout=WAKING)
    machine.succeed(f"test -s {state}")
    # The margin wears recording's mark while it records, as for the screen.
    desktop.reported("sets margin ", marked, since)
    desktop.capture()
    desktop.type_in("stop", selecting="Stop recording")
    assert desktop.showing()["selected"]["side"] == "a video from the camera"
    since = desktop.mark()
    # It stops as Return would, and the viewfinder takes the camera back.
    with desktop.told("shows card Video saved", timeout=WAKING):
        machine.send_key("ret")
    machine.wait_until_fails(f"test -e {state}", timeout=timedelta(seconds=10))
    # And the mark goes once the recording's file does.
    desktop.reported("sets margin ", lambda rest: not marked(rest), since)
    machine.wait_until_succeeds(desktop.in_session(CAMERA), timeout=WAKING)
    machine.send_key("esc")
    camera.gone()
    # One killed outright leaves its file behind, and its process gone says it isn't recording.
    desktop.as_account(
        f"""echo '{{"pid":999999,"stop":"/tmp/stop","file":"/tmp/v.mp4"}}' > {state}"""
    )
    desktop.capture()
    desktop.launcher_until(lambda shown: "A region" in shown["rows"])
    assert "Stop recording" not in desktop.showing()["rows"], desktop.showing()
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"rm {state}")


def test_a_camera_is_a_thing_its_parts_a_step_in_and_return_its_viewfinder(
    desktop: Desktop,
) -> None:
    """A camera is a thing: its parts a step in, and Return its viewfinder."""
    machine = desktop.machine
    holders = "/run/camera-holders/camera"
    # Written whole, as a loopback's holders are, so the stand-in is the machine's own camera,
    # held by a program.
    machine.succeed(f"echo chrome > {holders}.1 && mv {holders}.1 {holders}")
    # The answer leads to the camera, rather than saying its parts again.
    desktop.answer("camera")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "in use by chrome"
    )
    # Return's own Copy, then the camera.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Copy", "Built-in camera"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.answer("built-in camera")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Built-in camera")
    assert desktop.showing()["selected"]["side"] == "in use by chrome"
    desktop.step_in()
    # Who has it first, then what it gives, from the loopback's own file in sysfs.
    # The rate is the loopback's own, whatever the stand-in's writer asks.
    parts = [thing["reads"] for thing in desktop.showing()["things"]]
    assert len(parts) == 2, parts  # noqa: PLR2004 # Who has it, and what it gives.
    assert parts[0] == "In use \u00b7 chrome"
    assert re.fullmatch("Streaming \u00b7 YUYV 640 \u00d7 480 at [0-9]+", parts[1]), parts
    desktop.photograph("launcher-camera")
    desktop.step_in()
    assert desktop.showing()["rows"] == ["chrome"]
    machine.send_key("left")
    desktop.launcher_until(lambda shown: len(shown["held"]) == 1)
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [])
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Built-in camera")
    machine.send_key("ret")
    machine.wait_until_succeeds(desktop.in_session(CAMERA), timeout=WAKING)
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(CAMERA), timeout=timedelta(seconds=30))
    machine.succeed(f"rm {holders}")
    # Its notice goes before anything after looks at the screen.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


def test_the_lock_screen_takes_only_the_right_password(desktop: Desktop) -> None:
    """The lock screen takes only the right password, and shows its picture meanwhile."""
    machine = desktop.machine
    with desktop.told("keys lock"):
        machine.send_key("meta_l-l")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")
    # PAM answers a wrong password after a delay, which the lock says.
    with desktop.told("refused lock"):
        machine.send_chars("wrong-password\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")

    # Its picture fades in, as the wallpaper's does.
    def pictured(last: bool) -> bool:  # As retry calls it.
        """Say whether the lock screen shows its picture rather than a flat colour."""
        _, colours, _ = desktop.colours_and_light("locked")
        assert not last or colours > PICTURE_COLOURS, "the lock screen shows a flat colour"
        return colours > PICTURE_COLOURS

    retry(pictured, timeout=timedelta(seconds=30))
    with desktop.told("released lock"):
        machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q false")


def test_lock_screen_returns_once_the_lock_is_drawn(desktop: Desktop) -> None:
    """What locks before the machine sleeps returns only once niri has drawn the lock.

    So nothing else shows on waking.
    """
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_the_lock_lets_go_without_its_password_when_released(desktop: Desktop) -> None:
    """The lock lets go without its password when released, and starts empty the next time.

    swayidle releases it when logind says to unlock, as after a resume from hibernation, where
    the disk's prompt has already asked for the password.
    """
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    # Half a password, left in the field.
    desktop.machine.send_chars("half-a-pass")
    with desktop.told("released lock"):
        desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    # Released again while unlocked, nothing happens.
    desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    # The next lock takes the password alone, so nothing was left behind.
    desktop.as_account("lock-screen")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_polkits_prompt_lets_the_account_start_a_system_service(desktop: Desktop) -> None:
    """Polkit's prompt lets the account start a system service."""
    machine = desktop.machine
    with desktop.told("keys polkit"):
        desktop.as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
    desktop.photograph("polkit")
    machine.send_chars(f"{desktop.values['secret']}\n")
    machine.wait_for_file("/run/polkit-allowed", timeout=timedelta(seconds=30))


def test_the_askpass_asks_for_a_yes_a_pin_and_a_touch(desktop: Desktop) -> None:
    """The askpass asks for a yes, a PIN and a touch, each answered as OpenSSH expects."""
    machine = desktop.machine
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.photograph("askpass")
    machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("gone askpass"):
        machine.send_key("esc")
    desktop.answered_with("confirm", 1)

    with desktop.told("keys askpass"):
        desktop.ask(
            "secret", f"Enter PIN for ED25519-SK key /home/{desktop.account}/.ssh/id_ed25519_sk: "
        )
    desktop.photograph("askpass-pin")
    machine.send_chars("2468\n")
    desktop.answered_with("secret", 0, "2468\n")

    # A touch takes no keys, and goes once ssh stops waiting on it.
    with desktop.told("drawn askpass"):
        desktop.ask("none", "Confirm user presence for key ED25519-SK SHA256:abc")
    with desktop.told("gone askpass"):
        desktop.as_account("pkill -TERM -f '^[^ ]*bash [^ ]*ssh-askpass Confirm'")
    desktop.answered_with("none", 1)
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")


def test_the_askpass_finds_the_shell_for_a_program_started_before_the_desktop(
    desktop: Desktop,
) -> None:
    """The askpass finds the shell for a program started before the desktop.

    linux-id's service starts with the account's systemd, before the desktop says which
    display it's on, and an askpass it runs knows no display.
    """
    with desktop.told("keys askpass"):
        desktop.as_account(
            "systemd-run --user --collect --quiet -- env -u WAYLAND_DISPLAY "
            f"{desktop.values['asking']} confirm '{SIGNING}'"
        )
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)


def test_the_askpass_asks_a_restarted_shell_again_and_gives_up_on_a_gone_one(
    desktop: Desktop,
) -> None:
    """The askpass asks a restarted shell again, and gives up on a gone one.

    The shell holds an ask only in memory, as a switch restarts it.
    """
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("keys askpass", timeout=timedelta(seconds=60)):
        desktop.as_account("systemctl --user restart quickshell")
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.as_account("systemctl --user stop quickshell")
    desktop.answered_with("confirm", 1, timeout=timedelta(seconds=60))
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")
    desktop.as_account("systemctl --user start quickshell")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
    )


def test_the_lock_holds_when_the_shell_restarts_and_only_for_its_niri(desktop: Desktop) -> None:
    """The lock holds when the shell restarts, and only for its niri.

    A switch restarts the shell, and can come while Joel is away and the screens are locked.
    niri holds a lock whose shell died, so the shell it starts again has to take that lock
    back, or nothing is left to type the password into.
    """
    holder = "$XDG_RUNTIME_DIR/quickshell-lock-holder"
    desktop.as_account("lock-screen")
    since = desktop.mark()
    desktop.as_account("systemctl --user restart quickshell")
    for word in ("secured lock", "keys lock"):
        desktop.reported(word, lambda rest: rest == "", since, timeout=timedelta(seconds=60))
    desktop.as_account("qs ipc call lock isLocked | grep -qx true")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")

    def started_unlocked() -> None:
        """Restart the shell, and check that it starts with the screens unlocked."""
        desktop.as_account("systemctl --user restart quickshell")
        desktop.machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
        )
        desktop.as_account("qs ipc call lock isLocked | grep -qx false")
        desktop.as_account(f"grep -qx unlocked {LOCKED}")

    # Let go, the lock isn't taken again.
    started_unlocked()
    # The runtime directory outlives a session while the account
    # lingers, so a lock a crashed niri held is left to that niri.
    desktop.as_account(f"echo $XDG_RUNTIME_DIR/niri.wayland-1.1.sock > {holder}")
    desktop.as_account(f"echo locked > {LOCKED}")
    started_unlocked()


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along.

    A binding that throws or assigns the wrong type only warns, and only once a surface draws
    it, which can be long after the start, so this comes last.
    """
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

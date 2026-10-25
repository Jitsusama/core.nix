"""The desktop test's captures and locks area.

Here the desktop captures and locks: screenshots, recordings, the camera, the lock screen,
polkit's prompt and the askpass. tests/desktop.nix runs it after desktop_prelude.py, the boot
and the helpers every area shares, so the names it takes from there are imported only for type
checkers.
"""

import json
import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from desktop_prelude import (
        ACCOUNT,
        ASKING,
        FFMPEG_HEADLESS,
        PICTURE_COLOURS,
        SECRET,
        SIGNING,
        STARTING,
        answered_with,
        as_account,
        ask,
        capture,
        captured,
        colours_and_light,
        in_session,
        launcher_until,
        mark,
        newest_size,
        photograph,
        pictures,
        reported,
        showing,
        told,
        type_in,
    )
    from driver_names import machine, retry, subtest, t

with subtest("Print opens Capture, where Return takes the whole screen"):
    capture()
    screen = showing()["selected"]
    t.assertEqual(screen["name"], "The whole screen")
    t.assertEqual(screen["chord"], "\u21e7 Print", "the whole screen's row shows its chord")
    photograph("capture")
    with told("shows card Screenshot captured"), captured():
        machine.send_key("ret")

with subtest("Capture takes a window found by its title, on any workspace"):
    as_account("niri msg action spawn -- kitty --title Seashells")
    machine.wait_until_succeeds(
        in_session('niri msg --json windows | grep -qF \'"title":"Seashells"\''),
        timeout=STARTING,
    )
    # The window the keys were in, though niri says none has focus
    # while the launcher has the keys.
    capture()
    type_in("this window", selecting="This window \u00b7 Seashells")
    t.assertEqual(showing()["selected"]["chord"], "Alt Print")
    with captured():
        machine.send_key("ret")
    windows = json.loads(as_account("niri msg --json windows"))
    seashells = next(w for w in windows if w["title"] == "Seashells")
    t.assertEqual(newest_size(), tuple(seashells["layout"]["window_size"]))
    # Moved to a workspace of its own and left there.
    as_account("niri msg action move-window-to-workspace-down")
    as_account("niri msg action focus-workspace-up")
    capture()
    type_in("seashells", selecting="Seashells")
    with captured():
        machine.send_key("ret")
    windows = json.loads(as_account("niri msg --json windows"))
    seashells = next(w for w in windows if w["title"] == "Seashells")
    t.assertEqual(newest_size(), tuple(seashells["layout"]["window_size"]))
    as_account(f"niri msg action close-window --id {seashells['id']}")

with subtest("Capture records the window, and says why when niri can't share it"):
    as_account("niri msg action spawn -- kitty --title Tidepool")
    machine.wait_until_succeeds(
        in_session('niri msg --json windows | grep -qF \'"title":"Tidepool"\''),
        timeout=STARTING,
    )
    capture()
    type_in("record this window", selecting="Record this window \u00b7 Tidepool")
    t.assertEqual(showing()["selected"]["chord"], "Ctrl Alt Print")
    # niri here offers no screencast on the session bus, so the
    # recording can't start, and the notice says why rather than it
    # failing unseen.
    with told("shows card The window could not be recorded"):
        machine.send_key("ret")
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    print(
        machine.wait_until_succeeds(
            in_session(kept + ' | grep -o \'"summary":"The window could not[^}]*\''),
            timeout=10,
        )
    )
    windows = json.loads(as_account("niri msg --json windows"))
    pool = next(w for w in windows if w["title"] == "Tidepool")
    as_account(f"niri msg action close-window --id {pool['id']}")

with subtest("A region is picked by the keys, and Escape goes back to Capture"):
    # The picker covers the screen niri gives it, whatever cage leaves.
    logical = json.loads(as_account("niri msg --json focused-output"))["logical"]
    wide, high = logical["width"], logical["height"]
    # It opens on the middle quarter; an arrow moves it a step, and
    # Shift and an arrow size it from its bottom right corner.
    width, height = round(wide / 2), round(high / 2)
    x, y = round((wide - width) / 2), round((high - height) / 2)
    pictures_before = int(machine.succeed(f"ls {pictures}/*.png | wc -l"))

    # Return at once takes the box it opened on, which has to be there
    # from its first frame, though the window learns its size later.
    capture()
    type_in("region", selecting="A region")
    with told("keys picker"):
        machine.send_key("ret")
    with told(f"picked {x},{y} {width}x{height}"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        f"ls {pictures}/*.png | wc -l | grep -qx {pictures_before + 1}", timeout=15
    )
    t.assertEqual(newest_size(), (width, height))

    capture()
    type_in("region", selecting="A region")
    with told("keys picker"):
        machine.send_key("ret")
    for key in ("left", "up", "shift-right", "shift-right"):
        machine.send_key(key)
    # The pictures before leave their notices, which go on their own.
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    photograph("picker")
    with told(f"picked {x - 16},{y - 16} {width + 32}x{height}"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        f"ls {pictures}/*.png | wc -l | grep -qx {pictures_before + 2}", timeout=15
    )
    t.assertEqual(newest_size(), (width + 32, height))

    # One cell's letters and another's make the box between them.
    capture()
    type_in("region", selecting="A region")
    with told("keys picker"):
        machine.send_key("ret")
    with told("picker typed b"):
        machine.send_chars("aab")
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    photograph("picker-cells")
    rows = round(16 * high / wide)
    with told(f"picked 0,0 {round(2 * wide / 16)}x{round(2 * high / rows)}"):
        machine.send_chars("b\n")

    # Escape clears the letters typed first, then goes back to Capture,
    # taking nothing.
    capture()
    type_in("region", selecting="A region")
    with told("keys picker"):
        machine.send_key("ret")
    machine.send_chars("c")
    machine.send_key("esc")
    with told("gone picker"):
        machine.send_key("esc")
    launcher_until(lambda shown: shown["keys"] and shown["held"] == ["captures"])
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])

with subtest("the camera takes a picture and a video by the keys"):
    camera = 'niri msg --json windows | grep -qF \'"app_id":"camera"\''
    capture()
    type_in("picture from the camera", selecting="camera")
    machine.send_key("ret")
    machine.wait_until_succeeds(in_session(camera), timeout=60)
    # Return before the camera wakes takes the picture once it has, and then the viewfinder
    # goes, as a screenshot's would.
    with told("shows card Picture copied", timeout=60):
        machine.send_key("ret")
    machine.wait_until_fails(in_session(camera), timeout=10)

    def viewfinder(opened_for: str) -> None:
        """Open the camera again from Capture's row for it."""
        capture()
        type_in(opened_for, selecting=f"A {opened_for}")
        machine.send_key("ret")
        machine.wait_until_succeeds(in_session(camera), timeout=60)

    def probe(entries: str, files: str) -> str:
        """Give what ffprobe says of each file there, which it reads one at a time."""
        ffprobe = f"{FFMPEG_HEADLESS}/bin/ffprobe -v error -of csv=p=0"
        each = f'for f in {files}; do {ffprobe} -show_entries {entries} "$f" || exit; done'
        return machine.succeed(each)

    sizes = probe("stream=width,height", f"/home/{ACCOUNT}/Pictures/Camera/*.jpg")
    t.assertEqual(sizes.split(), ["640,480"])

    videos = f"/home/{ACCOUNT}/Videos/Camera"
    # The notices kept, none until one that isn't transient arrives.
    kept = in_session(
        "find ~/.local/state/quickshell -name notices.json -exec cat {} + 2>/dev/null; true"
    )

    def recording(count: int, last: bool) -> bool:
        """Say whether there are so many videos, the newest grown, so recording."""
        failed = re.search(
            r'"summary":"The video could not be recorded",[^}]*', machine.succeed(kept)
        )
        if failed is not None:
            # What the recording said, and who had the camera then.
            print(machine.execute("cat /run/user/1000/camera-recording.log")[1])
            holders = (
                "for p in /proc/[0-9]*; do"
                " ls -l $p/fd 2>/dev/null | grep -q video9 && echo open $p $(cat $p/comm);"
                " grep -qs video9 $p/maps && echo mapped $p $(cat $p/comm); done; true"
            )
            print(machine.succeed(holders))
            raise AssertionError(failed.group(0))
        grown = (
            f"[ $(ls {videos}/*.mp4 | wc -l) = {count} ]"
            f' && [ $(stat -c %s "$(ls -t {videos}/*.mp4 | head -1)") -gt 50000 ]'
        )
        status, _ = machine.execute(grown)
        if last and status != 0:
            print(machine.execute("ps -eww -o pid,args | grep -aE 'ffmpeg|mpv|sh -c'")[1])
            print(as_account("pw-top -b -n 2 || true"))
            print(machine.execute(f"ls -la /run/user/1000 {videos}")[1])
            print(machine.execute("cat /run/user/1000/camera-recording.log")[1])
            print(as_account("journalctl --user --no-pager -n 60 -o cat"))
        assert not last or status == 0, f"video {count} never grew"
        return status == 0

    def saved(key: str) -> None:
        """Press a key that stops the recording, and wait for the notice that it's saved.

        A run has gone a minute without it, so what the recording was doing then is printed.
        """
        try:
            with told("shows card Video saved", timeout=60):
                machine.send_key(key)
        except AssertionError:
            print(
                machine.execute("ps -eww -o pid,stat,wchan:32,args | grep -aE '[f]fmpeg|[m]pv'")[1]
            )
            print(
                machine.execute(
                    "ls -la /run/user/1000/camera-*; cat /run/user/1000/camera-recording.log"
                )[1]
            )
            print(as_account("pw-top -b -n 2 || true"))
            print(as_account("journalctl --user --no-pager -n 60 -o cat"))
            raise

    # Tab makes Return record, and Return again stops it, the viewfinder going once the video
    # is saved.
    viewfinder("picture from the camera")
    machine.send_key("tab")
    machine.send_key("ret")
    retry(lambda last: recording(1, last), timeout_seconds=60)
    photograph("camera")
    saved("ret")
    machine.wait_until_fails(in_session(camera), timeout=30)
    # Escape while recording saves the video, then closes.
    viewfinder("video from the camera")
    machine.send_key("ret")
    retry(lambda last: recording(2, last), timeout_seconds=60)
    saved("esc")
    machine.wait_until_fails(in_session(camera), timeout=30)
    streams = probe("stream=codec_name", f"{videos}/*.mp4")
    t.assertEqual(streams.split(), ["h264", "aac"] * 2)

    # A picture is kept as others see Joel, not mirrored as the viewfinder
    # shows him: it matches what the camera gives better than its mirror.
    ffmpeg = f"{FFMPEG_HEADLESS}/bin/ffmpeg"
    machine.succeed(f"{ffmpeg} -v error -f v4l2 -i /dev/video9 -frames:v 1 -y /tmp/given.png")
    picture = machine.succeed(f"ls /home/{ACCOUNT}/Pictures/Camera/*.jpg | head -1").strip()

    def likeness(turn: str) -> float:
        """Give how alike the picture kept and the camera's own are, turned this way."""
        compared = f"[1]{turn}format=yuv420p[given];[0]format=yuv420p[kept];[kept][given]psnr"
        said = machine.succeed(
            f'{ffmpeg} -i "{picture}" -i /tmp/given.png -lavfi "{compared}" -f null - 2>&1'
        )
        found = re.search(r"average:([0-9.]+)", said)
        assert found is not None, said
        return float(found.group(1))

    t.assertGreater(likeness(""), likeness("hflip,") + 3)

with subtest("the lock screen takes only the right password"):
    with told("keys lock"):
        machine.send_key("meta_l-l")
    as_account("qs ipc call lock isLocked | grep -q true")
    # PAM answers a wrong password after a delay, which the lock says.
    with told("refused lock"):
        machine.send_chars("wrong-password\n")
    as_account("qs ipc call lock isLocked | grep -q true")

    # Its picture fades in, as the wallpaper's does.
    def pictured(last: bool) -> bool:
        """Say whether the lock screen shows its picture rather than a flat colour."""
        _, colours, _ = colours_and_light("locked")
        if last:
            t.assertGreater(
                colours, PICTURE_COLOURS, "the lock screen shows a flat colour, not its picture"
            )
        return colours > PICTURE_COLOURS

    retry(pictured, timeout_seconds=30)
    with told("released lock"):
        machine.send_chars(f"{SECRET}\n")
    as_account("qs ipc call lock isLocked | grep -q false")

# What locks before the machine sleeps returns only once niri has drawn
# the lock on every screen, so nothing else shows on waking.
with subtest("lock-screen returns once the lock is drawn"):
    marker = "$XDG_RUNTIME_DIR/quickshell-locked"
    as_account(f"grep -qx unlocked {marker}")
    as_account("lock-screen")
    as_account(f"grep -qx locked {marker}")
    with told("released lock"):
        machine.send_chars(f"{SECRET}\n")
    as_account(f"grep -qx unlocked {marker}")

with subtest("polkit's prompt lets the account start a system service"):
    with told("keys polkit"):
        as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
    photograph("polkit")
    machine.send_chars(f"{SECRET}\n")
    machine.wait_for_file("/run/polkit-allowed", timeout=30)

with subtest("the askpass asks for a yes, a PIN and a touch"):
    with told("keys askpass"):
        ask("confirm", SIGNING)
    photograph("askpass")
    machine.send_key("ret")
    answered_with("confirm", 0)

    with told("keys askpass"):
        ask("confirm", SIGNING)
    with told("gone askpass"):
        machine.send_key("esc")
    answered_with("confirm", 1)

    with told("keys askpass"):
        ask("secret", f"Enter PIN for ED25519-SK key /home/{ACCOUNT}/.ssh/id_ed25519_sk: ")
    photograph("askpass-pin")
    machine.send_chars("2468\n")
    answered_with("secret", 0, "2468\n")

    # A touch takes no keys, and goes once ssh stops waiting on it.
    with told("drawn askpass"):
        ask("none", "Confirm user presence for key ED25519-SK SHA256:abc")
    with told("gone askpass"):
        as_account("pkill -TERM -f '^[^ ]*bash [^ ]*ssh-askpass Confirm'")
    answered_with("none", 1)
    as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")

# linux-id's service starts with the account's systemd, before the desktop says
# which display it's on, and an askpass it runs knows no display.
with subtest("the askpass finds the shell for a program started before the desktop"):
    with told("keys askpass"):
        as_account(
            "systemd-run --user --collect --quiet -- env -u WAYLAND_DISPLAY "
            f"{ASKING} confirm '{SIGNING}'"
        )
    machine.send_key("ret")
    answered_with("confirm", 0)

# The shell holds an ask only in memory, as a switch restarts it.
with subtest("the askpass asks a restarted shell again, and gives up on a gone one"):
    with told("keys askpass"):
        ask("confirm", SIGNING)
    with told("keys askpass", timeout=60):
        as_account("systemctl --user restart quickshell")
    machine.send_key("ret")
    answered_with("confirm", 0)

    with told("keys askpass"):
        ask("confirm", SIGNING)
    as_account("systemctl --user stop quickshell")
    answered_with("confirm", 1, timeout=60)
    as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")
    as_account("systemctl --user start quickshell")
    machine.wait_until_succeeds(in_session("qs ipc show | grep -q askpass"), timeout=STARTING)

# A switch restarts the shell, and can come while Joel is away and the
# screens are locked. niri holds a lock whose shell died, so the shell
# it starts again has to take that lock back, or nothing is left to
# type the password into.
with subtest("the lock holds when the shell restarts, and only for its niri"):
    locked = "$XDG_RUNTIME_DIR/quickshell-locked"
    holder = "$XDG_RUNTIME_DIR/quickshell-lock-holder"
    as_account("lock-screen")
    since = mark()
    as_account("systemctl --user restart quickshell")
    for word in ("secured lock", "keys lock"):
        reported(word, lambda rest: rest == "", since, timeout=60)
    as_account("qs ipc call lock isLocked | grep -qx true")
    as_account(f"grep -qx locked {locked}")
    with told("released lock"):
        machine.send_chars(f"{SECRET}\n")
    as_account(f"grep -qx unlocked {locked}")

    def started_unlocked() -> None:
        """Restart the shell, and check that it starts with the screens unlocked."""
        as_account("systemctl --user restart quickshell")
        machine.wait_until_succeeds(in_session("qs ipc show | grep -q askpass"), timeout=STARTING)
        as_account("qs ipc call lock isLocked | grep -qx false")
        as_account(f"grep -qx unlocked {locked}")

    # Let go, the lock isn't taken again.
    started_unlocked()
    # The runtime directory outlives a session while the account
    # lingers, so a lock a crashed niri held is left to that niri.
    as_account(f"echo $XDG_RUNTIME_DIR/niri.wayland-1.1.sock > {holder}")
    as_account(f"echo locked > {locked}")
    started_unlocked()

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

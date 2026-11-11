"""The desktop test's captures area.

Here the desktop captures: screenshots of the screen, a window or a region, recordings, and
the camera's pictures and videos.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it: unlocked, the shell running, nothing of its
own still open, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import NOTICES, retry
from PIL import Image, ImageChops, ImageStat

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# The camera's window, which the viewfinder is.
CAMERA = 'niri msg --json windows | grep -qF \'"app_id":"camera"\''
# The keys that close the viewfinder, as they close Joel's other programs.
ZZ = ("shift-z", "shift-z")
# Where the shell keeps the notices it holds.
# How long a camera may take to wake, or a recording to start, stop or be saved.
WAKING = timedelta(seconds=60)
# How long a stopped recording may take to be saved. Over a still screen, wf-recorder went
# on until something moved, 58 s later on the net (2026-11-09), so the bound keeps well
# under that.
STOPPED = timedelta(seconds=15)
# How far apart, from 0 to 255, the screen with the launcher and without it are where the
# card is drawn, past what the clock and a cursor change; and the least of the screen the
# card covers for a picture to be judged by it.
CARD_APART = 32
CARD_LEAST = 0.02
# How far down the screen the margin's marks are drawn, and more, above the launcher's card.
MARGIN_HIGH = 48
# A part of a picture: its left, top, right and bottom.
type Box = tuple[int, int, int, int]
# Now's row for a recording, and how long it says it has run.
RECORDING = re.compile(r"Recording \u00b7 (\d+):(\d\d)")


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def test_print_opens_capture_where_return_takes_the_whole_screen(desktop: Desktop) -> None:
    """Print opens Capture, where Return takes the whole screen."""
    desktop.capture()
    screen = desktop.showing()["selected"]
    assert screen["name"] == "The Whole Screen"
    # Print, pressed here, takes it, so that's its chord here rather than Shift+Print.
    assert screen["chord"] == "Print", "the whole screen's row shows its chord"
    desktop.shoot("capture")
    with desktop.told("shows card Screenshot captured"), desktop.captured():
        desktop.machine.send_key("ret")


def test_print_over_the_launcher_takes_it_in_and_leaves_it_on_its_row(desktop: Desktop) -> None:
    """Print, pressed while the launcher shows a row, takes the screen with it and leaves it so.

    Joel pressed Print to capture what the launcher showed, and it stepped into Capture
    instead, so the key takes the screen as it is (docs/design.md, A Capture Key Takes the
    Screen as It Is). The picture is judged on the card's own pixels.
    """
    machine = desktop.machine
    bare, seen, shown = the_launcher_on_a_row(desktop)
    with desktop.captured():
        machine.send_key("sysrq")
    still_shown(desktop, shown)
    picture = the_newest_picture(desktop, "taken-over-the-launcher.png")
    the_card_in(picture, seen, bare)


def test_print_in_capture_closes_it_and_takes_what_is_beneath(desktop: Desktop) -> None:
    """Print, pressed again in Capture, closes the launcher and takes the screen beneath it.

    So Print twice always captures the work, whatever Print does over the launcher.
    """
    machine = desktop.machine
    quiet(desktop)
    bare = desktop.settled_screen("capture-beneath-closed")
    desktop.capture()
    seen = the_card_drawn(desktop, bare, "capture-beneath-open")
    with desktop.captured():
        machine.send_key("sysrq")
    desktop.launcher_until(lambda now: not now["open"])
    picture = the_newest_picture(desktop, "taken-beneath-capture.png")
    the_card_in(picture, bare, seen)


def test_ctrl_print_over_the_launcher_records_it_in(desktop: Desktop) -> None:
    """Ctrl+Print, pressed while the launcher shows a row, records it and leaves it so.

    The same chord stops it. Its first and last frames are judged on the card's pixels, as
    a picture is; a video's colours are near the screen's, not the same. Each holds the
    margin's recording mark as the screen does while it records, since a first frame
    without it would look as if the screen wasn't being recorded.
    """
    machine = desktop.machine
    mark = "/run/user/1000/before-recording"
    desktop.as_account(f"mkdir -p ~/Videos/Recordings && touch {mark}")
    bare, seen, shown = the_launcher_on_a_row(desktop)
    # The margin wears recording's mark from the start, as niri says it records.
    with desktop.told("holds margin recording", timeout=WAKING):
        machine.send_key("ctrl-sysrq")
    still_shown(desktop, shown)
    # The mark drawn is a redraw since niri began to cast, and niri gives a recording, which
    # asks for each frame with damage, a frame at each redraw, so the video holds one; and
    # its stop takes one last frame, so the video ends on what was shown then. Its file fills
    # only once the encoder has a buffer's worth, which over a still screen took up to a
    # minute (2026-11-09), so it isn't waited for.
    marked = the_mark_drawn(desktop, seen)
    videos = f"-newer {mark} -type f -name '*.mp4'"
    # Its notice comes once the file is finished, which its process outlives, waiting on
    # the notice's Play It. The screen is still as it stops, so this is the stop heard at
    # once rather than at something's next move, nor its frames dropped
    # (wf-recorder-stops-at-once.patch).
    with desktop.told("shows card Recording saved", timeout=STOPPED):
        machine.send_key("ctrl-sysrq")
    still_shown(desktop, shown)
    # Named by the time it began, so with a space in it.
    recorded = desktop.as_account(f"find ~/Videos/Recordings {videos}").splitlines()
    assert len(recorded) == 1, recorded
    video = shlex.quote(recorded[0])
    ffmpeg = f"{desktop.values['ffmpeg_headless']}/bin/ffmpeg -v error"
    for name, where in (("recorded-first.png", ""), ("recorded-last.png", "-sseof -1")):
        frame = f"/run/user/1000/{name}"
        machine.succeed(f"{ffmpeg} {where} -i {video} -frames:v 1 -update 1 -y {frame}")
        taken = kept(desktop, frame)
        box = the_card_in(taken, seen, bare)
        the_mark_in(taken, marked.crop(box), seen.crop(box))


def the_mark_drawn(desktop: Desktop, unmarked: Image.Image) -> Image.Image:
    """Give the screen once its top edge holds the margin's recording mark.

    The shell says it holds the mark before niri shows the frame, and a guest stalled for a
    moment holds still as well, so a screen taken once it's still can come before the mark
    (the net's run at e6b9c86c). It's taken again until the mark is in its pixels.
    """
    screens: list[Image.Image] = []

    def drawn(_: bool) -> bool:
        screens.append(desktop.settled_screen("recording-the-launcher"))
        return the_mark_between(screens[-1], unmarked).getbbox() is not None

    retry(drawn, timeout=WAKING)
    return screens[-1]


def the_mark_between(marked: Image.Image, unmarked: Image.Image) -> Image.Image:
    """Give where two screens differ along the top edge, above the card, as a mask."""
    top = (0, 0, marked.width, MARGIN_HIGH)
    differs = ImageChops.difference(marked.crop(top), unmarked.crop(top)).convert("L")
    return differs.point(lambda v: 255 if v > CARD_APART else 0)


def the_launcher_on_a_row(desktop: Desktop) -> tuple[Image.Image, Image.Image, Json]:
    """Open the launcher on its second row, giving the screen without it, with it, and it.

    The second, so a launcher that went back to where it opens would be seen to.
    """
    quiet(desktop)
    bare = desktop.settled_screen("capture-the-launcher-closed")
    desktop.machine.send_key("meta_l-spc")
    desktop.launcher_ready()
    desktop.down_to(desktop.showing()["rows"][1])
    shown = desktop.showing()
    return bare, the_card_drawn(desktop, bare, "capture-the-launcher-open"), shown


def the_card_drawn(desktop: Desktop, bare: Image.Image, name: str) -> Image.Image:
    """Give the screen once the launcher's card is in its pixels, against the screen without it.

    The launcher says it's open and holds the keys before niri shows its frame, and a guest
    stalled for a moment holds still as well, so a screen taken once it's still can come
    before the card: the net's run at 1e9c63e6 took the screen beneath it twice. It's taken
    again until the card covers enough of it to judge a picture by.
    """
    screens: list[Image.Image] = []

    def drawn(_: bool) -> bool:
        screens.append(desktop.settled_screen(name))
        return covered(screens[-1], bare) > CARD_LEAST

    retry(drawn, timeout=WAKING)
    return screens[-1]


def covered(one: Image.Image, other: Image.Image) -> float:
    """Give how much of the screen differs between two, as far as the card is drawn apart."""
    return ImageStat.Stat(card_between(one, other)).mean[0] / 255


def card_between(one: Image.Image, other: Image.Image) -> Image.Image:
    """Give where two screens differ as the card does, past the clock and a cursor, as a mask."""
    differs = ImageChops.difference(one, other).convert("L")
    return differs.point(lambda v: 255 if v > CARD_APART else 0)


def quiet(desktop: Desktop) -> None:
    """Wait until no notice shows, since one is on the screen but kept out of a picture."""
    desktop.machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


def still_shown(desktop: Desktop, shown: Json) -> None:
    """Wait until the launcher shows as it did, open and holding the keys on the same row."""
    desktop.launcher_until(
        lambda now: (
            now["open"]
            and now["keys"]
            and now["held"] == shown["held"]
            and (now["selected"] or {}).get("name") == (shown["selected"] or {}).get("name")
        )
    )


def the_newest_picture(desktop: Desktop, name: str) -> Image.Image:
    """Give the newest screenshot, kept in the gallery by this name."""
    newest = desktop.as_account(f"ls -t {desktop.pictures}/*.png | head -1").strip()
    desktop.machine.succeed(f"cp {shlex.quote(newest)} /run/user/1000/{name}")
    return kept(desktop, f"/run/user/1000/{name}")


def kept(desktop: Desktop, taken: str) -> Image.Image:
    """Copy what was taken out of the machine, for the gallery, and give it."""
    desktop.machine.copy_from_machine(taken)
    return Image.open(desktop.machine.out_dir / taken.rsplit("/", 1)[1]).convert("RGB")


def the_mark_in(frame: Image.Image, marked: Image.Image, unmarked: Image.Image) -> None:
    """Fail unless a recording's frame holds the margin's mark, as the screen recording does.

    The mark is where the screen while recording and before it differ along the top edge,
    above the card. The frame must be at least four times nearer the one than the other
    there.
    """
    top = (0, 0, frame.width, MARGIN_HIGH)
    frame, marked, unmarked = frame.crop(top), marked.crop(top), unmarked.crop(top)
    mark = the_mark_between(marked, unmarked)
    assert mark.getbbox() is not None, "the screen while recording holds no mark along the top"
    near, far = apart(frame, marked, mark), apart(frame, unmarked, mark)
    print(f"on the mark, at {mark.getbbox()}: {near:.1f} from marked, {far:.1f} from before")
    assert near * 4 < far, f"on the mark: {near:.1f} from marked, {far:.1f} from before"


def the_card_in(taken: Image.Image, like: Image.Image, unlike: Image.Image) -> Box:
    """Fail unless, where the launcher's card is drawn, what was taken is the one screen's.

    The card is where the screen with the launcher and without it differ, so it's found
    from the pixels alone. What was taken must be at least four times nearer the one than
    the other there; the rest of the screen is the same in both, and says nothing. Gives
    the box of the screens what was taken fills.
    """
    card = card_between(like, unlike)
    shown = covered(like, unlike)
    assert shown > CARD_LEAST, f"the card covers only {shown:.1%} of the screen, too little"
    box = where_taken(taken, like, card)
    like, unlike, card = like.crop(box), unlike.crop(box), card.crop(box)
    near, far = apart(taken, like, card), apart(taken, unlike, card)
    print(f"on the card, at {box[:2]}: {near:.1f} from the one meant, {far:.1f} from the other")
    assert near * 4 < far, f"on the card: {near:.1f} from the one meant, {far:.1f} from the other"
    return box


def where_taken(taken: Image.Image, screen: Image.Image, card: Image.Image) -> Box:
    """Give the box of QEMU's screen that niri's output, and so what was taken, fills.

    QEMU's screen is larger than the output niri draws (1280 by 800 to 1272 by 768), so a
    picture is laid on it where it best matches, judged off the card, where every screen
    meant is the same.
    """
    wide, high = taken.size
    off = ImageChops.invert(card)

    def box(x: int, y: int) -> Box:
        return (x, y, x + wide, y + high)

    places = [
        (x, y) for x in range(screen.width - wide + 1) for y in range(screen.height - high + 1)
    ]
    x, y = min(places, key=lambda at: apart(taken, screen.crop(box(*at)), off.crop(box(*at))))
    return box(x, y)


def apart(one: Image.Image, other: Image.Image, where: Image.Image) -> float:
    """Give how far apart two pictures of one size are where the mask is, from 0 to 255."""
    difference = ImageChops.difference(one, other).convert("L")
    return float(ImageStat.Stat(difference, where).mean[0])


def test_capture_holds_what_it_took_lately_a_picture_shown_in_the_drawer(desktop: Desktop) -> None:
    """Capture holds what it took lately, newest first, its drawer saying what it is.

    Its drawer says where and what it is, then when it was taken and its size from the folder's
    read, then its dimensions once file-preview has said them.
    """
    desktop.capture()
    with desktop.captured():
        desktop.machine.send_key("ret")
    newest = desktop.machine.succeed(f"ls -t {desktop.pictures}/*.png | head -1").strip()
    desktop.capture()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == newest.rsplit("/", 1)[1]
            and shown["selected"]["side"] in {"just now", "1 min ago"}
            and shown["drawer"]["out"]
            and shown["drawer"]["eyebrow"] == "in ~/Pictures/Screenshots \u00b7 picture"
            and shown["drawer"]["facts"] == ["Taken", "Size", "Dimensions"]
        )
    )
    desktop.shoot("capture-recent")
    desktop.close_launcher()


def test_capture_takes_a_window_found_by_its_title_on_any_workspace(desktop: Desktop) -> None:
    """Capture takes a window found by its title, on any workspace."""
    desktop.opened("Seashells")
    # The window the keys were in, though niri says none has focus
    # while the launcher has the keys.
    desktop.capture()
    desktop.type_in("this window", selecting="This Window \u00b7 Seashells")
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
    desktop.type_in("record this window", selecting="Record This Window \u00b7 Tidepool")
    assert desktop.showing()["selected"]["chord"] == "Ctrl Alt Print"
    # niri here offers no screencast on the session bus, so the
    # recording can't start, and the notice says why rather than it
    # failing unseen.
    with desktop.told("shows card The window could not be recorded"):
        desktop.machine.send_key("ret")
    print(
        desktop.machine.wait_until_succeeds(
            desktop.notice_kept('.summary | startswith("The window could not")'),
            timeout=timedelta(seconds=10),
        )
    )
    desktop.close(pool)


def open_picker(desktop: Desktop) -> None:
    """Open Capture's region picker, ready for the keys."""
    desktop.capture()
    desktop.type_in("region", selecting="A Region")
    with desktop.told("keys picker"):
        desktop.machine.send_key("ret")


def test_a_region_is_picked_by_a_vim_users_keys(desktop: Desktop) -> None:
    """A region is picked by a Vim user's keys: moves, sizes and cells, then Return."""
    machine = desktop.machine
    # The picker covers the screen niri gives it, whatever cage leaves.
    logical = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]
    wide, high = logical["width"], logical["height"]
    # It opens on the middle quarter; h j k l move it a step, and H J K L size it from its
    # bottom right corner, an arrow moving as its letter and Shift with one sizing.
    width, height = round(wide / 2), round(high / 2)
    x, y = round((wide - width) / 2), round((high - height) / 2)
    pictures = desktop.pictures
    # Run alone, nothing has been taken yet, so there may be no pictures to count.
    pictures_before = int(machine.succeed(f"(ls {pictures}/*.png 2>/dev/null || true) | wc -l"))

    def pictures_taken(count: int) -> None:
        """Wait until there are so many more pictures than there were."""
        machine.wait_until_succeeds(
            f"ls {pictures}/*.png | wc -l | grep -qx {pictures_before + count}",
            timeout=timedelta(seconds=15),
        )

    # Return at once takes the box it opened on, which has to be there
    # from its first frame, though the window learns its size later.
    open_picker(desktop)
    with desktop.told(f"picked {x},{y} {width}x{height}"):
        machine.send_key("ret")
    pictures_taken(1)
    assert desktop.newest_size() == (width, height)

    # A count before a move takes as many steps.
    open_picker(desktop)
    machine.send_chars("2h")
    machine.send_key("up")
    machine.send_chars("L")
    machine.send_key("shift-right")
    # The pictures before leave their notices, which go on their own.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.shoot("picker")
    with desktop.told(f"picked {x - 32},{y - 16} {width + 32}x{height}"):
        machine.send_key("ret")
    pictures_taken(2)
    assert desktop.newest_size() == (width + 32, height)

    # f and one cell's letters, then f and another's, make the box between them.
    open_picker(desktop)
    with desktop.told("picker typed f b"):
        machine.send_chars("faafb")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.shoot("picker-cells")
    rows = round(16 * high / wide)
    with desktop.told(f"picked 0,0 {round(2 * wide / 16)}x{round(2 * high / rows)}"):
        machine.send_chars("b\n")


def test_minus_goes_back_from_the_picker_to_capture_and_q_closes_it(
    desktop: Desktop,
) -> None:
    """- goes back from the region picker to Capture, Space ? lists its keys, :q closes."""
    machine = desktop.machine
    # Escape cancels a waiting f, and at rest does nothing, so - is what goes back to Capture,
    # taking nothing.
    open_picker(desktop)
    with desktop.told("picker typed f c"):
        machine.send_chars("fc")
    machine.send_key("esc")
    machine.send_key("esc")
    # Space ? lists its keys, and an Escape with nothing begun puts them away.
    with desktop.told("picker lists keys"):
        machine.send_chars(" ?")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.shoot("picker-keys")
    with desktop.told("picker hides keys"):
        machine.send_key("esc")
    with desktop.told("gone picker"):
        machine.send_chars("-")
    desktop.launcher_until(lambda shown: shown["keys"] and shown["held"] == ["captures"])
    desktop.close_launcher()
    # :q closes it, going nowhere.
    open_picker(desktop)
    with desktop.told("gone picker"):
        machine.send_chars(":q\n")
    assert not desktop.showing()["open"], "the picker's :q opened the launcher"


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
        status, failed = self.machine.execute(
            self.desktop.notice_kept('.summary == "The video could not be recorded"')
        )
        if status == 0:
            # What the recording said, and who had the camera then.
            print(self.machine.execute("cat /run/user/1000/camera-recording.log")[1])
            holders = (
                "for p in /proc/[0-9]*; do"
                " ls -l $p/fd 2>/dev/null | grep -q video9 && echo open $p $(cat $p/comm);"
                " grep -qs video9 $p/maps && echo mapped $p $(cat $p/comm); done; true"
            )
            print(self.machine.succeed(holders))
            raise AssertionError(failed)
        grown = (
            f"[ $(ls {self.videos}/*.mp4 | wc -l) = {count} ]"
            f' && [ $(stat -c %s "$(ls -t {self.videos}/*.mp4 | head -1)") -gt 50000 ]'
        )
        status, _ = self.machine.execute(grown)
        if last and status != 0:
            self.said()
        assert not last or status == 0, f"video {count} never grew"
        return status == 0

    def saved(self, *keys: str) -> None:
        """Press the keys that stop the recording, and wait for the notice that it's saved.

        A run has gone a minute without it, so what the recording was doing then is printed.
        """
        try:
            with self.desktop.told("shows card Video saved", timeout=WAKING):
                for key in keys:
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
    """The camera takes a photo and a video by the keys, and keeps a photo unmirrored."""
    camera = Camera(desktop)
    machine = desktop.machine
    camera.viewfinder("photo from the camera")
    # Return before the camera wakes takes the photo once it has, and then the viewfinder
    # goes, as a screenshot's would.
    with desktop.told("shows card Photo copied", timeout=WAKING):
        machine.send_key("ret")
    camera.gone()
    sizes = camera.probe("stream=width,height", f"{camera.pictures}/*.jpg")
    assert sizes.split() == ["640,480"]

    # Tab makes Return record, and Return again stops it, the viewfinder going once the video
    # is saved.
    camera.viewfinder("photo from the camera")
    machine.send_key("tab")
    machine.send_key("ret")
    retry(lambda last: camera.recording(1, last), timeout=WAKING)
    desktop.shoot("camera", moving="the stand-in camera's test pattern")
    camera.saved("ret")
    camera.gone()
    # ZZ while recording saves the video, then closes. The arrows choose that way, so Right
    # after Left is still a video.
    camera.viewfinder("video from the camera")
    machine.send_key("left")
    machine.send_key("right")
    machine.send_key("ret")
    retry(lambda last: camera.recording(2, last), timeout=WAKING)
    camera.saved(*ZZ)
    camera.gone()

    streams = camera.probe("stream=codec_name", f"{camera.videos}/*.mp4")
    assert streams.split() == ["h264", "aac"] * 2

    # Left is a photo, even in a viewfinder opened for a video.
    camera.viewfinder("video from the camera")
    machine.send_key("left")
    with desktop.told("shows card Photo copied", timeout=WAKING):
        machine.send_key("ret")
    camera.gone()

    # Escape and q close nothing, as in all of Joel's programs; ZZ closes, keeping nothing.
    kept = machine.succeed(f"ls {camera.pictures} {camera.videos}")
    camera.viewfinder("photo from the camera")
    machine.send_key("esc")
    machine.send_key("q")
    machine.succeed(desktop.in_session(CAMERA))
    for key in ZZ:
        machine.send_key(key)
    camera.gone()
    assert machine.succeed(f"ls {camera.pictures} {camera.videos}") == kept

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
    # None yet counts as none, as when this runs alone, before the camera
    # has made one.
    videos = int(machine.succeed(f"(ls {camera.videos}/*.mp4 2>/dev/null || true) | wc -l"))

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
    desktop.type_in("stop", selecting="Stop Recording")
    assert desktop.showing()["selected"]["side"] == "a video from the camera"
    since = desktop.mark()
    # From the find, Return lands on it, since stopping changes how things stand, and the
    # second stops it as Return would; the viewfinder takes the camera back.
    desktop.land()
    with desktop.told("shows card Video saved", timeout=WAKING):
        machine.send_key("ret")
    machine.wait_until_fails(f"test -e {state}", timeout=timedelta(seconds=10))
    # And the mark goes once the recording's file does.
    desktop.reported("sets margin ", lambda rest: not marked(rest), since)
    machine.wait_until_succeeds(desktop.in_session(CAMERA), timeout=WAKING)
    for key in ZZ:
        machine.send_key(key)
    camera.gone()
    # One killed outright leaves its file behind, and its process gone says it isn't recording.
    desktop.as_account(
        f"""echo '{{"pid":999999,"stop":"/tmp/stop","file":"/tmp/v.mp4"}}' > {state}"""
    )
    desktop.capture()
    desktop.launcher_until(lambda shown: "A Region" in shown["rows"])
    assert "Stop Recording" not in desktop.showing()["rows"], desktop.showing()
    desktop.close_launcher()
    desktop.as_account(f"rm {state}")


def test_a_recording_already_running_says_how_long_it_has_run(desktop: Desktop) -> None:
    """A recording first seen running says how long since it began, not since it was seen.

    The shell restarts with every switch, and a recording running through one once started
    again from nothing. The recording here is a process that has run since the desktop
    started, named as a camera video's, as the camera's viewfinder names its own. :set
    recording? says it records and :set norecording asks it to stop, as Stop Recording
    does; the process isn't a recorder's, so it's asked and never told.
    """
    state = "/run/user/1000/camera-recording.json"
    recorder = desktop.as_account("pgrep -o -x niri").strip()

    def ran() -> int:
        return int(desktop.as_account(f"ps -o etimes= -p {recorder}"))

    def length(shown: Json) -> int | None:
        # A row whose recording ended stays in place, gone, as Now keeps it.
        found = [RECORDING.match(thing["reads"]) for thing in shown["things"] if not thing["gone"]]
        each = [one for one in found if one is not None]
        return int(each[0][1]) * 60 + int(each[0][2]) if each else None

    before = ran()
    desktop.as_account(
        f"""echo '{{"pid":{recorder},"stop":"/tmp/stop","file":"/tmp/v.mp4"}}' > {state}"""
    )
    try:
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_until(lambda shown: length(shown) is not None)
        said = length(desktop.showing())
        after = ran()
        assert said is not None
        assert before - 1 <= said <= after + 1, (before, said, after)
        desktop.normal()
        run_line(desktop, "set recording?")
        desktop.launcher_until(
            lambda shown: shown["command"] is None and shown["commandSaid"] == "recording"
        )
        desktop.as_account("rm -f /tmp/stop")
        run_line(desktop, "set norecording")
        desktop.machine.wait_until_succeeds("test -e /tmp/stop", timeout=timedelta(seconds=10))
    finally:
        desktop.as_account(f"rm -f {state}")
    desktop.launcher_until(lambda shown: length(shown) is None)
    desktop.close_launcher()


def run_line(desktop: Desktop, line: str) -> None:
    """Type a line on the : line, wait until the shell shows it whole, then press Return.

    Typed with its Return in one burst, the Return was lost once just after niri lagged, the
    line left standing unrun (the net, core 5a9d3fa6), so it's pressed once the line is there.
    """
    desktop.machine.send_chars(f":{line}")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == line)
    desktop.machine.send_key("ret")


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
    # Ctrl+] goes in, as Return does: the camera, then the copy.
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Built-In Camera", "Copy"])
    desktop.close_launcher()
    desktop.answer("built-in camera")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Built-In Camera")
    assert desktop.showing()["selected"]["side"] == "in use by chrome"
    desktop.follow()
    # Return's own Open first, then who has it, then what it gives, from
    # the loopback's own file in sysfs. The rate is the loopback's own,
    # whatever the stand-in's writer asks.
    parts = [thing["reads"] for thing in desktop.showing()["things"]]
    assert len(parts) == 3, parts  # noqa: PLR2004 # Open, who has it, and what it gives.
    assert parts[:2] == ["Open", "In Use \u00b7 chrome"], parts
    assert re.fullmatch("Streaming \u00b7 YUYV 640 \u00d7 480 at [0-9]+", parts[2]), parts
    desktop.shoot("launcher-camera")
    desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "In Use")
    desktop.follow()
    # Copying what it says comes last, since nothing inside does anything.
    assert desktop.showing()["rows"] == ["chrome", "Copy"]
    desktop.step_out()
    desktop.launcher_until(lambda shown: len(shown["held"]) == 1)
    desktop.step_out()
    desktop.launcher_until(lambda shown: shown["held"] == [])
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Built-In Camera")
    machine.send_key("ret")
    machine.wait_until_succeeds(desktop.in_session(CAMERA), timeout=WAKING)
    for key in ZZ:
        machine.send_key(key)
    machine.wait_until_fails(desktop.in_session(CAMERA), timeout=timedelta(seconds=30))
    machine.succeed(f"rm {holders}")
    # Its notice goes before anything after looks at the screen.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))


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

"""The boot and the helpers every area of the desktop test shares.

tests/desktop.nix runs this, then an area's subtests (desktop-<area>-vm-test.py), as one
script, and its opening comment says what the test proves. run-subtests cuts that script into
subtests at each top-level `with subtest(`, so what's here runs whichever are chosen. Nothing
here is a subtest: a subtest goes in its area's file, and a helper two areas use comes here.
The Nix values the script needs reach it through one JSON file, which it reads once, below.
"""

import contextlib
import json
import re
import shlex
import tempfile
import time
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypedDict, cast

from PIL import Image, ImageStat

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from driver_names import machine, retry, t


class Values(TypedDict):
    """What tests/desktop.nix gives the script: the account, the theme and paths in the store."""

    # The account the desktop runs as, and its password.
    account: str
    secret: str
    # The programs tests/desktop.nix writes for the test to run inside the machine.
    asking: str
    ask_portal: str
    touchscreen: str
    tuple_window: str
    # A picture, a document, a text and a track to open, as a download would be.
    picture: str
    page: str
    text: str
    track: str
    # The packages whose programs the test runs by their paths.
    coreutils: str
    dbus: str
    ffmpeg_headless: str
    fontconfig_bin: str
    pipewire: str
    wev: str
    zenity: str
    # The theme's colours the test looks for, and its monospaced face.
    accent: str
    action: str
    sunken: str
    mono_family: str


VALUES: Values = json.loads(Path("@values@").read_text())
ACCOUNT = VALUES["account"]
SECRET = VALUES["secret"]
ASKING = VALUES["asking"]
ASK_PORTAL = VALUES["ask_portal"]
TOUCHSCREEN = VALUES["touchscreen"]
TUPLE_WINDOW = VALUES["tuple_window"]
PICTURE = VALUES["picture"]
PAGE = VALUES["page"]
TEXT = VALUES["text"]
TRACK = VALUES["track"]
COREUTILS = VALUES["coreutils"]
DBUS = VALUES["dbus"]
FFMPEG_HEADLESS = VALUES["ffmpeg_headless"]
FONTCONFIG_BIN = VALUES["fontconfig_bin"]
PIPEWIRE = VALUES["pipewire"]
WEV = VALUES["wev"]
ZENITY = VALUES["zenity"]
ACCENT = VALUES["accent"]
ACTION = VALUES["action"]
SUNKEN = VALUES["sunken"]
MONO_FAMILY = VALUES["mono_family"]

# A picture holds more colours than this, and a flat colour, or a screen not yet drawn, far
# fewer.
PICTURE_COLOURS = 1000
# The stand-in panel's brightness above which it has followed a brighter room up from the
# dim screen picked in a dim one.
FOLLOWED_THE_ROOM = 15
# How far a pixel can be from the action's colour, summed over red, green and blue, and
# still be its glyph's.
NEAR_THE_ACTION = 40
# How wide and how high one glyph is at most, in pixels.
ONE_GLYPH = 30
# How far in from the screen's right edge, in pixels, the window under the notices rests,
# so its edge is far from the corner's.
FROM_THE_CORNER = 100
# The longest a program may take to start and show its window. A wait
# ends when it does, so this is only a ceiling: on a host busy with
# other machines and builds, a cold kitty or GIMP took past 30
# seconds, and a wait on a program starting fails then, not the step.
STARTING = 120
# More rows than this name something across everything the launcher is asked for in its
# survey, so the rows were looked at rather than all empty.
ROWS_LOOKED_AT = 100
# The same for the Wi-Fi sign-in window, which starts GTK and WebKit cold.
SIGNING_IN = 180
# How far in, in pixels, from the screen's edges and what niri keeps clear along them, a
# window set in a corner rests at most.
FROM_THE_EDGE = 48
# The share of the screen's width a window set wide takes at least.
NEARLY_ALL = 0.9
# CAP_SYS_NICE's bit among a process's capabilities.
CAP_SYS_NICE = 23
# Bytes of sound: a recording past this many has started, and one that grows by more after
# a stall is still recording.
RECORDING = 100000

# A JSON object as niri or the shell gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]


def in_session(command: str) -> str:
    """Give a command as the account, inside its niri session, where qs finds the shell."""
    environment = (
        "export XDG_RUNTIME_DIR=/run/user/1000; "
        "export $(systemctl --user show-environment | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)='); "
    )
    return f"su - {ACCOUNT} -c " + shlex.quote(environment + command)


def as_account(command: str) -> str:
    """Run a command as the account, inside its session, and give what it printed."""
    return machine.succeed(in_session(command))


def shell_said(*words: str) -> str:
    """Give which of these words the shell's journal holds, one a line.

    It's searched in the machine: by the end of a run the whole journal is
    megabytes, and a busy host has hung the driver bringing it back.
    """
    wanted = " ".join(f"-e {shlex.quote(word)}" for word in words)
    return as_account(
        f"journalctl --user -u quickshell --no-pager -o cat | grep -o -F {wanted} | sort -u; true"
    )


def brightness() -> int:
    """Give the stand-in panel's brightness."""
    return int(machine.succeed("cat /sys/class/backlight/stand-in/brightness"))


# The kernel's stand-in battery has two supplies, mains and USB, and UPower
# counts the machine on battery only with both off.
def power(state: str) -> None:
    """Turn both of the stand-in's supplies on or off."""
    parameters = "/sys/module/test_power/parameters"
    for supply in ["ac", "usb"]:
        machine.succeed(f"echo {state} > {parameters}/{supply}_online")


def wait_for_surface(namespace: str) -> None:
    """Wait until niri lists one of the shell's surfaces by its namespace."""
    machine.wait_until_succeeds(in_session(f"niri msg layers | grep -q {namespace}"), timeout=30)


# The shell reports what it does to its journal (Report.qml), so the test
# waits on what happened, never on a guess at how long it takes. A point
# in the journal is marked before the step that should bring a report,
# and only what's said after it counts, so a report made before the wait
# begins still ends it.
def mark() -> str:
    """Mark the point the shell's journal has reached."""
    said = as_account("journalctl --user --unit=quickshell --lines=1 --show-cursor --output=cat")
    return said.rsplit("-- cursor: ", 1)[1].strip()


def reported(prefix: str, test: Callable[[str], object], since: str, timeout: int = 30) -> None:
    """Wait until something said after a point starts with the prefix and passes the test.

    The point is one mark gave, or "" for the boot. What the shell did say is printed when it
    never says what's wanted.
    """
    deadline = time.monotonic() + timeout
    start = since

    def never() -> str:
        point = f"--after-cursor={shlex.quote(start)}" if start else "--boot"
        print(as_account(f"journalctl --user --unit=quickshell --no-pager --output=cat {point}"))
        return f"the shell never reported {prefix!r} as wanted"

    while True:
        left = int(deadline - time.monotonic())
        assert left > 0, never()
        status, said = machine.execute(
            in_session(f"reports {shlex.quote(since)} {left} {shlex.quote(prefix)}"),
            timeout=left + 30,
        )
        assert status == 0, never()
        for line in said.splitlines():
            kind, what = line.split("\t", 1)
            if kind == "cursor":
                since = what
            elif what.startswith(prefix) and test(what[len(prefix) :]):
                return


@contextlib.contextmanager
def told(what: str, timeout: int = 30) -> Iterator[None]:
    """Wait, after what's done inside it, for the shell to say exactly this."""
    since = mark()
    yield
    reported(what, lambda rest: rest == "", since, timeout)


def held_still(timeout: int = 10, span: float = 0.5) -> bool:
    """Say whether the screen holds still for half a second before the time is up.

    Half a second is twice the theme's longest movement: niri composes a frame a moment after
    it's asked to, windows and surfaces move as they come, and a busy host can draw two frames
    in a row the same mid-motion.
    """
    deadline = time.monotonic() + timeout
    with tempfile.TemporaryDirectory() as folder:
        shot = Path(folder) / "shot.ppm"
        last: bytes | None = None
        since = time.monotonic()
        while time.monotonic() < deadline:
            machine.send_monitor_command(f"screendump {shot}")
            now = shot.read_bytes()
            if now != last:
                last, since = now, time.monotonic()
            elif time.monotonic() - since >= span:
                return True
    return False


def photograph(name: str, timeout: int = 10) -> None:
    """Photograph the screen once it holds still, for a person to look at.

    One that never holds still is kept as it is.
    """
    held_still(timeout)
    machine.screenshot(name)


def showing() -> Json:
    """Give what the launcher shows, from its showing call.

    That's whether it holds the keys, what's typed and held, and the row Return takes. Under
    load the launcher lags the keys sent to it, so each step waits on this.
    """
    return cast("Json", json.loads(as_account("qs ipc call launcher showing")))


def launcher_until(test: Callable[[Json], object], timeout: int = 30) -> None:
    """Wait until what the launcher shows passes the test.

    The launcher reports its state each time it changes, so a wait reads it once and then
    hears every change.
    """
    since = mark()
    if not test(showing()):
        reported("showing ", lambda said: test(json.loads(said)), since, timeout)


# The launcher's surface is always listed, so whether it's open is what it says of itself.
LAUNCHER_OPEN = """qs ipc call launcher showing | grep -q '"open":true'"""


def launcher_ready() -> None:
    """Wait until the launcher is open and holds the keys.

    niri hands a surface the keys a frame after it asks for them, and a letter typed in that
    frame goes to the window underneath.
    """
    launcher_until(lambda shown: shown["open"] and shown["keys"])


def type_in(text: str, selecting: str = "") -> None:
    """Type into the launcher, and wait until it holds every letter with Return's row settled.

    An answer still being looked up closes the launcher without copying. A row that comes from
    something found later, such as a window or a copy, can be named to wait for as well.
    """
    machine.send_chars(text)

    def holds(shown: Json) -> object:
        row = shown["selected"]
        name = row["name"].lower() if row else ""
        settled = row is None or row["settled"]
        return shown["typed"].endswith(text) and settled and selecting.lower() in name

    launcher_until(holds)


def answer(typed: str) -> None:
    """Open the launcher, and ask it what's typed."""
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in(typed)


def step_in(key: str = "right") -> None:
    """Press a key that steps into the selected row, and wait until the launcher holds it.

    That's the right arrow into a thing's verbs, or Return into what a verb asks.
    """
    before = len(showing()["held"])
    machine.send_key(key)
    launcher_until(lambda shown: len(shown["held"]) > before)


def window_titled(title: str) -> Json:
    """Give the window niri lists under this title."""
    windows: list[Json] = json.loads(as_account("niri msg --json windows"))
    return next(window for window in windows if window["title"] == title)


def focused_kitty() -> Json:
    """Give a kitty window, focused, opening one first if there's none."""
    windows: list[Json] = json.loads(as_account("niri msg --json windows"))
    kitties = [window for window in windows if window["app_id"] == "kitty"]
    if not kitties:
        as_account("niri msg action spawn -- kitty")
        machine.wait_until_succeeds(
            in_session('niri msg --json windows | grep -qF \'"app_id":"kitty"\''),
            timeout=STARTING,
        )
        windows = json.loads(as_account("niri msg --json windows"))
        kitties = [window for window in windows if window["app_id"] == "kitty"]
    as_account(f"niri msg action focus-window --id {kitties[0]['id']}")
    return kitties[0]


# The window the launcher's window subtests find by its title.
NOTES = "Notes for Tuesday"


def notes_window() -> Json:
    """Give the notes' window, focused, opening it first if it isn't open."""
    windows: list[Json] = json.loads(as_account("niri msg --json windows"))
    if not any(window["title"] == NOTES for window in windows):
        as_account(f"niri msg action spawn -- kitty --title '{NOTES}'")
        machine.wait_until_succeeds(
            in_session(f"niri msg windows | grep -q '{NOTES}'"), timeout=STARTING
        )
    notes = window_titled(NOTES)
    as_account(f"niri msg action focus-window --id {notes['id']}")
    return notes


def drawn_at(window: Json) -> tuple[int, int, int, int]:
    """Give where a floating window draws on the screen: left, top, width, height."""
    layout = window["layout"]
    left = int(layout["tile_pos_in_workspace_view"][0] + layout["window_offset_in_tile"][0])
    top = int(layout["tile_pos_in_workspace_view"][1] + layout["window_offset_in_tile"][1])
    width, height = layout["window_size"]
    return left, top, width, height


def settles_on(wanted: str, read: Callable[[], str], timeout: int = 15) -> str | None:
    """Give what a reading of the screen settles on.

    It's read until it gives the colour wanted, since niri moves a window to where it says it
    is over a few frames, and the last reading is given if it never does. A reading niri can't
    give yet, a window not yet floating, reads as nothing.
    """
    readings: list[str | None] = [None]

    def settled(_: bool) -> bool:
        try:
            readings.append(read())
        except KeyError, TypeError:
            readings.append(None)
        return readings[-1] == wanted

    with contextlib.suppress(Exception):
        retry(settled, timeout_seconds=timeout)
    return readings[-1]


def commonest_colour_in(title: str, picture: str, wanted: str) -> str | None:
    """Give the commonest colour inside a window, a little in from its edges, once it's wanted.

    niri says where a window is only once it floats, so it floats first. The first GTK window
    has taken over 30 seconds to open on a CI runner building the kernel beside the test, so
    this waits longer than the rest.
    """
    machine.wait_until_succeeds(
        in_session(f'niri msg --json windows | grep -qF \'"title":"{title}"\''), timeout=120
    )
    as_account(f"niri msg action move-window-to-floating --id {window_titled(title)['id']}")

    def commonest() -> str:
        photograph(picture)
        left, top, width, height = drawn_at(window_titled(title))
        image = Image.open(machine.out_dir / f"{picture}.png").convert("RGB")
        inside = image.crop((left + 20, top + 20, left + width - 20, top + height - 20))
        # Pillow gives no counts when there are more colours than asked
        # for, and there can't be more than there are pixels.
        counts = inside.getcolors(inside.width * inside.height)
        assert counts is not None
        _, colour = max(counts)
        # An RGB picture's colours are each a red, a green and a blue.
        return "#{:02x}{:02x}{:02x}".format(*cast("tuple[int, int, int]", colour))

    colour = settles_on(wanted, commonest)
    as_account(f"niri msg action close-window --id {window_titled(title)['id']}")
    return colour


machine.wait_for_unit("multi-user.target")
# niri builds its software renderer before it opens its socket, which on a
# busy host has taken past two minutes.
machine.wait_until_succeeds("ls /run/user/1000/niri.*.sock", timeout=STARTING)
as_account(
    "NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg action spawn --"
    " systemctl --user import-environment WAYLAND_DISPLAY NIRI_SOCKET"
)
machine.wait_until_succeeds(
    in_session("systemctl --user show-environment | grep -q WAYLAND_DISPLAY")
)
# niri sets these as a session, then starts it.
as_account("systemctl --user set-environment XDG_CURRENT_DESKTOP=niri XDG_SESSION_TYPE=wayland")
as_account("systemctl --user start compositor")
# Every subtest past the first few needs the shell, so it starts with
# the session rather than in a subtest, and any subtest can run alone.
as_account("systemctl --user start quickshell")
machine.wait_until_succeeds(in_session("qs ipc show | grep -q launcher"), timeout=STARTING)


def colours_and_light(
    picture: str, part: tuple[float, float, float, float] = (0, 0, 1, 1)
) -> tuple[Image.Image, int, float]:
    """Take a picture of the screen, and give how many colours a part holds and how bright it is.

    A flat colour holds one, and the light is from 0 to 255. The part is a box of fractions of
    the screen: left, top, right and bottom.
    """
    machine.screenshot(picture)
    image = Image.open(machine.out_dir / f"{picture}.png").convert("RGB")
    w, h = image.size
    box = image.crop((int(part[0] * w), int(part[1] * h), int(part[2] * w), int(part[3] * h)))
    counts = box.getcolors(box.width * box.height)
    assert counts is not None
    return image, len(counts), ImageStat.Stat(box.convert("L")).mean[0]


def meter_shows(namespace: str, kind: str, change: str) -> None:
    """Make a change the shell is sure to hear, and photograph the meter that answers it.

    Quickshell hears a node change only once it has bound the node's audio, a moment after
    WirePlumber names it the default, which the meter says, so one change is made after that
    and its meter answers it.
    """
    said = as_account(f"wpctl inspect @DEFAULT_AUDIO_{kind}@")
    found = re.search(r'node\.name = "([^"]+)"', said)
    assert found is not None, f"no default {kind.lower()} has a name"
    node = found.group(1)
    reported(f"hears {namespace} {node}", lambda rest: rest == "", "", timeout=60)
    with told(f"drawn {namespace}"):
        as_account(change)
    photograph(namespace)


# A session as pi keeps one: its quest's README, front matter first, and
# its presence while it runs; and its window, titled as pi titles its own:
# its address, its quest cut short, its folder, which a hyphen doesn't end.
PI_QUEST = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-TESTQA"
PI_PRESENCE = "~/.local/state/pi/agentic-harness.pi/mesh-workflow/presence"
PI_WINDOW = "\u03c0 - TESTQA A quest of\u2026 - my-tree"


def presence_says(said: Json) -> None:
    """Replace the session's presence with this, by rename, as pi does."""
    as_account(
        f"echo '{json.dumps(said)}' > {PI_PRESENCE}/.testqa"
        f" && mv {PI_PRESENCE}/.testqa {PI_PRESENCE}/testqa.json"
    )


def pi_session() -> None:
    """Make pi's session, working, with its window and another to be in, unless they're there.

    Each of pi's subtests starts from it, so any of them runs alone.
    """
    as_account(
        f"mkdir -p {PI_QUEST} {PI_PRESENCE} && printf '%s\\n' '---' 'title: no' '---'"
        " '# A quest of its own & whole'"
        f' > {PI_QUEST}/README.md && echo \'{{"session":"01a1face-test",'
        '"addresses":["TESTQA"],"busy":true,'
        '"owns":["QEST-20261022-TESTQA"],"worksOn":"Tidy the tests",'
        '"groups":["#desk"],'
        '"owner":{"startedAt":\'$(date +%s%3N)\'}}\''
        f" > {PI_PRESENCE}/testqa.json"
    )
    windows: list[Json] = json.loads(as_account("niri msg --json windows"))
    for title in [PI_WINDOW, "Elsewhere"]:
        if any(window["title"] == title for window in windows):
            continue
        as_account(f"niri msg action spawn -- kitty --title '{title}'")
        machine.wait_until_succeeds(
            in_session(f'niri msg --json windows | grep -qF \'"title":"{title}"\''),
            timeout=STARTING,
        )


def copied_then_closed(text: str, flags: str = "", served: str | None = None) -> None:
    """Copy from a program that then closes.

    wl-copy in the foreground serves the copy until it's stopped. It's stopped once the
    clipboard hands the copy back, not after a guess: a two-second guess once ended a copy the
    history never heard.
    """
    served = served or f"[ \"$(wl-paste --no-newline 2>/dev/null)\" = '{text}' ]"
    as_account(
        f"echo -n '{text}' | wl-copy --foreground {flags} > /dev/null 2>&1 & p=$!; "
        f"for i in $(seq 100); do {served} && break; sleep 0.1; done; kill $p"
    )


def pasted_the_picture(_: bool) -> bool:
    """Say whether the clipboard hands back the picture as it was copied."""
    pasted = in_session(f"wl-paste --type image/png | cmp -s - {PICTURE}")
    return machine.execute(pasted)[0] == 0


def opens_in(path: str, kind: str, app_id: str) -> None:
    """Open a file as a click on it would, and wait for the program meant for it to take focus.

    A program's first start here, drawn in software, has taken 17 seconds on a busy host. If
    none does, it says what would open the file, what opening it said, and what's open.
    """
    as_account(f"niri msg action spawn -- sh -c 'xdg-open {path} > /tmp/opened 2>&1'")
    try:
        machine.wait_until_succeeds(
            in_session(f'niri msg --json focused-window | grep -q \'"app_id":"{app_id}"\''),
            timeout=60,
        )
    except Exception:
        print(as_account(f"xdg-mime query default {kind}; cat /tmp/opened; niri msg windows"))
        raise


def listen() -> None:
    """Listen to what the notification server says to programs, as they hear it."""
    as_account(
        f"{DBUS}/bin/dbus-monitor --session"
        " \"type='signal',interface='org.freedesktop.Notifications'\""
        " > /tmp/signals 2>&1 < /dev/null & echo $! > /tmp/listening"
    )
    # It's listening once it hears a signal of its own.
    probe = (
        f"{DBUS}/bin/dbus-send --session --type=signal /probe org.freedesktop.Notifications.Probe"
    )
    machine.wait_until_succeeds(
        in_session(f"{probe}; grep -q member=Probe /tmp/signals"), timeout=10
    )


def tokened() -> None:
    """Check that the program was sent a token to bring its window forward, then the choice.

    That's the order a compositor needs.
    """
    said = machine.succeed("cat /tmp/signals")
    machine.succeed("kill $(cat /tmp/listening)")
    token = re.search(r"member=ActivationToken\n\s+uint32 \d+\n\s+string \"([^\"]+)\"", said)
    t.assertIsNotNone(token, said)
    t.assertLess(said.index("member=ActivationToken"), said.index("member=ActionInvoked"), said)


def offer_to_play(picture: str, offers: str = "") -> tuple[float, float]:
    """Show a notice offering to play, and give where its action's glyph is drawn.

    Where is a fraction of the screen, and the glyph is the only thing in the action colour.
    notify-send waits for the answer, so it runs on its own, and the notice stays a minute, so
    only a click closes it sooner.
    """
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    with told(f"shows card {picture}"):
        as_account(
            f"notify-send --expire-time=60000 {offers} --action=play='Play it' '{picture}'"
            " 'y.mp4' > /tmp/answer 2>&1 < /dev/null &"
        )
    action = [int(ACTION[i : i + 2], 16) for i in (1, 3, 5)]
    drawn: list[tuple[int, int]] = []
    size = [1, 1]

    # The card's first frames may not hold the glyph yet.
    def glyph_drawn(last: bool) -> bool:
        photograph(picture)
        image = Image.open(machine.out_dir / f"{picture}.png").convert("RGB")
        pixels = image.tobytes()
        size[:] = [image.width, image.height]
        drawn[:] = [
            (x, y)
            for y in range(image.height)
            for x in range(image.width)
            if sum(abs(pixels[3 * (y * image.width + x) + c] - action[c]) for c in range(3))
            < NEAR_THE_ACTION
        ]
        if last:
            assert drawn, "the action's glyph isn't drawn"
        return bool(drawn)

    retry(glyph_drawn, timeout_seconds=15)
    xs, ys = [x for x, _ in drawn], [y for _, y in drawn]
    assert max(xs) - min(xs) < ONE_GLYPH, f"more than one glyph: {drawn}"
    assert max(ys) - min(ys) < ONE_GLYPH, f"more than one glyph: {drawn}"
    return sum(xs) / len(xs) / size[0], sum(ys) / len(ys) / size[1]


def click(x: float, y: float) -> None:
    """Click there, by QEMU's tablet, which spans the screen from 0 to 32767.

    The driver says a command's arguments are strings, so the events go through JSON to reach
    it as they are.
    """

    def tablet(*events: Json) -> None:
        assert machine.qmp_client is not None
        machine.qmp_client.send("input-send-event", json.loads(json.dumps({"events": events})))

    tablet(
        {"type": "abs", "data": {"axis": "x", "value": int(x * 32767)}},
        {"type": "abs", "data": {"axis": "y", "value": int(y * 32767)}},
    )
    # QEMU hands the guest each batch in the order sent, so the press
    # lands where the pointer went.
    tablet({"type": "btn", "data": {"down": True, "button": "left"}})
    tablet({"type": "btn", "data": {"down": False, "button": "left"}})


def answered() -> str:
    """Give what notify-send said once its notice closed.

    That's the action if one was done, and nothing if the notice was only put away.
    """
    machine.wait_until_fails("pgrep -f '[n]otify-send .*--action=play'", timeout=15)
    return machine.succeed("cat /tmp/answer").strip()


# The margin is there only while something's on, each thing its glyph
# alone, and what each is, in words, is the launcher's What's on: Super+O
# or a click on a glyph opens it, Return does a thing's first action and
# the right arrow steps into the rest.
margin = "niri msg layers | grep -q margin"
notices = "niri msg layers | grep -q notifications"


def margin_sets(*wanted: tuple[str, str, str]) -> Callable[[str], bool]:
    """Test what the margin says it draws: each glyph's mark, kind and colour, left to right."""

    def sets(rest: str) -> bool:
        drawn = [(glyph["mark"], glyph["kind"], glyph["tone"]) for glyph in json.loads(rest)]
        return drawn == list(wanted)

    return sets


def screen_now() -> Image.Image:
    """Give the screen as it is, from QEMU's own picture of it."""
    with tempfile.TemporaryDirectory() as folder:
        shot = Path(folder) / "shot.ppm"
        machine.send_monitor_command(f"screendump {shot}")
        return Image.open(shot).convert("RGB")


def changed_in_the_corner(before: Image.Image, after: Image.Image) -> tuple[float, float]:
    """Give the middle of what changed in the screen's top right corner, as a fraction of it.

    That's a glyph the margin drew, and its shadows, which fall round it.
    """
    width, height = after.size
    was, now = before.tobytes(), after.tobytes()

    def moved(x: int, y: int) -> int:
        at = 3 * (y * width + x)
        return sum(abs(was[at + c] - now[at + c]) for c in range(3))

    corner = [
        (x, y)
        for y in range(height // 20)
        for x in range(width - width // 8, width)
        if moved(x, y) > NEAR_THE_ACTION
    ]
    assert corner, "nothing changed in the top right corner"
    xs, ys = [x for x, _ in corner], [y for _, y in corner]
    return (min(xs) + max(xs)) / 2 / width, (min(ys) + max(ys)) / 2 / height


def whats_on(selected: str) -> None:
    """Wait until the launcher holds What's on and the keys, with this thing's row selected."""
    launcher_until(
        lambda shown: (
            shown["open"]
            and shown["keys"]
            and shown["held"] == ["what's on"]
            and (shown["selected"] or {}).get("name") == selected
        )
    )


# Tuple's log, which only a call writes, and which the daemon rewrites as
# it starts.
TUPLE_LOG = "~/.local/share/tuple/0/log.txt"


def tuple_said(message: str, ago: int = 0) -> None:
    """Write a line to Tuple's log as the daemon writes it, as if so many seconds since."""
    line = f"$(date -u -d '-{ago} sec' +%H:%M:%S.000)|1|1|info|{message}"
    as_account(f'mkdir -p $(dirname {TUPLE_LOG}) && echo "{line}" >> {TUPLE_LOG}')


# Where keyboard-lights says whether caps lock is on.
CAPITALS = "/run/keyboard-lights/capslock"

# Print opens the launcher's Capture collection, where a picture of the
# screen, a window or a region is chosen by the keys, and niri says
# when it's taken.
pictures = f"/home/{ACCOUNT}/Pictures/Screenshots"


def capture() -> None:
    """Press Print, which opens Capture, and wait until its rows are ready for the keys."""
    machine.send_key("sysrq")
    launcher_until(
        lambda shown: (
            shown["keys"] and shown["held"] == ["captures"] and shown["selected"] is not None
        )
    )


@contextlib.contextmanager
def captured() -> Iterator[None]:
    """Wait for niri to say it took a picture, after what's done inside this."""
    since = mark()
    yield
    reported("captured ", lambda path: path.startswith(f"{pictures}/"), since)


def newest_size() -> tuple[int, int]:
    """Give the newest picture's width and height.

    A PNG says them in its header, from byte 16.
    """
    newest = f"ls -t {pictures}/*.png | head -1"
    size = machine.succeed(f"{newest} | xargs -d '\\n' od -An -tu1 -j16 -N8")
    b = [int(n) for n in size.split()]
    return (b[2] * 256 + b[3], b[6] * 256 + b[7])


# ssh-tpm-agent asks before a key confirmed on use signs, and for a
# key's PIN, and ssh for a security key's touch, all through the
# shell's askpass, which answers as OpenSSH's contract says.
def ask(kind: str, prompt: str) -> None:
    """Ask through the askpass, started by the account's systemd as ssh-tpm-agent is.

    That way it has what the agent has.
    """
    as_account(f"systemd-run --user --collect --quiet -- {ASKING} {kind} '{prompt}'")


def answered_with(kind: str, code: int, answer: str = "", timeout: int = 30) -> None:
    """Wait until an ask is answered, and check how it exited and what it gave back."""
    exited = f"$XDG_RUNTIME_DIR/asked-{kind}.exit"
    machine.wait_until_succeeds(in_session(f"test -e {exited}"), timeout=timeout)
    t.assertEqual(as_account(f"cat {exited}").strip(), str(code))
    t.assertEqual(as_account(f"cat $XDG_RUNTIME_DIR/asked-{kind}.answer"), answer)


# What ssh-tpm-agent asks before Joel's key signs.
SIGNING = (
    "Allow use of key joel-signing?\\nKey fingerprint SHA256:abc."
    "\\nRequested by ssh-keygen (1234) \u2190 tpm-sign \u2190 git \u2190 zsh"
)

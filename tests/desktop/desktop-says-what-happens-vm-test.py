"""The desktop test's says what happens area.

Here the desktop says what happens: notices, the margin and what it holds, Tuple's calls and
caps lock. tests/desktop.nix runs it after desktop_prelude.py, the boot and the helpers every
area shares, so the names it takes from there are imported only for type checkers.
"""

import json
import shlex
import time
from datetime import timedelta
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from desktop_prelude import (
        ACCOUNT,
        CAPITALS,
        FROM_THE_CORNER,
        FROM_THE_EDGE,
        LAUNCHER_OPEN,
        NEARLY_ALL,
        STARTING,
        TRACK,
        TUPLE_WINDOW,
        Json,
        answered,
        as_account,
        changed_at_the_top,
        click,
        drawn_at,
        held_still,
        in_session,
        launcher_ready,
        launcher_until,
        listen,
        margin,
        margin_points,
        margin_sets,
        mark,
        notices,
        offer_to_play,
        photograph,
        power,
        reported,
        screen_now,
        settled_screen,
        shell_said,
        showing,
        step_in,
        tokened,
        told,
        tuple_said,
        type_in,
        wait_for_surface,
        whats_on,
        window_titled,
    )
    from driver_names import machine, retry, subtest, t

# How far right of a glyph's middle a click lands, in pixels: inside its
# cell, and past where what takes the pointer would end had it stayed half
# a cell to the left, where the row was before it slid.
BESIDE_THE_MIDDLE = 2

with subtest("niri's settings failing to load is said until they load again"):
    # niri keeps the settings it had when a load fails, and says so for a
    # few seconds only. A setting it doesn't know stands in for one only a
    # newer niri knows, as srgb-gamut was to the niri running on 2026-10-24.
    settings = f"/home/{ACCOUNT}/.config/niri/config.kdl"
    meant = machine.succeed(f"readlink {settings}").strip()
    with told("shows card The window manager kept its old settings"):
        machine.succeed(
            f"cp --remove-destination {meant} {settings}"
            f" && echo 'a-setting-niri-does-not-know' >> {settings}"
            f" && chown {ACCOUNT} {settings}"
        )
    with told("gone notifications"):
        machine.succeed(f"ln -sfn {meant} {settings}")
    machine.wait_until_fails(
        in_session("test -e $XDG_RUNTIME_DIR/niri-settings-notice"), timeout=10
    )


with subtest("a notice that came and went is found again in the launcher"):
    # The notice the first notification's subtest left to go by itself,
    # sent and let go here when that one hasn't run.
    kept = as_account(
        "find ~/.local/state/quickshell -name notices.json"
        ' -exec grep -l \'"summary":"Build finished"\' {} + 2>/dev/null; true'
    )
    if not kept.strip():
        with told("drawn notifications"):
            as_account("notify-send 'Build finished' 'the laptop is ready'")
        machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    # One its sender says not to keep isn't, though it's the newest match.
    as_account("notify-send --transient 'Build finished twice' 'not kept'")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    # Its row at home counts the one missed earlier, since it left by itself.
    photograph("launcher-home-missed")
    type_in("build fin", selecting="the laptop is ready")
    photograph("launcher-notices")
    # Nothing sent it that can be gone to, so Return copies what it said.
    machine.send_key("ret")

    def told_again(_: bool) -> bool:
        """Say whether the clipboard holds what the notice said."""
        said = as_account("wl-paste --no-newline")
        return said == "Build finished\nthe laptop is ready"

    retry(told_again, timeout_seconds=10)
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # With none on screen, Super+N opens all of them, which counts
    # them seen.
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    machine.send_key("meta_l-n")
    launcher_ready()
    photograph("launcher-notices-all")
    machine.send_key("esc")


with subtest("a notice's action is done by the keys, and one not acted on stays unread"):
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # It stays a minute, so it's still at the front when the keys come.
    with told("shows card Lunch"):
        as_account("notify-send --expire-time=60000 'Lunch' 'at noon'")
    # notify-send waits for the answer, so it runs on its own. The
    # photograph waits for its card's frames to settle, which is when
    # Qt would have warned of a layout that never does.
    with told("shows card Recording saved"):
        as_account(
            "notify-send --action=play='Play it' 'Recording saved' 'x.mp4'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    photograph("notice-with-action")
    # Qt only warns when a layout never settles, which left the
    # card's words piled at its top.
    log = shell_said("polish() loop")
    assert "polish() loop" not in log, "a notice's layout never settles"
    # Super+N takes the stack, which waits while it holds the keys.
    # The newer notice waits behind the older; Down brings it to the
    # front, the right arrow steps into its verbs, what it offers
    # first, as a launcher row's are, Down chooses the next and Left
    # steps back, and Return does the one chosen, with a token to
    # come forward.
    listen()
    with told("keys notices"):
        machine.send_key("meta_l-n")
    photograph("notices-held")
    machine.send_key("down")
    with told("notice verb Play it"):
        machine.send_key("right")
    with told("notice verb Copy what it said"):
        machine.send_key("down")
    photograph("notices-held-verbs")
    with told("notice back"):
        machine.send_key("left")
    with told("notice verb Play it"):
        machine.send_key("right")
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx play /tmp/answer", timeout=15)
    tokened()
    # Shift+Backspace lets the rest go, which gives the keys back.
    with told("lets notices go"):
        machine.send_key("shift-backspace")
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=10)
    # The history is kept on disk, the one acted on read and the one
    # that only arrived still unread.
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(
        in_session(kept + ' | grep -q \'"summary":"Recording saved"[^}]*"read":true\''),
        timeout=10,
    )
    as_account(kept + ' | grep -q \'"summary":"Lunch"[^}]*"read":false\'')


with subtest("a click on a notice's action does it, and anywhere else puts it away"):
    since = mark()
    x, y = offer_to_play("notice-to-click")
    click(x, y)
    assert answered() == "play", "a click on the action put the notice away instead"
    # A little above the action is the notice's message.
    x, y = offer_to_play("notice-to-put-away")
    click(x, y - 0.04)
    assert answered() == "", "a click on the message did the action"
    # Neither was missed, so the margin never once said one waits: a
    # notice arriving or leaving mustn't flash it.
    said = as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    t.assertNotIn("holds margin waiting", said, "the margin flashed a waiting notice")

with subtest("a click on a notice opens what it's about, with a token to come forward"):
    listen()
    # Its default is the card's click, so the one glyph is Play it's.
    x, y = offer_to_play("notice-to-open", offers="--action=default=Open")
    click(x, y - 0.04)
    assert answered() == "default", "a click on the message didn't open what it's about"
    tokened()

with subtest("Return on a notice in the launcher opens what it's about, with a token"):
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    listen()
    with told("shows card Thread with Sam"):
        as_account(
            "notify-send --expire-time=60000 --action=default=Open 'Thread with Sam' 'see it'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    # Super+N takes the stack, and again hands on to the history.
    with told("keys notices"):
        machine.send_key("meta_l-n")
    machine.send_key("meta_l-n")
    launcher_ready()
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Sam")
    )
    machine.send_key("ret")
    machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=15)
    tokened()

with subtest("a notice whose time is up still does what it offers from the launcher"):
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    machine.wait_until_fails(in_session(notices), timeout=timedelta(seconds=30))
    # It says no time of its own, so it goes by itself after the ten
    # seconds one offering something has, unanswered; and notify-send,
    # which stops waiting once a time it was given is up, waits on.
    with told("rests Quarry take"):
        as_account(
            "notify-send --expire-time=0 --action=play='Play it' 'Quarry take' 'z.mp4'"
            " > /tmp/answer 2>&1 < /dev/null &"
        )
    machine.wait_until_fails(in_session(notices), timeout=timedelta(seconds=10))
    # Gone from the screen but still open: closing it would have ended
    # notify-send, as it ends a program's hold on what it offered.
    machine.succeed("pgrep -f '[n]otify-send .*--action=play'")
    listen()
    # With none on screen, Super+N opens the history, newest first.
    machine.send_key("meta_l-n")
    launcher_ready()
    # Its row is one line; below the rows it reads whole, summary over
    # body.
    launcher_until(lambda shown: (shown["selected"] or {}).get("reads") == "Quarry take\nz.mp4")
    step_in()
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Play it")
    machine.send_key("ret")
    assert answered() == "play", "a notice whose time was up lost what it offered"
    tokened()


with subtest("the notices sit a step inside the window under them"):
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    # A window of its own, the only one in view, two thirds of the
    # screen wide and centred, so its edge is far from the corner's;
    # it goes again after.
    as_account("niri msg action spawn -- kitty --title 'Under the notices'")
    machine.wait_until_succeeds(
        in_session("niri msg --json windows | grep -q '\"Under the notices\"'"), timeout=30
    )
    under = window_titled("Under the notices")
    as_account(f"niri msg action focus-window --id {under['id']}")
    screen = json.loads(as_account("niri msg --json focused-output"))["logical"]["width"]
    wide = int(screen * 2 / 3)
    as_account(f"niri msg action set-column-width {wide}")
    # niri centres a column by the width its window has, and kitty takes
    # its new width only when it next draws, so centring before then
    # leaves the column to grow rightwards from where it was centred.
    retry(
        lambda _: window_titled("Under the notices")["layout"]["tile_size"][0] >= wide,
        timeout_seconds=15,
    )
    as_account("niri msg action center-column")
    space = json.loads(as_account("cat ~/.config/jitsusama/theme.json"))["space"]

    # Where it will be once the view rests, from niri.
    def settled(last: bool) -> bool:
        """Say whether niri has the window where it will be once the view rests."""
        layout = window_titled("Under the notices")["layout"]
        place = layout["tile_pos_in_workspace_view"]
        if last:
            # Where niri has it, and what else is on the screen, to say why when it's elsewhere.
            print(f"UNDER THE NOTICES at {place}, {layout['tile_size']} wide, screen {screen}")
            print(as_account("niri msg windows"))
            t.assertIsNotNone(place, "niri doesn't say where a tiled window is")
        return place is not None and screen - (place[0] + layout["tile_size"][0]) > FROM_THE_CORNER

    retry(settled, timeout_seconds=15)
    with told("shows card Inside the window"):
        as_account("notify-send --expire-time=60000 'Inside the window' 'a step from its edges'")
    photograph("notice-inside")
    # The nearest window wholly on screen, as the shell picks it.
    ws = window_titled("Under the notices")["workspace_id"]
    tiles = [
        (w["layout"]["tile_pos_in_workspace_view"], w["layout"]["tile_size"])
        for w in json.loads(as_account("niri msg --json windows"))
        if w["workspace_id"] == ws and w["layout"]["tile_pos_in_workspace_view"]
    ]
    whole = [(p, s) for p, s in tiles if p[0] >= 0 and p[0] + s[0] <= screen]
    (left, top), (width, _) = min(whole, key=lambda t: (-(t[0][0] + t[1][0]), t[0][1]))
    step = space["s"]
    wanted_at = [
        int(screen - (left + width) + step + 0.5),
        int(max(top, 0) + step + 0.5),
        min(380, int(width - 2 * step)),
    ]
    notices_at = as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[places notices [0-9 ]*' | tail -n 1"
    ).split()[-3:]
    t.assertEqual(
        [int(n) for n in notices_at], wanted_at, "the notices aren't a step inside the window"
    )
    as_account("qs ipc call notices putAway")
    as_account(f"niri msg action close-window --id {under['id']}")
    machine.wait_until_fails(
        in_session("niri msg --json windows | grep -q '\"Under the notices\"'"), timeout=30
    )

with subtest("the launcher keeps the machine awake, lid shut too, then lets it sleep"):
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("keep awake")
    photograph("launcher-keep-awake")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"{held} | grep -qw idle", timeout=10)
    machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
    # Kept awake, a suspend asked for by the power key or the launcher
    # still goes through, and so does the lid's on battery.
    machine.fail(f"{held} | grep -q sleep")
    # Without power the lid is let go, and taken again when it's back.
    power("off")
    machine.wait_until_fails(f"{held} | grep -q handle-lid-switch", timeout=10)
    machine.succeed(f"{held} | grep -qw idle")
    power("on")
    machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    photograph("launcher-home-awake")
    type_in("let it sleep")
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=10)


with subtest("a camera opened is said, then shows in the margin, and the launcher says by what"):
    machine.wait_until_fails(in_session(margin), timeout=10)
    # What's true as the shell starts isn't news, so it's waited out.
    reported("settles arrivals", lambda rest: rest == "", "")
    # Written whole, as the camera's module writes it. A program opened
    # it by itself, so it's said as a notice of the camera's.
    since = mark()
    with told("shows card chrome has the camera open"):
        machine.succeed(
            "echo chrome > /run/camera-holders/.camera"
            " && mv /run/camera-holders/.camera /run/camera-holders/camera"
        )
    photograph("notice-camera")
    # A card or a mark: once the card goes, the glyph shows, on air, so
    # strong, and the card wasn't missed, since the glyph says it.
    machine.wait_until_fails(in_session(notices), timeout=30)
    reported("sets margin ", margin_sets(("camera", "onAir", "strong")), since)
    photograph("margin-camera")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("camera")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "in use by chrome")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # Let go, it lingers a moment, muted, then goes.
    since = mark()
    machine.succeed(
        ": > /run/camera-holders/.camera"
        " && mv /run/camera-holders/.camera /run/camera-holders/camera"
    )
    reported("sets margin ", margin_sets(("camera", "onAir", "muted")), since)
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("a USB camera's stream shows in the margin too, and leaves with the camera"):
    # A camera the machine doesn't name, written as usb-camera-holders
    # writes one: a file named for the camera, there only while it is.
    usb = "/run/camera-holders/MX Brio 705 for Business"
    with told("drawn margin"):
        machine.succeed(f"echo zoom > '{usb}.1' && mv '{usb}.1' '{usb}'")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("camera")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "in use by zoom")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    machine.succeed(f"rm '{usb}'")
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("Keep Awake shows in the margin, and What's on says it and lets it sleep"):
    # Keep Awake was just let go, and the margin leaves as it fades.
    machine.wait_until_fails(in_session(margin), timeout=10)
    # Its own, so it runs alone: whether something holds the machine awake.
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    # A camera's card from before is gone first, so what changes along the
    # top is the glyph alone.
    machine.wait_until_fails(in_session(notices), timeout=30)
    # Left on, so muted.
    held_still()
    empty = screen_now()
    since = mark()
    with told("drawn margin"):
        as_account("systemctl --user start caffeinate.service")
    reported("sets margin ", margin_sets(("awake", "leftOn", "muted")), since)
    # Super+O opens What's on at its row, and while it's open the mark its
    # keys are on wears the accent, nothing grown; closed, it's as it was.
    glyph = changed_at_the_top(empty, settled_screen("margin-awake"))
    since = mark()
    machine.send_key("meta_l-o")
    whats_on("Keep Awake")
    reported("sets margin ", margin_points("awake"), since)
    # Settled, as Joel sees it.
    held_still()
    photograph("launcher-whats-on")
    since = mark()
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    reported("sets margin ", margin_points(""), since)
    # Another mark comes and goes, so the row slides aside and back, and
    # what takes the pointer has to follow it there.
    since = mark()
    as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    reported("holds margin ", lambda rest: sorted(rest.split()) == ["awake", "muted"], since)
    held_still()
    since = mark()
    as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
    reported("holds margin ", lambda rest: rest.split() == ["awake"], since)
    held_still()
    glyph = (glyph[0] + BESIDE_THE_MIDDLE / screen_now().width, glyph[1])
    # A click there opens it the same, and Return lets the machine sleep.
    click(*glyph)
    try:
        whats_on("Keep Awake")
    except AssertionError:
        # Where the click went, beside where the shell's surfaces are.
        print(f"clicked at {glyph} of the screen")
        print(as_account("niri msg layers; niri msg --json focused-output"))
        raise
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=10)
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("do not disturb holds a notice, shows it held, and ends from the margin"):
    machine.wait_until_fails(in_session(notices), timeout=30)
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("do not disturb")
    machine.send_key("ret")
    wait_for_surface("margin")
    # The shell decides as the notice arrives, so once it says it held
    # one, there's no card to come.
    with told("held Held back"):
        as_account("notify-send 'Held back' 'until it ends'")
    machine.fail(in_session(notices))
    photograph("margin-quiet")
    # Its row ends it, and a step in offers the rest, what's held last:
    # an hour more comes first only when it ends by itself.
    machine.send_key("meta_l-o")
    whats_on("Do Not Disturb")
    step_in()
    if (showing()["selected"] or {}).get("name") != "Show what's held":
        machine.send_key("down")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Show what's held")
    machine.send_key("ret")
    launcher_until(lambda shown: shown["keys"] and shown["held"] == ["notifications"])
    photograph("launcher-held")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    machine.send_key("meta_l-o")
    whats_on("Do Not Disturb")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(margin), timeout=10)
    as_account("test ! -s $XDG_RUNTIME_DIR/quiet")

with subtest("a missed notice shows its glyph, and What's on shows the notices"):
    # On screen it isn't waiting yet; once it goes by itself, it is, and
    # it wants Joel, so it's strong.
    since = mark()
    with told("drawn margin"), told("shows card Peter"):
        as_account("notify-send --expire-time=1000 'Peter' 'are you there?'")
    machine.wait_until_fails(in_session(notices), timeout=15)
    reported("sets margin ", margin_sets(("waiting", "wants", "strong")), since)
    photograph("margin-waiting")
    machine.send_key("meta_l-o")
    whats_on("Notices")
    machine.send_key("ret")
    launcher_until(lambda shown: shown["keys"] and shown["held"] == ["notifications"])
    photograph("launcher-from-margin")
    machine.send_key("esc")
    # Opening them counted them seen, so nothing's waiting.
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("waiting that lasts a minute shows in the margin, quietly"):
    # lag-watch would write its own window over a stand-in's, so it rests.
    as_account("systemctl --user stop lag-watch.service")
    lagged = "$XDG_RUNTIME_DIR/lag-watch/status.json"

    def lag_says(worst: str, who: str, ago: int) -> None:
        """Write a window as lag-watch does, renamed in, its run begun so long ago."""
        now = int(machine.succeed("date +%s").strip())
        window = {
            "ended": now,
            "trouble": [f"{who} waited on {worst}"] if worst else [],
            "worst": worst,
            "who": who,
            "since": now - ago if worst else None,
        }
        as_account(
            f"mkdir -p $(dirname {lagged}) && echo '{json.dumps(window)}' > {lagged}.new"
            f" && mv {lagged}.new {lagged}"
        )

    # A moment's wait says nothing.
    since = mark()
    lag_says("memory", "apps", 10)
    time.sleep(2)
    said = as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    t.assertNotRegex(said, r"holds margin [^\n]*\blag\b", "a moment's wait showed")
    # A minute of it does, left on, so muted. A run that began a minute
    # ago is another run, so the last one ends first.
    lag_says("", "", 0)
    time.sleep(2)
    lag_says("memory", "apps", 70)
    reported("sets margin ", margin_sets(("lag", "leftOn", "muted")), since)
    photograph("margin-lag")
    # Over, it goes.
    lag_says("", "", 0)
    reported("holds margin", lambda rest: "lag" not in rest.split(), since)
    as_account("systemctl --user start lag-watch.service")

with subtest("the microphone muted shows in the margin, and Return unmutes it"):
    try:
        with told("drawn margin"):
            as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    except AssertionError:
        # Which source Pipewire calls the default, whether it's muted,
        # and what Pipewire holds for it, beside what Quickshell heard.
        print(as_account("wpctl status; wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
        print(as_account("pw-dump stand-in-microphone"))
        raise
    machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
    photograph("margin-muted")
    machine.send_key("meta_l-o")
    whats_on("Microphone")
    machine.send_key("ret")
    muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
    machine.wait_until_fails(in_session(muted), timeout=10)
    machine.wait_until_fails(in_session(margin), timeout=10)
    # Unmuting shows the meter a moment, which the next pictures
    # would hold for as long as the host takes to reach them.
    machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)

# Dictation shows at the bottom centre, never in the margin, and says what
# it heard, from a stand-in for its service (desktop.nix).
with subtest("dictation says what it's doing, and when it heard nothing"):
    dictation = "niri msg layers | grep -q dictation"
    # The volume's and the microphone's meters share the bottom centre, and
    # show for a moment as the shell first hears them, so they're let go.
    meters = "niri msg layers | grep -Eq 'Namespace: \"(volume|microphone)\"'"
    machine.wait_until_fails(in_session(meters), timeout=15)
    since = mark()
    as_account("dictation toggle")
    reported("says dictation listening", lambda rest: rest == "", since)
    wait_for_surface("dictation")
    photograph("dictation-listening")
    # Typing shows once it lasts, and a press meanwhile says it's still
    # writing; the stand-in types for five seconds.
    as_account("dictation toggle")
    reported("says dictation typing", lambda rest: rest == "", since)
    as_account("dictation toggle")
    reported("says dictation still writing", lambda rest: rest == "", since)
    photograph("dictation-still-writing")
    reported("says dictation heard nothing", lambda rest: rest == "", since)
    photograph("dictation-heard-nothing")
    machine.wait_until_fails(in_session(dictation), timeout=10)

# Any player that tells the desktop what it plays shows in the margin
# while it plays, and the media keys' command shows what it is.
with subtest("what's playing shows in the margin, and a media key shows what it is"):
    machine.wait_until_fails(in_session(margin), timeout=10)
    # A track that starts by itself shows its card a moment, which
    # goes before the margin is photographed.
    with told("drawn notifications"), told("drawn margin"):
        as_account(f"systemd-run --user --unit=player mpv --ao=null --no-video --loop {TRACK}")
    machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
    photograph("margin-playing")
    # The play key pauses it, and the card says what was paused.
    with told("drawn notifications"):
        as_account("qs ipc call music toggle")
    photograph("music-card")
    # Paused, nothing's playing, so the margin goes; the card goes by itself.
    machine.wait_until_fails(in_session(margin), timeout=10)
    machine.wait_until_fails(in_session(notices), timeout=15)
    as_account("systemctl --user stop player")


# A call comes before the other apps when the machine is busy.
with subtest("Tuple's daemon comes before the other apps"):
    comes = as_account("systemctl --user show tuple --property=CPUWeight --property=Nice")
    t.assertEqual(sorted(comes.split()), ["CPUWeight=1000", "Nice=-10"])

with subtest("a Tuple call shows in the margin while it lasts"):
    # Only a call writes these lines, so they're written here, with
    # the daemon stopped, since it rewrites its log as it starts.
    as_account("systemctl --user stop tuple")
    machine.wait_until_fails(in_session(margin), timeout=10)
    with told("drawn margin"):
        tuple_said("call connected")
    photograph("margin-call")
    tuple_said("Caller.cpp:985|invalidating call")
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("people coming and going in a Tuple call are said, and Joel can leave"):
    # Whoever comes is "a guest", not a number, which Tuple's contacts
    # would be asked about, and with the daemon stopped it starts one.
    # The call before stops it too, so this one stops it itself and runs
    # alone the same.
    as_account("systemctl --user stop tuple")
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action={0}'"
    # Those there as Joel joins aren't news, so the call connected a
    # while before the guest comes.
    with told("drawn margin"):
        tuple_said("call connected", ago=10)
    with told("shows card Someone joined the call"):
        tuple_said("peer joined call: a guest")
    photograph("notice-joined")
    with told("shows card Someone left the call"):
        tuple_said("peer left call: a guest")
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so Super+N takes the stack at once: held, it waits.
    with told("keys notices"):
        machine.send_key("meta_l-n")
    machine.succeed(offered.format("leave"))
    photograph("notice-alone")
    # Leaving runs tuple call leave, which starts the stopped daemon,
    # whose new log ends the call. Super+N again opens the history on the
    # newest notice's offers.
    machine.send_key("meta_l-n")
    launcher_ready()
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Leave the call")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(margin), timeout=15)
    machine.wait_until_fails(offered.format("leave"), timeout=10)
    as_account("tuple stop")

with subtest("a Tuple call ringing wants Joel, and one not answered is kept as missed"):
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action=show'"
    machine.wait_until_fails(in_session(margin), timeout=10)
    with told("shows card Someone is calling"):
        tuple_said("received incoming call: a caller")
    machine.succeed(offered)
    # The ring's card says it, so the margin's phone waits for it to go.
    photograph("notice-ringing")
    # The caller gives up: the ring's notice goes, and a missed call stays.
    tuple_said("Caller.cpp:985|invalidating call")
    machine.wait_until_fails(offered, timeout=10)
    kept = "grep -q 'Missed a call from someone' ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(in_session(kept), timeout=10)
    # Once it goes by itself it waits, and the margin shows it.
    machine.wait_until_fails(in_session(notices), timeout=15)
    wait_for_surface("margin")
    machine.send_key("meta_l-n")
    launcher_ready()
    machine.send_key("esc")
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("a copied call's link can be joined until something else is copied"):
    link = "https://tuple.app/c/from-a-chat"
    as_account(f"echo -n 'pair with me? {link}' | wl-copy > /dev/null 2>&1")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("join", selecting="Join the call")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

    # A screenshot is copied as an image alone, with no text to read.
    as_account("printf '\\211PNG\\r\\n' | wl-copy --type image/png > /dev/null 2>&1")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("join")

    def joins_nothing(shown: Json) -> bool:
        """Say whether the launcher's selected row joins no Tuple call."""
        row = shown["selected"]
        return row is None or row["is"] != "tuple:join"

    launcher_until(joins_nothing)
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)

with subtest("Tuple's windows find their places: the face in a corner, a shared screen wide"):
    # Through X11, as Tuple draws them, so niri starts xwayland-satellite
    # for them, and they reach it titled by their part. Tuple's daemon
    # is a service, started with no token from niri to be let take the
    # keys, which xwayland-satellite would hand on, so neither has one.
    def tuple_opens(part: str) -> Json:
        """Open a window as Tuple's part of this name does, and give it as niri lists it."""
        as_account(f"niri msg action spawn -- env -u XDG_ACTIVATION_TOKEN {TUPLE_WINDOW} {part}")
        machine.wait_until_succeeds(
            in_session(f'niri msg --json windows | grep -q \'"title":"{part}"\''),
            timeout=STARTING,
        )
        return window_titled(part)

    span = json.loads(as_account("niri msg --json focused-output"))["logical"]
    # Tuple's own panel floats.
    tuple_panel = tuple_opens("tuple-main")
    assert tuple_panel["is_floating"], tuple_panel
    # Joel is at work in a window of his own when the other's face
    # comes, and keeps the keys. (xwayland-satellite hands a new X11
    # window the keys while another of its windows has them, which no
    # rule of niri's overrides.)
    as_account("niri msg action spawn -- kitty --title 'at work'")
    machine.wait_until_succeeds(
        in_session('niri msg --json focused-window | grep -q \'"title":"at work"\''),
        timeout=STARTING,
    )
    tuple_face = tuple_opens("tuple-webcams")
    t.assertEqual(tuple_face["app_id"], "Tuple")
    assert tuple_face["is_floating"], tuple_face
    assert not tuple_face["is_focused"], tuple_face
    assert window_titled("at work")["is_focused"], tuple_face
    left, top, width, height = drawn_at(tuple_face)
    t.assertEqual((width, height), (384, 216))
    # In the bottom-right corner, a little in from the screen's edges
    # and whatever niri keeps clear along them.
    assert 0 < span["width"] - (left + width) <= FROM_THE_EDGE, tuple_face
    assert 0 < span["height"] - (top + height) <= FROM_THE_EDGE, tuple_face

    tuple_view = tuple_opens("tuple-screen")
    assert not tuple_view["is_floating"], tuple_view
    assert tuple_view["layout"]["window_size"][0] >= span["width"] * NEARLY_ALL, tuple_view
    photograph("tuple-windows")
    as_account("pkill -f '[t]uple-window'")
    as_account(f"niri msg action close-window --id {window_titled('at work')['id']}")
    machine.wait_until_fails(
        in_session("niri msg --json windows | grep -q 'tuple-\\|at work'"), timeout=30
    )

with subtest("caps lock shows in the margin while it's on"):
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=10)
    wait_for_surface("margin")
    photograph("margin-capitals")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 0 {CAPITALS}", timeout=10)
    machine.wait_until_fails(in_session(margin), timeout=10)

with subtest("caps lock still shows after its watcher restarts"):
    # Quickshell keeps running, so it must still hear the file.
    machine.succeed("systemctl restart keyboard-lights")
    machine.wait_for_unit("keyboard-lights.service")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=10)
    wait_for_surface("margin")
    machine.send_key("caps_lock")
    machine.wait_until_fails(in_session(margin), timeout=10)

# Super+Shift+N puts the cards away, and each is still there unread.
with subtest("Super+Shift+N puts the notices away"):
    with told("shows card Standup"):
        as_account("notify-send --urgency=critical 'Standup' 'in five minutes'")
    with told("gone notifications"):
        machine.send_key("meta_l-shift-n")
    kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    machine.wait_until_succeeds(
        in_session(kept + ' | grep -q \'"summary":"Standup"[^}]*"read":false\''),
        timeout=10,
    )

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

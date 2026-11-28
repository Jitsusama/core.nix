"""The desktop test's keeps copies area.

Here the clipboard keeps what's copied, text and pictures, past the programs that copied
them, gives it back by Super+V into the window it opened over, and forgets it all when asked;
and a password Passwords copies pastes, is never kept or read by a fenced command, and clears.

tests/desktop/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import functools
import json
import math
import time
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
import vm_helpers
from desktop_session import STARTING, retry

if TYPE_CHECKING:
    from desktop_session import Desktop, Json


# Each test closes the launcher after it, so one failing with it open fails alone.
pytestmark = pytest.mark.usefixtures("launcher_closed_after")


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
    desktop.shoot("launcher-copies-all")
    desktop.type_in("tues", selecting="Notes for Tuesday")
    desktop.shoot("launcher-copies")
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
    # It copies what it pastes itself, so it holds when run alone: run by
    # its name it found no copy to paste, and failed 3 of 3 (16 November).
    # A copy made again moves to the front and is heard again, so each is
    # waited on whether this case runs alone or after the first. A newer
    # copy goes above the notes before each paste, so only the history
    # putting the notes back can paste them.
    with desktop.told("heard copy Notes for Tuesday"):
        desktop.as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")

    def copy_newer() -> None:
        """Copy something newer than the notes, so the clipboard holds that."""
        with desktop.told("heard copy a newer copy"):
            desktop.as_account("echo -n 'a newer copy' | wl-copy > /dev/null 2>&1")

    copy_newer()
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
    # Paste first, and Copy beside it for only copying, inside by Ctrl+],
    # since Return pastes.
    desktop.follow()
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

    copy_newer()
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
    # The shell says so as it asks the helper to paste, which then presses
    # the keys; Return any sooner would close the field still empty.
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
    tall = desktop.showing()["drawer"]["cardHeight"]
    with desktop.told("showed Image 256 \u00d7 256"):
        desktop.type_in("image", selecting="Image 256 \u00d7 256")
    # The picture shows in the drawer, out of the card's right side and
    # within the screen, and the card keeps its height for it.
    desktop.launcher_until(lambda shown: shown["drawer"]["out"] is True)
    drawer = desktop.showing()["drawer"]
    assert drawer["title"] == "Image 256 \u00d7 256", drawer
    assert drawer["cardHeight"] == tall, f"the card grew for the picture: {drawer}, from {tall}"
    assert drawer["height"] == tall, drawer
    screen = json.loads(desktop.as_account("niri msg --json focused-output"))
    assert drawer["cardAt"] < drawer["at"], drawer
    assert drawer["at"] + drawer["width"] <= screen["logical"]["width"], (drawer, screen)
    desktop.shoot("launcher-copies-picture")
    machine.send_key("ret")
    retry(desktop.pasted_the_picture, timeout=timedelta(seconds=10))
    # Found by a search typed at home, it's in the drawer the same, as the
    # drawer is the selected row's wherever it's found. A program leads what
    # "image" finds, and the copy, found once copies are read, below it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("image")
    desktop.launcher_until(lambda shown: "Image 256 \u00d7 256" in shown["rows"])
    # While the clipboard holds it, GIMP's act on it is found too, as the
    # shell's one watcher of the clipboard hears the copy's type.
    desktop.launcher_until(lambda shown: "New Image From the Copied Picture" in shown["rows"])
    desktop.down_to("Image 256 \u00d7 256")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True and shown["drawer"]["title"] == "Image 256 \u00d7 256"
        )
    )
    desktop.close_launcher()
    # And on GIMP's own page, where the launcher's designer sees the act
    # among GIMP's others. Return would open GIMP's window, so it's only
    # looked at.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("gnu image", selecting="GIMP")
    desktop.into_program()
    desktop.launcher_until(lambda shown: "New Image From the Copied Picture" in shown["rows"])
    desktop.shoot("launcher-gimp-copied-picture")
    desktop.close_launcher()
    a_new_drawing_is_the_template_copied_and_leads_inkscapes_recent(desktop)


def a_new_drawing_is_the_template_copied_and_leads_inkscapes_recent(desktop: Desktop) -> None:
    """Open a new drawing from Inkscape's page, and find it leading its Recent.

    While the picture is copied, Inkscape's page holds New Drawing From the Copied Picture too,
    saying its size. Return on New Drawing gives Inkscape, stood in, a fresh copy of the
    account's own template in the drawings folder, the account's settings reaching the act;
    the stand-in adds it to the list of files opened lately as Inkscape does, and the
    provider's watch of that list and the account's inkscape-drawings put it first in
    Inkscape's Recent, its drawer saying the template's size. What the act writes, the
    template left be, and the order, sizes and words of what Recent holds are held offscreen
    (the-launcher-lists-inkscapes-drawings-lately).
    """
    machine = desktop.machine
    given = "/run/user/1000/inkscape-given"
    template = f"/home/{desktop.account}/.config/inkscape/templates/default.svg"
    drawn = f"/home/{desktop.account}/Pictures/Drawings"
    machine.succeed(f"rm -f {given}")
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("inkscape", selecting="Inkscape")
        desktop.into_program()
        desktop.launcher_until(
            lambda shown: (
                {"New Drawing", "New Drawing From the Copied Picture"} <= set(shown["rows"])
            )
        )
        desktop.down_to("New Drawing")
        machine.send_key("ret")
        machine.wait_until_succeeds(f"test -s {given}", timeout=timedelta(seconds=30))
        drawing = machine.succeed(f"cat {given}").strip()
        assert drawing.startswith(f"{drawn}/Drawing "), drawing
        assert drawing.endswith(".svg"), drawing
        machine.succeed(f"cmp '{drawing}' {template}")
        name = drawing.rsplit("/", 1)[-1]

        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("inkscape", selecting="Inkscape")
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()

        def leads(shown: Json) -> bool:
            """Say whether the new drawing leads Recent, selected, its drawer drawn whole."""
            row = shown["selected"] or {}
            drawer = shown.get("drawer") or {}
            return (
                row.get("name") == name
                and row.get("side") == "just now"
                and drawer.get("out") is True
                and drawer.get("drawn") == name
                and drawer.get("eyebrow") == "in ~/Pictures/Drawings \u00b7 drawing"
                and drawer.get("drawnFacts") == ["Dimensions", "Size", "Path"]
                and (drawer.get("values") or [None])[0] == "210 \u00d7 297\u00a0mm"
            )

        desktop.launcher_until(leads)
    finally:
        desktop.close_launcher()
        machine.succeed(f"rm -rf {given} {drawn}")


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
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("desktop ")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["desktop"] and "Clipboard" in shown["rows"],
    )
    assert "Forget the Clipboard" not in desktop.showing()["rows"]
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("forget the clip", selecting="Forget the Clipboard")
    desktop.shoot("launcher-forget-copies")
    # Return writes :forget on the command line, since forgetting ends what's kept; a
    # fresh Return there asks, by the row's own name, as it can't be taken back and costs
    # every copy, and a Return more does it.
    desktop.line("forget")
    with desktop.told("asked Forget the Clipboard?"), desktop.told("ask ready"):
        machine.send_key("ret")
    with desktop.told("forgot the copies"):
        machine.send_key("ret")
    assert desktop.as_account("wl-paste --no-newline 2>&1 || true") == ""
    # The pictures' files go with them.
    assert desktop.as_account("ls -A $XDG_RUNTIME_DIR/clipboard-history") == ""


# The script's own lines, each said once its phase holds
# (modules/pi/tests): the walls before any window holds the focus, and
# the focus the secret copy's phase needs.
WALLS_HOLD = "WALLS HOLD: "
FOCUSED = "FOCUSED: "
READ = "READ: "
# Where the script's words are kept as it runs, so a run cut off at the
# timeout still says which phases held: Joel's runtime folder, his alone.
X11_SAID = "$XDG_RUNTIME_DIR/x11-fence-said"
# The script bounds its waits to about two minutes; this leaves it room.
X11_ENDING = timedelta(minutes=4)
# Where the script stops at a step of the satellite's, by the line that
# step says once it holds and the failure it ends with where it doesn't.
# Those are the focus subtest's to fail while the satellite's stall is
# quarantined, and every other failure is the fence subtest's.
SATELLITE_STEPS = (
    (FOCUSED, "FAIL: niri focused no X11 window"),
    (READ, "FAIL: Joel's own X11 client didn't read his copy"),
)


def fence_failure(said: str) -> str | None:
    """Say how the fence's walls weren't shown to hold in what the script said, or None.

    The walls are tried before the satellite's steps and again after both, so a run that
    stopped at one of those steps, by that step's own failure, proves the walls it reached.
    The failure is found by its words, not where it falls, so lines another program wrote
    can't stand in for it; the script's end is the line written after it ended.
    """
    lines = said.rstrip().splitlines()
    if not any(line.startswith(WALLS_HOLD) for line in lines):
        return "the fence's walls weren't shown to hold"
    ended = lines[-1]
    if ended == "ended 0":
        return None
    failures = [line for line in lines if line.startswith("FAIL: ")]
    for mark, failure in SATELLITE_STEPS:
        if not any(line.startswith(mark) for line in lines):
            stopped = (
                ended == "ended 1" and failures[-1:] != [] and failures[-1].startswith(failure)
            )
            return None if stopped else "the walls held, then the script failed otherwise"
    return "the fence let X11 through"


def said_line(said: str, mark: str) -> bool:
    """Whether a line of the script's own opens with `mark`, never words it quoted."""
    return any(line.startswith(mark) for line in said.splitlines())


@functools.cache
def x11_fence_said(desktop: Desktop) -> str:
    """Run the X11 fence's script once, for both subtests that read what it said."""
    # A raise is kept as words, since a cache keeps none, and both
    # subtests judge the one run rather than the second running it again.
    try:
        # niri imports its X11 display as a session does, run as its child
        # so it holds what niri set; the session's start imports the rest.
        desktop.as_account("niri msg action spawn -- systemctl --user import-environment DISPLAY")
        desktop.machine.wait_until_succeeds(
            desktop.in_session("systemctl --user show-environment | grep -q '^DISPLAY='")
        )
        # The script says each case it tried, and fails naming the one
        # that let the secret through (modules/pi/tests); this area alone
        # has it.
        # Its waits run on the guest's clock, so they're stretched by the
        # host's busyness as as_account stretches the wait for its end.
        # The one pace, read once, stretches both, so the script's waits
        # always end inside the time given for its end.
        pace = math.ceil(vm_helpers.pace(vm_helpers.busyness()))
        return desktop.machine.succeed(
            desktop.in_session(
                f"X11_PACE={pace} pi-fence-keeps-x11-from-secrets > {X11_SAID} 2>&1;"
                f" echo ended $? >> {X11_SAID};"
                f" cat {X11_SAID}"
            ),
            timeout=X11_ENDING * pace,
        )
    except Exception as error:  # noqa: BLE001 # Said by the subtests that read it.
        so_far = desktop.diagnosis(f"cat {X11_SAID} 2>&1 || true")
        return f"{so_far}\nthe script didn't end: {error!r}"


def test_an_x11_window_of_joels_takes_the_focus(desktop: Desktop) -> None:
    """An X11 window of Joel's takes the focus through the satellite."""
    said = x11_fence_said(desktop)
    # The walls come first, so a run that never reached the focus says so
    # by its own cause rather than as a window that never took it. Their
    # line is missing too where X never answered or the script didn't end,
    # which proves no breach, so this says only that they weren't shown.
    assert said_line(said, WALLS_HOLD), (
        f"the walls weren't shown to hold, so the focus went untried: {said}"
    )
    assert said_line(said, FOCUSED), f"no X11 window was seen to take the focus: {said}"
    assert said_line(said, READ), f"Joel's own X11 client didn't read his copy: {said}"
    # The secret copy's walls follow, so it's this subtest's too: a run
    # whose walls failed there fails the next one as well, so the
    # quarantine excuses only a run stopped at one of the satellite's steps.
    assert said.rstrip().endswith("ended 0"), f"the secret copy's phase didn't end clean: {said}"


def test_a_fenced_command_reads_no_secret_copy_through_x11(desktop: Desktop) -> None:
    """A fenced command reads no secret copy through X11, where Joel's own client does."""
    said = x11_fence_said(desktop)
    # The walls are tried before any window takes the focus, so this
    # proves them in every run, and fails in every run they let X through.
    # Refusing a connection owes nothing to what the clipboard holds, so
    # the walls are proven with or without the copy.
    failure = fence_failure(said)
    assert failure is None, f"{failure}: {said}"


# Passwords' copy, walked as Joel walks it: the stand-in for op lists the
# console's login (tests/desktop/stand-ins/op-stand-in.sh), whose Copy the Password copies it.
ENTRY = "Lighthouse Console"
STAND_IN = "stand-in-password"
# Passwords clears its copy this long after it's made (modules/passwords/src/main.rs).
CLEARS = timedelta(seconds=30)
# A page that takes a paste, saying what it holds in its title, which niri lists.
PAGE = (
    "data:text/html,<title>paste-page</title>"
    '<input autofocus oninput="document.title = &quot;pasted:&quot; + this.value">'
)


def copy_a_password(desktop: Desktop) -> float:
    """Copy the console's password from Passwords, and give when it pasted.

    When is monotonic, once the copy first pastes, a moment after it began.

    The first step in asks 1Password, through op, and leads with it locked until it's
    answered, so the place is opened again once it has, to stand as Joel finds it then. Its
    Copy the Password is chosen by name among the entry's acts, since the entry's own Return
    types it instead while a window is under the launcher, as a terminal or a page is here.

    A copy from before still held is cleared first, so the paste found after is this copy's
    own: a copy that never happened, or a password typed instead, can't pass on the last one.
    Any other copy is left as it is, for the step that reads what came before.
    """
    machine = desktop.machine
    desktop.as_account(
        f"if wl-paste --no-newline 2>/dev/null | grep -qx {STAND_IN}; then wl-copy --clear; fi"
    )
    machine.wait_until_fails(
        desktop.in_session(f"wl-paste --no-newline 2>/dev/null | grep -qx {STAND_IN}"),
        timeout=timedelta(seconds=10),
    )
    desktop.answer("passwords")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "collection:passwords"
    )
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: ENTRY in shown["rows"])
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher collection passwords")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == ENTRY)
    desktop.follow()
    desktop.launcher_until(lambda shown: "Copy the Password" in shown["rows"])
    desktop.down_to("Copy the Password")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(
        desktop.in_session(f"wl-paste --no-newline | grep -qx {STAND_IN}"),
        timeout=timedelta(seconds=10),
    )
    return time.monotonic()


def focused(desktop: Desktop, title: str) -> None:
    """Wait until niri's focused window is titled so."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"niri msg focused-window | grep -q {title}"), timeout=STARTING
    )


# Joel's rule for turning copying on (2026-11-16): each path it opens runs for real first. A
# password Passwords copies pastes into a terminal and into a page, a command pi fences can't
# read it while it's held, the clipboard's history never keeps it, and it's gone in 30 s.
# Each path is a case of its own, so a failure names the path that broke, and each copies
# afresh, so none rests on what another left.


def test_a_password_copied_pastes_into_a_terminal(desktop: Desktop) -> None:
    """A password copied pastes into a terminal, by its Ctrl+Shift+V."""
    machine = desktop.machine
    desktop.as_account(
        "niri msg action spawn -- kitty --title paste-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/pasted'"
    )
    focused(desktop, "paste-here")
    copy_a_password(desktop)
    focused(desktop, "paste-here")
    machine.send_key("ctrl-shift-v")
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -qx {STAND_IN} /tmp/pasted"), timeout=timedelta(seconds=10)
    )
    # Each pattern's bracket keeps it from matching the shell running it, whose own
    # command line holds the words, and which pkill would otherwise end mid-command.
    desktop.as_account("pkill -f '[c]at > /tmp/pasted'; rm -f /tmp/pasted")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q paste-here"), timeout=timedelta(seconds=30)
    )


def test_a_password_copied_pastes_into_a_page(desktop: Desktop) -> None:
    """A password copied pastes into a page, by Ctrl+V."""
    machine = desktop.machine
    # The browser is opened first, since it's slow to start here.
    desktop.as_account(
        "niri msg action spawn -- chromium --ozone-platform=wayland --disable-gpu --no-first-run "
        "--no-default-browser-check --password-store=basic "
        f"--user-data-dir=/tmp/paste-page-profile '--app={PAGE}'"
    )
    focused(desktop, "paste-page")
    copy_a_password(desktop)
    focused(desktop, "paste-page")
    machine.send_key("ctrl-v")
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg focused-window | grep -q 'pasted:{STAND_IN}'"),
        timeout=timedelta(seconds=10),
    )
    desktop.as_account("pkill -f '[p]aste-page-profile'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q pasted:"), timeout=timedelta(seconds=30)
    )


def test_a_password_copied_is_never_kept(desktop: Desktop) -> None:
    """A password copied is never kept by the clipboard's history."""
    machine = desktop.machine
    # Super+V lists no row of it, which is where a kept text copy would show, since the history
    # keeps text in memory alone; and no picture it writes to its folder holds it either. A
    # secret's copy says nothing as it's heard, so a copy after it is waited on: the history
    # hears copies in turn, so by then it has heard the password's too.
    copy_a_password(desktop)
    with desktop.told("heard copy after the password"):
        desktop.as_account("echo -n 'after the password' | wl-copy > /dev/null 2>&1")
    machine.send_key("meta_l-v")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("reads") == "after the password"
    )
    assert STAND_IN not in json.dumps(desktop.showing()), desktop.showing()["rows"]
    desktop.close_launcher()
    machine.fail(desktop.in_session(f"grep -rqF {STAND_IN} $XDG_RUNTIME_DIR/clipboard-history"))


# What wl-paste says when nothing is offered, as the clipboard stands once a copy the history
# didn't keep goes with its program (wl-clipboard's wl-paste.c), exiting 1. It's looked for as
# a line of its own, since wl-paste or libwayland may say more on lines beside it.
NOTHING_COPIED = "Nothing is copied"

# What a paste reads that lands as the copier ends: the copy's pipe closes with nothing in it,
# so wl-paste gives nothing and exits 0.
CUT_OFF = "|0"


def nothing_copied(said: str) -> bool:
    """Say whether a read of the clipboard, ending with wl-paste's code, found nothing offered."""
    told, _, code = said.rpartition("|")
    return code == "1" and NOTHING_COPIED in (line.strip() for line in told.splitlines())


# How far from its 30 s a clear may come, timed from the copy's first paste. That's a moment
# after the copy began, so a hold timed from it is never longer than the copy's own, and a clear
# come early can't pass for one held; each read takes a moment of its own on a busy host.
CLEARS_EARLY = timedelta(seconds=5)
CLEARS_LATE = timedelta(seconds=15)


def test_a_password_copied_clears_once_its_time_is_up(desktop: Desktop) -> None:
    """A password copied is held, then gone from the clipboard once its 30 s are up."""
    machine = desktop.machine
    copied = copy_a_password(desktop)
    password = f"{STAND_IN}|0"

    def held_for() -> timedelta:
        """Say how long the copy has been held, from its first paste."""
        return timedelta(seconds=time.monotonic() - copied)

    def read() -> str:
        """Say what the clipboard pastes, with what wl-paste said, and the code it ends with."""
        return desktop.as_account("wl-paste --no-newline 2>&1; printf '|%s' $?")

    # Until its time, only the password is read, so a read cut off or nothing offered then is a
    # clear come early. Then the password, or a read cut off as the copier ends, is a wait, and
    # only nothing offered is the clear, so a read that fails another way, or another copy, ends
    # the wait by what it said rather than passing for the clear. One that never comes fails
    # with what the clipboard last read.
    said = read()
    while not nothing_copied(said):
        held = held_for()
        if held < CLEARS - CLEARS_EARLY:
            assert said == password, f"the clipboard read {said!r} after only {held}"
        assert said in {password, CUT_OFF}, (
            f"the clipboard held neither the password nor none: {said!r}"
        )
        assert held <= CLEARS + CLEARS_LATE, f"no clear came in {held}: the clipboard read {said!r}"
        machine.sleep(timedelta(seconds=1))
        said = read()
    held = held_for()
    assert held >= CLEARS - CLEARS_EARLY, f"the copy went after only {held}"
    assert held <= CLEARS + CLEARS_LATE, f"the copy was held {held}"
    # The history offers a copy again a moment after the program holding one goes, so nothing
    # offered once could be that moment: a beat later, it's still nothing.
    machine.sleep(timedelta(seconds=1))
    again = read()
    assert nothing_copied(again), f"the clear didn't stay: a moment on, it read {again!r}"


def test_a_password_copied_is_never_read_fenced(desktop: Desktop) -> None:
    """A command pi fences can't read a password copied while it's held."""
    # The fence's own tests are modules/passwords/tests'; this area alone has the script.
    copy_a_password(desktop)
    said = desktop.as_account(f"fence-reads-no-copy {STAND_IN} 2>&1; echo ended $?")
    # The script says why it failed, a read copy or its own trouble, so that's passed on.
    assert said.rstrip().endswith("ended 0"), f"fence-reads-no-copy failed: {said}"


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

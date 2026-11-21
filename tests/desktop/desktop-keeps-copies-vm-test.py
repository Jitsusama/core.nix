"""The desktop test's keeps copies area.

Here the clipboard keeps what's copied, text and pictures, past the programs that copied
them, gives it back by Super+V into the window it opened over, and forgets it all when asked;
and a password Passwords copies pastes, is never kept or read by a fenced command, and clears.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import time
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
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


def test_a_fenced_command_reads_no_secret_copy_through_x11(desktop: Desktop) -> None:
    """A fenced command reads no secret copy through X11, where Joel's own client does."""
    # niri imports its X11 display as a session does, run as its child so
    # it holds what niri set; the session's start here imports only the rest.
    desktop.as_account("niri msg action spawn -- systemctl --user import-environment DISPLAY")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("systemctl --user show-environment | grep -q '^DISPLAY='")
    )
    # The script says each case it tried, and fails naming the one that
    # let the secret through (modules/pi/tests); this area alone has it.
    said = desktop.as_account("pi-fence-keeps-x11-from-secrets 2>&1; echo ended $?")
    assert said.rstrip().endswith("ended 0"), f"the fence let X11 through: {said}"


# Passwords' copy, walked as Joel walks it: the stand-in for op lists the
# console's login (desktop/op-stand-in.sh), whose Return copies its password.
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
    """Copy the console's password from Passwords by its Return, and give when, monotonic.

    The first step in asks 1Password, through op, and leads with it locked until it's
    answered, so the place is opened again once it has, to stand as Joel finds it then.
    """
    machine = desktop.machine
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
    copied = time.monotonic()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])
    machine.wait_until_succeeds(
        desktop.in_session(f"wl-paste --no-newline | grep -qx {STAND_IN}"),
        timeout=timedelta(seconds=10),
    )
    return copied


def focused(desktop: Desktop, title: str) -> None:
    """Wait until niri's focused window is titled so."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"niri msg focused-window | grep -q {title}"), timeout=STARTING
    )


# Joel's rule for turning copying on (2026-11-16): each path it opens runs for real first. A
# password Passwords copies pastes into a terminal and into a page, a command pi fences can't
# read it while it's held, the clipboard's history never keeps it, and it's gone in 30 s.


def test_a_password_copied_pastes_is_never_kept_or_fenced_and_clears(desktop: Desktop) -> None:
    """A password copied pastes, is never kept or read fenced, and clears in 30 s."""
    machine = desktop.machine
    # Into a terminal, by its Ctrl+Shift+V.
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

    # Into a page, by Ctrl+V, the browser opened first since it's slow to start here.
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

    # Never kept: Super+V lists no row of it, which is where a kept text copy would show, since
    # the history keeps text in memory alone; and no picture it writes to its folder holds it
    # either. A secret's copy says nothing as it's heard, so a copy after it is waited on: the
    # history hears copies in turn, so by then it has heard the password's too.
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

    # Gone once its time is up, and held till then.
    copied = copy_a_password(desktop)
    def cleared(_: bool) -> bool:
        """Say whether the clipboard no longer holds the password."""
        return desktop.as_account("wl-paste --no-newline 2>/dev/null || true") != STAND_IN

    retry(cleared, timeout=CLEARS + timedelta(seconds=15))
    held = timedelta(seconds=time.monotonic() - copied)
    assert held >= CLEARS - timedelta(seconds=5), f"the copy went after only {held}"

    # A fenced command can't read it while it's held (modules/passwords/tests); this area
    # alone has the script.
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

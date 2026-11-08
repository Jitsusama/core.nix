"""The desktop test's opens files area.

Here the desktop opens files: from the launcher's home and its Recent, through the portal's
chooser, Google Drive's included, and in swayimg, zathura and the reader.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import STARTING, drawn_at, laid_out, retry

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
    desktop.shoot("launcher-home")
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
    # Capture lodges nothing of another program's: what was taken is its
    # Recent, and a picture there holds Edit in GIMP after Open. It still
    # opens on the whole screen.
    desktop.as_account("qs ipc call launcher collection capture")
    desktop.launcher_ready()
    others = {"Edit the Newest Screenshot", "Look Through the Screenshots"}
    desktop.launcher_until(
        lambda shown: (
            "Recent" in shown["rows"]
            and not others & set(shown["rows"])
            and (shown["selected"] or {}).get("name") == "The Whole Screen"
        ),
    )
    desktop.down_to("Recent")
    desktop.follow()
    taken = picture.rsplit("/", 1)[-1]
    desktop.launcher_until(lambda shown: shown["rows"][:1] == [taken])
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Open", "Edit in GIMP", "Copy the Path"])
    desktop.close_launcher()
    # GIMP holds the newest screenshot among its own acts.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # GIMP holds it on its page, which Tab opens from where its name is typed.
    desktop.type_in("gnu image", selecting="GNU Image Manipulation Program")
    desktop.into_program()
    desktop.launcher_until(lambda shown: "Edit the Newest Screenshot" in shown["rows"])
    desktop.close_launcher()
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
# with the one taken, and the portal says it opened; asked again,
# closed by :q from normal, which Escape gives, the launcher opens
# nothing, and the portal says it was refused; and each ask's socket
# goes once it's answered. Each time the apps offered change
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
    desktop.shoot("open-with")
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
    desktop.close_launcher()
    machine.wait_until_succeeds(f"grep -qx 1 {portal_said}", timeout=timedelta(seconds=60))
    machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))
    machine.fail(opened_with)
    machine.succeed(f"rm {portal_png}")


# A program asks the portal for a file to open, and then to save, in a
# folder, as Chrome and Slack do, and the launcher chooses it rather than
# GTK's dialog: it opens in that folder, its path the program's title and
# then the folder, every folder on the way held above it. Normal's gx
# opens a file in its own program, the ask still waiting; what's typed
# finds a file there and Return opens it. Saving, the line holds the name
# the program gave, in insert, the caret before its extension: Ctrl+H
# deletes back, and in normal w b e 0 $ x dw cw and u edit it as Vim's
# do; j goes down into the folders, Return on one opens it, the name
# kept, and Return on the name saves it there. A name already there asks
# once, n saving nothing and y saving over it. - goes up a folder, Ctrl+O
# and Ctrl+I back and forward, and Escape at rest does nothing; :q chooses
# nothing. The portal hands the program each path picked, and each ask's
# socket goes once it's answered.


def test_the_launcher_chooses_a_file_to_open_or_save_for_the_portal(desktop: Desktop) -> None:
    """The launcher chooses a file to open or save, for the portal."""
    machine = desktop.machine
    notes = f"/home/{desktop.account}/chosen-notes"
    chose = "/run/user/1000/portal-chose"
    sockets_gone = "! ls /run/user/1000/open-with/*.sock"
    # A folder beside the notes, the next one in their folder, for ]b to go to.
    beside = f"{notes}-beside"
    # A folder inside, for a save to go down into.
    inner = f"{notes}/inner"
    desktop.as_account(
        f"mkdir -p {notes} {beside} {inner} && touch {notes}/plan.md {notes}/list.md"
        f" {notes}/.kept.md"
        " && echo iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk"
        "+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        f" | base64 -d > {notes}/look.png"
    )
    # The files, then each folder from /home down to the notes.
    held = 1 + notes.count("/")
    # The shell's watchers, each an inotify of its own, counted before
    # any ask: each folder held is read and watched only while it's asked.
    watchers = (
        "ls -l /proc/$(systemctl --user show quickshell -p MainPID --value)/fd"
        " | grep -c inotify; true"
    )
    watching = int(desktop.as_account(watchers))

    def choose(mode: str, name: str = "") -> None:
        """Ask the portal for a file in the notes, and wait until the launcher holds them."""
        title = "Save the Notes" if mode == "save" else "Open the Notes"
        machine.succeed(f"rm -f {chose}")
        desktop.as_account(
            "systemd-run --user --collect --unit=choose-a-file"
            f" {desktop.values['choose_a_file']} {mode} {notes} {name}"
        )
        # With the keys, since an ask is shown a moment before they're its.
        desktop.launcher_until(
            lambda shown: (
                shown["keys"]
                and shown["path"] == [title, "~/chosen-notes"]
                and len(shown["held"]) == held
                and shown["selected"] is not None
            ),
            timeout=timedelta(seconds=60),
        )

    def answered(*lines: str) -> None:
        """Wait until the portal said these lines, and the ask's socket went."""
        said = "\\n".join(lines)
        machine.wait_until_succeeds(
            f'[ "$(cat {chose})" = "$(printf \'{said}\')" ]', timeout=timedelta(seconds=60)
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        machine.wait_until_succeeds(sockets_gone, timeout=timedelta(seconds=10))

    choose("open")
    desktop.shoot("file-chooser")
    go_beside(desktop, held)
    goes_up_and_back(desktop, chose, held)
    open_own(desktop, chose)
    commands_open(desktop, held)
    desktop.on_find()
    machine.send_chars("plan")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "plan.md")
    machine.send_key("ret")
    answered("0", f"file://{notes}/plan.md")
    takes_a_selection(desktop, choose, answered, notes)

    choose("save", "saved.md")
    name_line(desktop, held)
    answered("0", f"file://{inner}/kept.md")

    choose("save", "plan.md")
    replace_once(desktop, chose, "plan.md")
    answered("0", f"file://{notes}/plan.md")

    saved_by_commands(desktop, choose, answered, notes)
    desktop.as_account(f"rm -r {notes} {beside}")
    # Three asks later, the shell watches no more than it did before them.
    try:
        machine.wait_until_succeeds(
            desktop.in_session(f'[ "$({watchers})" -le {watching} ]'),
            timeout=timedelta(seconds=30),
        )
    except AssertionError:
        after = desktop.as_account(watchers).strip()
        kept = f"the shell holds {after} inotify handles after three asks, {watching} before"
        raise AssertionError(kept) from None


def saved_by_commands(
    desktop: Desktop,
    choose: Callable[..., None],
    answered: Callable[..., None],
    notes: str,
) -> None:
    """Save by :w! over a name there, without asking, and by ZZ, as :x; answer none by :q."""
    choose("save", "list.md")
    ran(desktop, "w!")
    answered("0", f"file://{notes}/list.md")
    choose("save", "zz.md")
    desktop.normal()
    desktop.machine.send_chars("ZZ")
    answered("0", f"file://{notes}/zz.md")
    # :q answers nothing, a dialog's cancel, since Escape closes nothing.
    choose("open")
    ran(desktop, "q", closes=True)
    answered("1")


def takes_a_selection(
    desktop: Desktop,
    choose: Callable[..., None],
    answered: Callable[..., None],
    notes: str,
) -> None:
    """Take files by V and Return, and gather them across folders by :arga and :'<,'>arga.

    Each key waits on what the launcher says after it, so one the picker didn't take fails
    there: V's rows tinted, :arga's count in the mode word's room, and the portal's answer.
    """
    machine = desktop.machine
    beside = f"{notes}-beside"
    desktop.as_account(f"touch {beside}/far.md {beside}/near.md")

    def chosen(name: str, rows: int) -> None:
        """Wait until visual takes these many rows, the cursor on this one."""
        desktop.launcher_until(
            lambda shown: (
                shown["mode"] == "visual"
                and len(shown["tinted"]) == rows
                and (shown["selected"] or {}).get("name") == name
            )
        )

    # Return over a V selection answers with every file in it.
    choose("open-several")
    desktop.normal()
    desktop.down_to("list.md")
    machine.send_chars("V")
    chosen("list.md", 1)
    machine.send_chars("j")
    chosen("look.png", 2)
    machine.send_key("ret")
    answered("0", f"file://{notes}/list.md", f"file://{notes}/look.png")

    # :arga gathers the cursor row; in the folder beside, :'<,'>arga gathers
    # a V selection, leaving visual as Vim's : does; Return then answers with
    # all that's gathered, in order.
    choose("open-several")
    desktop.normal()
    desktop.down_to("plan.md")
    ran(desktop, "arga")
    desktop.launcher_until(lambda shown: shown["placeSays"] == "1 gathered")
    machine.send_chars("]b")
    desktop.launcher_until(lambda shown: shown["path"][-1:] == ["~/chosen-notes-beside"])
    desktop.down_to("far.md")
    machine.send_chars("Vj")
    taken = 2
    chosen("near.md", taken)
    machine.send_chars(":")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("things") == taken)
    machine.send_chars("arga")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["placeSays"] == "3 gathered"
            and shown["command"] is None
            and shown["mode"] == "normal"
        )
    )
    machine.send_key("ret")
    answered(
        "0",
        f"file://{notes}/plan.md",
        f"file://{beside}/far.md",
        f"file://{beside}/near.md",
    )


def ran(desktop: Desktop, line: str, *, closes: bool = False) -> None:
    """Type a command on the launcher's line from normal, and wait until it's run."""
    desktop.normal()
    desktop.machine.send_chars(f":{line}")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == line)
    desktop.machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            (not shown["open"])
            if closes
            else (shown["command"] is None and shown["commandSaid"] == "")
        )
    )


def commands_open(desktop: Desktop, held: int) -> None:
    """Run the picker's commands on an Open ask: :set hidden, :mkdir, :cd and :e."""
    ran(desktop, "set hidden")
    desktop.launcher_until(lambda shown: ".kept.md" in shown["rows"])
    ran(desktop, "set nohidden")
    desktop.launcher_until(lambda shown: ".kept.md" not in shown["rows"])
    ran(desktop, "mkdir made")
    desktop.launcher_until(lambda shown: "made" in shown["rows"], timeout=timedelta(seconds=30))
    ran(desktop, "cd made")
    desktop.launcher_until(
        lambda shown: (
            shown["path"][-1:] == ["~/chosen-notes/made"] and len(shown["held"]) == held + 1
        )
    )
    ran(desktop, "e ~/chosen-notes/")
    desktop.launcher_until(
        lambda shown: shown["path"][-1:] == ["~/chosen-notes"] and len(shown["held"]) == held
    )
    # Each of :cd and :e is one jump, as Vim's are, so Ctrl+O goes back to
    # the folder :cd went to and Ctrl+I comes forward again.
    jumped(desktop, "ctrl-o", "~/chosen-notes/made", held + 1)
    jumped(desktop, "ctrl-i", "~/chosen-notes", held)


def jumped(desktop: Desktop, key: str, folder: str, held: int) -> None:
    """Press Ctrl+O or Ctrl+I, and wait until the picker shows this folder, this deep."""
    desktop.machine.send_key(key)
    desktop.launcher_until(
        lambda shown: shown["path"][-1:] == [folder] and len(shown["held"]) == held
    )


def go_beside(desktop: Desktop, held: int) -> None:
    """In normal, go by ]b to the folder beside the notes, held where it was, and back by [b."""
    machine = desktop.machine
    desktop.normal()
    machine.send_chars("]b")
    desktop.launcher_until(
        lambda shown: shown["path"][-1:] == ["~/chosen-notes-beside"] and len(shown["held"]) == held
    )
    machine.send_chars("[b")
    desktop.launcher_until(
        lambda shown: shown["path"][-1:] == ["~/chosen-notes"] and len(shown["held"]) == held
    )
    # Each is one jump, as Vim's :bnext is, rather than a step out and one
    # in, so Ctrl+O goes back to the folder beside, never to their parent.
    beside = 2
    desktop.launcher_until(lambda shown: shown["jumps"] == {"back": beside, "ahead": 0})
    jumped(desktop, "ctrl-o", "~/chosen-notes-beside", held)
    jumped(desktop, "ctrl-i", "~/chosen-notes", held)


def goes_up_and_back(desktop: Desktop, chose: str, held: int) -> None:
    """Go up by -, back by Ctrl+O and Ctrl+I and in by Return; Escape at rest does nothing.

    Escape in normal at rest leaves the ask waiting, as it closes nothing (Joel, 2026-11-05), so
    the - after it still finds the picker. Each - goes to the folder holding this one, the
    cursor on the folder it came from, up to /, where - has nowhere to go and says so rather
    than leaving the ask for the launcher's home. l and h mean nothing on a row, saying Return
    and - instead, and the arrows are Vim's: Down and Up move as j and k, Right and Left nowhere.
    """
    machine = desktop.machine
    # In /home, the account's own folder is listed as home is said everywhere, ~.
    home = "~"

    def stands(folder: str, depth: int, name: str) -> None:
        """Wait until the picker shows this folder this deep, the cursor on this row."""
        desktop.launcher_until(
            lambda shown: (
                shown["open"]
                and shown["path"][-1:] == [folder]
                and len(shown["held"]) == depth
                and (shown["selected"] or {}).get("name") == name
            )
        )

    def says(*saying: str) -> None:
        """Wait until the foot says this for the key just pressed."""
        desktop.launcher_until(lambda shown: shown["saying"] == list(saying))

    desktop.normal()
    machine.send_key("esc")
    machine.send_chars("-")
    stands("~", held - 1, "chosen-notes")
    machine.fail(f"test -e {chose}")
    machine.send_chars("l")
    says("l not here", "\u21b5 open")
    machine.send_chars("h")
    says("h not here", "- up")
    # The arrows beside go nowhere either, whatever the launcher says of them.
    for key in ["right", "left"]:
        machine.send_key(key)
        desktop.launcher_until(lambda shown: shown["saying"][-1:] != ["- up"])
        stands("~", held - 1, "chosen-notes")
        machine.send_chars("h")
        says("h not here", "- up")
    at = desktop.showing()["current"]
    machine.send_key("down")
    desktop.launcher_until(lambda shown: shown["current"] == at + 1)
    machine.send_key("up")
    stands("~", held - 1, "chosen-notes")
    machine.send_chars("-")
    stands("/home", held - 2, home)
    jumped(desktop, "ctrl-o", "~", held - 1)
    stands("~", held - 1, "chosen-notes")
    jumped(desktop, "ctrl-i", "/home", held - 2)
    machine.send_chars("-")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == "home"
    )
    machine.send_chars("-")
    says("- not here")
    # The dialog's top is still a place: the path says it is /.
    top = desktop.showing()
    assert len(top["held"]) == 1, top["held"]
    assert top["path"][-1:] == ["/"], top["path"]
    # Return goes back down, folder by folder, to where the ask opened.
    for folder, name in [("/home", home), ("~", "chosen-notes")]:
        machine.send_key("ret")

        def inside(shown: Json, folder: str = folder) -> bool:
            """Say whether the picker shows this folder, a row selected."""
            return bool(shown["path"][-1:] == [folder] and shown["selected"] is not None)

        desktop.launcher_until(inside)
        desktop.down_to(name)
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["path"][-1:] == ["~/chosen-notes"] and len(shown["held"]) == held
    )


def open_own(desktop: Desktop, chose: str) -> None:
    """Open look.png in its own program by gx, and wait until the ask has the keys again."""
    machine = desktop.machine
    desktop.down_to("look.png")
    machine.send_chars("gx")
    looking = f"pgrep -u {desktop.account} -f '[s]wayimg.*look.png'"
    machine.wait_until_succeeds(looking, timeout=STARTING)
    machine.succeed(f"pkill -KILL -u {desktop.account} -f '[s]wayimg.*look.png'")
    machine.wait_until_fails(looking, timeout=timedelta(seconds=30))
    # Opened as Return opens it, the portal would have its answer.
    machine.fail(f"test -e {chose}")
    desktop.launcher_until(
        lambda shown: shown["keys"] and shown["path"][-1:] == ["~/chosen-notes"],
        timeout=timedelta(seconds=30),
    )


def replace_once(desktop: Desktop, chose: str, name: str) -> None:
    """Save as a name already there: Return asks once, n saves nothing, and y saves over it."""
    machine = desktop.machine
    before = name.rsplit(".", 1)[0]
    desktop.launcher_until(lambda shown: shown["typed"] == name and shown["caret"] == len(before))
    asked = f"Replace {name}? (y/n)"
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["question"] == asked and shown["takesAnswer"])
    machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["question"] == "" and shown["open"])
    # Saved over at once, the portal would have its answer by now.
    machine.fail(f"test -e {chose}")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["question"] == asked and shown["takesAnswer"])
    machine.send_chars("y")


def name_line(desktop: Desktop, held: int) -> None:
    """Edit a save's name, saved.md, with every key its line takes, and save it in inner.

    Each key waits on what the line says after it, so one the line didn't take fails there.
    """
    machine = desktop.machine

    def says(typed: str, caret: int, mode: str) -> None:
        """Wait until the line holds these words, its caret here, in this mode."""
        desktop.launcher_until(
            lambda shown: (
                shown["typed"] == typed
                and shown["caret"] == caret
                and shown["mode"] == mode
                and (shown["selected"] or {}).get("name") == typed
            )
        )

    says("saved.md", 5, "insert")
    machine.send_key("ctrl-h")
    says("save.md", 4, "insert")
    machine.send_chars("d")
    says("saved.md", 5, "insert")
    machine.send_key("esc")
    says("saved.md", 4, "normal")
    for key, typed, caret in [
        ("0", "saved.md", 0),
        ("x", "aved.md", 0),
        ("u", "saved.md", 0),
        ("$", "saved.md", 7),
        ("b", "saved.md", 6),
        ("0", "saved.md", 0),
        ("e", "saved.md", 4),
        ("w", "saved.md", 5),
        ("0", "saved.md", 0),
    ]:
        machine.send_chars(key)
        says(typed, caret, "normal")
    machine.send_chars("dw")
    says(".md", 0, "normal")
    machine.send_chars("u")
    says("saved.md", 0, "normal")
    machine.send_chars("cw")
    says(".md", 0, "insert")
    machine.send_chars("kept")
    says("kept.md", 4, "insert")
    machine.send_key("esc")
    says("kept.md", 3, "normal")
    for key, caret in [("h", 2), ("l", 3)]:
        machine.send_chars(key)
        says("kept.md", caret, "normal")
    # The name is the line's row, above the rows, never listed among them;
    # j goes down from it into the folders and k back up to it, where h and l
    # move its caret again. In a folder, Return steps in, the name kept on
    # the line.
    rows = desktop.showing()["rows"]
    assert "kept.md" not in rows, rows
    down = "j" * (rows.index("inner") + 1)
    machine.send_chars(down)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "inner")
    machine.send_chars("k" * len(down))
    desktop.launcher_until(
        lambda shown: shown["current"] == -1 and (shown["selected"] or {}).get("name") == "kept.md"
    )
    machine.send_chars("h")
    says("kept.md", 2, "normal")
    machine.send_chars(down)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "inner")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) == held + 1
            and shown["typed"] == "kept.md"
            and (shown["selected"] or {}).get("name") == "kept.md"
        )
    )
    machine.send_key("ret")


def test_the_launchers_picker_fetches_a_file_from_google_drive_for_the_portal(
    desktop: Desktop,
) -> None:
    """The launcher's picker finds Google Drive, steps into it and hands over a file fetched."""
    machine = desktop.machine
    notes = f"/home/{desktop.account}/drive-notes"
    chose = "/run/user/1000/portal-chose"
    # The stand-in Google fetches a file into its own folder, its bytes its id.
    fetched = "/run/user/1000/google-files/drive/menu/menu.pdf"
    desktop.as_account("systemctl --user restart google.service")
    try:
        # The shell hears the stand-in before the picker asks: its meeting is found.
        desktop.answer("design review")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Design Review"
        )
        desktop.close_launcher()
        machine.succeed(f"rm -f {chose}")
        desktop.as_account(f"mkdir -p {notes} && touch {notes}/plan.md")
        desktop.as_account(
            "systemd-run --user --collect --unit=choose-from-drive"
            f" {desktop.values['choose_a_file']} open {notes}"
        )
        desktop.launcher_until(
            lambda shown: shown["path"] == ["Open the Notes", "~/drive-notes"],
            timeout=timedelta(seconds=60),
        )
        # Typed in any folder, Google Drive is found, and steps in to its three.
        machine.send_chars("drive")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Google Drive"
        )
        desktop.follow()
        desktop.launcher_until(
            lambda shown: shown["rows"][:3] == ["My Drive", "Shared with Me", "Shared Drives"]
        )
        desktop.down_to("My Drive")
        desktop.follow()
        desktop.launcher_until(lambda shown: "menu.pdf" in shown["rows"])
        desktop.shoot("file-chooser-drive")
        desktop.down_to("menu.pdf")
        machine.send_key("ret")
        # Fetched first, then handed over, and the launcher closes.
        machine.wait_until_succeeds(
            f'[ "$(cat {chose})" = "$(printf \'0\\nfile://{fetched}\')" ]',
            timeout=timedelta(seconds=60),
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        assert desktop.as_account(f"cat {fetched}") == "menu", fetched
    finally:
        desktop.as_account("systemctl --user stop google.service")
        desktop.as_account(f"rm -rf {notes}")


def test_the_launchers_picker_saves_into_google_drive_for_the_portal(
    desktop: Desktop,
) -> None:
    """The launcher's picker saves into Google Drive by the path Google's program uploads from."""
    machine = desktop.machine
    notes = f"/home/{desktop.account}/drive-saves-notes"
    chose = "/run/user/1000/portal-chose"
    staged = f"/home/{desktop.account}/.local/state/google/drive-saves/root"
    desktop.as_account("systemctl --user restart google.service")
    try:
        desktop.answer("design review")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Design Review"
        )
        desktop.close_launcher()
        machine.succeed(f"rm -f {chose}")
        desktop.as_account(f"rm -rf {staged} && mkdir -p {notes}")
        desktop.as_account(
            "systemd-run --user --collect --unit=save-into-drive"
            f" {desktop.values['choose_a_file']} save {notes} draft.md"
        )
        desktop.launcher_until(
            lambda shown: shown["path"] == ["Save the Notes", "~/drive-saves-notes"],
            timeout=timedelta(seconds=60),
        )
        # The line holds the name, so - steps out to home with it, where
        # Google Drive is listed, and holds only what a file can be saved in.
        desktop.normal()
        machine.send_chars("-")
        desktop.launcher_until(
            lambda shown: shown["path"] == ["Save the Notes", "~"] and shown["typed"] == "draft.md"
        )
        desktop.down_to("Google Drive")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:2] == ["My Drive", "Shared Drives"])
        desktop.down_to("My Drive")
        desktop.step_in("ret")
        # My Drive's line is the name given, saved by Return.
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "draft.md")
        desktop.shoot("file-chooser-drive-save")
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f'[ "$(cat {chose})" = "$(printf \'0\\nfile://{staged}/draft.md\')" ]',
            timeout=timedelta(seconds=60),
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.as_account(f"test -d {staged}")
    finally:
        # An ask left open would hold the launcher for the next subtest.
        desktop.as_account("systemctl --user stop save-into-drive.service || true")
        desktop.close_launcher()
        desktop.as_account("systemctl --user stop google.service")
        desktop.as_account(f"rm -rf {notes} {staged}")


def test_a_picture_opens_floating_in_swayimg_and_e_takes_it_to_gimp(desktop: Desktop) -> None:
    """A picture opens floating in swayimg, and e takes it to GIMP."""
    machine = desktop.machine
    # Beside the screenshots, never among them, since the region
    # screenshot's subtest waits for a picture there of its own.
    looked = f"/home/{desktop.account}/snowflake.png"
    desktop.as_account(f"cp {desktop.values['picture']} {looked}")
    desktop.opens_in(looked, "image/png", "swayimg")
    try:
        swayimg = desktop.window_titled("Swayimg: snowflake.png")
        assert swayimg["is_floating"], swayimg
        # The screen less the screen rung and its border on each side,
        # centred, as every floating frame stands. Without swayimg's size
        # given, it's its own 1280 across, off a laptop's screen.
        left, _, width, _ = drawn_at(swayimg)
        output = json.loads(desktop.as_account("niri msg --json focused-output"))
        wide = output["logical"]["width"]
        assert abs(width - (wide - desktop.values["floating_room"])) <= wide * 0.02, swayimg
        assert abs(left + width / 2 - wide / 2) <= wide * 0.02, swayimg
        # The steps before leave notices, which go on their own.
        machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=timedelta(seconds=30),
        )
        desktop.shoot("swayimg")
        machine.send_key("e")
        machine.wait_until_succeeds(
            f"pgrep -u {desktop.account} -f '[g]imp.*{looked}'", timeout=STARTING
        )
        machine.wait_until_fails(
            f"pgrep -u {desktop.account} -x swayimg", timeout=timedelta(seconds=10)
        )
    finally:
        # A swayimg left open would be the next subtest's, and GIMP goes
        # before it draws, as in the screenshot's subtest.
        machine.succeed(f"pkill -KILL -u {desktop.account} -x swayimg || true")
        machine.succeed(f"pkill -KILL -u {desktop.account} -f '[g]imp' || true")
        machine.wait_until_fails(
            f"pgrep -u {desktop.account} -x swayimg || pgrep -u {desktop.account} -f '[g]imp'",
            timeout=timedelta(seconds=30),
        )
        machine.succeed(f"rm -f {looked}")


def test_swayimg_typed_lists_the_picture_it_shows_under_it_and_return_steps_in(
    desktop: Desktop,
) -> None:
    """Swayimg, hidden by its own entry, is a program whose picture is listed under it typed."""
    machine = desktop.machine
    looked = f"/home/{desktop.account}/tidepool.png"
    desktop.as_account(f"cp {desktop.values['picture']} {looked}")
    desktop.opens_in(looked, "image/png", "swayimg")
    try:
        desktop.answer("swayimg")

        def under_swayimg(shown: Json) -> bool:
            """Say whether swayimg is selected, saying it has one open, the picture under it."""
            rows = shown["rows"]
            selected = shown["selected"] or {}
            if selected.get("is") != "program:swayimg":
                return False
            at = rows.index(selected["name"])
            note = str(selected.get("note", ""))
            return "tidepool.png" in rows[at + 1 : at + 4] and note.startswith("1 open")

        desktop.launcher_until(under_swayimg)
        # The picture it shows is listed once, as open, and so never counted as recent too.
        assert desktop.showing()["selected"]["note"] == "1 open", desktop.showing()["selected"]
        desktop.shoot("launcher-swayimg-typed")
        # Starting it would open nothing, so Return steps into its page,
        # which holds its Recent and its keys, and never starts it.
        desktop.step_in("ret")
        desktop.launcher_until(
            lambda shown: "Recent" in shown["rows"] and "Shortcuts" in shown["rows"]
        )
    finally:
        desktop.close_launcher()
        machine.succeed(f"pkill -KILL -u {desktop.account} -x swayimg || true")
        machine.wait_until_fails(
            f"pgrep -u {desktop.account} -x swayimg", timeout=timedelta(seconds=30)
        )
        machine.succeed(f"rm -f {looked}")


def test_reader_typed_lists_the_page_it_shows_under_it_and_tab_opens_its_page(
    desktop: Desktop,
) -> None:
    """The reader typed lists the page it shows a depth in under it; Tab opens its page."""
    machine = desktop.machine
    read = f"/home/{desktop.account}/tides.md"
    desktop.as_account(f"printf '# Tides\\n\\nLow at noon.\\n' > {read}")
    desktop.opens_in(read, "text/markdown", "reader")
    try:
        desktop.answer("reader")

        def under_reader(shown: Json) -> bool:
            """Say whether Reader is selected, saying it has one open, the page under it."""
            rows = shown["rows"]
            selected = shown["selected"] or {}
            if selected.get("is") != "program:reader":
                return False
            at = rows.index(selected["name"])
            note = str(selected.get("note", ""))
            return "tides.md" in rows[at + 1 : at + 4] and note.startswith("1 open")

        desktop.launcher_until(under_reader)
        # Tab opens its page, Go To first, as Return goes to it, and its Recent there.
        desktop.into_program()
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Go To" and "Recent" in shown["rows"]
            )
        )
    finally:
        desktop.close_launcher()
        # Its wrapper names the process otherwise, so it's found by what it reads.
        machine.succeed(f"pkill -KILL -u {desktop.account} -f '[r]eader.*{read}' || true")
        machine.wait_until_fails(
            f"pgrep -u {desktop.account} -f '[r]eader.*{read}'", timeout=timedelta(seconds=30)
        )
        machine.succeed(f"rm -f {read}")


def test_zathura_typed_lists_the_document_it_shows_under_it_and_tab_opens_its_page(
    desktop: Desktop,
) -> None:
    """Zathura typed lists the document it shows a depth in under it; Tab opens its page."""
    machine = desktop.machine
    read = f"/home/{desktop.account}/charts.pdf"
    desktop.as_account(f"cp {desktop.values['page']} {read}")
    desktop.opens_in(read, "application/pdf", "org.pwmt.zathura")
    try:
        desktop.answer("zathura")

        def under_zathura(shown: Json) -> bool:
            """Say whether Zathura is selected, saying it has one open, the document under it."""
            rows = shown["rows"]
            selected = shown["selected"] or {}
            if selected.get("is") != "program:org.pwmt.zathura":
                return False
            at = rows.index(selected["name"])
            note = str(selected.get("note", ""))
            return "charts.pdf" in rows[at + 1 : at + 4] and note.startswith("1 open")

        desktop.launcher_until(under_zathura)
        desktop.into_program()
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Go To" and "Recent" in shown["rows"]
            )
        )
    finally:
        desktop.close_launcher()
        # Its wrapper names the process otherwise, so it's found by its command line.
        machine.succeed(f"pkill -KILL -u {desktop.account} -f '[z]athura' || true")
        machine.wait_until_fails(
            f"pgrep -u {desktop.account} -f '[z]athura'", timeout=timedelta(seconds=30)
        )
        machine.succeed(f"rm -f {read}")


def test_colon_in_swayimg_opens_the_command_line_where_q_closes_it(desktop: Desktop) -> None:
    """: in swayimg opens the launcher's command line low, under the picture, and :q closes it."""
    machine = desktop.machine
    looked = f"/home/{desktop.account}/commanded.png"
    desktop.as_account(f"cp {desktop.values['picture']} {looked}")
    desktop.opens_in(looked, "image/png", "swayimg")
    machine.send_key("shift-semicolon")
    desktop.launcher_until(
        lambda shown: (
            shown["keys"]
            and {"rotate Turn it, by 90, 180 or 270 degrees", "{n} That picture, by its number"}
            <= set((shown["command"] or {}).get("menu", []))
            and (shown["command"] or {}).get("over") == ["commanded.png"]
            # It sits low, its foot a twelfth of the screen up, so the
            # picture stays in view above it.
            and abs(shown["card"]["foot"] - shown["card"]["screen"] * 11 / 12) <= 1
        )
    )
    desktop.shoot("swayimg-command-line")
    machine.send_chars("q")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "q")
    machine.send_key("ret")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -x swayimg", timeout=timedelta(seconds=10)
    )
    desktop.launcher_until(lambda shown: not shown["open"])
    machine.succeed(f"rm -f {looked}")


def test_a_document_opens_in_zathura_beside_the_rest_and_zz_closes_it(desktop: Desktop) -> None:
    """A document opens in zathura, beside the rest, and ZZ closes it, as q no longer does."""
    machine = desktop.machine
    desktop.opens_in(desktop.values["page"], "application/pdf", "org.pwmt.zathura")
    windows = json.loads(desktop.as_account("niri msg --json windows"))
    zathura = next(window for window in windows if window["app_id"] == "org.pwmt.zathura")
    assert not zathura["is_floating"], zathura
    desktop.shoot("zathura")
    machine.send_key("shift-z")
    machine.send_key("shift-z")
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[z]athura'", timeout=timedelta(seconds=10)
    )


def test_a_markdown_file_opens_in_the_reader_and_takes_neovims_keys(desktop: Desktop) -> None:
    """A markdown file opens in the reader, which takes Neovim's keys and reopens where it was.

    Where the reader was when it closed is kept beside its state, so what each key did is
    read back from there: G and ZZ keep it at the end, opening again comes back to the end,
    and gg then Ctrl+D then ZZ keep it half a screen down.
    """
    machine = desktop.machine
    page = f"/home/{desktop.account}/page.md"
    desktop.as_account(
        f"{{ echo '# A Page'; for n in $(seq 120); do echo; "
        f'echo "Paragraph $n, a line or two of words for the keys to go past."; done; }} > {page}'
    )

    reader = reader_opened(desktop, page)
    desktop.shoot("reader")
    machine.send_key("shift-g")
    close_the_reader(desktop)
    assert reader_closed_at(desktop, reader)["at_end"], "G didn't go to the end, or ZZ didn't close"

    reader = reader_opened(desktop, page)
    close_the_reader(desktop)
    assert reader_closed_at(desktop, reader)["at_end"], "the reader didn't open where it closed"

    reader = reader_opened(desktop, page)
    machine.send_key("g")
    machine.send_key("g")
    machine.send_key("ctrl-d")
    close_the_reader(desktop)
    kept = reader_closed_at(desktop, reader)
    assert not kept["at_end"], kept
    assert kept["block"] != 0 or kept["into_pt"] != 0, "Ctrl+D didn't move from the top"
    machine.succeed(f"rm {page}")


def test_the_reader_closed_straight_after_a_jump_keeps_its_place_and_dies_in_nothing(
    desktop: Desktop,
) -> None:
    """The reader closed straight after a jump keeps its place and dies in nothing.

    Closing just after G used to close it while its loader still made the landing's textures, and
    the driver's own teardown at exit freed what the loader used: it died in Mesa now and
    then, after keeping its place or before. Opened on a long page, G then ZZ at once, again
    and again, keeps the end each time, and no thread of the reader faults.
    """
    machine = desktop.machine
    page = f"/home/{desktop.account}/long.md"
    desktop.as_account(
        f"{{ echo '# A Long Page'; for n in $(seq 600); do echo; "
        f'echo "Paragraph $n, a line or two of words for the jump to land past."; done; }} > {page}'
    )
    for _ in range(8):
        desktop.as_account("rm -f ~/.local/state/reader/places/*.json")
        reader = reader_opened(desktop, page)
        machine.send_key("shift-g")
        close_the_reader(desktop)
        assert reader_closed_at(desktop, reader)["at_end"], "G then ZZ at once didn't keep the end"
    machine.succeed(f"rm {page}")


def close_the_reader(desktop: Desktop) -> None:
    """Close the reader as Neovim closes, by ZZ: q does nothing, since nothing records."""
    desktop.machine.send_key("shift-z")
    desktop.machine.send_key("shift-z")


def reader_opened(desktop: Desktop, page: str) -> tuple[str, int]:
    """Open a page in the reader as a click would, and give its process id and kernel lines.

    Its command line is the inner wrapper's path and the file, since each wrapper script
    starts the next by its path, so a pattern of the command's own name never matches it:
    the reader is known by its id instead. The kernel's lines so far are counted, so only
    what this reader did is read for faults.
    """
    kernel_lines = int(desktop.machine.succeed("dmesg | wc -l"))
    desktop.opens_in(page, "text/markdown", "reader")
    pid = desktop.as_account("pgrep -u $(id -u) -n -x .reader-wrapped").strip()
    return pid, kernel_lines


def reader_closed_at(desktop: Desktop, reader: tuple[str, int]) -> dict[str, object]:
    """Where the reader closed, once its process has gone, having faulted nowhere."""
    machine = desktop.machine
    pid, kernel_lines = reader
    # Gone, or ended and not yet reaped.
    machine.wait_until_fails(
        f"test -e /proc/{pid}/status && ! grep -q '^State:[[:space:]]*Z' /proc/{pid}/status",
        timeout=timedelta(seconds=10),
    )
    faults = machine.execute(
        f"dmesg | tail -n +{kernel_lines + 1} | grep -E"
        " ' (\\.reader-wrapped|loader|painter-[0-9]+|setter|reloader)"
        "\\[[0-9]+\\]: segfault'"
    )[1]
    assert not faults, f"a thread of the reader faulted:\n{faults}"
    kept: dict[str, object] = json.loads(
        desktop.as_account("cat ~/.local/state/reader/places/*.json")
    )
    return kept


def test_the_reader_started_from_programs_shows_what_to_read(desktop: Desktop) -> None:
    """The reader started from the launcher's Programs, with no file, shows what to read.

    It used to exit at once, saying only how it's started, so choosing it did nothing. Now it
    sets a page of markdown files, each a link, says it shows none of his files while that
    page is up, and g x then the first link's letter opens that file in the same window.
    """
    machine = desktop.machine
    notes = f"/home/{desktop.account}/notes"
    desktop.as_account(
        f"mkdir -p {notes} && printf '# To Read\\n\\nWords.\\n' > {notes}/to-read.md"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("reader", selecting="Reader")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session('niri msg --json focused-window | grep -q \'"app_id":"reader"\''),
        timeout=STARTING,
    )
    pid = desktop.as_account("pgrep -u $(id -u) -n -x .reader-wrapped").strip()
    start = desktop.as_account("cat /run/user/$(id -u)/reader/start.md")
    assert start.startswith("# What to Read"), start
    assert f"(<{notes}/to-read.md>)" in start, start
    desktop.shoot("reader-start")
    showing = f"~/.local/state/reader/showing/{pid}"
    machine.fail(desktop.in_session(f"test -s {showing}"))
    first = re.search(r"\]\(<(.+?)>\)", start)
    assert first, start
    machine.send_key("g")
    machine.send_key("x")
    machine.send_key("a")
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -qF {first.group(1)} {showing}"), timeout=timedelta(seconds=20)
    )
    close_the_reader(desktop)
    machine.wait_until_fails(
        f"pgrep -u {desktop.account} -f '[.]reader-wrapped'", timeout=timedelta(seconds=10)
    )
    machine.succeed(f"rm -r {notes}")


# kitty's Recent holds the folders zoxide has learned, read again as
# zoxide's database changes, even one it made after the shell started.
# Return on one opens a kitty window there through the kitty already open,
# over its socket, rather than starting another.


def test_chrome_typed_lists_its_tabs_then_its_sites_under_it_and_tab_opens_its_page(
    desktop: Desktop,
) -> None:
    """Chrome typed lists its tabs, then the sites it visited, a depth in under it; Tab goes in.

    The machine's Chrome is a stand-in whose lists are files the test writes, in the form
    chrome-recent-sites prints them: a tab a line (its id, window, whether it's in front, when
    it was looked at, its address and title), and a site a line (its address and title). Closed,
    the tab its session kept is a site it visited, never open beside a Return that starts it;
    with a window, the tab is open, Return goes to Chrome, and its page opens on Go To.
    """
    visited = "/run/user/1000/chrome-visited"
    open_now = "/run/user/1000/chrome-open"
    sites = "https://estuary.example.org/\tEstuary Notes\nhttps://reef.example.org/\tReef Survey\n"
    tab = "7\t1\ttrue\t1794070000000\thttps://tide.example.org/\tTide Tables\n"
    desktop.as_account(f"printf '%s' {shlex.quote(sites)} > {visited}")
    desktop.as_account(f"printf '%s' {shlex.quote(tab)} > {open_now}")
    window = "kitty --class google-chrome --title 'Tide Tables - Google Chrome'"

    def under_chrome(note: str, verb: str) -> Callable[[Json], bool]:
        """Say whether Chrome is selected, saying this, with its tab and its sites under it."""

        def holds(shown: Json) -> bool:
            rows = shown["rows"]
            selected = shown["selected"] or {}
            if selected.get("is") != "program:google-chrome":
                return False
            at = rows.index("Google Chrome")
            listed = rows[at + 1 : at + 4] == ["Tide Tables", "Estuary Notes", "Reef Survey"]
            return listed and (selected.get("note"), selected.get("verb")) == (note, verb)

        return holds

    try:
        desktop.answer("chrome")
        desktop.launcher_until(under_chrome("3 recent", "start"))
        # Return's place is as wide as the verbs the rows listed say, so the slack before it
        # is less than one of them: held as wide as every verb the words name, it left a gap
        # wider than "start" itself (VKCF3Z, 2026-11-07).
        foot = laid_out(desktop)["foot"]
        into = next(at for at, word in enumerate(foot) if word["text"] == "Tab")
        tab_in, after = foot[into + 1], foot[into + 2]
        verb = next(word for word in foot[into + 2 :] if word["text"] == "start")
        slack = after["starts"] - (tab_in["starts"] + tab_in["width"])
        assert slack < verb["width"], (slack, foot)
        desktop.close_launcher()
        desktop.as_account(f"niri msg action spawn -- {window}")
        desktop.machine.wait_until_succeeds(
            desktop.in_session("niri msg focused-window | grep -q google-chrome"),
            timeout=STARTING,
        )
        desktop.answer("chrome")
        desktop.launcher_until(under_chrome("1 tab \u00b7 2 recent", "go to"))
        # Each sits a depth in under Chrome, which is all that ties it there.
        rows = laid_out(desktop)["rows"]
        depths = {row["reads"].split(" \u00b7 ")[0]: row["depth"] for row in rows}
        # Its say counts the tab, so its kind alone follows its name, never "1 window" too.
        chrome = next(row["reads"] for row in rows if row["reads"].startswith("Google Chrome"))
        assert chrome.split(" \u00b7 ")[1] == "web browser", chrome
        assert [depths.get(name) for name in ("Google Chrome", "Tide Tables", "Reef Survey")] == [
            0,
            1,
            1,
        ], depths
        desktop.shoot("launcher-chrome-typed")
        # Tab opens Chrome's page, its first row Return's own, and its Recent in it.
        desktop.into_program()
        desktop.launcher_until(
            lambda shown: (
                len(shown["held"]) == 1 and shown["current"] == 0 and "Recent" in shown["rows"]
            )
        )
        assert desktop.showing()["selected"]["name"] == "Go To", desktop.showing()["selected"]
        desktop.shoot("launcher-chrome-page")
    finally:
        desktop.close_launcher()
        # The bracket keeps pkill from matching the shell that runs it.
        desktop.as_account(f"pkill -f 'kitty --clas[s] google-chrome'; rm -f {visited} {open_now}")


def test_kitty_typed_lists_the_folders_it_visited_under_it(desktop: Desktop) -> None:
    """Kitty typed lists the folders zoxide learned under it, kitty first, so Return switches."""
    folder = f"/home/{desktop.account}/src/estuary"
    desktop.as_account(f"mkdir -p {folder} && zoxide add {folder}")
    try:
        desktop.answer("kitty")

        def under_kitty(shown: Json) -> bool:
            """Say whether kitty is selected with the folder among the rows just under it."""
            rows = shown["rows"]
            if "kitty" not in rows or (shown["selected"] or {}).get("name") != "kitty":
                return False
            at = rows.index("kitty")
            # And kitty says what it holds, as the cursor reads it.
            said = re.search(r"\d+ recent", str(shown["selected"].get("note", "")))
            return "estuary" in rows[at + 1 : at + 4] and said is not None

        desktop.launcher_until(under_kitty)
        desktop.shoot("launcher-kitty-typed")
    finally:
        desktop.close_launcher()
        desktop.as_account(f"zoxide remove {folder} && rmdir {folder}")


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
    desktop.into_program()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(lambda shown: "tide" in shown["rows"])
    desktop.shoot("launcher-kitty-recent")
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

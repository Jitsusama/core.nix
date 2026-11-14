"""The desktop test's opens files area.

Here the desktop opens files: from the launcher's home and its Recent, through the portal's
chooser, Google Drive's included, and in swayimg, zathura and the reader; and goes to one a
Neovim in kitty holds, from Neovim's page.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    LATEST_DOWNLOAD,
    STARTING,
    asked_of,
    chrome_download_gone,
    chrome_downloaded,
    chrome_gone,
    chrome_opened,
    chrome_visited,
    drawn_at,
    laid_out,
    neovim_in_kitty,
    neovims_quit,
    retry,
)

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
    home = ["Now", "Agents", "Programs", "People", "Passwords", "Desktop", "Machine", "Network",
            "Keyboard"]  # fmt: skip
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
    desktop.type_in("gnu image", selecting="GIMP")
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
    """Save as a name already there: Return asks once, n saves nothing, and Return saves over."""
    machine = desktop.machine
    before = name.rsplit(".", 1)[0]
    desktop.launcher_until(lambda shown: shown["typed"] == name and shown["caret"] == len(before))
    asked = f"Replace {name}?"
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["question"] == asked and shown["takesAnswer"])
    machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["question"] == "" and shown["open"])
    # Saved over at once, the portal would have its answer by now.
    machine.fail(f"test -e {chose}")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["question"] == asked and shown["takesAnswer"])
    machine.send_key("ret")


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
    """The launcher's picker finds Google Drive, opens a file by gx, and hands over one fetched."""
    machine = desktop.machine
    notes = f"/home/{desktop.account}/drive-notes"
    chose = "/run/user/1000/portal-chose"
    # The stand-in Google fetches a file into the cache, as the program does, its bytes its id.
    fetched = f"/home/{desktop.account}/.cache/google/drive/menu/menu.pdf"
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
        # One of Google's own kinds says at the foot the file a program is handed.
        desktop.normal()
        desktop.down_to("Quarterly Plan")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Quarterly Plan"
                and shown["footer"] == "as .docx"
            )
        )
        desktop.down_to("menu.pdf")
        # Its row says its size and when it changed, as a file here does.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("side", "").startswith("4\u00a0B \u00b7 ")
        )
        desktop.shoot("file-chooser-drive-file")
        # Normal's gx fetches it and opens it in its own program, the ask still waiting.
        machine.send_chars("gx")
        reading = f"pgrep -u {desktop.account} -f '[z]athura.*menu.pdf'"
        machine.wait_until_succeeds(reading, timeout=STARTING)
        machine.succeed(f"pkill -KILL -u {desktop.account} -f '[z]athura.*menu.pdf'")
        machine.wait_until_fails(reading, timeout=timedelta(seconds=30))
        machine.fail(f"test -e {chose}")
        desktop.launcher_until(
            lambda shown: shown["keys"] and (shown["selected"] or {}).get("name") == "menu.pdf",
            timeout=timedelta(seconds=30),
        )
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
        # which holds its Recent and its keys, and never starts it. It lands
        # on something to look through: the screenshots' gallery while there
        # are any, else the gallery of the folder of the picture it showed,
        # which is offered either way.
        desktop.step_in("ret")
        home = "Look Through ~"
        leads = {"Look Through the Screenshots", home}
        desktop.launcher_until(
            lambda shown: (
                {"Recent", "Shortcuts", home} <= set(shown["rows"])
                and (shown["selected"] or {}).get("name") in leads
            )
        )
        # It says how many pictures the folder holds, once they're counted,
        # its drawer the folder's plate, previewing its newest picture, the
        # one just shown, and Return opens its gallery on the picture, newest
        # first.
        desktop.down_to(home)
        desktop.launcher_until(
            lambda shown: (
                re.fullmatch(r"\d+[\xa0 ]pictures?", (shown["selected"] or {}).get("side", ""))
                is not None
                and (shown.get("drawer") or {}).get("out") is True
                and shown["drawer"]["facts"] == ["Newest", "Added", "Pictures"]
                and (shown["drawer"]["preview"] or {}).get("picture") == looked
            )
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"pgrep -u {desktop.account} -f '[s]wayimg --gallery .*{looked}'",
            timeout=timedelta(seconds=15),
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
    """Zathura typed lists the document it shows under it; Tab opens its page, saying its page."""
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
        # Its row says the page zathura says it's on, read over its bus,
        # in open's place, and no folder, since it's in the home.
        desktop.down_to("charts.pdf")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "p. 1 of 1")
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
    and gg then Ctrl+D then ZZ keep it half a screen down, with the mark ma set there. The
    launcher's Recent in the reader's page then leads with the file, saying beside it the
    heading it was left under, how far down and when, as reader --places lists it on the
    machine, the mark a row of its own inside it, and Return there reads on where it was left.
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
    machine.send_chars("ma")
    close_the_reader(desktop)
    kept = reader_closed_at(desktop, reader)
    assert not kept["at_end"], kept
    assert kept["block"] != 0 or kept["into_pt"] != 0, "Ctrl+D didn't move from the top"
    assert kept.get("heading") == "A Page", kept
    marks = cast("list[Json]", kept.get("marks", []))
    assert [mark.get("letter") for mark in marks] == ["a"], kept

    left = re.compile("^A Page \u00b7 \\d{1,2}% \u00b7 (just now|\\d+\u00a0min ago)$")
    try:
        desktop.answer("reader")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is") == "program:reader"
        )
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: bool(left.match(str((shown["selected"] or {}).get("side", ""))))
        )
        assert desktop.showing()["selected"]["name"] == "page.md", desktop.showing()
        # Inside it, the mark is a row of its own, by its letter, and Read On
        # leads, as the row's Return does.
        desktop.follow()
        desktop.launcher_until(
            lambda shown: (
                "Read From Mark a" in shown["rows"]
                and (shown["selected"] or {}).get("name") == "Read On"
            )
        )
        desktop.shoot("launcher-reader-marks")
        kernel_lines = int(machine.succeed("dmesg | wc -l"))
        machine.send_key("ret")
        reader = (reader_focused(desktop), kernel_lines)
        desktop.launcher_until(lambda shown: not shown["open"])
        # Closed at once, it keeps the place it opened on, which is where it
        # was left rather than the top.
        close_the_reader(desktop)
        again = reader_closed_at(desktop, reader)
        assert again["block"] == kept["block"], (kept, again)
        moved = abs(cast("float", again["into_pt"]) - cast("float", kept["into_pt"]))
        assert moved < 1, (kept, again)
    finally:
        desktop.close_launcher()
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


def reader_focused(desktop: Desktop) -> str:
    """Wait until a reader the launcher started holds the keys, and give its process id."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session('niri msg --json focused-window | grep -q \'"app_id":"reader"\''),
        timeout=STARTING,
    )
    return desktop.as_account("pgrep -u $(id -u) -n -x .reader-wrapped").strip()


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
    pid = reader_focused(desktop)
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
    with a window, the tab is open, Return goes to Chrome, and its page opens on Go To. Its
    downloads are a third, a file a line (its path, size, when it finished and where from),
    which Chrome's own provider reads as its page opens: the file is in its Recent, and Open
    the Latest Download opens it as the desktop opens a file, in swayimg.
    """
    chrome_visited(desktop)
    downloaded = chrome_downloaded(desktop)

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
        chrome_opened(desktop)
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
        # Tab opens Chrome's page, its first row Return's own, and its Recent in it,
        # and its downloads read as it opens, Open the Latest Download after the rest.
        desktop.into_program()
        desktop.launcher_until(
            lambda shown: (
                len(shown["held"]) == 1
                and shown["current"] == 0
                and {"Recent", LATEST_DOWNLOAD} <= set(shown["rows"])
            )
        )
        assert desktop.showing()["selected"]["name"] == "Go To", desktop.showing()["selected"]
        desktop.shoot("launcher-chrome-page")
        chrome_download_opened(desktop, downloaded)
    finally:
        desktop.close_launcher()
        chrome_gone(desktop)
        chrome_download_gone(desktop, downloaded)


def chrome_download_opened(desktop: Desktop, downloaded: str) -> None:
    """From Chrome's page, find the download in its Recent, then open it by its act.

    Its Recent holds it, its size and when beside it, under a minute ago unless the machine
    is slow to get there. Open the Latest Download says its name, and opens it as the desktop
    opens a file, a picture in swayimg.
    """
    machine = desktop.machine
    name = downloaded.rsplit("/", 1)[-1]
    sized = re.compile("^[\\d.]+\u00a0[kMG]?B \u00b7 (just now|\\d+\u00a0min ago)$")
    desktop.down_to("Recent")
    desktop.step_in()
    desktop.launcher_until(lambda shown: name in shown["rows"])
    desktop.down_to(name)
    desktop.launcher_until(
        lambda shown: bool(sized.match(str((shown["selected"] or {}).get("side", ""))))
    )
    desktop.shoot("launcher-chrome-downloads")
    desktop.step_out()
    desktop.down_to(LATEST_DOWNLOAD)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == name)
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"pgrep -u {desktop.account} -f '[s]wayimg.*{downloaded}'", timeout=STARTING
    )


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


def test_kittys_recent_opens_a_window_in_a_folder_and_neovims_page_goes_to_a_buffer(
    desktop: Desktop,
) -> None:
    """Recent in kitty opens a window in a folder, and Neovim's page goes to a buffer in one.

    Both reach a kitty already open through its remote control: kitty's Recent lists a folder
    zoxide learned, and Return opens a window there. Neovim's walk follows
    (neovims_open_files_are_gone_to).
    """
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
    neovims_open_files_are_gone_to(desktop)


# Neovim's page in Programs lists every file each Neovim in a kitty window
# holds, asked over the server each told its window of as its screen came
# up (modules/neovim/showing.lua), and Return goes to that very buffer in
# its own window. The walk only reads each Neovim from outside, so the
# account's own configuration is what runs (desktop_session.py's neovim_in_kitty).


def neovims_open_files_are_gone_to(desktop: Desktop) -> None:
    """Neovim's page lists each file two Neovims hold, and Return goes to one's buffer.

    One Neovim holds two files in two tabs, the first changed and unsaved, and the other one
    file. Neovim's row counts the three open, each page row says whether it's modified and which
    tab shows it, as the page's offscreen cases do (tests/program-acts.qml), no other row names
    one again, as Recent did before it left out what's open, and Return on the file in the first
    Neovim's second tab, in the window not looked at last, brings that window forward with that
    buffer current in it, opening no other.
    """
    machine = desktop.machine
    folder = f"/home/{desktop.account}/soundings"
    tides, currents, reefs = (f"{folder}/{name}.txt" for name in ("tides", "currents", "reefs"))
    desktop.as_account(
        f"mkdir -p {folder} && printf '%s\\n' depths | tee {tides} {currents} {reefs}"
    )
    neovims: list[tuple[int, str]] = []
    try:
        neovims.append(
            neovim_in_kitty(desktop, folder, "-p", tides, currents, "+call setline(1, 'shoals')")
        )
        neovims.append(neovim_in_kitty(desktop, folder, reefs))
        (tabbed, server), _ = neovims
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("neovim")

        def counts_them_open(shown: Json) -> bool:
            """Say whether Neovim's row is selected, counting the three files open."""
            selected = shown["selected"] or {}
            return selected.get("is") == "program:nvim" and "3 open" in selected.get("note", "")

        desktop.launcher_until(counts_them_open)
        desktop.into_program()
        # Each file a row, said modified and by its tab before its folder,
        # as the page's offscreen cases say it.
        held = {
            f"file:{tides}": "tides.txt \u00b7 modified \u00b7 tab 1 \u00b7 ~/soundings",
            f"file:{currents}": "currents.txt \u00b7 tab 2 \u00b7 ~/soundings",
            f"file:{reefs}": "reefs.txt \u00b7 shown \u00b7 ~/soundings",
        }
        names = ("tides.txt", "currents.txt", "reefs.txt")

        def lists_each(shown: Json) -> bool:
            """Say whether every file the Neovims hold is a row, saying how it stands."""
            reads = {thing["is"]: thing["reads"] for thing in shown["things"]}
            again = [
                said
                for key, said in reads.items()
                if key not in held and any(name in said for name in names)
            ]
            return not again and all(
                reads.get(key, "").startswith(said) for key, said in held.items()
            )

        desktop.launcher_until(lists_each)
        desktop.shoot("launcher-neovim-page")
        windows = {window["id"] for window in desktop.windows()}
        desktop.type_in("currents", selecting="currents.txt")
        machine.send_key("ret")

        def went_to_its_buffer(_: bool) -> bool:
            """Say whether the first Neovim's window is focused, its buffer current in it."""
            focused = json.loads(desktop.as_account("niri msg --json focused-window")) or {}
            current = asked_of(desktop, server, "--remote-expr", "expand('%:p')")
            return focused.get("id") == tabbed and current.strip() == currents

        retry(went_to_its_buffer, timeout=timedelta(seconds=30))
        assert {window["id"] for window in desktop.windows()} == windows, "it opened a window"
    finally:
        desktop.close_launcher()
        # Each quits unsaved, so the change made here is never written.
        neovims_quit(desktop, neovims)
        desktop.as_account(f"rm -r {folder}")


def test_mpvs_recent_says_where_a_file_stopped(desktop: Desktop) -> None:
    """Mpv's Recent holds a file it stopped part way through, saying where, to resume.

    The account's own lister reads the account's own watch_later folder here, the one connection
    no tier below has whole, as far as the drawer drawing what mpv kept. Its order among the rest,
    a file watched whole, each option said and what Return runs are held offscreen
    (the-launcher-plays-on-where-mpv-stopped), and mpv keeping them and playing on from where it
    stopped with mpv's own settings (mpv-says-where-it-stopped), since mpv opens no window in this
    machine, which has no GPU: its video output aborts as it starts.
    """
    clip = f"/home/{desktop.account}/Videos/stopped.mkv"
    later = f"/home/{desktop.account}/.local/state/mpv/watch_later"
    # Where mpv stopped in it and the speed it played at, written as mpv
    # writes them quitting part way, named by its path's hash.
    kept = f"{later}/$(printf %s {clip} | md5sum | cut -c 1-32 | tr a-f A-F)"
    desktop.as_account(
        f"mkdir -p {later} ~/Videos && mpv --no-config --really-quiet --ovc=ffv1"
        f" 'av://lavfi:testsrc=duration=60:size=64x64:rate=2' --o={clip}"
        f" && printf '# %s\\nstart=12.000000\\nspeed=1.500000\\n' {clip} > {kept}"
    )
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("mpv", selecting="mpv Media Player")
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()

        def says_where(shown: Json) -> bool:
            """Say whether the clip is first in Recent, selected, saying where and when, drawn.

            It's the newest thing mpv left, and nothing else in Recent has a time. Its length is
            the last frame's end, which mpv's encoding puts a frame short of a minute, and it was
            left as it was written, under a minute ago unless the machine is slow to get here, a
            count of minutes kept to its unit with a no-break space.
            Its drawer is drawn once it's out and draws its own title, every fact known.
            """
            row = shown["selected"] or {}
            where = re.fullmatch(
                r"0:12 of (0:59|1:00) \u00b7 (just now|\d+\u00a0min ago)", str(row.get("side", ""))
            )
            drawer = shown.get("drawer") or {}
            return (
                row.get("name") == "stopped.mkv"
                and row.get("verb") == "resume"
                and where is not None
                and drawer.get("out") is True
                and drawer.get("shown") is True
                and drawer.get("drawn") == "stopped.mkv"
                and drawer.get("eyebrow") == "in ~/Videos \u00b7 video"
                and drawer.get("drawnFacts") == ["Still to Play", "Speed", "Size", "Path"]
                and None not in (drawer.get("values") or [None])
            )

        desktop.launcher_until(says_where)
        desktop.shoot("launcher-mpv-stopped")
    finally:
        desktop.close_launcher()
        desktop.as_account(f"rm -f {clip} {kept}")


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

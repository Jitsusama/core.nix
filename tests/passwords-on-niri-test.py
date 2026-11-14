"""The Passwords place, asking the passwords daemon the shell runs as its own child, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh. The daemon
is the store's own, which the check gives the shell to run, with no socket, with one password
manager: a stand-in for 1Password's, named as 1Password's is (tests/passwords-stand-in.sh),
which never runs op, and logs where this file's environment says. It starts signed out. Nothing
asks it anything until Joel steps into Passwords, which asks it once, and while it's signed out
the place leads with 1Password's row, its side saying it's locked, whose Return unlocks it, and
a window of 1Password's opening for that ask closes the launcher and takes the keys. Refused the
first time, as a dismissed prompt is, the launcher says why the next time it opens. Once in,
GitHub leads as a favourite, its username at its side, its Return copies its password through
the daemon to the clipboard and the launcher goes, the margin wearing the clipboard's mark and
the foot, opened again, saying when it clears; resting on it reads it in full, its drawer never
saying its password, and stepped into, it holds its acts in the design's order. Open the
Website, Chrome's a stand-in that writes down what it's given, hands Chrome GitHub's website as
the launcher goes, its foot saying 1Password offers to fill it there. A walk from outside the
launcher, as `launcher show` and `launcher do` make, finds no entry and asks the manager
nothing.
"""

from __future__ import annotations

import json
import os
import shlex
import shutil
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

LOGS = Path(os.environ["LOGS"])
# What the stand-in manager was asked, a line each.
ASKED = LOGS / "passwords-asked.log"
# The stand-in's only secret, which the clipboard holds and no screen shows.
STAND_IN = "stand-in-password"
# The row leading the place while 1Password is locked: the manager, by its name.
LOCKED = "1Password"
# What the foot says, opened again, after 1Password didn't let the launcher in.
REFUSED = "1Password asks you to let the launcher in"
# The app id 1Password's windows carry, as its desktop entry names it.
PROMPT = "com.onepassword.OnePassword"
# How many unlocks the stand-in hears before it's let in: the one it refuses, then its own.
UNLOCKS = 2
# What the stand-in Chrome was given, a line an argument.
CHROME_GIVEN = LOGS.resolve() / "chrome-given.log"
# The act opening an entry's website, in Chrome, where 1Password's extension fills it.
WEBSITE = "Open the Website"
# How long a place opened from outside is watched for asking as Joel: an ask a moment
# after the opening reaches the stand-in in well under a second.
QUIET = timedelta(seconds=3)


def stand_in_chrome() -> None:
    """Give the shell a Chrome to find by its desktop entry, writing down what it's given."""
    bash = shutil.which("bash")
    assert bash is not None, "no bash to stand in for Chrome"
    program = LOGS.resolve() / "google-chrome"
    program.write_text(f'#!{bash}\nprintf "%s\\n" "$@" > {CHROME_GIVEN}\n')
    program.chmod(0o755)
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    entry = data / "applications" / "google-chrome.desktop"
    entry.parent.mkdir(parents=True, exist_ok=True)
    entry.write_text(f"[Desktop Entry]\nType=Application\nName=Google Chrome\nExec={program} %U\n")


def asked() -> list[Json]:
    """Give each ask the stand-in manager has heard."""
    if not ASKED.exists():
        return []
    return [cast("Json", json.loads(line)) for line in ASKED.read_text().splitlines()]


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run, which starts the passwords daemon itself."""
    LOGS.mkdir(parents=True, exist_ok=True)
    stand_in_chrome()
    # The shell's environment is this file's, and the daemon's the shell's, so the stand-in
    # manager logs here.
    os.environ["PASSWORDS_ASKED"] = str(ASKED.resolve())
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.close_launcher()


def opened(shell: Shell) -> None:
    """Open Passwords as Joel does, by keys: "passwords" typed at home, and Return.

    A place opened by a line from outside the launcher asks nothing as his (the test of
    `collection`), so only keys niri delivered step in as Joel.
    """
    shell.answer("passwords")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Passwords")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["held"] == ["passwords"])


def asks() -> list[str]:
    """Give what each ask the stand-in manager has heard asked."""
    return [str(one["ask"]) for one in asked()]


def let_in(shell: Shell) -> None:
    """Let the launcher in from the row leading Passwords, unless it's let in already.

    The stand-in refuses the first, as 1Password does a dismissed prompt, so the launcher,
    opened again, says why, and the second lets it in.
    """
    if asks().count("let-in") >= UNLOCKS:
        return
    if asks().count("let-in") == 0:
        unlocked(shell)
        shell.launcher_until(lambda _: "let-in" in asks())
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        # Told on opening, it's said where a key's word is, until the first key.
        shell.launcher_until(lambda shown: shown["saying"] == [REFUSED])
        shell.close_launcher()
    # The refused let-in was followed by a list too, so only a let-in past
    # those asked already says this one was heard: Unlock asks as the
    # launcher goes, after the test has moved on.
    before = asks().count("let-in")
    unlocked(shell)
    # Let in, the daemon reads the list afresh.
    shell.launcher_until(lambda _: listed_after_let_in(before))


def listed_after_let_in(before: int) -> bool:
    """Say whether a let-in past the `before` asked already was heard, then a list."""
    heard = asks()
    if heard.count("let-in") <= before:
        return False
    last = len(heard) - 1 - heard[::-1].index("let-in")
    return "list" in heard[last:]


def unlocked(shell: Shell) -> None:
    """Press Unlock on the row leading Passwords, which asks 1Password as the launcher goes."""
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == [LOCKED])
    selected = cast("Json", shell.showing()["selected"])
    assert (selected["verb"], selected["side"]) == ("unlock", "locked"), selected
    # Letting in changes how it stands, so Return from the find lands on it,
    # and a second does it, as Let the Launcher In's does in Programs.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["mode"] == "normal"
            and (shown["selected"] or {}).get("name") == LOCKED
            and not shown["landedFresh"]
        )
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["open"])


def focus_reaches(shell: Shell, window: Json) -> None:
    """Wait for niri to focus this window, which the launcher asks of it as it closes.

    The ask goes down niri's socket after the launcher has said it closed, so a busy
    builder can read the focus before niri has done it (2026-11-10).
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    focused = json.loads(shell.as_account("niri msg --json focused-window"))
    while focused is None or focused["id"] != window["id"]:
        assert time.monotonic() < deadline, f"niri never focused {window['title']}: {focused}"
        time.sleep(launcher_on_niri.POLL)
        focused = json.loads(shell.as_account("niri msg --json focused-window"))


def test_passwords_asks_nothing_until_stepped_into_and_leads_with_letting_it_in(
    shell: Shell,
) -> None:
    """Passwords asks its manager nothing until stepped into, and leads with letting it in."""
    # The shell has watched the daemon since it started, and asked nothing of
    # the manager, so 1Password would have asked Joel nothing.
    shell.launcher_until(lambda shown: not shown["open"])
    assert asked() == [], asked()
    opened(shell)
    # Stepping in is Joel's key, so the list is asked once, as his.
    shell.launcher_until(lambda _: asks() == ["list"])
    assert asked()[0]["asked"] is True, asked()
    # Refused, as 1Password is when its prompt is dismissed, so the place
    # leads with the row that lets the launcher in.
    shell.launcher_until(lambda shown: shown["rows"][:1] == [LOCKED])
    # The list being Joel's ask, 1Password's prompt coming for it closes the launcher and
    # takes the keys, never coming up beneath it (Joel, 2026-11-10: "the unlock screen shows
    # below the launcher"). A foot window carrying 1Password's app id stands in for it.
    with shell.told(f"handed to {PROMPT}"):
        prompt = shell.opened("Unlock 1Password", PROMPT)
    shell.launcher_closed()
    focus_reaches(shell, prompt)
    shell.close(prompt)
    # Home's row says nothing after its name and that it's locked at its right, and its
    # drawer names what it holds, the manager, never Check Every Password, its act
    # (VKCF3Z, db2d4ed6 and dae7fb67).
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:passwords" and thing["reads"] == "Passwords \u00b7 locked"
            for thing in shown["things"]
        )
    )
    shell.down_to("Passwords")
    shell.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("out") is True and shown["drawer"].get("body") == LOCKED
        )
    )
    shell.close_launcher()
    let_in(shell)


def test_an_entrys_return_copies_its_password_and_the_foot_says_when_it_clears(
    shell: Shell,
) -> None:
    """An entry's Return copies its password and goes, its mark in the margin till it clears."""
    let_in(shell)
    opened(shell)
    # Let in, the favourite leads, then what changed lately.
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["GitHub", "Bank Card"])
    row = cast("Json", shell.showing()["selected"])
    assert row["name"] == "GitHub", row
    assert (row["side"], row["note"]) == ("ada", "github.com"), row
    assert row["verb"] == "copy", row
    # Rested on, its drawer comes out and its Website says its note, so the row says it
    # nowhere else, and its right nothing while nothing's happening to it.
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["out"]
            and (shown["noted"], shown["stands"], shown["footer"]) == ("", "", "")
        )
    )
    # Rested on, it's read in full first, so the copy has nothing more to read.
    shell.launcher_until(
        lambda shown: "One-Time Code" in str((shown["selected"] or {}).get("drawer") or {})
    )
    # The launcher goes as it copies, as every copy does, so Joel can paste at once, and the
    # margin wears the clipboard's mark until the copy clears.
    since = shell.mark()
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["open"])
    shell.reported("holds margin ", lambda marks: "clipboard" in marks.split(), since)
    # wl-copy takes the clipboard a moment after the shell's mark, and wl-paste fails on one
    # still empty, so it's read until the copy lands rather than once.
    shell.launcher_until(lambda _: shell.execute("wl-paste --no-newline") == (0, STAND_IN))
    # Opened again, the margin's mark says it's on the clipboard, so the row and the foot say
    # no copy of it, the row its username alone (VKCF3Z, 2026-11-08); nothing on screen says
    # the password: not the row, nor its drawer, nor the foot.
    opened(shell)
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "GitHub")
    shown = shell.showing()
    assert "copied" not in str(shown["footer"]), shown["footer"]
    assert cast("Json", shown["selected"])["side"] == "ada", shown["selected"]
    assert STAND_IN not in json.dumps(shown)


def test_resting_on_an_entry_reads_it_in_full_and_it_holds_its_acts_in_order(
    shell: Shell,
) -> None:
    """Resting on an entry reads it in full, and stepped into, it holds its acts in order."""
    let_in(shell)
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["GitHub"])
    # Rested on, it's read in full, not as Joel's own ask, since the manager has let the
    # launcher in, so 1Password would ask him nothing; its drawer says what that told.
    shell.launcher_until(lambda _: "get" in asks())
    gets = [one for one in asked() if one["ask"] == "get"]
    assert all(one.get("asked") is not True for one in gets), gets
    shell.launcher_until(
        lambda shown: "One-Time Code" in str((shown["selected"] or {}).get("drawer") or {})
    )
    shown = shell.showing()
    drawer = cast("Json", (shown["selected"] or {}).get("drawer") or {})
    facts = {fact.get("label"): fact.get("value") for fact in drawer.get("facts", [])}
    assert "Password" not in facts, drawer
    assert "Username" not in facts, drawer
    assert drawer.get("eyebrow") == "in Private \u00b7 login", drawer
    assert facts.get("One-Time Code") == "set up", drawer
    assert facts.get("Strength") == "fantastic", drawer
    assert STAND_IN not in json.dumps(shown)
    # Read already, stepping in holds every act in the design's order, its one-time code's
    # among them.
    shell.step_in("ctrl-bracketright")
    acts = [
        "Copy the Password",
        "Copy the One-Time Code",
        "Copy the Username",
        WEBSITE,
        "Show in 1Password",
        "Copy the Link",
    ]
    shell.launcher_until(lambda shown: shown["rows"] == acts)
    shell.launcher_until(lambda shown: shown["held"] == ["passwords", "GitHub"])


def test_open_the_website_hands_chrome_the_website_as_the_launcher_goes(shell: Shell) -> None:
    """Open the Website hands Chrome the website as the launcher goes, saying who fills it."""
    let_in(shell)
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["GitHub"])
    # Read in full first, so no act arrives above it once the selection is there.
    shell.launcher_until(
        lambda shown: "One-Time Code" in str((shown["selected"] or {}).get("drawer") or {})
    )
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: WEBSITE in shown["rows"])
    shell.down_to(WEBSITE)
    # The launcher never fills it, and the row says so at its right, where it says how it
    # stands, with no note after its name.
    shell.launcher_until(
        lambda shown: (
            shown["footer"] == "1Password offers to fill it on the page"
            and shown["stands"] == shown["footer"]
            and shown["noted"] == ""
        )
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["open"])
    shell.launcher_until(
        lambda _: CHROME_GIVEN.exists() and CHROME_GIVEN.read_text() == "https://github.com/login\n"
    )


def test_a_walk_from_outside_the_launcher_reaches_no_entry(shell: Shell) -> None:
    """A walk from outside the launcher, as any process's qs ipc call makes, finds no entry.

    `launcher show` and `launcher do` walk its places for any process of the account's, so
    Passwords holds nothing for them and asks its daemon nothing: only Joel's keys reach an
    entry, an act on one, a manager's let-in or a check.
    """
    let_in(shell)
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["GitHub"])
    shell.close_launcher()
    before = len(asked())
    # As the terminal's launcher calls them, after `--`, since `show` is qs ipc's own command
    # too; what qs says when it fails is on its standard error, said here too.
    walk = f"qs -p {shell.folder} ipc call -- launcher"
    path = shlex.quote("\u203aPasswords")
    shown = shell.as_account(f"{walk} show {path} 2>&1")
    for held in ["GitHub", "Bank Card", "Check Every Password"]:
        assert held not in shown, f"a walk from outside found {held}: {shown}"
    walked = json.dumps({"steps": ["Passwords", "GitHub", "Copy the Password"]})
    done = json.loads(shell.as_account(f"{walk} doing {shlex.quote(walked)}"))
    assert done.get("missing") == "GitHub", f"a do from outside reached GitHub: {done}"
    assert len(asked()) == before, f"a walk from outside asked: {asked()[before:]}"
    shell.launcher_until(lambda shown: not shown["open"])


def test_a_place_opened_from_outside_the_launcher_asks_nothing_as_joel(shell: Shell) -> None:
    """Passwords opened by `collection`, which any process of the account's can call, asks as none.

    Only a key niri delivered to the launcher is Joel's, so a place opened over qs ipc or the
    launcher's socket lists without `asked`, and 1Password is never brought to ask him.
    """
    shell.launcher_until(lambda shown: not shown["open"])
    before = len(asked())
    shell.as_account(shell.ipc("collection", "passwords"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["passwords"])
    shell.launcher_until(lambda _: "list" in asks()[before:])
    since = asked()[before:]
    assert all(one.get("asked") is not True for one in since), f"asked as Joel: {since}"
    assert "let-in" not in asks()[before:], f"it asked to be let in: {since}"


def test_a_place_a_terminal_or_the_catalogue_opens_asks_nothing_as_joel(shell: Shell) -> None:
    """Passwords asked open by `launcher do` or the catalogue's run is never stepped into.

    Both open a place for any process of the account's, walking to it while the launcher is
    walked from outside, when Passwords holds no row (Passwords.qml's `outside`). So the step
    into it finds nowhere to go and the launcher opens at home: it never opens, and asks its
    daemon nothing as Joel. Its asks are watched for a few seconds after, since what's proven
    is that none comes.
    """
    walk = f"qs -p {shell.folder} ipc call -- launcher"
    catalogue = json.loads(shell.as_account(f"{walk} list"))
    place = next(one for one in catalogue if one["words"] == "Passwords" and one["in"] == "")
    opens = {
        "launcher do": f"{walk} doing {shlex.quote(json.dumps({'steps': ['Passwords']}))}",
        "the catalogue's run": f"{walk} run {shlex.quote(place['id'])}",
    }
    for way, command in opens.items():
        shell.launcher_until(lambda shown: not shown["open"])
        before = len(asked())
        answer = shell.as_account(command)
        shell.launcher_until(lambda shown: shown["open"])
        quiet_until = time.monotonic() + QUIET.total_seconds()
        while time.monotonic() < quiet_until:
            since = asked()[before:]
            assert all(one.get("asked") is not True for one in since), f"{way} asked: {since}"
            assert "let-in" not in asks()[before:], f"{way} asked to be let in: {since}"
            time.sleep(0.1)
        held = shell.showing()["held"]
        assert held == [], f"{way} answered {answer} and stepped into {held}"
        shell.close_launcher()

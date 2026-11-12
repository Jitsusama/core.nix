"""The desktop test's launcher moves windows area.

Here the launcher opens kitty, lists its repositories and machines, goes to one of its tabs,
finds, frames and moves windows, goes back by Super+Tab, and shows each layer's shortcuts.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
from functools import partial
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    LAUNCHER_OPEN,
    STARTING,
    drawn_at,
    laid_out,
    retry,
    settles_on,
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


def test_the_launcher_finds_and_opens_kitty_and_learns_that_kitty_was_meant(
    desktop: Desktop,
) -> None:
    """The launcher finds and opens kitty, and learns that kitty was meant."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    # The foot's time keeps only its own room, as the date gives way to the
    # keys a letter brings: it kept the date's, a gap after it.
    time = next(
        word for word in laid_out(desktop)["foot"] if re.fullmatch(r"\d\d:\d\d", word["text"])
    )
    # Its room is measured as it's said and its ink as it's drawn, so they
    # may round apart by a pixel.
    assert abs(time["width"] - time["inked"]) <= 1, f"the time keeps more room: {time}"
    desktop.shoot("launcher")
    machine.send_key("ret")
    # The first window opened can take as long as the first GTK one.
    machine.wait_until_succeeds(
        desktop.in_session("niri msg windows | grep -i kitty"), timeout=STARTING
    )
    # What was picked for what was typed is kept for next time.
    picked = '\'"kitty":{"program:kitty"\''
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -rqsF {picked} ~/.local/state/quickshell"),
        timeout=dt.timedelta(seconds=10),
    )
    # And what the opening took, the file's last line: five letters, a
    # Return, kitty picked for them. The pipe writes it on its own time.
    last = "tail -n 1 $(find ~/.local/state/quickshell -name launcher-trials.jsonl)"
    machine.wait_until_succeeds(
        desktop.in_session(f"{last} | grep -qF program:kitty"), timeout=dt.timedelta(seconds=10)
    )
    trial = json.loads(desktop.as_account(last))
    assert trial["picked"] == "program:kitty", trial
    assert trial["typed"] == "kitty", trial
    assert trial["keys"]["letters"] == len("kitty"), trial
    assert trial["keys"]["returns"] == 1, trial
    assert trial["ms"] > 0, trial
    assert not trial["missed"], trial
    # Each key timed from its search to its frame, which comes after; keys
    # typed faster than frames share one, timed by the last.
    timed = trial["timed"]
    assert 1 <= timed["count"] <= len("kitty"), timed
    assert timed["drawnMost"] >= timed["searchedMost"] >= 0, timed


# The border is all that marks the focused window, and niri leaves
# borders off unless its config turns them on.


def test_the_focused_window_is_framed_in_the_accent(desktop: Desktop) -> None:
    """The focused window is framed in the accent."""
    # The kitty the launcher opened, or one of its own when run alone.
    bordered = desktop.focused_kitty()
    desktop.as_account(f"niri msg action move-window-to-floating --id {bordered['id']}")

    def beside() -> str:
        """Give the colour just left of the focused window, where its border is."""
        image = desktop.settled_screen("focused")
        left, top, _, height = drawn_at(
            json.loads(desktop.as_account("niri msg --json focused-window"))
        )
        # An RGB picture's pixels are each a red, a green and a blue.
        pixel = cast("tuple[int, int, int]", image.getpixel((left - 1, top + height // 2)))
        return "#{:02x}{:02x}{:02x}".format(*pixel)

    framed = settles_on(desktop.values["accent"].lower(), beside)
    desktop.as_account(f"niri msg action move-window-to-tiling --id {bordered['id']}")
    assert framed == desktop.values["accent"].lower()


# niri opens a window in a column of its own, and the shell folds it
# in just below the focused one.


def test_a_terminal_opens_below_the_focused_window(desktop: Desktop) -> None:
    """A terminal opens below the focused window."""
    machine = desktop.machine
    anchor = desktop.focused_kitty()
    desktop.as_account("qs ipc call grid below kitty")
    kitties = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    machine.wait_until_succeeds(desktop.in_session(f"{kitties} | grep -qx 2"), timeout=STARTING)

    def placed(_: bool) -> bool:
        """Say whether the new terminal sits just below the one focused before."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        at = {w["id"]: w["layout"]["pos_in_scrolling_layout"] for w in windows}
        new = next(w["id"] for w in windows if w["id"] != anchor["id"])
        here, there = at[anchor["id"]], at[new]
        return bool(there[0] == here[0] and there[1] == here[1] + 1)

    retry(placed, timeout=dt.timedelta(seconds=15))


# The launcher finds a window by its title, and Return goes to it.


def test_the_launcher_switches_to_a_window_by_its_title(desktop: Desktop) -> None:
    """The launcher switches to a window by its title."""
    machine = desktop.machine
    # A window to go back to before Notes for Tuesday, so the subtest
    # stands alone as well as after those that leave one (UTMDGL,
    # 2026-11-03: alone, focus-window-previous had nowhere to go).
    desktop.focused_kitty()
    desktop.as_account("niri msg action spawn -- kitty --title 'Notes for Tuesday'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg windows | grep -q 'Notes for Tuesday'"),
        timeout=STARTING,
    )
    desktop.as_account("niri msg action focus-window-previous")
    machine.wait_until_fails(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("tuesday", selecting="Notes for Tuesday")
    desktop.shoot("launcher-window")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )


# What kitty's page in Programs lists beside its tabs, which the account has none of until the
# test gives it stand-ins: the repositories under ~/src, the one zoxide saw visited last first
# though its name sorts after, and the machines ssh's config names, the one zsh's history
# reached most first though the config names it second and the other was reached since.
# How long a shot waits for a selected row that reads on to settle: twice through, a few
# seconds each, and then the screen's own settling (docs/design.md, A long name reads on).
READ_ON_TWICE = dt.timedelta(seconds=30)
REPOSITORIES = ("tide-tables", "sea-charts")
MACHINES = ("ferry", "lighthouse")
KITTY_ACTS = ("New Window in a Repository", "SSH to a Machine")

# Each name is under .test, which nothing in the VM answers, and the key is no key, so nothing
# could reach a machine even if a row were run.
SSH_CONFIG = """\
Host lighthouse
    HostName lighthouse.harbour.test
    User keeper
    Port 2222

Host ferry
    HostName ferry.harbour.test
    User deckhand
    Port 2200
"""
KNOWN_HOSTS = "[ferry.harbour.test]:2200 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIStandInNotAKey\n"


def kitty_stand_ins(desktop: Desktop) -> None:
    """Give the account two repositories and two machines for kitty's page to list."""
    now = int(desktop.as_account("date +%s"))
    hour, day = 60 * 60, 24 * 60 * 60
    for name in REPOSITORIES:
        desktop.as_account(f"git init -q ~/src/{name}")
    # The order comes from zoxide's record of visits, which z's form sets the time of, so
    # neither visit has to wait for the clock to pass a second.
    home = desktop.as_account("printf %s ~")
    visits = "".join(
        f"{home}/src/{name}|1|{now - ago}\n"
        for name, ago in zip(REPOSITORIES, (hour, 6 * day), strict=True)
    )
    desktop.as_account(
        f'z=$(mktemp) && printf %s {shlex.quote(visits)} > "$z"'
        ' && _Z_DATA="$z" zoxide import z --merge && rm "$z"'
    )
    desktop.as_account(
        "mkdir -p -m 700 ~/.ssh"
        f" && printf %s {shlex.quote(SSH_CONFIG)} > ~/.ssh/config"
        f" && printf %s {shlex.quote(KNOWN_HOSTS)} > ~/.ssh/known_hosts"
    )
    # The history is the file the launcher was told to read, in zsh's extended form: ferry three
    # times, last a day ago, and lighthouse once, two hours ago, so use leads over recency.
    providers = desktop.as_account("cat ~/.config/quickshell/Providers.qml")
    history = re.search(r'"history":"([^"]+)"', providers)
    assert history, "the launcher was told of no zsh history"
    reached = (("ferry", 3 * day), ("ferry", 2 * day), ("ferry", day), ("lighthouse", 2 * hour))
    lines = "".join(f": {now - ago}:0;ssh {name}\n" for name, ago in reached)
    path = shlex.quote(history[1])
    desktop.as_account(f'mkdir -p "$(dirname {path})" && printf %s {shlex.quote(lines)} >> {path}')


def said_of(shown: Json, name: str) -> list[str]:
    """Give what the row of this name reads, part by part, or nothing while none is shown."""
    for thing in shown["things"]:
        parts = thing["reads"].split(" \u00b7 ")
        if parts[0] == name:
            return cast("list[str]", parts)
    return []


def test_the_launcher_goes_to_a_kitty_tab_inside_its_window(desktop: Desktop) -> None:
    """The launcher lists kitty's tabs, and Return on one goes to it through kitty.

    One path through the connection: kitty's watcher telling the programs daemon of the tab
    made after the shell first read the list, open-now asking a real kitty over its socket,
    the tab matched to niri's window, and kitty's remote control bringing it forward once niri has
    focused the window. Beside Tabs are the repositories fd finds under ~/src in zoxide's
    order, and the machines ssh's files name, by zsh's history: each read from the account's
    own files. No row of them is run, since each would open a window. Each case of the rows'
    words and order is tests/kitty-tabs.qml's.
    """
    machine = desktop.machine
    kitty_stand_ins(desktop)
    kitty = f"kitten @ --to unix:@kitty-{desktop.focused_kitty()['pid']}"
    # A second tab, named so its row is known, with the first in front again.
    desktop.as_account(f"{kitty} launch --type=tab --tab-title 'Second Tab' --cwd /tmp")
    desktop.as_account(f"{kitty} focus-tab --match index:0")
    focused = f'{kitty} ls --match-tab state:focused | grep -qF \'"title": "Second Tab"\''
    machine.wait_until_fails(desktop.in_session(focused), timeout=dt.timedelta(seconds=10))
    # The shell reads kitty's repositories, machines and tabs ahead, as the launcher closes, so
    # the stand-ins made since it started are read once it has opened and closed.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.close_launcher()

    def counted(shown: Json) -> bool:
        return "Tabs" in shown["rows"] and all(
            said_of(shown, act)[1:2] == ["2"] for act in KITTY_ACTS
        )

    # Each act says how many it holds, in its place from the page's first frame, since a row
    # arriving once a page is drawn would join its end.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.into_program()
    desktop.launcher_until(counted)
    shown = desktop.showing()
    page = ["New Window Below", *KITTY_ACTS, "Tabs", "Recent"]
    assert [row for row in shown["rows"] if row in page] == page, shown["rows"]
    desktop.shoot("launcher-kitty-program")
    walks_kittys_machines(desktop)
    walks_kittys_repositories(desktop)
    desktop.down_to("Tabs")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Second Tab" in shown["rows"])
    desktop.down_to("Second Tab")
    desktop.shoot("launcher-kitty-tabs")
    machine.send_key("ret")
    machine.wait_until_succeeds(desktop.in_session(focused), timeout=dt.timedelta(seconds=10))
    # kitty's match takes a space as "and", so the tab is matched by its first word.
    desktop.as_account(f"{kitty} close-tab --match title:^Second")


def walks_kittys_machines(desktop: Desktop) -> None:
    """Step into SSH to a Machine, and back out to kitty's page, opening nothing."""
    desktop.down_to("SSH to a Machine")
    desktop.step_in()
    desktop.launcher_until(lambda shown: set(MACHINES) <= set(shown["rows"]))
    shown = desktop.showing()
    assert [row for row in shown["rows"] if row in MACHINES] == list(MACHINES), shown["rows"]
    # How many times each was reached is the history's, read by grep.
    assert "3 times" in said_of(shown, "ferry"), shown["things"]
    assert "once" in said_of(shown, "lighthouse"), shown["things"]
    desktop.down_to("ferry")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["shown"]
            and shown["drawer"]["title"] == "ferry"
        )
    )
    # Its own block's settings, as ssh would be told them by hand, and the kind of key
    # known_hosts holds where they reach.
    drawer = desktop.showing()["drawer"]
    facts = dict(zip(drawer["drawnFacts"], drawer["values"], strict=True))
    assert facts.get("Address") == "deckhand@ferry.harbour.test:2200", facts
    assert "Known Keys" in facts, facts
    desktop.shoot("launcher-kitty-ssh")
    desktop.step_out()


def walks_kittys_repositories(desktop: Desktop) -> None:
    """Step into New Window in a Repository, and back out to kitty's page, opening nothing."""
    desktop.down_to("New Window in a Repository")
    desktop.step_in()
    desktop.launcher_until(lambda shown: set(REPOSITORIES) <= set(shown["rows"]))
    shown = desktop.showing()
    rows = shown["rows"]
    assert [row for row in rows if row in REPOSITORIES] == list(REPOSITORIES), rows
    # When zoxide saw it, read from its database, and never ~/src, which every row would say.
    assert said_of(shown, "tide-tables")[1:] == ["1 h ago"], shown["things"]
    desktop.shoot("launcher-kitty-repositories")
    desktop.step_out()


# Super+Tab opens the launcher on every other window, the one used last
# first, so Return goes back to it, and a few letters find any other,
# among two dozen as easily as among two, with nothing held down. The
# windows are one kitty's, so the machine holds them all.


def test_super_tab_goes_back_to_the_window_before_and_finds_any_by_typing(desktop: Desktop) -> None:
    """Super+Tab goes back to the window before, and finds any by typing."""
    machine = desktop.machine
    desktop.notes_window()
    # The last one's title is far wider than the launcher, as a
    # browser tab's can be.
    scratch = [
        *[f"Scratch {n:02}" for n in range(1, 25)],
        (
            "Scratch 25 and a title that goes on well past the edge of the launcher"
            " and the screen, as the titles of web pages often do"
        ),
    ]
    for title in scratch:
        desktop.as_account(
            "niri msg action spawn -- kitty --single-instance"
            f" --instance-group scratch --title '{title}'"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg windows | grep -q '{title}'"),
            timeout=STARTING,
        )
    notes = desktop.window_titled("Notes for Tuesday")
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    closed = desktop.in_session(LAUNCHER_OPEN)

    def tab_to(typed: str, title: str) -> None:
        """Open the windows with Super+Tab, type, and wait until the window titled is chosen."""
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["held"] == ["windows"])
        if typed:
            desktop.type_in(typed, selecting=title)
        desktop.launcher_until(
            lambda shown: shown["selected"] and shown["selected"]["name"] == title
        )

    def went(title: str) -> None:
        """Press Return, and wait until the launcher closes on the window titled."""
        machine.send_key("ret")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg focused-window | grep -q '{title}'"),
            timeout=dt.timedelta(seconds=10),
        )

    # The window used last is chosen as it opens, and this one isn't
    # there to choose.
    tab_to("", scratch[-1])
    # Its long title reads on twice, then rests, as it's shot.
    desktop.shoot("launcher-windows", READ_ON_TWICE)
    went("Scratch 25")
    tab_to("", "Notes for Tuesday")
    went("Notes for Tuesday")
    # A few letters find one among them all, and nothing looser.
    tab_to("scr 07", "Scratch 07")
    went("Scratch 07")
    # A program's name finds its windows, the one used last first.
    tab_to("kitty", "Notes for Tuesday")
    desktop.close_launcher()
    # Inside kitty, each of its windows holds what its own row among the
    # windows does (Joel, 2026-10-26: one thing, one set of insides,
    # everywhere). The one Joel is in, Scratch 07, isn't among them.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.follow()
    desktop.type_in("scratch 08", selecting="Scratch 08")
    # In normal, where Ctrl+] goes in, as Return opens the window; on the
    # find letters type, so it names none.
    desktop.normal()
    told = (desktop.showing()["selected"] or {}).get("told", [])
    assert "Tab in" in told, told
    desktop.follow()
    desktop.launcher_until(lambda shown: {"Size", "Float"} <= set(shown["rows"]))
    desktop.close_launcher()
    for title in scratch:
        desktop.as_account(
            f"niri msg action close-window --id {desktop.window_titled(title)['id']}"
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q Scratch"), timeout=dt.timedelta(seconds=30)
    )
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")


# Ctrl+] goes into what can be done to a window, and a verb
# that needs a target asks for it in the same field.


def test_the_launcher_moves_a_window_and_copies_its_title(desktop: Desktop) -> None:
    """The launcher moves a window and copies its title."""
    machine = desktop.machine
    notes = desktop.notes_window()

    def into_notes(verb: str) -> None:
        """Find the notes' window in the launcher, step into it, and type a verb."""
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("tuesday", selecting="Notes for Tuesday")
        desktop.follow()
        desktop.type_in(verb)

    into_notes("copy")
    desktop.shoot("launcher-verbs")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == "Notes for Tuesday"

    into_notes("move")
    desktop.step_in("ret")
    desktop.shoot("launcher-target")
    # The step ends in normal, where Return moves it to the workspace.
    machine.send_key("ret")

    def moved(_: bool) -> bool:
        """Say whether the notes' window is on another workspace now."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        now = next(w for w in windows if w["id"] == notes["id"])
        return bool(now["workspace_id"] != notes["workspace_id"])

    retry(moved, timeout=dt.timedelta(seconds=10))


def test_the_window_joel_is_in_holds_what_can_be_done_to_it(desktop: Desktop) -> None:
    """The window Joel is in holds what can be done to it, as This window."""
    machine = desktop.machine
    notes = desktop.notes_window()
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Named This window, the window's title beside it.
    desktop.type_in("this window", selecting="This Window")
    selected = desktop.showing()["selected"]
    assert selected["side"] == "Notes for Tuesday", selected
    # Return steps in, to what can be done to it, all but going to it,
    # since it's the one he's in. Typed, on the find, the foot says "\u21b5 open",
    # as normal does, since stepping in has one word wherever it's done, and
    # no arrow, since the find's arrows move the caret.
    assert selected["told"] == ["\u21b5 open"], selected
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Close" in shown["rows"])
    rows = desktop.showing()["rows"]
    assert "Go To" not in rows, rows
    assert "Bring Here" not in rows, rows
    # Full width makes its column the screen's width, as niri says.
    before = desktop.window_titled("Notes for Tuesday")
    desktop.type_in("full", selecting="Full Width")
    desktop.land()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])

    def widened(_: bool) -> bool:
        """Say whether the notes' window has grown wider."""
        now = desktop.window_titled("Notes for Tuesday")
        return bool(now["layout"]["window_size"][0] > before["layout"]["window_size"][0])

    retry(widened, timeout=dt.timedelta(seconds=10))
    # Size gives it a share of the screen, half of it narrower than all.
    wide = desktop.window_titled("Notes for Tuesday")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("this window", selecting="This Window")
    desktop.step_in("ret")
    desktop.type_in("size", selecting="Size")
    # Its sizes are a step in, and the foot says Return opens them, on the
    # find as in normal.
    told = (desktop.showing()["selected"] or {}).get("told", [])
    assert told[:1] == ["\u21b5 open"], told
    desktop.step_in("ret")
    desktop.type_in("half", selecting="Half the Screen")
    desktop.land()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])

    def halved(_: bool) -> bool:
        """Say whether the notes' window is narrower than it was at full width."""
        now = desktop.window_titled("Notes for Tuesday")
        return bool(now["layout"]["window_size"][0] < wide["layout"]["window_size"][0])

    retry(halved, timeout=dt.timedelta(seconds=10))


def test_q_closes_only_the_launcher_and_no_key_reaches_the_window_behind(
    desktop: Desktop,
) -> None:
    """:q closes only the launcher, and the window behind it never hears a key of it.

    Escape puts the line in normal mode, and :q there closes it; the window hears neither
    Escape nor the Return that runs the line.
    """
    # Joel, 2026-10-26: "Escape closes only the launcher; the key never
    # reaches the program behind it", which holds for :q's Return now that
    # Escape never closes (2026-11-05). wev says every key its window hears,
    # held as it gets the keyboard or pressed and let go after. The launcher
    # goes on Return's press, so the keyboard may come back to the window
    # with the key still down: it's then told the key is held, and told of
    # its release, which no program acts on. Only a press would reach it.
    machine = desktop.machine
    values = desktop.values
    log = "/run/user/1000/wev-escape.log"
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=wev-escape -- sh -c "
        f"'{values['coreutils']}/bin/stdbuf -oL {values['wev']}/bin/wev > {log} 2>&1'"
    )
    retry(lambda _: any(w["title"] == "wev" for w in desktop.windows()), timeout=STARTING)
    wev = next(w for w in desktop.windows() if w["title"] == "wev")
    desktop.as_account(f"niri msg action focus-window --id {wev['id']}")

    def entered() -> int:
        """Give how many times wev's window has had the keyboard."""
        return int(machine.succeed(f"grep -c 'wl_keyboard] enter' {log} || true").strip())

    retry(lambda _: entered() >= 1, timeout=STARTING)
    before = entered()
    try:
        # Opened as Joel opens it, by its chord, so its keys are his too.
        machine.send_key("meta_l-spc")
        desktop.launcher_ready()
        desktop.close_launcher()
        # The keyboard is wev's again, and a key pressed after reaches it,
        # so all it was given as the launcher went is in its log.
        retry(lambda _: entered() > before, timeout=dt.timedelta(seconds=10))
        machine.send_key("a")
        retry(
            lambda _: "sym: a " in machine.succeed(f"cat {log}"),
            timeout=dt.timedelta(seconds=10),
        )
        heard = machine.succeed(f"cat {log}")
        assert not re.search(r"state: 1 \(pressed\)\s*\n\s*sym: (Escape|Return)", heard), heard
    finally:
        desktop.as_account("systemctl --user stop wev-escape")


def test_niri_tells_the_shell_it_gives_the_keys_on_demand(desktop: Desktop) -> None:
    """The session niri starts tells the shell it gives the keys as a surface asks on demand.

    The shell asks on demand only once told, and exclusively before, since a switch restarts
    it under the niri already running, and an older niri never gives a mapped surface the
    keys on demand. What the newest surface and a click do with them is proven on a nested
    niri (tests/focus-on-niri-test.py); this is the wire from niri to the shell.
    """
    shell = "/proc/$(systemctl --user show quickshell -p MainPID --value)/environ"
    environment = desktop.as_account(f"tr '\\0' '\\n' < {shell}").splitlines()
    assert "NIRI_ON_DEMAND_WHEN_ASKED=1" in environment, (
        "the shell wasn't told niri gives the keys on demand, so it asks exclusively"
    )


# In normal mode d does a row's ending action and y its copy, as Vim's d
# and y do a line's: dd, a count's rows or a motion's span. A window's
# Close can't be put back, so d over more than one asks once on the line,
# as Vim's ask_yesno does: only y says yes, n, Escape and Ctrl+C say no,
# any other key does nothing, and nothing answers it in its first 300 ms.


def test_dd_and_dj_close_windows_once_asked_and_yy_copies_a_title(desktop: Desktop) -> None:
    """Dd closes a window, 3dd, dG and d2j close two once asked, and yG and yy copy titles."""
    machine = desktop.machine
    first = desktop.opened("Ending first")
    second = desktop.opened("Ending second")
    third = desktop.opened("Ending third")
    endings = {first["id"], second["id"], third["id"]}
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("ending")
    desktop.launcher_until(
        lambda shown: sorted(shown["rows"][:3]) == ["Ending first", "Ending second", "Ending third"]
    )
    desktop.normal()
    copies_the_titles(desktop, len(endings))
    # dd closes the one window at once: its program keeps its own work,
    # as Vim's :close refuses a modified buffer, so the language marks
    # Close guarded and one asks nothing.
    with desktop.told("ended 1"):
        machine.send_chars("dd")

    def left() -> set[int]:
        """Give the ending windows still open."""
        return endings & {window["id"] for window in desktop.windows()}

    retry(lambda _: len(left()) == len(endings) - 1, timeout=dt.timedelta(seconds=10))
    asks_once_for_the_rest(desktop, left)
    retry(lambda _: not left(), timeout=dt.timedelta(seconds=10))
    assert desktop.showing()["open"], "d closed the launcher"


def copies_the_titles(desktop: Desktop, windows: int) -> None:
    """Copy every row's title with yG, a line each in order, then the top row's with yy."""
    machine = desktop.machine
    shown = desktop.showing()["rows"]
    since = desktop.mark()
    machine.send_chars("yG")
    desktop.reported("yanked ", lambda count: int(count) >= windows, since)
    copied = desktop.as_account("wl-paste --no-newline").split("\n")
    assert copied[:windows] == shown[:windows], copied
    desktop.launcher_until(lambda now: now["open"] and now["flashing"] == [])
    # yy copies the row's title and flashes it, the launcher still open.
    since = desktop.mark()
    machine.send_chars("yy")
    desktop.reported("showing ", lambda said: json.loads(said).get("flashing") == [0], since)
    desktop.reported("yanked 1", lambda rest: rest == "", since)
    assert desktop.as_account("wl-paste --no-newline") == shown[0]
    desktop.launcher_until(lambda now: now["open"] and now["flashing"] == [])


def unasked(shown: Json) -> bool:
    """Say whether the ask has gone, its rows untinted, the launcher still open."""
    return bool(shown["open"] and shown["question"] == "" and shown["tinted"] == [])


def asks_once_for_the_rest(desktop: Desktop, left: Callable[[], set[int]]) -> None:
    """Ask over the two windows left by 3dd, dG and d2j, answering no, nothing, then yes."""
    machine = desktop.machine
    open_before = len(left())
    # Its row stays in its place, gone, until a key changes what's found,
    # and d never counts it. So 3dd from it takes the other two and asks
    # once, on the line, by their count and kind, the two rows it covers
    # tinted; n says no, and the launcher stays.
    desktop.launcher_until(lambda shown: shown["things"][0]["gone"])
    with desktop.told("ask ready"), desktop.told("asked Close 2 windows? (y/n)"):
        machine.send_chars("3dd")
    # A copy yG or yy made can arrive after dd and find again, taking the
    # gone row out, so the two it covers are the windows still open,
    # wherever they sit.
    asking = desktop.showing()
    assert asking["question"] == "Close 2 windows? (y/n)", asking
    tinted = [asking["things"][at] for at in asking["tinted"]]
    assert len(tinted) == open_before, asking
    assert not any(row["gone"] for row in tinted), asking
    with desktop.told("answered no"):
        machine.send_chars("n")
    desktop.launcher_until(unasked)
    # dG takes every row from the cursor down, the copies yG and yy left
    # among them; Ctrl+C says no.
    since = desktop.mark()
    with desktop.told("ask ready"):
        machine.send_chars("dG")
    desktop.reported("asked Close 2 windows", lambda rest: rest.endswith("(y/n)"), since)
    with desktop.told("answered no"):
        machine.send_key("ctrl-c")
    desktop.launcher_until(unasked)
    assert len(left()) == open_before, desktop.windows()
    # A y typed on from d2j, inside the ask's first 300 ms, does nothing,
    # and nor does any key but y, n, Escape and Ctrl+C once it's ready; y
    # then closes both.
    with desktop.told("ask ready"), desktop.told("asked Close 2 windows? (y/n)"):
        machine.send_chars("d2jy")
    machine.send_chars("x")
    desktop.launcher_until(lambda shown: shown["question"] == "Close 2 windows? (y/n)")
    assert len(left()) == open_before, desktop.windows()
    with desktop.told("ended 2"):
        machine.send_chars("y")


def names_each_mode(desktop: Desktop) -> None:
    """See the foot's start name the find, normal and the command line, ending in normal.

    Each mode is three capitals there (Joel, 2026-11-05: "the mode should be 3 letters, in
    caps"): the find's FND, normal's NOR and the command line's CMD.
    """
    machine = desktop.machine
    desktop.launcher_until(lambda shown: shown["modeMark"] == "FND")
    desktop.normal()
    desktop.launcher_until(lambda shown: shown["modeMark"] == "NOR")
    machine.send_chars(":")
    desktop.launcher_until(lambda shown: shown["modeMark"] == "CMD")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["modeMark"] == "NOR")


def visual_keeps_its_keys(desktop: Desktop) -> None:
    """See visual's foot keep d, y, : and Space ? beside Return, and Space ? and F1 list.

    Return's count comes first, the foot drawing it last; the command line is drawn furthest
    from Return, to give way first, d is named by the cursor row's ending, and the way to
    every key stays just before Return (VKCF3Z, 2026-11-05). Space ? lists the keys over two
    rows and lets them go, the selection standing, and F1 does the same, as help does in
    normal.
    """
    told = desktop.showing()["selected"]["told"]
    assert told == ["\u21b5 open 2", ": commands", "d close", "y copy", "Space ? keys"], told
    desktop.machine.send_chars(" ?")
    desktop.launcher_until(lambda shown: shown["listing"] and shown["tinted"] == [0, 1])
    desktop.machine.send_chars(" ?")
    desktop.launcher_until(lambda shown: not shown["listing"] and shown["tinted"] == [0, 1])
    desktop.machine.send_key("f1")
    desktop.launcher_until(lambda shown: shown["listing"] and shown["tinted"] == [0, 1])
    desktop.machine.send_key("f1")
    desktop.launcher_until(lambda shown: not shown["listing"] and shown["tinted"] == [0, 1])


def test_v_and_j_choose_windows_for_y_d_and_return(desktop: Desktop) -> None:
    """V and j choose windows, y copies their titles, d closes them once asked, Return opens."""
    machine = desktop.machine
    opened = {
        title: desktop.opened(title)["id"]
        for title in ("Chosen first", "Chosen second", "Chosen third")
    }
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("chosen")
    desktop.launcher_until(lambda shown: set(shown["rows"][:3]) == set(opened))
    names_each_mode(desktop)
    # niri lists them in its own order, so the walk takes the first two
    # rows as it finds them and leaves the third.
    rows = desktop.showing()["rows"]
    taken, kept = rows[:2], rows[2]
    # Ctrl+C ends visual as Escape does, as in Vim, the launcher staying.
    machine.send_chars("V")
    desktop.launcher_until(lambda shown: shown["mode"] == "visual")
    machine.send_key("ctrl-c")
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["mode"] == "normal" and shown["tinted"] == []
    )
    # V begins visual line on the cursor row, which it tints, the mark saying VIS and the mode
    # word's room how many it holds; j takes the row below too, and the foot says how many
    # Return opens.
    machine.send_chars("V")
    desktop.launcher_until(
        lambda shown: (
            shown["mode"] == "visual"
            and shown["modeMark"] == "VIS"
            and shown["modeWord"] == "1 chosen"
        )
    )
    assert desktop.showing()["tinted"] == [0], desktop.showing()
    machine.send_chars("j")
    desktop.launcher_until(
        lambda shown: shown["tinted"] == [0, 1] and shown["modeWord"] == "2 chosen"
    )
    visual_keeps_its_keys(desktop)
    # y copies both titles, a line each, flashes them, and ends visual.
    since = desktop.mark()
    machine.send_chars("y")
    desktop.reported("yanked 2", lambda rest: rest == "", since)
    assert desktop.as_account("wl-paste --no-newline") == "\n".join(taken)
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and shown["tinted"] == [])
    # A colon opens the command line on the selection, as Vim's '<,'>,
    # and Escape gives the selection back, as it stood.
    machine.send_chars("k")
    desktop.launcher_until(lambda shown: shown["current"] == 0)
    machine.send_chars("Vj:")
    desktop.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("from") == "visual"
            and shown["command"]["range"] == "'<,'>"
            and shown["command"]["things"] == len(taken)
        )
    )
    machine.send_key("esc")
    desktop.launcher_until(
        lambda shown: (
            shown["command"] is None and shown["mode"] == "visual" and shown["tinted"] == [0, 1]
        )
    )
    # d asks once for both and y closes both, the third left.
    with desktop.told("ask ready"), desktop.told("asked Close 2 windows? (y/n)"):
        machine.send_chars("d")
    with desktop.told("ended 2"):
        machine.send_chars("y")

    def closed(_: bool) -> bool:
        """Say whether the two chosen windows have gone and the third stays."""
        left = {window["id"] for window in desktop.windows()}
        return not {opened[title] for title in taken} & left and opened[kept] in left

    retry(closed, timeout=dt.timedelta(seconds=10))
    # Return over the selection opens what it holds: the third window,
    # going to it as its own Return does, which lets the launcher go. The
    # closed rows stay in their place, gone, so it's gone to by its number.
    shown = desktop.showing()
    at = next(
        at
        for at, thing in enumerate(shown["things"])
        if not thing["gone"] and thing["reads"].startswith(f"{kept} ")
    )
    machine.send_chars(f"{at + 1}G")
    desktop.launcher_until(lambda shown: shown["current"] == at)
    machine.send_chars("V")
    desktop.launcher_until(lambda shown: shown["mode"] == "visual")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])
    retry(
        lambda _: desktop.window_titled(kept)["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )


# The rest of Vim's visual on the rows: v as V, since a row has no letters;
# o and gv; ip, a paragraph of rows, a run of one kind; { and } by those
# runs; the operator pressed again taken with visual's; x as d; and the
# rows holding still while chosen, what arrives waiting as "1 new" in the
# foot (VKCF3Z, 2026-11-05).
PARAGRAPH = ("Paragraph first", "Paragraph second", "Paragraph third")
ARRIVING = "Paragraph fourth"


def test_v_o_gv_ip_and_a_window_waiting_behind_what_is_chosen(desktop: Desktop) -> None:
    """V's o, gv, ip, { }, yy taken once and x on windows, a window waiting while they're chosen."""
    for title in PARAGRAPH:
        desktop.opened(title)
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("paragraph")
    desktop.launcher_until(lambda shown: len(shown["rows"]) == len(PARAGRAPH))
    desktop.normal()
    chooses_and_brings_back(desktop)
    moves_by_runs_and_takes_them(desktop)
    desktop.close_launcher()
    waits_behind_what_is_chosen(desktop)


def waits_behind_what_is_chosen(desktop: Desktop) -> None:
    """Open a window while Super+Tab's rows are chosen: it waits as 1 new, then comes in last.

    A typed search said again finds only what it found, so a new window
    arrives in the windows list, which is all of them.
    """
    machine = desktop.machine
    machine.send_key("meta_l-tab")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: shown["held"] == ["windows"])
    desktop.normal()
    machine.send_chars("V")
    desktop.launcher_until(lambda shown: shown["mode"] == "visual")
    before = desktop.showing()["rows"]
    arriving = desktop.opened(ARRIVING)
    desktop.launcher_until(lambda shown: shown["waiting"] == 1 and shown["rows"] == before)
    # The row that comes in is the one the list now holds that it didn't:
    # niri may make the new window the one Joel is in, which gives back
    # the one he was, so it's known by the order kept, not its name.
    machine.send_key("esc")
    desktop.launcher_until(
        lambda shown: (
            shown["waiting"] == 0
            and len(shown["rows"]) == len(before) + 1
            and shown["rows"][: len(before)] == before
        )
    )
    desktop.close_launcher()
    desktop.close(arriving)


def chooses_and_brings_back(desktop: Desktop) -> None:
    """Choose by v and j, go to the other end with o, and bring it back with gv."""
    machine = desktop.machine
    machine.send_chars("v")
    desktop.launcher_until(
        lambda shown: shown["mode"] == "visual" and shown["modeWord"] == "1 chosen"
    )
    machine.send_chars("j")
    desktop.launcher_until(lambda shown: shown["current"] == 1 and shown["tinted"] == [0, 1])
    machine.send_chars("o")
    desktop.launcher_until(lambda shown: shown["current"] == 0 and shown["tinted"] == [0, 1])
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and shown["tinted"] == [])
    machine.send_chars("G")
    desktop.launcher_until(lambda shown: shown["current"] == len(PARAGRAPH) - 1)
    machine.send_chars("gv")
    desktop.launcher_until(lambda shown: shown["mode"] == "visual" and shown["tinted"] == [0, 1])
    # ip takes the run the cursor's in: the three windows, one kind.
    machine.send_chars("ip")
    desktop.launcher_until(
        lambda shown: shown["tinted"] == [0, 1, 2] and shown["modeWord"] == "3 chosen"
    )
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal")


def moves_by_runs_and_takes_them(desktop: Desktop) -> None:
    """Move by runs with } and {, copy a run with V j y y once, and close one with x."""
    machine = desktop.machine
    # } goes past the run of windows, which reaches the end, so to its last.
    machine.send_chars("gg}")
    desktop.launcher_until(lambda shown: shown["current"] == len(PARAGRAPH) - 1)
    machine.send_chars("{")
    desktop.launcher_until(lambda shown: shown["current"] == 0)
    # y again just after visual's is taken with it: two titles copied, and
    # no y left waiting for a motion.
    since = desktop.mark()
    machine.send_chars("Vjyy")
    desktop.reported("yanked 2", lambda rest: rest == "", since)
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and shown["flashing"] == [])
    assert not desktop.showing()["modeWord"].startswith("y"), desktop.showing()
    # x over one window closes it at once, as d would.
    with desktop.told("ended 1"):
        machine.send_chars("Vx")
    desktop.launcher_until(lambda shown: any(thing["gone"] for thing in shown["things"]))


# A window closed while Super+Tab shows it stays in its place, gone, until
# a key: rows closing up under the cursor was one of the ways the launcher
# moved with nothing pressed.


def test_a_window_closed_while_shown_stays_in_its_place_until_a_key(desktop: Desktop) -> None:
    """A window closed while shown stays in its place until a key."""
    machine = desktop.machine
    # Opened first, so it isn't the focused window, which Super+Tab leaves out.
    leaving = desktop.opened("Leaving soon")
    desktop.notes_window()
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.send_key("meta_l-tab")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: shown["held"] == ["windows"] and "Leaving soon" in shown["rows"]
    )
    before = desktop.showing()
    desktop.close(leaving)

    def gone(shown: Json) -> bool:
        return any(thing["gone"] for thing in shown["things"])

    desktop.launcher_until(gone)
    after = desktop.showing()
    assert after["rows"] == before["rows"], f"the rows moved: {before['rows']} to {after['rows']}"
    assert after["selected"]["name"] == before["selected"]["name"], "the selection moved"
    left = next(thing for thing in after["things"] if thing["gone"])
    assert left["reads"].startswith("Leaving soon"), left
    # A key searches afresh, and the window's row goes.
    desktop.type_in("n")
    machine.send_key("backspace")
    desktop.launcher_until(
        lambda shown: shown["typed"] == "" and "Leaving soon" not in shown["rows"]
    )
    assert not gone(desktop.showing()), desktop.showing()
    desktop.close_launcher()


# A window's verbs, stepped into from Super+Tab, follow it as it changes
# while its row reads the same: floated, its Float says Tile.


def test_a_windows_verbs_follow_it_while_theyre_shown(desktop: Desktop) -> None:
    """A window's verbs follow it while they're shown."""
    machine = desktop.machine
    notes = desktop.notes_window()
    # Another window in front, so the notes' is among Super+Tab's.
    desktop.as_account("niri msg action spawn -- kitty --title 'In front of the notes'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'In front of the notes'"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-tab")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: shown["held"] == ["windows"])
    desktop.type_in("tuesday", selecting="Notes for Tuesday")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Float" in shown["rows"])
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    desktop.launcher_until(lambda shown: "Tile" in shown["rows"] and "Float" not in shown["rows"])
    desktop.close_launcher()
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    front = desktop.window_titled("In front of the notes")
    desktop.as_account(f"niri msg action close-window --id {front['id']}")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q 'In front of'"),
        timeout=dt.timedelta(seconds=30),
    )


# One walk, as its subtest was: each step starts where the last left it.
def test_super_shift_shows_every_layers_shortcuts_and_runs_the_desktops(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """Super+Shift+/ shows every layer's shortcuts, and runs the desktop's."""
    machine = desktop.machine
    windows = len(json.loads(desktop.as_account("niri msg --json windows")))
    # Over no window only the desktop's keys work, so the layers lead with
    # them: the empty workspace niri keeps last on the focused screen.
    shortcut_spaces: list[Json] = json.loads(desktop.as_account("niri msg --json workspaces"))
    shortcut_screen = next(space for space in shortcut_spaces if space["is_focused"])["output"]
    shortcut_empty = max(
        (space for space in shortcut_spaces if space["output"] == shortcut_screen),
        key=lambda space: int(space["idx"]),
    )
    desktop.as_account(f"niri msg action focus-workspace {shortcut_empty['idx']}")
    machine.send_key("meta_l-shift-slash")
    desktop.launcher_ready()
    # Its top is the layers, each with its count and as many of its first
    # groups as fit whole; the machine has no Neovim or pi, so no layer of
    # theirs, nor Chrome, so none of its either. After the terminal's come
    # btop's, which every machine has, the picture viewer's, the markdown
    # reader's, the document viewer's, the player's, Google's seven
    # programs', whose keys are their pages' in any browser, Slack's (which
    # this area's machine brings, for Now's Slack piece), 1Password's,
    # GIMP's, Inkscape's and Spotify's.
    layers = [
        "Desktop",
        "Terminal",
        "Processes",
        "Picture Viewer",
        "Markdown Reader",
        "Document Viewer",
        "Player",
        "Mail",
        "Calendar",
        "Meet",
        "Docs",
        "Sheets",
        "Slides",
        "Drive",
        "Slack",
        "Passwords",
        "Picture Editor",
        "Drawing Editor",
        "Music",
    ]
    desktop.launcher_until(lambda shown: shown["rows"] == layers)
    # Nothing follows a place's name but how many keys stand at its right;
    # its groups wait in its drawer (VKCF3Z's ruling on places).
    desktop_row = desktop.showing()["things"][0]["reads"]
    assert re.fullmatch(r"Desktop \u00b7 \d+", desktop_row), desktop_row
    desktop.shoot("launcher-shortcut-layers")
    # The desktop holds its groups, by what a chord acts on.
    desktop.follow()
    groups = ["Windows", "Columns", "Workspaces", "Screens", "Launcher", "Typing"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(groups)] == groups)
    desktop.shoot("launcher-shortcut-groups")
    # What's typed there finds a group by its name, before every chord in
    # the layer that its words start, flat; a group as big as Workspaces
    # holds groups of its own, then what none of them takes.
    desktop.type_in("work", selecting="Workspaces")
    assert "Go to workspace 3" in desktop.showing()["rows"]
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "desktop", "workspaces"])
    inner = ["Focus", "The Column", "The Window", "Above and Below"]
    assert desktop.showing()["rows"][: len(inner)] == inner
    assert "Show every workspace" in desktop.showing()["rows"]
    # The field says where that is as one path, in the rows' own case.
    assert desktop.showing()["path"] == ["Shortcuts", "Desktop", "Workspaces"]
    # Back out: to the desktop with "work" typed, then Shortcuts, each - a step,
    # since no key clears the find from normal.
    for _step in range(2):
        desktop.step_out()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"] and shown["rows"] == layers)
    # At the top, what's typed finds a chord in any layer, flat.
    desktop.type_in("paste")
    desktop.shoot("launcher-shortcuts")
    for _ in "paste":
        machine.send_key("backspace")
    # Each field cleared before the next is typed, or a busy shell takes
    # the new keys between the last backspaces (QGB4ON, 2026-10-30).
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    # The touchscreen's swipes are the desktop's, found by "swipe": one
    # doing what a chord does is that chord's row, saying the swipe in
    # words, and one no chord does is drawn as its fingers and its way.
    swipe = "swipe workspace below"
    desktop.type_in(swipe, selecting="Go to the workspace below")
    below = desktop.showing()
    assert not below["selected"]["chord"].startswith("3 "), below["selected"]
    # The swipe is another way to do it, so its drawer says it, under Also (VKCF3Z, 358703a5).
    also = [fact["value"] for fact in below["selected"]["drawer"]["facts"]]
    assert "3 fingers up" in also, below["selected"]["drawer"]
    assert below["rows"].count("Go to the workspace below") == 1, below["rows"]
    desktop.shoot("launcher-swipes")
    for _ in swipe:
        machine.send_key("backspace")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    swipe = "swipe stop showing"
    desktop.type_in(swipe, selecting="Stop showing every workspace")
    assert desktop.showing()["selected"]["chord"].startswith("4 ")
    for _ in swipe:
        machine.send_key("backspace")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    desktop.type_in("open a terminal")
    machine.send_key("ret")

    def opened(_: bool) -> bool:
        """Say whether a window has opened since the shortcuts did."""
        return len(json.loads(desktop.as_account("niri msg --json windows"))) > windows

    # kitty starts cold, drawn in software, which on a machine running
    # other tests too has taken 10 seconds and more than 19, so it's
    # given a minute.
    retry(opened, timeout=dt.timedelta(minutes=1))

    # Shortcuts is stepped into from its row, and its path says it once.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("shortcuts", selecting="Shortcuts")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["path"] == ["Shortcuts"])
    # Inside it, a group named what's typed, whole, comes first, and is
    # Return's row.
    desktop.type_in("workspaces", selecting="Workspaces")
    assert desktop.showing()["rows"][0] == "Workspaces"
    desktop.close_launcher()


# Windows standing in for Google's programs in Chrome, which this machine doesn't run: a kitty
# given the app id and title Chrome gives Mail's own window, and one given Chrome's app id and
# the title of a tab showing a document; each by its layer's name and its picture's name
# (launcher/KeyLayers.js reads both, VKCF3Z, 2026-11-07).
GOOGLE_WINDOWS = [
    ("chrome-mail.google.com__-Default", "Inbox - joel@example.com - Gmail", "Mail", "mail"),
    ("google-chrome", "Notes - Google Docs - Google Chrome", "Docs", "docs-tab"),
]


def on_layer(layer: str, shown: Json) -> bool:
    """Say whether Shortcuts holds the layer named, as its path ends."""
    return cast("list[str]", shown["path"])[-1:] == [layer]


def over_googles_programs(desktop: Desktop) -> None:
    """Open Shortcuts over Mail's window and a Docs tab, each on its own keys, leading.

    The program is told by the window's app id or its title's end, so a Docs tab in Chrome
    names Docs and Mail's own window names Mail (VKCF3Z, 2026-11-07).
    """
    machine = desktop.machine
    for app, title, layer, picture in GOOGLE_WINDOWS:
        named_so = f"--class {shlex.quote(app)} --title {shlex.quote(title)}"
        desktop.as_account(f"niri msg action spawn -- kitty {named_so}")
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg --json windows | grep -qF {shlex.quote(title)}"),
            timeout=STARTING,
        )
        window = desktop.window_titled(title)
        try:
            desktop.as_account(f"niri msg action focus-window --id {window['id']}")
            machine.send_key("meta_l-shift-slash")
            desktop.launcher_ready()
            desktop.launcher_until(partial(on_layer, layer))
            desktop.shoot(f"launcher-shortcuts-over-{picture}")
            desktop.step_out()
            desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"])
            assert desktop.showing()["rows"][0] == layer, desktop.showing()["rows"]
            desktop.close_launcher()
        finally:
            desktop.close(window)


def test_super_shift_over_a_terminal_opens_on_the_terminals_shortcuts(desktop: Desktop) -> None:
    """Super+Shift+/ over a terminal, or one of Google's programs, opens on its shortcuts."""
    machine = desktop.machine
    desktop.focused_kitty()
    machine.send_key("meta_l-shift-slash")
    desktop.launcher_ready()
    # Over kitty the chord opens on the terminal's layer, its groups
    # already showing, the path saying whose keys they are.
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    assert desktop.showing()["path"] == ["Shortcuts", "Terminal"]
    assert desktop.showing()["rows"][:2] == ["Tabs", "Tab Numbers"]
    desktop.shoot("launcher-shortcuts-over-kitty")
    # Return steps into a group, and - back out to the
    # layers, the terminal's first and selected, saying it's the window's,
    # and the desktop's next.
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal", "tabs"])
    desktop.step_out()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    desktop.step_out()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"])
    assert desktop.showing()["rows"][:2] == ["Terminal", "Desktop"]
    assert (desktop.showing()["selected"] or {}).get("name") == "Terminal"
    assert desktop.showing()["footer"] == "the keys of the window you were in"
    # Return goes back in.
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    desktop.close_launcher()
    over_googles_programs(desktop)


def test_what_is_picked_inside_a_programs_page_is_learned_there(desktop: Desktop) -> None:
    """What's picked inside a program's page is learned for what was typed there.

    Learning applies wherever typing searches, inside a step too (RSCH-20261030-GB56HX), so
    "new" in kitty's page lands on the row picked for it last time. The cases are in
    tests/ranking.qml; this is the one path from a pick there to the next opening's order.
    """
    machine = desktop.machine

    def kittys_new() -> list[str]:
        """Open kitty's page, type new in it, and give the rows it finds."""
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("kitty", selecting="kitty")
        desktop.into_program()
        desktop.type_in("new", selecting="New Window")
        return cast("list[str]", desktop.showing()["rows"])

    found = kittys_new()
    assert found[:2] == ["New Window", "New Window Below"], found
    desktop.down_to("New Window Below")
    kitties = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    before = int(desktop.as_account(kitties).strip())
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f"{kitties} | grep -qx {before + 1}"), timeout=STARTING
    )
    # Kept under kitty's page and what was typed there, apart from the top.
    machine.wait_until_succeeds(
        desktop.in_session("grep -rqsE 'program:kitty.{1,3}new\"' ~/.local/state/quickshell"),
        timeout=dt.timedelta(seconds=10),
    )
    found = kittys_new()
    assert found[:2] == ["New Window Below", "New Window"], found
    desktop.close_launcher()


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

"""The desktop test's launcher goes places area.

Here the launcher goes places: pi's sessions, quests and runs, the volume, kitty, the windows it
finds and moves, and the shortcuts. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for
the desktop, started once a run (desktop_session.py), and waits at its start for what it needs,
so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    LAUNCHER_OPEN,
    PI_PRESENCE,
    PI_QUEST,
    PI_WINDOW,
    STARTING,
    drawn_at,
    retry,
    settles_on,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def test_the_launcher_shows_the_volumes_chords_and_escape_closes_it(desktop: Desktop) -> None:
    """The launcher shows the volume's chords, and Escape closes it."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.photograph("launcher-volume")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


# One walk, as its subtest was: each step starts where the last left it.
def test_the_launcher_finds_a_pi_session_by_its_quest_and_goes_to_its_window(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """The launcher finds a pi session by its quest, and goes to its window."""
    machine = desktop.machine
    desktop.pi_session()
    # What it spent today, as spend-watch writes it, replaced whole.
    spent = {
        "v": 2,
        "sessions": {"01a1face-test": "$12.50"},
        "helpers": {"01a1face-test": [{"on": "councils", "spent": "$4.00"}]},
    }
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo '{json.dumps(spent)}'"
        " > $XDG_RUNTIME_DIR/spend-watch/.c"
        " && mv $XDG_RUNTIME_DIR/spend-watch/.c $XDG_RUNTIME_DIR/spend-watch/collection.json"
    )
    # A VM test of its waiting its turn in the machine's queue.
    lined = {
        "v": 1,
        "heartbeat": float(machine.succeed("date +%s")),
        "machine": {},
        "jobs": [
            {
                "id": "t",
                "kind": "vm-test",
                "state": "waiting",
                "owner": {"session": "01a1face-test"},
                "what": "Test the shell",
                "since": machine.succeed("date -u +%FT%TZ").strip(),
                "place": 2,
            },
        ],
    }
    desktop.as_account(
        f"echo '{json.dumps(lined)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # From the other window, so going to the session's means a move.
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    # The launcher's foot counts it, working, as its presence says.
    since = desktop.mark()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.reported("stands ", lambda rest: "1 working" in rest, since)
    desktop.launcher_ready()
    # Its collection by what it's called, then the session named
    # whole, as its README is, not as its window's title.
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and "$12.50 today" in shown["selected"]["side"]
            and "2nd in line" in shown["selected"]["side"]
        )
    )
    desktop.photograph("launcher-agents")
    # Stepped into, what's known of it, each a row of its own.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Switch"
            and shown["selected"]["verb"] == "switch"
        )
    )
    # Each named first, as every part is, then what it says.
    for name, side in [
        ("Quest", r"QEST-20261022-TESTQA"),
        ("Address", r"TESTQA"),
        ("Started", r"\d\d:\d\d"),
        ("Spent today", r"\$12\.50"),
        ("Councils", r"\$4\.00"),
        ("Test the shell", r"2nd in line"),
        ("Works on", r"Tidy the tests"),
        ("Hears", r"#desk"),
    ]:
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's names.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"])  # noqa: B023
            )
        )
    desktop.photograph("launcher-agent-facts")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["agents"])
    desktop.as_account(
        "rm $XDG_RUNTIME_DIR/spend-watch/collection.json $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # Found by its address too.
    desktop.type_in("testqa", selecting="A quest of its own & whole")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # Its title is read once, outside what's drawn, so nothing drawn
    # reads it again as it changes.
    log = desktop.shell_said("Binding loop")
    assert "Binding loop" not in log, "a session's title is read inside a binding"

    # It comes to want Joel, its presence replaced by rename as pi's
    # is: the margin hangs pi's mark, and lets it go once answered.
    # From another window, so it's said as a notice too, whose
    # Return goes to the session's window.
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )

    # The margin may hold other marks, a notice missed earlier among
    # them, so it's pi's own that's waited on.
    # Wants begun now, as the shell only says those it saw begin.
    def begun() -> str:
        """Give the machine's time now, as pi writes when a want began."""
        return machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()

    question = {"kind": "question", "since": begun(), "said": "Which <way> & why?"}
    since = desktop.mark()
    # Said as written: the summary is plain, so its & isn't escaped.
    with desktop.told("shows card A quest of its own & whole asks you"):
        desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so Super+N takes the stack at once: held, it waits.
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    # The card says it, so pi's mark waits for the card to go: a card or
    # a mark, never both.
    told_since = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert not re.search("holds margin [^\\n]*\\bagent\\b", told_since), (
        "pi's mark showed beside its card"
    )
    # The same want, its file replaced again, says nothing new.
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    time.sleep(3)
    cards = (
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -c 'report\\[shows card A quest of its own & whole asks you'"
    )
    assert desktop.as_account(cards).strip() == "1", "the same want was said twice"
    since = desktop.mark()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=10),
    )
    # Its card gone, the want still stands, so pi's mark shows.
    desktop.reported("holds margin ", lambda rest: "agent" in rest.split(), since)
    desktop.photograph("margin-pi-wants")
    # Another, unanswered, from the other window: its notice says what
    # it wants, and goes when the want does, missed by nobody. Notices
    # missed earlier are let go first, so the margin's notices mark is
    # this one's alone.
    held = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
    )
    since = desktop.mark()
    desktop.as_account("qs ipc call notices seen")
    if "waiting" in held.split()[2:]:
        desktop.reported("holds margin ", lambda rest: "waiting" not in rest.split(), since)
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    with desktop.told("shows card A quest of its own & whole wants your yes"):
        desktop.presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "approval", "since": begun(), "said": "Push it?"},
            }
        )
    # Ended while its notice is up, the notice goes with it, well
    # before its own ten seconds would.
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=5),
    )
    # A third, left to time out while its want stands: it isn't
    # missed, since pi's mark says the want; ended, the mark goes.
    with desktop.told("shows card A quest of its own & whole is stuck without you"):
        desktop.presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "blocked", "since": begun(), "said": "Out of ideas."},
            }
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=30),
    )
    time.sleep(2)

    def margin_holds() -> list[str]:
        """Give what the margin last said it holds."""
        return desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
        ).split()[2:]

    assert "waiting" not in margin_holds(), "pi's notice counted as missed"
    since = desktop.mark()
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.reported("holds margin ", lambda rest: "agent" not in rest.split(), since)
    assert "waiting" not in margin_holds(), "pi's notice counted as missed"


def test_the_launcher_finds_pis_quests_going_to_the_session_on_one_and_opening_pi_on_another(
    desktop: Desktop,
) -> None:
    """The launcher finds pi's quests, going to the session on one and opening pi on another."""
    machine = desktop.machine
    desktop.pi_session()
    # Their READMEs as pi keeps them, front matter first: the session's
    # quest, and another in a tree of its own with no session on it.
    another = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-HALLWY"
    desktop.as_account(
        "printf '%s\\n' '---' 'id: QEST-20261022-TESTQA' 'kind: quest' 'status: active'"
        " 'priority: driving' 'rank: 1' '---' '# A quest of its own & whole'"
        f" > {PI_QUEST}/README.md"
        f" && mkdir -p {another} ~/hallway && printf '%s\\n' '---' 'id: QEST-20261022-HALLWY'"
        " 'kind: quest' 'status: active' 'priority: queued' 'rank: 1' 'trees:'"
        ' "  - path: $HOME/hallway"'
        f" '---' '# Paint the hallway' > {another}/README.md"
    )
    desktop.presence_says(
        {"addresses": ["TESTQA"], "busy": False, "owns": ["QEST-20261022-TESTQA"]}
    )
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Agents' one word for them finds them.
    desktop.type_in("quests")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Quests")
    machine.send_key("ret")
    # The one driving first, naming the session that has it loaded.
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["tasks"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and shown["selected"]["side"] == "driving \u00b7 TESTQA"
        )
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Paint the hallway"
            and shown["selected"]["side"] == "queued"
        )
    )
    desktop.photograph("launcher-quests")
    machine.send_key("up")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "A quest of its own & whole"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # One with no session, found by a word of its title, opens pi on
    # it in its tree.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("hallway", selecting="Paint the hallway")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(
            'grep -qx "$HOME/hallway QEST-20261022-HALLWY" $XDG_RUNTIME_DIR/pi-opened'
        ),
        timeout=dt.timedelta(seconds=30),
    )
    desktop.as_account(f"rm -rf {another} ~/hallway $XDG_RUNTIME_DIR/pi-opened")


def test_the_launcher_finds_pis_runs_going_to_the_session_that_started_one_and_reading_an_answer(
    desktop: Desktop,
) -> None:
    """The launcher finds pi's runs, going to the session that started one and reading an answer."""
    machine = desktop.machine
    desktop.pi_session()
    # A fleet as the harness keeps one, started by the session, whose
    # window's process stands in for pi's: one member still at work,
    # its process the other window's, and one done with its answer.
    leader = desktop.window_titled(PI_WINDOW)["pid"]
    crew = "~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew/reviewers"

    def keeps(path: str, said: Json) -> None:
        """Write what the harness keeps of the fleet, at a path inside it."""
        desktop.as_account(
            f"mkdir -p {crew}/{path.split('/', maxsplit=1)[0]}"
            f" && echo '{json.dumps(said)}' > {crew}/{path}"
        )

    asked = {"cwd": "/src/house", "args": ["cli.js", "--model", "anthropic/opus"]}
    for member, child, stands in [
        ("brush", desktop.window_titled("Elsewhere")["pid"], "running"),
        ("roller", 1, "complete"),
    ]:
        keeps(f"{member}/request.json", asked)
        keeps(f"{member}/lease.json", {"parentPid": leader, "childPid": child})
        keeps(
            f"{member}/progress.json",
            {"state": stands, "activity": "painting the trim" if stands == "running" else ""},
        )
    stamp = machine.succeed("date -u +%FT%T.000Z").strip()
    keeps(
        "roller/result.json",
        {
            "state": "complete",
            "startedAt": stamp,
            "completedAt": stamp,
            "usage": {"cost": {"total": 2.5}},
            "finalAssistantText": "Two coats.",
        },
    )
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "owner": {"pid": leader}})
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Agents' one word for them finds them.
    desktop.type_in("subagents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Subagents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["helpers"]
            and (shown["selected"] or {}).get("name") == "paint-crew"
            and re.fullmatch(
                r"helpers \u00b7 1 of 2 done \u00b7 (under a minute|\d+ min) \u00b7 TESTQA",
                shown["selected"]["side"],
            )
        )
    )
    desktop.photograph("launcher-runs")
    # Stepped into, its members after the way to its session.
    desktop.step_in()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Switch")
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "brush"
            and shown["selected"]["side"].startswith("painting the trim \u00b7 opus")
        )
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "roller"
            and shown["selected"]["side"] == "opus \u00b7 $2.50 \u00b7 under a minute"
        )
    )
    desktop.photograph("launcher-run-members")
    # Its answer, read in a terminal named for the run and the member.
    machine.send_key("ret")
    reading = "paint-crew \u00b7 roller"
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json windows | grep -q '{reading}'"),
        timeout=STARTING,
    )
    # Still open a moment on, so the pager holds the answer.
    time.sleep(1)
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(reading)['id']}")
    # Return on the run itself goes to its session.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("subagents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Subagents")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "paint-crew")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account(
        "rm -rf ~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew"
    )
    for title in [PI_WINDOW, "Elsewhere"]:
        desktop.as_account(
            f"niri msg action close-window --id {desktop.window_titled(title)['id']}"
        )
    desktop.as_account(f"rm -rf {PI_QUEST} {PI_PRESENCE}/testqa.json")
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q 'TESTQA\\|\"Elsewhere\"'"),
        timeout=dt.timedelta(seconds=30),
    )


def test_the_launcher_sets_the_volume_to_a_number_and_says_how_loud_it_is(desktop: Desktop) -> None:
    """The launcher sets the volume to a number, and says how loud it is."""
    machine = desktop.machine
    # Muted first, since a level asked for is one to be heard.
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SINK@ 1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume 40")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").endswith(" to 40%")
    )
    machine.send_key("ret")
    # Exactly that, and no longer muted.
    machine.wait_until_succeeds(
        desktop.in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.40'"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("40%, through ")
    )
    desktop.photograph("launcher-volume-told")
    # A step in, each device's level drawn as a bar.
    desktop.step_in()
    desktop.photograph("launcher-volume-parts")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


def test_the_launcher_finds_and_opens_kitty_and_learns_that_kitty_was_meant(
    desktop: Desktop,
) -> None:
    """The launcher finds and opens kitty, and learns that kitty was meant."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.photograph("launcher")
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
    # And what the opening took: five letters, a Return, kitty picked for them.
    kept = desktop.as_account(
        "cat $(find ~/.local/state/quickshell -name launcher-trials.json) 2>/dev/null || true"
    )
    trial = json.loads(kept or "[]")[-1]
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


# Joel types the moment he presses Super+Space. Whatever letters come
# before the launcher has the keys go to the window beneath, and the
# rest to the launcher's field, but not one may be lost or reordered
# between the two. A terminal beneath keeps every raw key it gets, so
# a letter the launcher missed is found there.


def test_the_launcher_loses_no_letter_typed_straight_after_its_chord(desktop: Desktop) -> None:
    """The launcher loses no letter typed straight after its chord."""
    machine = desktop.machine
    desktop.as_account(
        "niri msg action spawn -- kitty --title type-ahead "
        "sh -c 'stty -icanon -echo; cat > /tmp/type-ahead'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q type-ahead"),
        timeout=STARTING,
    )
    for gap in (dt.timedelta(milliseconds=ms) for ms in (10, 50, 100)):
        beneath = len(desktop.as_account("cat /tmp/type-ahead"))
        machine.send_key("meta_l-spc")
        machine.send_chars("slack", delay=gap)
        desktop.launcher_ready()

        def none_lost(_: bool, beneath: int = beneath, gap: dt.timedelta = gap) -> bool:
            """Say whether every letter typed went beneath or to the launcher, in order."""
            went = desktop.as_account("cat /tmp/type-ahead")[beneath:]
            held = desktop.showing()["typed"]
            print(f"TYPEAHEAD {gap.total_seconds()}: '{went}' went beneath, '{held}' held")
            return bool(went + held == "slack")

        retry(none_lost, timeout=dt.timedelta(seconds=10))
        machine.send_key("esc")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
        machine.wait_until_succeeds(
            desktop.in_session("niri msg focused-window | grep -q type-ahead"),
            timeout=dt.timedelta(seconds=10),
        )
    desktop.as_account("pkill -f 'cat > /tmp/type-ahead'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q type-ahead"),
        timeout=dt.timedelta(seconds=30),
    )


# The launcher finds a window by its title, and Return goes to it.


def test_the_launcher_switches_to_a_window_by_its_title(desktop: Desktop) -> None:
    """The launcher switches to a window by its title."""
    machine = desktop.machine
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
    desktop.photograph("launcher-window")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )


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
    desktop.photograph("launcher-windows")
    went("Scratch 25")
    tab_to("", "Notes for Tuesday")
    went("Notes for Tuesday")
    # A few letters find one among them all, and nothing looser.
    tab_to("scr 07", "Scratch 07")
    went("Scratch 07")
    # A program's name finds its windows, the one used last first.
    tab_to("kitty", "Notes for Tuesday")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    for title in scratch:
        desktop.as_account(
            f"niri msg action close-window --id {desktop.window_titled(title)['id']}"
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q Scratch"), timeout=dt.timedelta(seconds=30)
    )
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")


def test_super_tab_finds_a_pi_window_by_its_work_and_puts_one_waiting_next(
    desktop: Desktop,
) -> None:
    """Super+Tab finds a pi window by its work, and puts one waiting on Joel next."""
    machine = desktop.machine
    desktop.pi_session()
    pi = desktop.window_titled(PI_WINDOW)
    between = desktop.opened("Between the two")
    # Used in this order, so the pi window is the oldest of the three
    # Super+Tab lists, and Elsewhere is where Joel is.
    for window in [pi, between, desktop.notes_window(), desktop.window_titled("Elsewhere")]:
        desktop.as_account(f"niri msg action focus-window --id {window['id']}")
        focused = f"'\"id\":{window['id']},'"
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg --json focused-window | grep -q {focused}"),
            timeout=dt.timedelta(seconds=10),
        )
    closed = desktop.in_session(LAUNCHER_OPEN)

    def tabbed() -> None:
        """Open the windows with Super+Tab, and wait until they're listed."""
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["held"] == ["windows"])

    def shut() -> None:
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    # Its session's words, "Tidy the tests", find it, and so does its
    # quest's whole title, though its window says neither.
    for typed in ["tidy the", "whole"]:
        tabbed()
        desktop.type_in(typed)
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is") == f"window:{pi['id']}"
        )
        shut()
    # Waiting on Joel, it comes straight after the window used last, and
    # says so beside its name.
    question = {
        "kind": "question",
        "since": machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip(),
        "said": "Which way?",
    }
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    tabbed()
    desktop.launcher_until(lambda shown: shown["selected"]["name"] == "Notes for Tuesday")
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["is"] == f"window:{pi['id']}"
            and any(
                thing["is"] == f"window:{pi['id']}" and "waiting on you" in thing["reads"]
                for thing in shown["things"]
            )
        )
    )
    shut()
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.close(between)


# The right arrow steps into what can be done to a window, and a verb
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
        desktop.step_in()
        desktop.type_in(verb)

    into_notes("copy")
    desktop.photograph("launcher-verbs")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == "Notes for Tuesday"

    into_notes("move")
    desktop.step_in("ret")
    desktop.photograph("launcher-target")
    machine.send_key("ret")

    def moved(_: bool) -> bool:
        """Say whether the notes' window is on another workspace now."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        now = next(w for w in windows if w["id"] == notes["id"])
        return bool(now["workspace_id"] != notes["workspace_id"])

    retry(moved, timeout=dt.timedelta(seconds=10))


# Stepping back out of a collection finds the launcher as it was left,
# on the row that was selected, not back at the top.


def test_the_launcher_remembers_where_it_was_when_stepping_back(desktop: Desktop) -> None:
    """The launcher remembers where it was when stepping back."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()

    def selected() -> str:
        """Give the name of the launcher's selected row, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("name", ""))

    # Down past the first row to Programs, a collection the VM always has
    # some of (Agents, above it, is empty here).
    while True:
        before = selected()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name", "") != before  # noqa: B023
        )
        if desktop.showing()["selected"]["is"] == "collection:applications":
            break
    chosen = selected()
    desktop.step_in()
    machine.send_key("down")
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: shown["held"] == [] and (shown["selected"] or {}).get("name") == chosen
    )
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


def test_a_detail_inside_a_thing_is_found_from_the_root_and_return_goes_there(
    desktop: Desktop,
) -> None:
    """A detail inside a thing is found from the root, and Return goes there."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    where = "Desktop \u203a Screens \u203a "

    def spot(shown: Json) -> int:
        """Give where the screen's Rotation is listed, saying where it lives, or -1."""
        for at, thing in enumerate(shown["things"]):
            if thing["reads"].startswith("Rotation ") and where in thing["reads"]:
                return at
        return -1

    # A screen's rotation is a step inside the screen, inside Screens,
    # inside Desktop; from the root it's found by its name, saying so,
    # below the focused screen's answer.
    desktop.answer("rotation")
    desktop.launcher_until(lambda shown: spot(shown) >= 0)
    for _ in range(spot(desktop.showing())):
        machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Rotation"
            and str((shown["selected"] or {}).get("side", "")).startswith(where)
        )
    )
    # Return goes there: the crumbs say each step, and the spot is selected.
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["path"][:2] == ["Desktop", "Screens"]
            and len(shown["path"]) == len(["Desktop", "Screens", "a screen"])
            and (shown["selected"] or {}).get("name") == "Rotation"
        )
    )
    screen = desktop.showing()["path"][-1]
    # The left arrow goes back up, onto the screen it was inside.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Desktop", "Screens"]
            and (shown["selected"] or {}).get("name") == screen
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_the_launchers_home_lists_the_categories_to_browse_each_saying_what_it_holds(
    desktop: Desktop,
) -> None:
    """The launcher's home lists the categories to browse, each saying what it holds."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # The same rows every time: Now first, then each place in the order
    # the approved proposal gives them.
    home = ["Now", "Agents", "Programs", "Desktop", "Machine", "Network", "Disk", "Keyboard"]
    desktop.launcher_until(lambda shown: shown["rows"] == home)

    def selected_is() -> str:
        """Give what the launcher's selected row is, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("is", ""))

    def foot() -> tuple[int, int]:
        """Give where the foot's keys start, and how long how things stand reads."""
        shown = desktop.showing()
        return shown["keysAt"], len(shown["glance"])

    # Down through every row to the last. The foot holds still on the
    # way, whatever each row's keys say: the keys in their places, and
    # how things stand beside them as long (a minute turning keeps its
    # length).
    feet = {selected_is(): foot()}
    for _row in home[1:]:
        before = selected_is()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's row.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is", "") != before  # noqa: B023
        )
        feet[selected_is()] = foot()
    assert selected_is() == "collection:keyboard"
    assert len(set(feet.values())) == 1, f"the foot moved as the arrows went: {feet}"
    # Each says what it holds after its name, in its own order, from the
    # vocabulary; Now says what's on, or that nothing is.
    reads = {thing["is"]: thing["reads"] for thing in desktop.showing()["things"]}
    assert reads["category:desktop"].startswith("Desktop \u00b7 Windows, Screens, "), (
        f"desktop reads {reads['category:desktop']!r}"
    )
    assert reads["collection:on"].startswith("Now \u00b7 "), f"now reads {reads['collection:on']!r}"
    assert reads["collection:applications"].startswith("Programs \u00b7 "), reads
    # Home holds still while the machine's queue changes, which a busy
    # machine writes all the time: it once made home again and moved it
    # from under the selection. Nothing of the machine's shows at home,
    # so the queue asks nothing again there at all.
    scrolled = desktop.showing()
    since = desktop.mark()
    queued = {"v": 1, "heartbeat": float(machine.succeed("date +%s")), "machine": {}, "jobs": []}
    desktop.as_account(
        f"echo '{json.dumps(queued)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    still = desktop.showing()
    assert (still["scrolled"], still["selected"]) == (scrolled["scrolled"], scrolled["selected"]), (
        f"home moved as the queue changed: from {scrolled} to {still}"
    )
    desktop.photograph("launcher-home-categories")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    said = desktop.as_account(
        f"journalctl --user --unit=quickshell --no-pager --output=cat --after-cursor='{since}'"
    )
    assert "asked again by machine" not in said, f"the queue asked home again: {said}"
    # Tab, with nothing typed, does nothing and says nothing, since the
    # arrows reach every category; Return on one steps into it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    first = desktop.showing()["selected"]
    machine.send_key("tab")
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:agents")
    # Had Tab moved to the first place, the arrow would be past it.
    assert first["is"] == "collection:on", first
    for _step in range(2):
        machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "category:desktop")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["desktop"] and shown["rows"][:2] == ["Windows", "Screens"]
    )
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


def test_the_launchers_agents_hold_still_as_one_changes_the_selection_on_its_row(
    desktop: Desktop,
) -> None:
    """The launcher's agents hold still as one changes, the selection on its row."""
    machine = desktop.machine
    desktop.pi_session()
    # A second session in a window of its own, with its own presence. Its
    # title is the shape pi gives one now, the quest's short name and no
    # address, so only the title its presence says joins the two.
    other = "\u03c0 - Another - other-tree"

    def present(address: str, said: Json) -> None:
        """Replace a session's presence with this, by rename, as pi does."""
        name = address.lower()
        if address == "TESTQB":
            said = {**said, "title": other}
        desktop.as_account(
            f"echo '{json.dumps(said)}' > {PI_PRESENCE}/.{name}"
            f" && mv {PI_PRESENCE}/.{name} {PI_PRESENCE}/{name}.json"
        )

    present("TESTQB", {"addresses": ["TESTQB"], "busy": True})
    desktop.as_account(f"niri msg action spawn -- kitty --title '{other}'")
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json windows | grep -qF '{other}'"),
        timeout=STARTING,
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["agents"] and len(shown["rows"]) > 1)
    # The window titled the new way is its session, named by its title.
    assert "Another" in desktop.showing()["rows"], desktop.showing()["rows"]
    # The second, which would go first once it wants Joel.
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == shown["rows"][1])
    shown = desktop.showing()
    order = shown["rows"]
    chosen = shown["selected"]["name"]
    address = shown["selected"]["side"].split(" \u00b7 ")[0]
    stays = "TESTQA" if address == "TESTQB" else "TESTQB"
    # It comes to want Joel, a want older than the shell so no notice
    # takes the screen, and the other stops working.
    present(
        address,
        {
            "addresses": [address],
            "busy": False,
            "wants": {"kind": "question", "since": "2000-01-01T00:00:00Z", "said": "Which?"},
        },
    )
    present(stays, {"addresses": [stays], "busy": False})
    desktop.launcher_until(
        lambda shown: (
            "asks you" in (shown["selected"] or {}).get("side", "")
            and any("waiting" in thing["reads"] for thing in shown["things"])
        )
    )
    # Said in place: the same rows in the same order, the selection
    # still on its row.
    shown = desktop.showing()
    assert shown["rows"] == order, "the agents moved under the selection"
    assert shown["selected"]["name"] == chosen, "the selection left its row"
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(other)['id']}")
    desktop.as_account(f"rm -f {PI_PRESENCE}/testqb.json")
    present("TESTQA", {"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q TESTQB"),
        timeout=dt.timedelta(seconds=30),
    )


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
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


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
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Float" in shown["rows"])
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    desktop.launcher_until(lambda shown: "Tile" in shown["rows"] and "Float" not in shown["rows"])
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    front = desktop.window_titled("In front of the notes")
    desktop.as_account(f"niri msg action close-window --id {front['id']}")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q 'In front of'"),
        timeout=dt.timedelta(seconds=30),
    )


# The programs using the processors and the memory are read as the
# launcher opens, so they're among the parts the first time each is
# asked, even stepped into at once, where nothing is asked again.


def test_the_machines_programs_are_there_the_first_time_its_asked(desktop: Desktop) -> None:
    """The machine's programs are there the first time it's asked."""
    machine = desktop.machine
    for figure in ("cpu", "memory"):
        desktop.answer(figure)
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Using it" in shown["rows"])
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: not shown["open"])


def test_a_word_typed_whole_lands_on_what_it_says(desktop: Desktop) -> None:
    """A word typed whole lands on what it says."""
    machine = desktop.machine
    # "pi" is meant for pi's sessions, so there has to be one.
    desktop.pi_session()
    closed = desktop.in_session(LAUNCHER_OPEN)

    def selects(key: str) -> Callable[[Json], bool]:
        """Give a test that Return's row is the one with this key."""
        return lambda shown: bool(shown["selected"]) and shown["selected"]["is"] == key

    # "quiet" is do not disturb's own word, and only the start of the
    # volume's "quieter". "pi" is two letters, too few for a
    # collection's words to count by their start, so the Clipboard's
    # pictures don't. "copy" and "light" each once found nothing.
    lands: list[tuple[str, Callable[[Json], bool]]] = [
        ("quiet", selects("session:quiet")),
        (
            "pi",
            lambda shown: selects("collection:agents")(shown) and "Clipboard" not in shown["rows"],
        ),
        ("copy", selects("collection:clipboard")),
        ("light", lambda shown: "Brighten the screen" in shown["rows"]),
    ]
    for typed, holds in lands:
        desktop.answer(typed)
        desktop.launcher_until(holds)
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


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
    # theirs.
    layers = ["Desktop", "Terminal"]
    desktop.launcher_until(lambda shown: shown["rows"] == layers)
    desktop_row = desktop.showing()["things"][0]["reads"]
    assert re.fullmatch(r"Desktop \u00b7 \d+ \u00b7 Windows, Columns", desktop_row), desktop_row
    desktop.photograph("launcher-shortcut-layers")
    # The desktop holds its groups, by what a chord acts on.
    desktop.step_in()
    groups = ["Windows", "Columns", "Workspaces", "Screens", "Launcher", "Typing"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(groups)] == groups)
    desktop.photograph("launcher-shortcut-groups")
    # What's typed there finds a group by its name, before every chord in
    # the layer that its words start, flat; a group as big as Workspaces
    # holds groups of its own, then what none of them takes.
    desktop.type_in("work", selecting="Workspaces")
    assert "Go to workspace 3" in desktop.showing()["rows"]
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "desktop", "workspaces"])
    inner = ["Focus", "The column", "The window", "Above and below"]
    assert desktop.showing()["rows"][: len(inner)] == inner
    assert "Show every workspace" in desktop.showing()["rows"]
    # The field says where that is as one path, in the rows' own case.
    assert desktop.showing()["path"] == ["Shortcuts", "Desktop", "Workspaces"]
    # Back out: to the desktop with "work" typed, cleared, then the top.
    for _step in range(3):
        machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"] and shown["rows"] == layers)
    # At the top, what's typed finds a chord in any layer, flat.
    desktop.type_in("paste")
    desktop.photograph("launcher-shortcuts")
    for _ in "paste":
        machine.send_key("backspace")
    # The touchscreen's swipes are the desktop's, found by "swipe": one
    # doing what a chord does is that chord's row, saying the swipe in
    # words, and one no chord does is drawn as its fingers and its way.
    swipe = "swipe workspace below"
    desktop.type_in(swipe, selecting="Go to the workspace below")
    below = desktop.showing()
    assert not below["selected"]["chord"].startswith("3 "), below["selected"]
    assert "3 fingers up" in below["footer"], below["footer"]
    assert below["rows"].count("Go to the workspace below") == 1, below["rows"]
    desktop.photograph("launcher-swipes")
    for _ in swipe:
        machine.send_key("backspace")
    swipe = "swipe stop showing"
    desktop.type_in(swipe, selecting="Stop showing every workspace")
    assert desktop.showing()["selected"]["chord"].startswith("4 ")
    for _ in swipe:
        machine.send_key("backspace")
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
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["path"] == ["Shortcuts"])
    # Inside it, a group named what's typed, whole, comes first, and is
    # Return's row.
    desktop.type_in("workspaces", selecting="Workspaces")
    assert desktop.showing()["rows"][0] == "Workspaces"
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


def test_super_shift_over_a_terminal_opens_on_the_terminals_shortcuts(desktop: Desktop) -> None:
    """Super+Shift+/ over a terminal opens on the terminal's shortcuts."""
    machine = desktop.machine
    desktop.focused_kitty()
    machine.send_key("meta_l-shift-slash")
    desktop.launcher_ready()
    # Over kitty the chord opens on the terminal's layer, its groups
    # already showing, the path saying whose keys they are.
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    assert desktop.showing()["path"] == ["Shortcuts", "Terminal"]
    assert desktop.showing()["rows"][:2] == ["Tabs", "Tab numbers"]
    desktop.photograph("launcher-shortcuts-over-kitty")
    # The right arrow steps into a group, and the left back out to the
    # layers, the terminal's first and selected, saying it's the window's,
    # and the desktop's next.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal", "tabs"])
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"])
    assert desktop.showing()["rows"][:2] == ["Terminal", "Desktop"]
    assert (desktop.showing()["selected"] or {}).get("name") == "Terminal"
    assert desktop.showing()["footer"] == "the keys of the window you were in"
    # Return goes back in.
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


# The orchestrator keeps what waits on Joel in one file, and each quest
# keeps its own beside it.


def test_now_leads_with_what_waits_on_joel(desktop: Desktop) -> None:
    """Now leads with what waits on Joel, and Return goes to where it's done."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # pi's session is the orchestrator, by the name the harness gives it.
    desktop.pi_session()
    desktop.opened("Waiting room")
    queue = {
        "orchestrator": "TESTQA",
        "items": [
            {
                "id": "try",
                "title": "Try the new home",
                "why": "to see",
                "minutes": 10,
                "window": "Waiting room",
            },
            {
                "id": "pick",
                "title": "Pick a bucket",
                "why": "backups",
                "minutes": 1,
                "window": None,
            },
        ],
    }
    # A quest's own, ready to do now.
    asked = {"items": [{"id": "ask", "title": "Answer the ask", "minutes": 2, "ready": True}]}
    state = "$HOME/.local/state"
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(queue))} > {state}/mission/.q"
        f" && mv {state}/mission/.q {state}/mission/queue.json"
    )
    # One ready is said as a notice the moment it's heard, named by what
    # it asks, so it's never a second "Waiting on you" beside Now's.
    with desktop.told("shows card Answer the ask"):
        desktop.as_account(
            f"printf %s {shlex.quote(json.dumps(asked))} > {state}/mission/waiting/.q"
            f" && mv {state}/mission/waiting/.q {state}/mission/waiting/QUEST1.json"
        )
    # Another window in front, so going to the waiting room is seen.
    desktop.opened("Elsewhere")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Home's Now names the first Joel can do now, with its minutes.
    desktop.launcher_until(
        lambda shown: any(
            thing["reads"].startswith("Now \u00b7 Waiting on you: Answer the ask, 2 min")
            for thing in shown["things"]
        )
    )
    # Inside, they're listed by name, and Return goes to where one is done.
    desktop.type_in("waiting on you", selecting="Waiting on you")
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Answer the ask", "Pick a bucket", "Try the new home"]
    )
    machine.send_key("down")
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Try the new home"
    )
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled("Waiting room")["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # One naming no window is done with the orchestrator, in its window.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("waiting on you", selecting="Waiting on you")
    desktop.step_in()
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Pick a bucket")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled(PI_WINDOW)["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # Put back as it was, for the tests after.
    desktop.as_account(f"rm {state}/mission/queue.json {state}/mission/waiting/QUEST1.json")
    for title in ["Waiting room", "Elsewhere"]:
        window = desktop.window_titled(title)["id"]
        desktop.as_account(f"niri msg action close-window --id {window}")


# Joel, 2026-10-25: "text alignment and shit constantly shifting around".
# A word that moves without changing tells him nothing and costs him a
# look, so after every key each word on the card that reads as it did must
# be drawn where it was: in the field, in the foot, and in each row that
# is still the same thing saying the same.


def laid_out(desktop: Desktop) -> Json:
    """Give where the launcher draws each word, once two reads in a row agree."""
    was = None
    for _ in range(20):
        now = cast("Json", json.loads(desktop.as_account("qs ipc call launcher laidOut")))
        if now == was:
            return now
        was = now
    return cast("Json", was)


def moved(before: Json, after: Json) -> list[str]:
    """Say each word drawn in both that reads the same and isn't where it was."""
    shifts = []

    def compare(where: str, old: list[Json], new: list[Json]) -> None:
        unmatched: dict[str, list[Json]] = {}
        for word in old:
            unmatched.setdefault(word["text"], []).append(word)
        for word in new:
            same = unmatched.get(word["text"])
            if not same:
                continue
            was = same.pop(0)
            if (was["starts"], was["y"]) != (word["starts"], word["y"]):
                shifts.append(
                    f"{where}: {word['text']!r} moved from {was['starts']},{was['y']}"
                    f" to {word['starts']},{word['y']}"
                )

    compare("the field", before["field"], after["field"])
    compare("the foot", before["foot"], after["foot"])
    rows = {row["at"]: row for row in before["rows"]}
    for row in after["rows"]:
        was = rows.get(row["at"])
        if was and (was["is"], was["reads"]) == (row["is"], row["reads"]):
            compare(f"row {row['at']} ({row['reads']})", was["words"], row["words"])
    return shifts


def test_nothing_on_the_launcher_moves_unless_it_changes(desktop: Desktop) -> None:
    """Nothing on the launcher moves unless it changes, as letters, erasures and arrows go."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    before = laid_out(desktop)
    shifts: list[str] = []
    typed = ""
    keys = [*"sound", "backspace", "backspace", "down", "down", "up", *"nd", "down"]
    for step, key in enumerate(keys):
        was = desktop.showing()["selected"]
        if len(key) == 1:
            typed += key
            machine.send_chars(key)
        else:
            typed = typed[:-1] if key == "backspace" else typed
            machine.send_key(key)
        # An arrow is heard once the selection moves, a letter once it's held.
        arrowed = key in {"down", "up"}

        def heard(
            shown: Json, typed: str = typed, arrowed: bool = arrowed, was: Json = was
        ) -> bool:
            return shown["typed"] == typed and (not arrowed or shown["selected"] != was)

        desktop.launcher_until(heard)
        after = laid_out(desktop)
        desktop.photograph(f"launcher-still-{step:02}-{key}")
        shifts += [f"after {key!r} ({typed!r}) {shift}" for shift in moved(before, after)]
        before = after
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert not shifts, "words moved without changing:\n" + "\n".join(shifts)


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

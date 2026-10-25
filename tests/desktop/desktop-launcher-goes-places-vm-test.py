"""The desktop test's launcher goes places area.

Here the launcher goes places: pi's sessions, quests and runs, the volume, kitty, the windows it
finds and moves, and the shortcuts. tests/desktop.nix runs it after desktop_prelude.py, the boot
and the helpers every area shares, so the names it takes from there are imported only for type
checkers.
"""

import datetime as dt
import json
import re
import shlex
import time
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_prelude import (
        ACCENT,
        LAUNCHER_OPEN,
        PI_PRESENCE,
        PI_QUEST,
        PI_WINDOW,
        STARTING,
        Json,
        answer,
        as_account,
        drawn_at,
        focused_kitty,
        in_session,
        launcher_ready,
        launcher_until,
        mark,
        notes_window,
        photograph,
        pi_session,
        presence_says,
        reported,
        settled_screen,
        settles_on,
        shell_said,
        showing,
        step_in,
        told,
        type_in,
        window_titled,
    )
    from driver_names import machine, retry, subtest, t

# A photograph of the media keys' chords, whose marks lie past U+FFFF,
# and Escape leaving.
with subtest("the launcher shows the volume's chords, and Escape closes it"):
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("volume")
    photograph("launcher-volume")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


with subtest("the launcher finds a pi session by its quest, and goes to its window"):
    pi_session()
    # What it spent today, as spend-watch writes it, replaced whole.
    spent = {
        "v": 2,
        "sessions": {"01a1face-test": "$12.50"},
        "helpers": {"01a1face-test": [{"on": "councils", "spent": "$4.00"}]},
    }
    as_account(
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
    as_account(
        f"echo '{json.dumps(lined)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # From the other window, so going to the session's means a move.
    as_account(f"niri msg action focus-window --id {window_titled('Elsewhere')['id']}")
    # The launcher's foot counts it, working, as its presence says.
    since = mark()
    as_account("qs ipc call launcher toggle")
    reported("stands ", lambda rest: "1 working" in rest, since)
    launcher_ready()
    # Its collection by what it's called, then the session named
    # whole, as its README is, not as its window's title.
    type_in("agents")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and "$12.50 today" in shown["selected"]["side"]
            and "2nd in line" in shown["selected"]["side"]
        )
    )
    photograph("launcher-agents")
    # Stepped into, what's known of it, each a row of its own.
    step_in()
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Switch"
            and shown["selected"]["verb"] == "switch"
        )
    )
    # Each named first, as every part is, then what it says.
    for name, side in [
        ("Task", r"QEST-20261022-TESTQA"),
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
        launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"])  # noqa: B023
            )
        )
    photograph("launcher-agent-facts")
    machine.send_key("left")
    launcher_until(lambda shown: shown["held"] == ["agents"])
    as_account("rm $XDG_RUNTIME_DIR/spend-watch/collection.json $XDG_RUNTIME_DIR/jobs/queue.json")
    # Found by its address too.
    type_in("testqa", selecting="A quest of its own & whole")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # Its title is read once, outside what's drawn, so nothing drawn
    # reads it again as it changes.
    log = shell_said("Binding loop")
    assert "Binding loop" not in log, "a session's title is read inside a binding"

    # It comes to want Joel, its presence replaced by rename as pi's
    # is: the margin hangs pi's mark, and lets it go once answered.
    # From another window, so it's said as a notice too, whose
    # Return goes to the session's window.
    as_account(f"niri msg action focus-window --id {window_titled('Elsewhere')['id']}")

    # The margin may hold other marks, a notice missed earlier among
    # them, so it's pi's own that's waited on.
    # Wants begun now, as the shell only says those it saw begin.
    def begun() -> str:
        """Give the machine's time now, as pi writes when a want began."""
        return machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()

    question = {"kind": "question", "since": begun(), "said": "Which <way> & why?"}
    since = mark()
    # Said as written: the summary is plain, so its & isn't escaped.
    with told("shows card A quest of its own & whole asks you"):
        presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so Super+N takes the stack at once: held, it waits.
    with told("keys notices"):
        machine.send_key("meta_l-n")
    # The card says it, so pi's mark waits for the card to go: a card or
    # a mark, never both.
    told_since = as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    t.assertNotRegex(
        told_since, r"holds margin [^\n]*\bagent\b", "pi's mark showed beside its card"
    )
    # The same want, its file replaced again, says nothing new.
    presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    time.sleep(3)
    cards = (
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -c 'report\\[shows card A quest of its own & whole asks you'"
    )
    t.assertEqual(as_account(cards).strip(), "1", "the same want was said twice")
    since = mark()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q notifications"), timeout=dt.timedelta(seconds=10)
    )
    # Its card gone, the want still stands, so pi's mark shows.
    reported("holds margin ", lambda rest: "agent" in rest.split(), since)
    photograph("margin-pi-wants")
    # Another, unanswered, from the other window: its notice says what
    # it wants, and goes when the want does, missed by nobody. Notices
    # missed earlier are let go first, so the margin's notices mark is
    # this one's alone.
    held = as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
    )
    since = mark()
    as_account("qs ipc call notices seen")
    if "waiting" in held.split()[2:]:
        reported("holds margin ", lambda rest: "waiting" not in rest.split(), since)
    as_account(f"niri msg action focus-window --id {window_titled('Elsewhere')['id']}")
    with told("shows card A quest of its own & whole wants your yes"):
        presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "approval", "since": begun(), "said": "Push it?"},
            }
        )
    # Ended while its notice is up, the notice goes with it, well
    # before its own ten seconds would.
    presence_says({"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q notifications"), timeout=dt.timedelta(seconds=5)
    )
    # A third, left to time out while its want stands: it isn't
    # missed, since pi's mark says the want; ended, the mark goes.
    with told("shows card A quest of its own & whole is stuck without you"):
        presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "blocked", "since": begun(), "said": "Out of ideas."},
            }
        )
    machine.wait_until_fails(
        in_session("niri msg layers | grep -q notifications"), timeout=dt.timedelta(seconds=30)
    )
    time.sleep(2)

    def margin_holds() -> list[str]:
        """Give what the margin last said it holds."""
        return as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
        ).split()[2:]

    t.assertNotIn("waiting", margin_holds(), "pi's notice counted as missed")
    since = mark()
    presence_says({"addresses": ["TESTQA"], "busy": False})
    reported("holds margin ", lambda rest: "agent" not in rest.split(), since)
    t.assertNotIn("waiting", margin_holds(), "pi's notice counted as missed")

with subtest(
    "the launcher finds pi's quests, going to the session on one and opening pi on another"
):
    pi_session()
    # Their READMEs as pi keeps them, front matter first: the session's
    # quest, and another in a tree of its own with no session on it.
    another = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-HALLWY"
    as_account(
        "printf '%s\\n' '---' 'id: QEST-20261022-TESTQA' 'kind: quest' 'status: active'"
        " 'priority: driving' 'rank: 1' '---' '# A quest of its own & whole'"
        f" > {PI_QUEST}/README.md"
        f" && mkdir -p {another} ~/hallway && printf '%s\\n' '---' 'id: QEST-20261022-HALLWY'"
        " 'kind: quest' 'status: active' 'priority: queued' 'rank: 1' 'trees:'"
        ' "  - path: $HOME/hallway"'
        f" '---' '# Paint the hallway' > {another}/README.md"
    )
    presence_says({"addresses": ["TESTQA"], "busy": False, "owns": ["QEST-20261022-TESTQA"]})
    as_account(f"niri msg action focus-window --id {window_titled('Elsewhere')['id']}")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    # pi's word finds them, named Tasks in Joel's.
    type_in("quests")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Tasks")
    machine.send_key("ret")
    # The one driving first, naming the session that has it loaded.
    launcher_until(
        lambda shown: (
            shown["held"] == ["tasks"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and shown["selected"]["side"] == "driving \u00b7 TESTQA"
        )
    )
    machine.send_key("down")
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Paint the hallway"
            and shown["selected"]["side"] == "queued"
        )
    )
    photograph("launcher-quests")
    machine.send_key("up")
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "A quest of its own & whole"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # One with no session, found by a word of its title, opens pi on
    # it in its tree.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("hallway", selecting="Paint the hallway")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session('grep -qx "$HOME/hallway QEST-20261022-HALLWY" $XDG_RUNTIME_DIR/pi-opened'),
        timeout=dt.timedelta(seconds=30),
    )
    as_account(f"rm -rf {another} ~/hallway $XDG_RUNTIME_DIR/pi-opened")

with subtest(
    "the launcher finds pi's runs, going to the session that started one and reading an answer"
):
    pi_session()
    # A fleet as the harness keeps one, started by the session, whose
    # window's process stands in for pi's: one member still at work,
    # its process the other window's, and one done with its answer.
    leader = window_titled(PI_WINDOW)["pid"]
    crew = "~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew/reviewers"

    def keeps(path: str, said: Json) -> None:
        """Write what the harness keeps of the fleet, at a path inside it."""
        as_account(
            f"mkdir -p {crew}/{path.split('/', maxsplit=1)[0]}"
            f" && echo '{json.dumps(said)}' > {crew}/{path}"
        )

    asked = {"cwd": "/src/house", "args": ["cli.js", "--model", "anthropic/opus"]}
    for member, child, stands in [
        ("brush", window_titled("Elsewhere")["pid"], "running"),
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
    presence_says({"addresses": ["TESTQA"], "busy": False, "owner": {"pid": leader}})
    as_account(f"niri msg action focus-window --id {window_titled('Elsewhere')['id']}")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    # pi's word finds them, named Helpers in Joel's.
    type_in("runs")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Helpers")
    machine.send_key("ret")
    launcher_until(
        lambda shown: (
            shown["held"] == ["helpers"]
            and (shown["selected"] or {}).get("name") == "paint-crew"
            and re.fullmatch(
                r"helpers \u00b7 1 of 2 done \u00b7 (under a minute|\d+ min) \u00b7 TESTQA",
                shown["selected"]["side"],
            )
        )
    )
    photograph("launcher-runs")
    # Stepped into, its members after the way to its session.
    step_in()
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Switch")
    machine.send_key("down")
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "brush"
            and shown["selected"]["side"].startswith("painting the trim \u00b7 opus")
        )
    )
    machine.send_key("down")
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "roller"
            and shown["selected"]["side"] == "opus \u00b7 $2.50 \u00b7 under a minute"
        )
    )
    photograph("launcher-run-members")
    # Its answer, read in a terminal named for the run and the member.
    machine.send_key("ret")
    reading = "paint-crew \u00b7 roller"
    machine.wait_until_succeeds(
        in_session(f"niri msg --json windows | grep -q '{reading}'"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    # Still open a moment on, so the pager holds the answer.
    time.sleep(1)
    as_account(f"niri msg action close-window --id {window_titled(reading)['id']}")
    # Return on the run itself goes to its session.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("helpers")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Helpers")
    machine.send_key("ret")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "paint-crew")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    as_account("rm -rf ~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew")
    for title in [PI_WINDOW, "Elsewhere"]:
        as_account(f"niri msg action close-window --id {window_titled(title)['id']}")
    as_account(f"rm -rf {PI_QUEST} {PI_PRESENCE}/testqa.json")
    machine.wait_until_fails(
        in_session("niri msg --json windows | grep -q 'TESTQA\\|\"Elsewhere\"'"),
        timeout=dt.timedelta(seconds=30),
    )

with subtest("the launcher sets the volume to a number, and says how loud it is"):
    # Muted first, since a level asked for is one to be heard.
    as_account("wpctl set-mute @DEFAULT_AUDIO_SINK@ 1")
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("volume 40")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name", "").endswith(" to 40%"))
    machine.send_key("ret")
    # Exactly that, and no longer muted.
    machine.wait_until_succeeds(
        in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.40'"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("volume")
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("40%, through ")
    )
    photograph("launcher-volume-told")
    # A step in, each device's level drawn as a bar.
    step_in()
    photograph("launcher-volume-parts")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))

with subtest("the launcher finds and opens kitty, and learns that kitty was meant"):
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("kitty", selecting="kitty")
    photograph("launcher")
    machine.send_key("ret")
    # The first window opened can take as long as the first GTK one.
    machine.wait_until_succeeds(
        in_session("niri msg windows | grep -i kitty"), timeout=dt.timedelta(seconds=STARTING)
    )
    # What was picked for what was typed is kept for next time.
    picked = '\'"kitty":{"program:kitty"\''
    machine.wait_until_succeeds(
        in_session(f"grep -rqsF {picked} ~/.local/state/quickshell"),
        timeout=dt.timedelta(seconds=10),
    )

# The border is all that marks the focused window, and niri leaves
# borders off unless its config turns them on.
with subtest("the focused window is framed in the accent"):
    # The kitty the launcher opened, or one of its own when run alone.
    bordered = focused_kitty()
    as_account(f"niri msg action move-window-to-floating --id {bordered['id']}")

    def beside() -> str:
        """Give the colour just left of the focused window, where its border is."""
        image = settled_screen("focused")
        left, top, _, height = drawn_at(json.loads(as_account("niri msg --json focused-window")))
        # An RGB picture's pixels are each a red, a green and a blue.
        pixel = cast("tuple[int, int, int]", image.getpixel((left - 1, top + height // 2)))
        return "#{:02x}{:02x}{:02x}".format(*pixel)

    framed = settles_on(ACCENT.lower(), beside)
    as_account(f"niri msg action move-window-to-tiling --id {bordered['id']}")
    t.assertEqual(framed, ACCENT.lower())

# niri opens a window in a column of its own, and the shell folds it
# in just below the focused one.
with subtest("a terminal opens below the focused window"):
    anchor = focused_kitty()
    as_account("qs ipc call grid below kitty")
    kitties = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    machine.wait_until_succeeds(
        in_session(f"{kitties} | grep -qx 2"), timeout=dt.timedelta(seconds=STARTING)
    )

    def placed(_: bool) -> bool:
        """Say whether the new terminal sits just below the one focused before."""
        windows = json.loads(as_account("niri msg --json windows"))
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
with subtest("the launcher loses no letter typed straight after its chord"):
    as_account(
        "niri msg action spawn -- kitty --title type-ahead "
        "sh -c 'stty -icanon -echo; cat > /tmp/type-ahead'"
    )
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q type-ahead"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    for gap in (0.01, 0.05, 0.1):
        beneath = len(as_account("cat /tmp/type-ahead"))
        machine.send_key("meta_l-spc")
        machine.send_chars("slack", delay=gap)
        launcher_ready()

        def none_lost(_: bool, beneath: int = beneath, gap: float = gap) -> bool:
            """Say whether every letter typed went beneath or to the launcher, in order."""
            went = as_account("cat /tmp/type-ahead")[beneath:]
            held = showing()["typed"]
            print(f"TYPEAHEAD {gap}: '{went}' went beneath, '{held}' held")
            return bool(went + held == "slack")

        retry(none_lost, timeout=dt.timedelta(seconds=10))
        machine.send_key("esc")
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
        machine.wait_until_succeeds(
            in_session("niri msg focused-window | grep -q type-ahead"),
            timeout=dt.timedelta(seconds=10),
        )
    as_account("pkill -f 'cat > /tmp/type-ahead'")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q type-ahead"), timeout=dt.timedelta(seconds=30)
    )

# The launcher finds a window by its title, and Return goes to it.
with subtest("the launcher switches to a window by its title"):
    as_account("niri msg action spawn -- kitty --title 'Notes for Tuesday'")
    machine.wait_until_succeeds(
        in_session("niri msg windows | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    as_account("niri msg action focus-window-previous")
    machine.wait_until_fails(
        in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("tuesday", selecting="Notes for Tuesday")
    photograph("launcher-window")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )

# Super+Tab opens the launcher on every other window, the one used last
# first, so Return goes back to it, and a few letters find any other,
# among two dozen as easily as among two, with nothing held down. The
# windows are one kitty's, so the machine holds them all.
with subtest("Super+Tab goes back to the window before, and finds any by typing"):
    notes_window()
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
        as_account(
            "niri msg action spawn -- kitty --single-instance"
            f" --instance-group scratch --title '{title}'"
        )
        machine.wait_until_succeeds(
            in_session(f"niri msg windows | grep -q '{title}'"),
            timeout=dt.timedelta(seconds=STARTING),
        )
    notes = window_titled("Notes for Tuesday")
    as_account(f"niri msg action focus-window --id {notes['id']}")
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    closed = in_session(LAUNCHER_OPEN)

    def tab_to(typed: str, title: str) -> None:
        """Open the windows with Super+Tab, type, and wait until the window titled is chosen."""
        machine.send_key("meta_l-tab")
        launcher_ready()
        launcher_until(lambda shown: shown["held"] == ["windows"])
        if typed:
            type_in(typed, selecting=title)
        launcher_until(lambda shown: shown["selected"] and shown["selected"]["name"] == title)

    def went(title: str) -> None:
        """Press Return, and wait until the launcher closes on the window titled."""
        machine.send_key("ret")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        machine.wait_until_succeeds(
            in_session(f"niri msg focused-window | grep -q '{title}'"),
            timeout=dt.timedelta(seconds=10),
        )

    # The window used last is chosen as it opens, and this one isn't
    # there to choose.
    tab_to("", scratch[-1])
    photograph("launcher-windows")
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
        as_account(f"niri msg action close-window --id {window_titled(title)['id']}")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q Scratch"), timeout=dt.timedelta(seconds=30)
    )
    as_account(f"niri msg action focus-window --id {notes['id']}")

# The right arrow steps into what can be done to a window, and a verb
# that needs a target asks for it in the same field.
with subtest("the launcher moves a window and copies its title"):
    notes = notes_window()

    def into_notes(verb: str) -> None:
        """Find the notes' window in the launcher, step into it, and type a verb."""
        as_account("qs ipc call launcher toggle")
        launcher_ready()
        type_in("tuesday", selecting="Notes for Tuesday")
        step_in()
        type_in(verb)

    into_notes("copy")
    photograph("launcher-verbs")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    t.assertEqual(as_account("wl-paste --no-newline"), "Notes for Tuesday")

    into_notes("move")
    step_in("ret")
    photograph("launcher-target")
    machine.send_key("ret")

    def moved(_: bool) -> bool:
        """Say whether the notes' window is on another workspace now."""
        windows = json.loads(as_account("niri msg --json windows"))
        now = next(w for w in windows if w["id"] == notes["id"])
        return bool(now["workspace_id"] != notes["workspace_id"])

    retry(moved, timeout=dt.timedelta(seconds=10))

# Stepping back out of a collection finds the launcher as it was left,
# on the row that was selected, not back at the top.
with subtest("the launcher remembers where it was when stepping back"):
    as_account("qs ipc call launcher toggle")
    launcher_ready()

    def selected() -> str:
        """Give the name of the launcher's selected row, or nothing."""
        return cast("str", (showing()["selected"] or {}).get("name", ""))

    # Down past the first row to a collection's.
    while True:
        before = selected()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's name.
        launcher_until(
            lambda shown: (shown["selected"] or {}).get("name", "") != before  # noqa: B023
        )
        if showing()["selected"]["is"].startswith("collection:"):
            break
    chosen = selected()
    step_in()
    machine.send_key("down")
    machine.send_key("left")
    launcher_until(
        lambda shown: shown["held"] == [] and (shown["selected"] or {}).get("name") == chosen
    )
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])

with subtest("the launcher's home lists the categories to browse, each saying what it holds"):
    as_account("qs ipc call launcher toggle")
    launcher_ready()

    def selected_is() -> str:
        """Give what the launcher's selected row is, or nothing."""
        return cast("str", (showing()["selected"] or {}).get("is", ""))

    # Down past what's in front, the likely picks and the shelf, into
    # the categories, the first of them first.
    while not selected_is().startswith("category:"):
        before = selected_is()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's row.
        launcher_until(
            lambda shown: (shown["selected"] or {}).get("is", "") != before  # noqa: B023
        )
    t.assertEqual(selected_is(), "category:computer")
    # Each says what it holds after its name, from the vocabulary.
    reads = {thing["is"]: thing["reads"] for thing in showing()["things"]}
    t.assertTrue(
        reads["category:ai"].startswith("AI \u00b7 Agents, Tasks, Helpers, AI spend"),
        f"ai reads {reads['category:ai']!r}",
    )
    photograph("launcher-home-categories")
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])
    # Tab, with nothing typed, goes straight to them, and Return on one
    # steps into it.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    machine.send_key("tab")
    launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "category:computer")
    for _row in range(4):
        machine.send_key("down")
    launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "category:ai")
    step_in("ret")
    launcher_until(lambda shown: shown["held"] == ["ai"] and "Agents" in shown["rows"])
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])

with subtest("the launcher's agents hold still as one changes, the selection on its row"):
    pi_session()
    # A second session in a window of its own, with its own presence.
    other = "\u03c0 - TESTQB Another of its own - other-tree"

    def present(address: str, said: Json) -> None:
        """Replace a session's presence with this, by rename, as pi does."""
        name = address.lower()
        as_account(
            f"echo '{json.dumps(said)}' > {PI_PRESENCE}/.{name}"
            f" && mv {PI_PRESENCE}/.{name} {PI_PRESENCE}/{name}.json"
        )

    present("TESTQB", {"addresses": ["TESTQB"], "busy": True})
    as_account(f"niri msg action spawn -- kitty --title '{other}'")
    machine.wait_until_succeeds(
        in_session(f"niri msg --json windows | grep -qF '{other}'"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    type_in("agents")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    launcher_until(lambda shown: shown["held"] == ["agents"] and len(shown["rows"]) > 1)
    # The second, which would go first once it wants Joel.
    machine.send_key("down")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == shown["rows"][1])
    shown = showing()
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
    launcher_until(
        lambda shown: (
            "asks you" in (shown["selected"] or {}).get("side", "")
            and any("waiting" in thing["reads"] for thing in shown["things"])
        )
    )
    # Said in place: the same rows in the same order, the selection
    # still on its row.
    shown = showing()
    t.assertEqual(shown["rows"], order, "the agents moved under the selection")
    t.assertEqual(shown["selected"]["name"], chosen, "the selection left its row")
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])
    as_account(f"niri msg action close-window --id {window_titled(other)['id']}")
    as_account(f"rm -f {PI_PRESENCE}/testqb.json")
    present("TESTQA", {"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        in_session("niri msg --json windows | grep -q TESTQB"), timeout=dt.timedelta(seconds=30)
    )

# A window's verbs, stepped into from Super+Tab, follow it as it changes
# while its row reads the same: floated, its Float says Tile.
with subtest("a window's verbs follow it while they're shown"):
    notes = notes_window()
    # Another window in front, so the notes' is among Super+Tab's.
    as_account("niri msg action spawn -- kitty --title 'In front of the notes'")
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q 'In front of the notes'"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    machine.send_key("meta_l-tab")
    launcher_ready()
    launcher_until(lambda shown: shown["held"] == ["windows"])
    type_in("tuesday", selecting="Notes for Tuesday")
    step_in()
    launcher_until(lambda shown: "Float" in shown["rows"])
    as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    launcher_until(lambda shown: "Tile" in shown["rows"] and "Float" not in shown["rows"])
    machine.send_key("esc")
    launcher_until(lambda shown: not shown["open"])
    as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    front = window_titled("In front of the notes")
    as_account(f"niri msg action close-window --id {front['id']}")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q 'In front of'"),
        timeout=dt.timedelta(seconds=30),
    )

# The programs using the processors and the memory are read as the
# launcher opens, so they're among the parts the first time each is
# asked, even stepped into at once, where nothing is asked again.
with subtest("the machine's programs are there the first time it's asked"):
    for figure in ("cpu", "memory"):
        answer(figure)
        step_in()
        launcher_until(lambda shown: "Using it" in shown["rows"])
        machine.send_key("esc")
        launcher_until(lambda shown: not shown["open"])

with subtest("a word typed whole lands on what it says"):
    # "pi" is meant for pi's sessions, so there has to be one.
    pi_session()
    closed = in_session(LAUNCHER_OPEN)

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
        answer(typed)
        launcher_until(holds)
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

with subtest("Super+Shift+/ shows every layer's shortcuts, and runs the desktop's"):
    windows = len(json.loads(as_account("niri msg --json windows")))
    machine.send_key("meta_l-shift-slash")
    launcher_ready()
    # Its top is the layers, each with its count and as many of its first
    # groups as fit whole; the machine has no Neovim or pi, so no layer of
    # theirs.
    layers = ["Desktop", "Touchscreen", "Terminal"]
    launcher_until(lambda shown: shown["rows"] == layers)
    desktop = showing()["things"][0]["reads"]
    t.assertRegex(desktop, r"^Desktop \u00b7 \d+ \u00b7 Windows, Columns$")
    photograph("launcher-shortcut-layers")
    # The desktop holds its groups, by what a chord acts on.
    step_in()
    groups = ["Windows", "Columns", "Workspaces", "Screens", "Programs", "Launcher"]
    launcher_until(lambda shown: shown["rows"][: len(groups)] == groups)
    photograph("launcher-shortcut-groups")
    # What's typed there finds a group by its name, before every chord in
    # the layer that its words start, flat; a group as big as Workspaces
    # holds groups of its own, then what none of them takes.
    type_in("work", selecting="Workspaces")
    t.assertIn("Go to workspace 3", showing()["rows"])
    step_in()
    launcher_until(lambda shown: shown["held"] == ["shortcuts", "desktop", "workspaces"])
    inner = ["Going to one", "Moving a column", "Moving a window"]
    t.assertEqual(showing()["rows"][: len(inner)], inner)
    t.assertIn("Show every workspace", showing()["rows"])
    # The field says where that is as one path, in the rows' own case.
    t.assertEqual(showing()["path"], ["Shortcuts", "Desktop", "Workspaces"])
    # Back out: to the desktop with "work" typed, cleared, then the top.
    for _step in range(3):
        machine.send_key("left")
    launcher_until(lambda shown: shown["held"] == ["shortcuts"] and shown["rows"] == layers)
    # At the top, what's typed finds a chord in any layer, flat.
    type_in("paste")
    photograph("launcher-shortcuts")
    for _ in "paste":
        machine.send_key("backspace")
    # The touchscreen's swipes are among them, found by "swipe", each
    # drawn as its fingers and then its way.
    swipe = "swipe workspace below"
    type_in(swipe, selecting="Go to the workspace below")
    t.assertTrue(showing()["selected"]["chord"].startswith("3 "))
    photograph("launcher-swipes")
    for _ in swipe:
        machine.send_key("backspace")
    type_in("open a terminal")
    machine.send_key("ret")

    def opened(_: bool) -> bool:
        """Say whether a window has opened since the shortcuts did."""
        return len(json.loads(as_account("niri msg --json windows"))) > windows

    # kitty starts cold, drawn in software, which on a machine running
    # other tests too has taken 10 seconds and more than 19, so it's
    # given a minute.
    retry(opened, timeout=dt.timedelta(minutes=1))

    # In the shortcuts category, a group is found by its name too, as the
    # Shortcuts row there.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    machine.send_chars("shortcuts ")
    launcher_until(lambda shown: shown["held"] == ["shortcuts"])
    # It holds Shortcuts alone, no second Desktop beside the layer.
    t.assertNotIn("Desktop", showing()["rows"])
    # A group named what's typed, whole, comes first, and is Return's row.
    type_in("workspaces", selecting="Workspaces")
    t.assertEqual(showing()["rows"][0], "Workspaces")
    # Stepping into Shortcuts from its category says Shortcuts once.
    for _ in "workspaces":
        machine.send_key("backspace")
    type_in("shortcuts", selecting="Shortcuts")
    step_in()
    launcher_until(lambda shown: shown["path"] == ["Shortcuts"])
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

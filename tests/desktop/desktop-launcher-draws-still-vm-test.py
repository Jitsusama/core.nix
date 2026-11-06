"""The desktop test's launcher draws still area.

Here the launcher draws only what changes: a list scrolled, a burst of changes, the foot's facts
and marks. That it draws nothing left still, glides a selection only by the arrows, draws it on
its row and reads on a long name is proven on a nested niri, in tests/launcher-on-niri-test.py.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import itertools
import json
import re
import shlex
import unicodedata
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    LAUNCHER_OPEN,
    STARTING,
    laid_out,
    retry,
    started_apart,
)

if TYPE_CHECKING:
    from collections.abc import Iterator

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


def test_a_closed_launcher_makes_nothing_of_the_machine_queue(desktop: Desktop) -> None:
    """Rewrite the machine queue's file while the launcher is closed, and see it read once open.

    On a busy machine the file is rewritten up to 12 times a second. The search's rows, which
    read Queue's note, were made again whole on each write with the launcher closed: two
    thirds of the shell's idle processor on 2026-10-29. Then each write was still read and
    heard, about 20 wakeups and 14 ms a write (2026-10-30), and then only noted, each write
    still waking it (2026-10-31). Now a closed launcher doesn't watch the queue at all, and
    reads it once as it opens, its note saying the last write.
    """
    jobs = "$XDG_RUNTIME_DIR/jobs"
    # Once as it opens, once as Agents makes its rows on opening, once as
    # the queue is read on opening, and three to spare for anything else
    # the desktop changes meanwhile: run soon after the VM starts, its
    # displays, networks and Machine were still said again as it opened,
    # which made one more (2026-10-30). Made again on each write, it would
    # be ten more.
    joins_on_opening = 6
    # The shell's first opening joins its sources as they first come, so
    # it's had before counting, and the test counts the same run first or
    # later.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.close_launcher()
    before = int(desktop.as_account("qs ipc call launcher joined"))
    since = desktop.mark()
    lined: Json = {"v": 1, "heartbeat": 0.0, "machine": {}, "jobs": []}
    for gib in range(11, 21):
        lined["heartbeat"] = float(gib)
        lined["machine"] = {"availableGib": gib}
        desktop.as_account(
            f"mkdir -p {jobs} && echo '{json.dumps(lined)}' > {jobs}/.q"
            f" && mv {jobs}/.q {jobs}/queue.json"
        )

    def watches() -> int:
        """How many of the shell's inotify watches are on the queue's folder."""
        return int(
            desktop.as_account(
                "p=$(systemctl --user show quickshell -p MainPID --value);"
                f" i=$(printf '%x' $(stat -c %i {jobs}));"
                ' cat /proc/$p/fdinfo/* 2>/dev/null | grep -c " ino:$i " || true'
            ).strip()
        )

    # Closed, a write never so much as wakes it: the folder isn't watched.
    assert watches() == 0, "the closed launcher watches the queue's folder"
    try:
        desktop.answer("queue")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Queue"
                and shown["selected"]["note"].startswith("20 GiB to spare")
            )
        )
        # Open, it is, so the count above looked where a watch would be.
        assert watches() > 0, "the open launcher doesn't watch the queue's folder"
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        heard = re.findall(r"report\[(heard jobs [^\]]*)\]", said)
        assert heard == ["heard jobs 20"], f"the closed launcher heard the queue: {heard}"
        joined = int(desktop.as_account("qs ipc call launcher joined")) - before
        # A loaded builder makes the VM lag, and each lag heard changes
        # Machine's Lag row, which can join them once more: six lags made
        # one more on 2026-11-04. None comes of the queue's writes.
        lags = len(re.findall(r"report\[heard lag ", said))
        assert joined <= joins_on_opening + lags, (
            f"the closed launcher joined its rows {joined} times, {lags} lags heard"
        )
    finally:
        desktop.close_launcher()
        desktop.as_account(f"rm -f {jobs}/queue.json")


def test_an_open_launcher_takes_a_burst_of_changes_together(desktop: Desktop) -> None:
    """Rewrite the machine queue's file 30 times in 3 s while the launcher is open.

    Each change a source says joins the whole corpus again and searches what's shown, so 86 of
    Slack's answers, a tenth of a second apart just after a restart, held the open launcher a
    minute (2026-10-30). A burst is taken as one refresh, then one each quarter second.
    """
    machine = desktop.machine
    jobs = "$XDG_RUNTIME_DIR/jobs"
    writes = 30

    # Each write holds another job, so the queue's rows change and it says
    # so, and says how much is to spare, which its report names.
    def lined(at: int) -> str:
        job = {
            "id": f"burst-{at}",
            "class": "bulk",
            "kind": "build",
            "state": "waiting",
            "owner": {"session": "elsewhere", "pid": 1},
            "what": "nix build .#lighthouse",
            "name": f"lighthouse {at}",
            "memoryGib": 1,
        }
        spare = {"availableGib": at}
        return json.dumps({"v": 1, "heartbeat": 0.0, "machine": spare, "jobs": [job]})

    # A tenth of a second apart, as Slack's answers came, so each is heard
    # as a change of its own: written at once, a busy shell hears several
    # as one, and there's nothing left to take together.
    burst = " && sleep 0.1 && ".join(
        f"echo '{lined(at)}' > {jobs}/.q && mv {jobs}/.q {jobs}/queue.json"
        for at in range(1, writes + 1)
    )
    desktop.as_account(f"mkdir -p {jobs}")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    try:
        since = desktop.mark()
        desktop.as_account(burst)
        desktop.reported("heard jobs ", lambda rest: rest == f"{writes}", since)
        # The last change is in once a refresh is said after it was heard.
        last = (
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
            f" | sed -n '/heard jobs {writes}]/,$p' | grep -q 'refreshed jobs'"
        )
        machine.wait_until_succeeds(desktop.in_session(last), timeout=dt.timedelta(seconds=10))
        heard = started_apart(desktop, since, "report[heard jobs ")
        refreshes = started_apart(desktop, since, "report[refreshed jobs]")
        assert len(heard) >= writes // 3, f"only {len(heard)} changes heard, too few to tell"
        # Taken one by one, each change heard was a refresh of its own, some
        # a tenth of a second after the last (2026-10-30). Together, there's
        # one at once and then at most four a second while they come, each
        # begun a quarter second after the last began. That's read from the
        # shell's own clock as each begins: the journal hears a refresh's
        # report as it ends, and on a loaded machine one slowed refresh put
        # its report 0.06 s before the next one's (2026-11-03).
        most = 2 + int(sum(heard) * 4)
        assert len(refreshes) <= most, f"{len(heard)} changes, {len(refreshes)} refreshes"
        began = [
            int(line.split("report[refresh of jobs began ", 1)[1].removesuffix("]"))
            for line in desktop.as_account(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(since)}"
            ).splitlines()
            if "report[refresh of jobs began " in line
        ]
        apart = [later - sooner for sooner, later in itertools.pairwise(began)]
        assert len(began) >= len(refreshes), f"{len(began)} begun, {len(refreshes)} said"
        # A coarse timer, Qt's own, may wake up to 5% early: 12 ms of 250.
        assert min(apart, default=1000) >= 235, f"refreshes began {apart} ms apart"  # noqa: PLR2004
    finally:
        desktop.close_launcher()
        desktop.as_account(f"rm -f {jobs}/queue.json")


# How many rows the launcher's card shows at once.
ROWS_SHOWN = 8


def wheel(desktop: Desktop, x: float, y: float, notches: int) -> None:
    """Turn the wheel there by QEMU's tablet, as desktop.click clicks, down for a positive count."""
    client = desktop.machine.qmp_client
    assert client is not None
    button = "wheel-down" if notches > 0 else "wheel-up"

    def tablet(*events: Json) -> None:
        client.send("input-send-event", json.loads(json.dumps({"events": events})))

    tablet(
        {"type": "abs", "data": {"axis": "x", "value": int(x * 32767)}},
        {"type": "abs", "data": {"axis": "y", "value": int(y * 32767)}},
    )
    for _ in range(abs(notches)):
        tablet({"type": "btn", "data": {"down": True, "button": button}})
        tablet({"type": "btn", "data": {"down": False, "button": button}})


def test_a_refresh_leaves_the_list_where_it_was_scrolled(desktop: Desktop) -> None:
    """A list scrolled by hand stays where it was scrolled as its place refreshes.

    Each refresh once brought the list back to the selection, which snapped a glide under way
    to its end and pulled a list scrolled by hand back to a selection out of sight.
    """
    # More windows that match than the card shows, and three that don't,
    # whose going refreshes what's found without changing it.
    found = [desktop.opened(f"Scrolled {at:02}") for at in range(1, ROWS_SHOWN + 5)]
    others = [desktop.opened(f"Other {at}") for at in range(1, 4)]
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("scrolled")
    desktop.launcher_until(
        lambda shown: (
            len([row for row in shown["rows"] if row.startswith("Scrolled ")]) == len(found)
            and (shown["selected"] or {}).get("name") == shown["rows"][0]
        )
    )
    first = desktop.showing()["rows"][0]
    wheel(desktop, 0.5, 0.5, 3)
    desktop.launcher_until(lambda shown: shown["scrolled"] > 0)
    scrolled = desktop.showing()["scrolled"]
    # After each refresh it's where the wheel left it, the selection still
    # on the first row, out of sight above.
    for window in others:
        since = desktop.mark()
        desktop.close(window)
        desktop.reported("refreshed ", lambda _rest: True, since)
        shown = desktop.showing()
        assert shown["scrolled"] == scrolled, shown
        assert (shown["selected"] or {}).get("name") == first, shown
    desktop.close_launcher()
    for window in found:
        desktop.close(window)


def closed_from_normal(desktop: Desktop) -> None:
    """Open the launcher and close it from normal, as Joel leaves it, before it's measured.

    Every opening but the first follows a close from normal, and the foot once kept normal's
    room from it on the find, which cut the glance's date and charge; a subtest run alone opened
    on a launcher never closed, and passed.
    """
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.normal()
    desktop.close_launcher()


def holds_room_only_for_what_it_draws(desktop: Desktop) -> None:
    """See the find's foot hold room for Return's widest word and the completion, and no more.

    The first letter brings Ctrl+Y's completion beside Return, which then starts just where
    the room held from the opening does: no further in, and not past it. Room once held for
    Shift+Return's "below" and the right arrow, which no foot here draws, cut the date at 1.5
    beside an empty stretch; held for Return alone, the completion cut it on the first letter
    (VKCF3Z, 2026-11-05: "room is held only for what this mode draws").
    """
    machine = desktop.machine
    opened = desktop.showing()
    assert [hint.split(" ")[0] for hint in opened["hints"]] == ["\u21b5"], opened["hints"]
    machine.send_chars("s")
    desktop.launcher_until(lambda shown: any("complete" in hint for hint in shown["hints"]))
    shown = desktop.showing()
    foot = laid_out(desktop)["foot"]
    completion = next(word for word in foot if word["text"].endswith("complete"))
    assert shown["keysFrom"] == opened["keysFrom"], (opened["keysFrom"], shown["keysFrom"])
    assert abs(completion["starts"] - opened["keysFrom"]) <= 1, (
        f"the keys held {completion['starts'] - opened['keysFrom']} px before the completion"
        f" at {completion['starts']}, from {opened['keysFrom']}: {shown['hints']}"
    )
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["typed"] == "")


def test_the_foots_facts_keep_their_room_as_they_read_shorter(desktop: Desktop) -> None:
    """A fact in the foot that reads shorter keeps its room, so nothing after it slides."""
    machine = desktop.machine
    closed_from_normal(desktop)
    # The kernel's stand-in battery, whose charge UPower reads as it changes.
    capacity = "/sys/module/test_power/parameters/battery_capacity"
    was = machine.succeed(f"cat {capacity}").strip()
    machine.succeed(f"echo 100 > {capacity}")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: " 100%" in shown["glance"])
    holds_room_only_for_what_it_draws(desktop)
    before = laid_out(desktop)
    # From 100% to 99%: the battery reads shorter, and what follows it
    # stays where it was.
    machine.succeed(f"echo 99 > {capacity}")
    desktop.launcher_until(lambda shown: " 99%" in shown["glance"])
    after = laid_out(desktop)
    # Back on the find from normal, the foot keeps only the find's room, so
    # the glance draws what it drew before, date and charge, its time aside
    # since a minute may turn (QIFT5D, 2026-11-05: facts gave way to normal's
    # keys, no longer drawn; Y2YBTS: a form normal forced outlived it).
    drew = desktop.showing()["glance"].split("  ")[1:]
    desktop.normal()
    machine.send_chars("/")
    desktop.launcher_until(
        lambda shown: shown["mode"] == "find" and shown["glance"].split("  ")[1:] == drew
    )
    desktop.close_launcher()
    machine.succeed(f"echo {was} > {capacity}")
    # It measures something: the battery is in the foot, and after it more.
    battery = next(word for word in after["foot"] if "99%" in word["text"])
    assert any(word["starts"] > battery["starts"] for word in after["foot"]), after["foot"]
    shifts = moved(
        {"field": [], "foot": before["foot"], "rows": []},
        {"field": [], "foot": after["foot"], "rows": []},
    )
    assert not shifts, "the foot's facts slid as the battery read shorter:\n" + "\n".join(shifts)


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
        desktop.close_launcher()
        machine.wait_until_succeeds(
            desktop.in_session("niri msg focused-window | grep -q type-ahead"),
            timeout=dt.timedelta(seconds=10),
        )
    desktop.as_account("pkill -f 'cat > /tmp/type-ahead'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q type-ahead"),
        timeout=dt.timedelta(seconds=30),
    )


# About as many people as Tuple lists on Joel's machine, said anew every
# 0.3 s to load the shell as Slack's answers do People.
CROWD_LOAD = 8000
# How many times the walk goes round.
WALKS = 2
# How many times each place stepped into is read for where its first row is.
READS = 3


def test_a_place_stepped_into_under_load_shows_its_first_row_at_its_top(
    desktop: Desktop,
) -> None:
    """A place stepped into while the shell is loaded shows its first row at its top.

    Joel saw empty space at the top of a menu, and glitches as he went through them, while
    his machine was busy (2026-10-31, 08:4x). Here the shell is kept busy by a source that
    knows thousands, said anew every 0.3 s, while the keys go as fast as his do.
    """
    machine = desktop.machine
    crowd = "/run/user/1000/crowd"
    machine.succeed(
        "cat > /tmp/crowd-load.sh <<'EOF'\n"
        f"i=0; while [ -e {crowd}-going ]; do i=$((i + 1));"
        f' echo "{CROWD_LOAD} $i" > {crowd}.new && mv {crowd}.new {crowd}; sleep 0.3; done\n'
        "EOF"
    )
    unlike: list[str] = []

    def first_rows(top: int | None) -> None:
        """Read the place shown a few times, noting each read whose first row isn't at top."""
        path = " > ".join(desktop.showing()["path"])
        for _ in range(READS):
            at = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["firstAt"]
            if at != top:
                unlike.append(f"{path}: {at}")

    since = desktop.mark()
    # As a unit of the account's own, since a loop started from su's
    # session ends with it.
    desktop.as_account(
        f"touch {crowd}-going && systemd-run --user --collect --unit=crowd-load"
        " sh /tmp/crowd-load.sh"
    )
    try:
        desktop.reported("crowd ", lambda rest: rest.startswith(f"{CROWD_LOAD} "), since)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        top = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["firstAt"]
        for _ in range(WALKS):
            desktop.down_to("Machine")
            desktop.follow()
            first_rows(top)
            # Down to its last place, below the fold, and straight into it.
            desktop.down_to("Firmware")
            desktop.follow()
            first_rows(top)
            desktop.step_out()
            desktop.step_out()
            desktop.launcher_until(lambda shown: shown["held"] == [])
            # And from home's last place straight into it.
            desktop.down_to("Keyboard")
            desktop.follow()
            first_rows(top)
            desktop.step_out()
            desktop.launcher_until(lambda shown: shown["held"] == [])
            desktop.previous_row()
        assert not unlike, f"the first row stood below the top ({top}): {unlike}"
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
    finally:
        # Quiet again, and known no more, so no later test finds the crowd.
        desktop.as_account(f"rm -f {crowd}-going")
        desktop.as_account(f"echo '0 0' > {crowd}.new && mv {crowd}.new {crowd}")


def marks_in(glance: str) -> list[str]:
    """Give the glyphs the foot's glance wears, in order: its marks, never its words."""
    # The theme's marks all sit in Unicode's private use areas.
    return [mark for mark in glance if unicodedata.category(mark) == "Co"]


def test_the_foots_marks_never_give_way_and_are_there_as_the_shell_starts(
    desktop: Desktop,
) -> None:
    """The foot's marks never give way to the keys, and are all there as the shell starts."""
    machine = desktop.machine
    desktop.pi_session()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # The battery, the network, the volume and the agents, at least.
    fewest = 4
    desktop.launcher_until(lambda shown: len(marks_in(shown["glance"])) >= fewest)
    home = marks_in(desktop.showing()["glance"])
    # A step's row with the most keys: Return, - and Ctrl+] in, and
    # no other chord with Ctrl. Every mark stays beside them (Joel, 2026-10-28:
    # "i want icons to be able to be there as much as possible").
    machine.send_chars("desktop ")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["desktop"] and "Do Not Disturb" in shown["rows"]
    )
    desktop.down_to("Do Not Disturb")
    # In normal, where Ctrl+] goes in; on the find letters type, so it names none.
    desktop.normal()
    shown = desktop.showing()
    told = shown["selected"]["told"]
    assert told[1:2] == ["Ctrl+] in"], shown["selected"]
    assert not any(key.startswith("Ctrl") for key in told[2:]), shown["selected"]
    assert marks_in(shown["glance"]) == home, (shown["glance"], shown["selected"]["told"])
    desktop.close_launcher()
    # A switch restarts the shell: the first foot it draws has every mark,
    # each read from its source as it starts, none waited on.
    desktop.as_account("systemctl --user restart quickshell")
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher showing"), timeout=dt.timedelta(seconds=60)
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    first = desktop.showing()["glance"]
    assert marks_in(first) == home, (first, home)
    desktop.close_launcher()


def test_programs_is_drawn_the_moment_return_steps_into_it(desktop: Desktop) -> None:
    """Programs is drawn the moment Return steps into it, from home."""
    machine = desktop.machine
    # A launcher left open by the subtest before is closed first, so the
    # toggle opens it at home.
    desktop.close_launcher()
    pid = desktop.as_account("systemctl --user show quickshell -p MainPID --value").strip()
    tick = int(desktop.as_account("getconf CLK_TCK").strip())

    def worked() -> float:
        """How long the shell has run on a processor, every thread of it, in milliseconds."""
        fields = desktop.as_account(f"cat /proc/{pid}/stat").rsplit(")", 1)[1].split()
        # utime and stime, the stat's 14th and 15th, counted past its name.
        return (int(fields[11]) + int(fields[12])) * 1000 / tick

    def stepped() -> tuple[float, str]:
        """Step into Programs from home, saying the shell's work across it and its timing."""
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.down_to("Programs")
        since = desktop.mark()
        before = worked()
        machine.send_key("ret")
        # Its programs drawn, not only the step taken: a step that threw
        # once left Programs blank until something else redrew it
        # (2026-10-28).
        desktop.launcher_until(
            lambda shown: shown["held"] == ["programs"] and len(shown["rows"]) > 0
        )
        cost = worked() - before
        # The launcher times a step to its frame as it does a key, and says
        # it once it's felt (Trials.js).
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)} | grep -a -F '\u2192 Programs'; true"
        ).strip()
        print(f"stepping into Programs: {said or 'within a frame'}, {cost:.0f} ms worked")
        desktop.close_launcher()
        return cost, said

    # The VM draws in software, several times slower than a laptop, and
    # takes about 130 ms to step in and draw; this holds it to a few times
    # that, so a step that throws or grows slower in kind fails, and the
    # laptop's own time is read from its journal. What's held is the
    # shell's own work across the step, not the time to its frame, which a
    # builder short of processors stretched past it with the step no slower
    # (566 ms to its frame on 2026-11-03). A loaded builder stretches the
    # work too, a processor shared with other machines doing less in the
    # same time: 460 ms, its parts each three times their usual, beside a
    # key taking 467 ms to its frame (2026-11-04). So the step is taken up
    # to three times and held by its least, which load seldom reaches every
    # time and a step slower in kind always does.
    most = 400
    tries = []
    for _ in range(3):
        tries.append(stepped())
        if tries[-1][0] <= most:
            break
    least = min(cost for cost, _ in tries)
    assert least <= most, f"stepping into Programs worked at least {least:.0f} ms: {tries}"


# Joel, 2026-10-25: "text alignment and shit constantly shifting around".
# A word that moves without changing tells him nothing and costs him a
# look, so after every key each word on the card that reads as it did must
# be drawn where it was: in the field, in the foot, and in each row that
# is still the same thing saying the same. In the foot a hint's key and
# its word read as one, so the last hint's key, which stands left of its
# word to end it on the inset (VKCF3Z, 2026-11-05), moves only as its
# word changes. A key's word stands a mark after it (KeyHints), nearer
# than any two facts stand.
A_KEYS_WORD = 10


def moved(before: Json, after: Json) -> list[str]:
    """Say each word drawn in both that reads the same and isn't where it was."""
    shifts = []

    def compare(where: str, old: list[Json], new: list[Json], *, paired: bool = False) -> None:
        def reads(words: list[Json], at: int) -> str:
            word, later = words[at], words[at + 1] if at + 1 < len(words) else None
            beside = (
                paired
                and later is not None
                and later["y"] == word["y"]
                and 0 <= later["starts"] - word["starts"] - word["inked"] <= A_KEYS_WORD
            )
            return f"{word['text']}\0{later['text'] if beside and later else ''}"

        unmatched: dict[str, list[Json]] = {}
        for at, word in enumerate(old):
            unmatched.setdefault(reads(old, at), []).append(word)
        for at, word in enumerate(new):
            same = unmatched.get(reads(new, at))
            if not same:
                continue
            was = same.pop(0)
            if (was["starts"], was["y"]) != (word["starts"], word["y"]):
                shifts.append(
                    f"{where}: {word['text']!r} moved from {was['starts']},{was['y']}"
                    f" to {word['starts']},{word['y']}"
                )

    compare("the field", before["field"], after["field"])
    compare("the foot", before["foot"], after["foot"], paired=True)
    rows = {row["at"]: row for row in before["rows"]}
    for row in after["rows"]:
        was = rows.get(row["at"])
        if was and (was["is"], was["reads"]) == (row["is"], row["reads"]):
            compare(f"row {row['at']} ({row['reads']})", was["words"], row["words"])
    return shifts


def test_the_pointer_over_the_rows_never_moves_the_selection(desktop: Desktop) -> None:
    """The pointer over the rows, resting or passing, never moves the selection; the keys do."""
    # Joel's pointer rests over the left of the card as he arrows (P9R1KI,
    # 2026-10-29), and the launcher isn't pointer-navigated.
    desktop.point(0.42, 0.45)
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    first = desktop.showing()["rows"]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == first[0])
    # Passing down over every row, then resting.
    for step in range(12):
        desktop.point(0.42, 0.3 + step * 0.04)
    desktop.held_still()
    assert (desktop.showing()["selected"] or {}).get("name") == first[0], desktop.showing()
    # The arrows go where they go, with the pointer resting over the rows
    # as they glide under it.
    desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == first[1])
    desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == first[2])
    desktop.held_still()
    assert (desktop.showing()["selected"] or {}).get("name") == first[2], desktop.showing()
    desktop.close_launcher()
    desktop.point(0.99, 0.99)


def test_a_refresh_draws_no_row_afresh(desktop: Desktop) -> None:
    """Machine reads itself again each second, and each row is kept rather than drawn again."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("machine ")
    desktop.launcher_until(lambda shown: shown["held"] == ["computer"])
    first = laid_out(desktop)

    def read_again(_: bool) -> bool:
        return int(laid_out(desktop)["made"]) >= int(first["made"]) + 3

    retry(read_again, timeout=dt.timedelta(seconds=30))
    then = laid_out(desktop)
    desktop.close_launcher()
    assert then["drawn"] == first["drawn"], (
        f"rows made {then['made'] - first['made']} times drew"
        f" {then['drawn'] - first['drawn']} rows afresh"
    )


def test_nothing_on_the_launcher_moves_unless_it_changes(desktop: Desktop) -> None:
    """Nothing on the launcher moves unless it changes, as letters, erasures and arrows go."""
    machine = desktop.machine
    closed_from_normal(desktop)
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
        # What the foot read before and after, so a move says what took
        # its room as well as what gave it.
        read = [" ".join(word["text"] for word in laid["foot"]) for laid in (before, after)]
        shifts += [
            f"after {key!r} ({typed!r}) {shift}, the foot reading {read[0]!r} then {read[1]!r}"
            for shift in moved(before, after)
        ]
        before = after
    desktop.close_launcher()
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

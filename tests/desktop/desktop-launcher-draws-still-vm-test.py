"""The desktop test's launcher draws still area.

Here the launcher draws only what changes: a list scrolled, a burst of changes, the foot's facts
and marks. That it draws nothing left still, glides a selection only by the arrows, draws it on
its row and reads on a long name is proven on a nested niri, in tests/launcher-on-niri-test.py.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import dataclasses
import datetime as dt
import itertools
import json
import math
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
    still waking it (2026-10-31). Then a closed launcher didn't watch the file at all. Now the
    machine's node watches it, and a closed launcher doesn't watch the node's record of it,
    so it's told nothing until it opens, and then the queue whole, its note saying the last
    write.
    """
    jobs = "$XDG_RUNTIME_DIR/jobs"
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

    # Closed, a write never so much as wakes it: the shell watches no file
    # of the queue's, and the node's record of it isn't watched, which the
    # reports below say, only the opening's heard. Nor are its rows joined
    # again while it's closed, even by a change it does hear, a window
    # opening, heard before it opens, since an open launcher joins them
    # as anything on the desktop changes: a budget for the opening's joins
    # failed whenever the desktop moved meanwhile (2026-11-06). The window
    # is seen heard by the lines Windows reads of it, since a closed
    # Windows holds it and makes no row, so says no change of its own.
    assert watches() == 0, "the closed launcher watches the queue's folder"
    lines = "qs ipc call launcher linesHeard | jq .windows"
    windows = int(desktop.as_account(lines))
    desktop.as_account("niri msg action spawn -- kitty --title still-closed sleep infinity")
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"test $({lines}) -gt {windows}"), timeout=STARTING
    )
    joined = int(desktop.as_account("qs ipc call launcher joined")) - before
    assert joined == 0, f"the closed launcher joined its rows {joined} times"
    try:
        desktop.answer("queue")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Queue"
                and shown["selected"]["note"].startswith("20 GiB to spare")
            )
        )
        # Open, it's told by the node, still watching no file of its own.
        assert watches() == 0, "the open launcher watches the queue's folder"
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        heard = re.findall(r"report\[(heard jobs [^\]]*)\]", said)
        assert heard == ["heard jobs 20"], f"the closed launcher heard the queue: {heard}"
    finally:
        desktop.close_launcher()
        # Bracketed, so the pattern doesn't match the shell running it.
        desktop.as_account(f"rm -f {jobs}/queue.json; pkill -f 'title still-close[d]' || true")


def test_an_open_launcher_takes_a_burst_of_changes_together(desktop: Desktop) -> None:
    """Rewrite the machine queue's file 30 times in 3 s while the launcher is open.

    Each change a source says joins the whole corpus again and searches what's shown, so 86 of
    Slack's answers, a tenth of a second apart just after a restart, held the open launcher a
    minute (2026-10-30). A burst is taken a refresh at a time, none sooner after the last than
    a frame, and no more refreshes than changes heard. How long each waits by what the last
    cost is held in tests/pacing.qml, and that the list keeps to those waits in keeps-up.
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
        # The node tells a watch a change no sooner than a quarter second
        # after the last (mycelium's PROTOCOL.md), so 3 s of writes are
        # heard as about 12.
        assert len(heard) >= writes // 4, f"only {len(heard)} changes heard, too few to tell"
        # Each change waits on the last refresh, and those that came while
        # it waited are taken together, so there are never more refreshes
        # than changes heard. When each began is read from the shell's own
        # clock as it begins: the journal hears a refresh's report as it
        # ends, and on a loaded machine one slowed refresh put its report
        # 0.06 s before the next one's (2026-11-03).
        assert len(refreshes) <= len(heard), f"{len(heard)} changes, {len(refreshes)} refreshes"
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
        # Two within a frame would draw only the second (launcher/Pacing.js).
        # A coarse timer, Qt's own, may wake up to 5% early: 1 ms of 16.
        assert min(apart, default=1000) >= 15, f"refreshes began {apart} ms apart"  # noqa: PLR2004
    finally:
        desktop.close_launcher()
        desktop.as_account(f"rm -f {jobs}/queue.json")


def test_an_act_shows_on_the_frame_after_its_key_with_another_source_changing(
    desktop: Desktop,
) -> None:
    """Return on Do Not Disturb shows on its row at once, with the queue changing too.

    Joel, 2026-11-07: "when i modify something in the launcher, like marking read, snoozing,
    ignore, whatever, it should show right away." A change an act made waited behind the
    quarter second the launcher paces other sources' changes by, as Agents' are on Joel's
    laptop, so the queue lists another job here ten times a second. And a place's row
    whose source no word names, as Desktop's Do Not Disturb is the session's, never showed
    what its act changed while the place was held.
    """
    jobs = "$XDG_RUNTIME_DIR/jobs"

    def lined(at: int) -> str:
        job = {"id": f"pacing-{at}", "class": "bulk", "kind": "build", "state": "waiting",
               "owner": {"session": "elsewhere", "pid": 1}, "what": "nix build .#lighthouse",
               "name": f"lighthouse {at}", "memoryGib": 1}  # fmt: skip
        return json.dumps({"v": 1, "heartbeat": 0.0, "jobs": [job]})

    burst = f"echo $$ > {jobs}/pid; " + "; ".join(
        f"echo '{lined(at)}' > {jobs}/.q && mv {jobs}/.q {jobs}/queue.json; sleep 0.1"
        for at in range(1, 301)
    )
    desktop.as_account(f"mkdir -p {jobs}")
    desktop.as_account(f"setsid bash -c {shlex.quote(burst)} > /dev/null 2>&1 < /dev/null &")
    took: list[tuple[int, int]] = []

    # The session's, whose Do Not Disturb it is, not another source a key lands beside.
    def timed(rest: str) -> bool:
        found = re.fullmatch(
            r"session after (\d+) ms, refreshed (-?\d+) ms after the key's work", rest
        )
        if found:
            took.append((int(found[1]), int(found[2])))
        return found is not None

    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.machine.send_chars("desktop ")
        desktop.launcher_until(
            lambda shown: shown["held"] == ["desktop"] and "Do Not Disturb" in shown["rows"]
        )
        desktop.down_to("Do Not Disturb")
        desktop.normal()
        # Four presses, on and off twice, each waited for before the next, so each is a
        # fresh press, and Do Not Disturb is left as it was. The refresh is said once
        # its frame is drawn, so the row says what changed by then. Only a change its key
        # carried at once is said as an act's (SHOWN_AT_ONCE).
        for named in ["Let Notices Show", "Do Not Disturb"] * 2:
            since = desktop.mark()
            desktop.machine.send_key("ret")
            desktop.reported("act shown in ", timed, since)
            selected = desktop.showing()["selected"]
            assert selected["name"] == named, (named, selected)
    finally:
        desktop.as_account(f"kill $(cat {jobs}/pid) || true")
        desktop.close_launcher()
        desktop.as_account(f"rm -f {jobs}/queue.json {jobs}/pid")
    # The times are the VM's, drawn without a graphics card on a shared builder, so they're
    # said, and the refresh is held by its being said at all.
    print(f"each act's frame after its key, and its refresh after the key's work, in ms: {took}")


def test_dd_on_a_notice_shows_on_the_frame_after_its_key(desktop: Desktop) -> None:
    """Dd on an unread notice in For You's Desktop shows it read on the frame after the key.

    Joel, 2026-11-07: "doing a dd on ntofications does not update the screen wright away".
    d marked it read in the shell at once, but the notices never said they'd changed, so the
    row went on saying unread until something else made the list again.
    """
    took: list[tuple[int, int]] = []

    def timed(rest: str) -> bool:
        found = re.fullmatch(
            r"(?:notices|on) after (\d+) ms, refreshed (-?\d+) ms after the key's work", rest
        )
        if found:
            took.append((int(found[1]), int(found[2])))
        return found is not None

    def lunch(shown: Json) -> bool:
        return shown["held"][-1:] == ["Desktop"] and LUNCH in shown["rows"]

    with desktop.told("shows card Peter"):
        desktop.as_account("notify-send --expire-time=60000 'Peter' 'lunch at noon?'")
    try:
        # Where Super+N goes with no cards, as the margin's Show Them does.
        desktop.as_account("qs ipc call launcher collection notices")
        desktop.launcher_until(lunch)
        # Its card steps aside while Desktop lists it, so it's in one place.
        desktop.machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=dt.timedelta(seconds=10),
        )
        desktop.down_to(LUNCH)
        desktop.normal()
        # Desktop holds only the unread, which its head says once, so a
        # row says its age alone until it's read.
        side = cast("str", desktop.showing()["selected"]["side"])
        assert not side.startswith(("unread", "read")), side
        since = desktop.mark()
        desktop.machine.send_chars("dd")
        # Said only as its key carried it at once (SHOWN_AT_ONCE).
        desktop.reported("act shown in ", timed, since)
        selected = desktop.showing()["selected"]
        assert selected["name"] == LUNCH, selected
        assert cast("str", selected["side"]).startswith("read \u00b7 "), selected
    finally:
        desktop.close_launcher()
    print(f"dd's frame after its key, and its refresh after the key's work, in ms: {took}")


# Peter's notice, as its row in Notices names it: by its title alone (VKCF3Z, 2026-11-08).
LUNCH = "Peter"

# SHOWN_AT_ONCE: the shell says "act shown in" only for a change its key's own work made,
# which it refreshes at once rather than on the quarter second it paces other changes by
# (Launcher.qml's refreshed). One left to the pace is never said, so the wait for it fails.
# Held to a frame at 60 Hz as well, it failed on the builder alone: at 2101c92b6 four
# presses' refreshes came 9, 6, 7 and 22 ms after the keys' work (2026-11-09).

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
    # Opened on Now, a home row, the foot teaches its chord beside Return and draws nothing
    # else (Y2YBTS, 2026-11-06: a home row hands its chord to the foot).
    taught = opened["hints"][1:]
    assert opened["hints"][0].startswith("\u21b5"), opened["hints"]
    assert all(hint.endswith("next time") for hint in taught), opened["hints"]
    machine.send_chars("s")
    desktop.launcher_until(lambda shown: any("complete" in hint for hint in shown["hints"]))
    shown = desktop.showing()
    foot = laid_out(desktop)["foot"]
    assert shown["keysFrom"] == opened["keysFrom"], (opened["keysFrom"], shown["keysFrom"])
    # The room the opening held is just the widest the find has drawn: what the opening's
    # keys took, or what the letter brought, and no more, but for Return held as wide as the
    # widest verb the words name, where its place draws only the rows' own (VKCF3Z,
    # 2026-11-07: the slack before Return's dot is only what the rows' verbs need).
    drawn = max(
        opened["keysRoom"]["said"] + opened["keysRoom"]["returnSlack"],
        shown["keysRoom"]["said"] + shown["keysRoom"]["returnSlack"],
    )
    assert abs(opened["keysRoom"]["held"] - drawn) <= 1, (
        f"the opening held {opened['keysRoom']['held']} px for the keys, which take"
        f" {opened['keysRoom']['said']} ({opened['hints']}) and then"
        f" {shown['keysRoom']['said']} ({shown['hints']})"
    )
    # And that's how they're drawn: the first key starts as far in from the status line's end
    # as they take, in the card's frame, where the foot's words are laid out.
    # The first is Tab in on a row holding parts, or else the completion's Ctrl+Y.
    first = next(word for word in foot if word["text"].startswith(("Tab", "Ctrl")))
    reach = shown["footEnd"] - first["starts"]
    assert abs(reach - shown["keysRoom"]["said"]) <= 1, (
        f"the keys take {shown['keysRoom']['said']} px and are drawn over {reach}"
    )
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["typed"] == "")


def shortens_the_facts_only_within_a_find(desktop: Desktop) -> None:
    """See the find's facts only shorten as kitty is typed, and read whole again in a new mode.

    kitty's row brings Ctrl+Y's completion and Super+Return's next time beside Tab in, more
    than the room held from the opening, so the glance would give way; where even its least
    leaves too little, keys give way rather than run over it, by need: the completion, whose
    ghost the field shows, goes first, and Tab in stays on every letter, as Joel asked of
    a program's name (VKCF3Z, 2026-11-06: at "k", "Tab in \u00b7 Super+\u21b5 next time
    \u00b7 \u21b5 start"). Once kitty is whole and its
    completion goes, the room it took stays bare rather than the glance reading longer again,
    since the foot jumping with each letter was Joel's sharpest complaint about it; that room
    is the record of a row that needed it, and it ends with the find (VKCF3Z, 2026-11-06).
    """
    machine = desktop.machine
    # The time aside, since a minute may turn.
    opened = desktop.showing()["glance"].split("  ")[1:]
    said = opened
    wanted = 0
    for typed in ("kitty"[:end] for end in range(1, len("kitty") + 1)):
        machine.send_chars(typed[-1])

        def typed_so_far(shown: Json, want: str = typed) -> bool:
            return bool(shown["typed"] == want)

        desktop.launcher_until(typed_so_far)
        shown = desktop.showing()
        now = shown["glance"].split("  ")[1:]
        wanted = max(wanted, shown["keysRoom"]["told"])
        assert "Tab in" in shown["hints"], (typed, shown["hints"])
        assert len("  ".join(now)) <= len("  ".join(said)), (typed, said, now)
        said = now
    # It measures something: kitty's keys asked for more room than the opening held.
    held = desktop.showing()["keysRoom"]["held"]
    assert wanted > held, (wanted, held)
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    desktop.normal()
    machine.send_chars("/")
    desktop.launcher_until(
        lambda shown: shown["mode"] == "find" and shown["glance"].split("  ")[1:] == opened
    )


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
    shortens_the_facts_only_within_a_find(desktop)
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
    # A step's row with the most keys: Return, - and Tab in, and
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
    assert told[1:2] == ["Tab in"], shown["selected"]
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


# How often one Return composes the list as it steps into a place.
COMPOSED_MOST = 1


@dataclasses.dataclass(frozen=True)
class Step:
    """What stepping into a place did: the list composed and rows drawn afresh, and its reach."""

    composed: int
    drawn: int
    # The rows in reach of the view, those shown and those the list builds
    # ahead of it, and every row the place holds.
    reach: int
    listed: int

    def within(self) -> bool:
        """Say whether it composed the list once and drew no rows out of reach."""
        return self.composed <= COMPOSED_MOST and self.drawn <= self.reach


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

    def stepped() -> Step:
        """Step into Programs from home, saying what it composed and drew, and its timing."""
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.down_to("Programs")
        since = desktop.mark()
        was = laid_out(desktop)
        before = worked()
        machine.send_key("ret")
        # Its programs drawn, not only the step taken: a step that threw
        # once left Programs blank until something else redrew it
        # (2026-10-28).
        desktop.launcher_until(
            lambda shown: shown["held"] == ["programs"] and len(shown["rows"]) > 0
        )
        cost = worked() - before
        now = laid_out(desktop)
        # Qt's ListView builds rows a cache's height past the view as well as
        # those shown, a row each way here (Launcher.qml's cacheBuffer). A
        # step into a fresh place builds every row in reach, so it draws
        # exactly this many and the bound is met with nothing to spare; a row
        # more is one built out of reach, which is what it's there to catch.
        reach = math.ceil((now["inside"]["height"] + now["ahead"]) / now["rowTall"])
        # The launcher times a step to its frame as it does a key, and says
        # it once it's felt (Trials.js).
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)} | grep -a -F '\u2192 Programs'; true"
        ).strip()
        step = Step(
            composed=int(now["made"]) - int(was["made"]),
            drawn=int(now["drawn"]) - int(was["drawn"]),
            reach=reach,
            listed=int(now["listed"]),
        )
        print(
            f"stepping into Programs: {said or 'within a frame'}, composed {step.composed}"
            f" times, {step.drawn} of its {step.listed} rows drawn afresh where {reach} are in"
            f" reach, {cost:.0f} ms worked"
        )
        desktop.close_launcher()
        return step

    # What's held is what the step does, which a loaded builder can't
    # stretch: the list composed once for the place stepped into, and no
    # more rows drawn afresh than are in reach of its view. The shell's
    # processor time across it, which held it before, is said only: a
    # builder shared with other machines stretched it to 470 to 1220 ms
    # against a bound of 400, with the step no slower in kind, four times
    # by 2026-11-09. A step that threw once left Programs blank, and one
    # that composes again or draws every row it holds fails here. A source
    # refreshing within the step composes the list once more on its own,
    # so the step is taken up to three times and passes if one keeps to it.
    tries = []
    for _ in range(3):
        tries.append(stepped())
        if tries[-1].within():
            break
    # Programs holds more rows than are in reach, so a step drawing every
    # one would fail.
    assert all(step.listed > step.reach for step in tries), tries
    assert any(step.within() for step in tries), f"stepping into Programs did more: {tries}"


# Joel, 2026-10-25: "text alignment and shit constantly shifting around".
# A word that moves without changing tells him nothing and costs him a
# look, so after every key each word on the card that reads as it did must
# be drawn where it was: in the field, in the foot, and in each row that
# is still the same thing saying the same. In the foot a hint's key and
# its word read as one, so the last hint's key, which stands left of its
# word to end it on the inset (VKCF3Z, 2026-11-05), moves only as its
# word changes. A key's word stands a mark after it (KeyHints), nearer
# than any two facts stand, and the dot before a hint stands a mark
# before its key, so the three read as one piece and the dot before
# Return moves with Return's word (VKCF3Z, 2026-11-06).
A_KEYS_WORD = 10
# The dot that joins two hints on a card's status line.
HINTS_DOT = "\u00b7"


def stays(was: Json, word: Json, slides: int) -> bool:
    """Say whether a word is where it was, or only slid along its line by slides.

    Rounded apart, Return's place and a word's start may differ by a pixel.
    """
    if (was["starts"], was["y"]) == (word["starts"], word["y"]):
        return True
    return (
        slides != 0 and word["y"] == was["y"] and abs(word["starts"] - was["starts"] - slides) <= 1
    )


def moved(before: Json, after: Json, slid: int = 0) -> list[str]:
    """Say each word drawn in both that reads the same and isn't where it was.

    The foot's words may go along their line by slid, as far as Return's place narrowed.
    """
    shifts = []

    def compare(
        where: str, old: list[Json], new: list[Json], *, paired: bool = False, slides: int = 0
    ) -> None:
        def reads(words: list[Json], at: int) -> str:
            word, later = words[at], words[at + 1] if at + 1 < len(words) else None
            beside = (
                paired
                and later is not None
                and later["y"] == word["y"]
                and 0 <= later["starts"] - word["starts"] - word["inked"] <= A_KEYS_WORD
            )
            if beside and word["text"] == HINTS_DOT:
                # A dot belongs to the hint after it, and reads as it does.
                return f"{word['text']}\0{reads(words, at + 1)}"
            return f"{word['text']}\0{later['text'] if beside and later else ''}"

        unmatched: dict[str, list[Json]] = {}
        for at, word in enumerate(old):
            unmatched.setdefault(reads(old, at), []).append(word)
        for at, word in enumerate(new):
            same = unmatched.get(reads(new, at))
            if not same:
                continue
            was = same.pop(0)
            if not stays(was, word, slides):
                shifts.append(
                    f"{where}: {word['text']!r} moved from {was['starts']},{was['y']}"
                    f" to {word['starts']},{word['y']}"
                )

    compare("the field", before["field"], after["field"])
    compare("the foot", before["foot"], after["foot"], paired=True, slides=slid)
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
    """Nothing on the launcher moves but what a key changed, as letters, erasures and arrows go.

    Letters and erasures may move the foot's keys as its rows change, since Return's place
    holds only the rows what's typed lists, and arrows move nothing (VKCF3Z, 2026-11-07).
    """
    machine = desktop.machine
    closed_from_normal(desktop)
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    was_place = desktop.showing()["keysRoom"]["place"]
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
        place = desktop.showing()["keysRoom"]["place"]
        after = laid_out(desktop)
        desktop.shoot(f"launcher-still-{step:02}-{key}")
        # What the foot read before and after, so a move says what took
        # its room as well as what gave it.
        read = [" ".join(word["text"] for word in laid["foot"]) for laid in (before, after)]
        shifts += [
            f"after {key!r} ({typed!r}) {shift}, the foot reading {read[0]!r} then {read[1]!r}"
            for shift in moved(before, after, 0 if arrowed else was_place - place)
        ]
        before = after
        was_place = place
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

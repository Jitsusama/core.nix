"""The desktop test's launcher holds meetings area.

Here the launcher finds, joins, answers, moves and cancels Google's meetings, and Meet starts
one.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# brought here.
from desktop_session import (
    LAUNCHER_OPEN,
    MISSION,
    STARTING,
    google,  # noqa: F401
    kept,
    now_rows,
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


@pytest.mark.usefixtures("google")
def test_a_meeting_about_to_start_is_found_by_its_name_and_return_joins_it(
    desktop: Desktop,
) -> None:
    """A meeting about to start is found by its name, says when it is, and Return joins it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Design Review starts five minutes from now, on Meet.
    desktop.answer("design review")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Design Review"
            and re.fullmatch(r"in [1-5] min", str(shown["selected"].get("side", "")))
        )
    )
    # Return joins its call: the Meet link opens in a window of its own,
    # and the launcher closes, since joining takes Joel out of it.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def first_in_recent(desktop: Desktop, name: str, side: str) -> None:
    """Go into the program held's Recent, and wait until its first row is the one named.

    That row is selected, saying at its side what `side` matches.
    """
    desktop.launcher_until(lambda shown: "Recent" in shown["rows"])
    desktop.down_to("Recent")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Recent"]
            and shown["rows"][:1] == [name]
            and (shown["selected"] or {}).get("name") == name
            and re.fullmatch(side, str(shown["selected"].get("side", ""))) is not None
        )
    )


@pytest.mark.usefixtures("google")
def test_calendar_typed_lists_its_next_meetings_under_it(desktop: Desktop) -> None:
    """Calendar typed lists its next meetings under it, Calendar first, so Return opens it.

    Calendar's Recent, Mail's and Meet's each hold what Joel had lately, newest first. What
    Calendar holds now, today's meetings, comes before its Recent, as open windows are for a
    program (VKCF3Z, c7d9e2bd).
    """
    machine = desktop.machine
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Retro was held a day before it started, so it's said by its day and the
    # time it started.
    day = r"(yesterday|[A-Z][a-z]+day) \d\d:\d\d"

    def under_calendar(shown: Json) -> bool:
        """Say whether Calendar is selected with today's meetings just under it, then Retro."""
        rows = cast("list[str]", shown["rows"])
        if "Calendar" not in rows or (shown["selected"] or {}).get("name") != "Calendar":
            return False
        at = rows.index("Calendar")
        # And Calendar counts today's, never naming a meeting listed just under it, before
        # what its Recent holds: Retro, once, though held twice.
        said = str(shown["selected"].get("note", ""))
        listed = rows[at + 1 : at + 5] == ["Design Review", "Standup", "Planning", "Retro"]
        return listed and re.fullmatch(r"\d+ today \u00b7 1 recent", said) is not None

    desktop.answer("calendar")
    desktop.launcher_until(under_calendar)
    desktop.shoot("launcher-calendar-typed")
    desktop.close_launcher()
    # In Programs, where nothing is listed under it, it names its next, the time and the name
    # apart, before its Recent's count.
    desktop.answer("programs")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "collection:applications"
    )
    desktop.step_in("ret")
    desktop.down_to("Calendar")
    desktop.launcher_until(
        lambda shown: (
            re.fullmatch(
                r"next in \d+ min \u00b7 Design Review \u00b7 1 recent",
                str(shown["selected"].get("note", "")),
            )
            is not None
        )
    )
    desktop.shoot("launcher-calendar-in-programs")
    # Its Recent holds Retro, saying when it was held; Return on a meeting over steps in to
    # it, as Show in Calendar does, since there's nothing to join.
    desktop.follow()
    first_in_recent(desktop, "Retro", day)
    # Being over, it notes nothing after its name, and its drawer says the time before.
    retro = desktop.showing()["selected"]
    assert not retro.get("note"), retro
    assert {"label": "Earlier", "value": "1 in the past 30 days"} in retro["drawer"]["facts"], retro
    desktop.shoot("launcher-calendar-recent")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"][-1:] == ["Retro"])
    desktop.close_launcher()
    mails_recent_opens(desktop, opened)
    meets_recent_joins(desktop, opened, day)
    shoot_recents_at_joels_scales(desktop, day)


def selected_named(name: str) -> Callable[[Json], bool]:
    """Give a wait for the row named being selected."""
    return lambda shown: (shown["selected"] or {}).get("name") == name


# When a mail the stand-in holds came, as its row's side says it (When.js's came): its time
# today, or the day alone, since the stand-in's were read and sent three and four hours before
# it started, which is yesterday from midnight to 04:00 on the VM's clock, UTC (RTOIMQ,
# 2026-11-09).
MAILED = r"(\d\d:\d\d|yesterday)"


def mails_recent_opens(desktop: Desktop, opened: str) -> None:
    """Mail's Recent holds what Joel read and what he sent, and Return opens one."""
    machine = desktop.machine
    # Mail's holds Trip Notes, which Joel read, saying who it's from and when; Return opens it.
    desktop.answer("mail")
    desktop.launcher_until(selected_named("Mail"))
    desktop.into_program()
    first_in_recent(desktop, "Trip Notes", rf"Grace Hopper \u00b7 {MAILED}")
    # What he sent names who it went to, never him (VKCF3Z, c7d9e2bd).
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Notes on the Plan"
            and re.fullmatch(
                rf"to Ada Lovelace, Grace Hopper and 1 more \u00b7 {MAILED}",
                str(shown["selected"].get("side")),
            )
            is not None
        )
    )
    desktop.shoot("launcher-mail-recent")
    desktop.down_to("Trip Notes")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://mail.google.com/mail/u/0/#all/trip' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    desktop.launcher_until(lambda shown: not shown["open"])


def meets_recent_joins(desktop: Desktop, opened: str, day: str) -> None:
    """Meet's Recent holds a call Joel was on, and Return twice joins it again."""
    machine = desktop.machine
    # Meet's holds Retro's call, saying who it was with and when; Return steps in to Join the
    # Meeting, so Return twice joins it again.
    desktop.answer("meet")
    desktop.launcher_until(selected_named("Meet"))
    desktop.into_program()
    first_in_recent(desktop, "Retro", rf"Ada Lovelace and Grace Hopper \u00b7 {day}")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Retro"]
            and (shown["selected"] or {}).get("name") == "Join the Meeting"
        )
    )
    machine.succeed(f"! grep -qxF https://meet.google.com/rtr-ospe-ctv {opened}")
    # Joining starts something, so Return writes :meet, and a fresh Return joins.
    desktop.line("meet")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF https://meet.google.com/rtr-ospe-ctv {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    desktop.launcher_until(lambda shown: not shown["open"])


def shoot_recents_at_joels_scales(desktop: Desktop, day: str) -> None:
    """Shoot Calendar's Recent and Mail's at 1.5 and 2, for the gallery.

    The launcher opens afresh at each, as an open one keeps the size it opened at. Their
    drawers are judged at 1, above, since this screen at 1.5 or 2 has no room beside the card.
    """
    if not desktop.values["shots"]:
        return
    for scale in ("1.5", "2"):
        desktop.as_account(f"niri msg output winit scale {scale}")
        for program, first, side in (
            ("Calendar", "Retro", day),
            ("Mail", "Trip Notes", rf"Grace Hopper \u00b7 {MAILED}"),
        ):
            desktop.answer(program.lower())
            desktop.launcher_until(selected_named(program))
            desktop.into_program()
            first_in_recent(desktop, first, side)
            desktop.shoot(f"launcher-{program.lower()}-recent-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


@pytest.mark.usefixtures("google")
def test_todays_meeting_is_said_in_now(desktop: Desktop) -> None:
    """Now lists today's next meeting as a row of its own, joining one about to start."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The next meeting is the stand-in's Design Review, five minutes on.
    desktop.answer("now")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:on")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["now"] and shown["selected"])
    # The meeting is the last of Now's own rows, after the time, the date,
    # the location and the weather, with no Today to step into.
    assert "Today" not in desktop.showing()["rows"], desktop.showing()["rows"]

    def after_the_moment(shown: Json) -> bool:
        """Whether Design Review follows the location and the weather, when there is one."""
        rows = cast("list[str]", shown["rows"])
        if "Design Review" not in rows:
            return False
        at = rows.index("Design Review")
        return at > 0 and rows[at - 1] in {"Location", "Weather"}

    desktop.launcher_until(after_the_moment)
    # It's the meeting coming too, which wants Joel, and it's that one
    # row, never a second beside it. Now opens on its first row, as every
    # place does, and the meeting is reached by moving down to it.
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "For You")
    desktop.down_to("Design Review")
    today = desktop.showing()
    assert today["rows"].count("Design Review") == 1, today["rows"]
    assert today["selected"]["verb"] == "join", today
    # Inside Now, its details slide out in the drawer beside the card, as
    # the meeting's one component gives them, each a name and a figure.
    # It draws them too, though the search before handed it the very same
    # details, which it let go of as it faded: it once came out empty.
    desktop.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("title") == "Design Review"
            and shown["drawer"]["out"] is True
            and shown["drawer"]["facts"][:1] == ["When"]
            and shown["drawer"]["drawn"] == "Design Review"
        )
    )
    desktop.shoot("launcher-meeting-drawer")
    # Return joins it, as the meeting's own row does everywhere.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # In Now, the drawer is out for the meeting, which has details, and
    # goes in once the selection rests on a row with none, as Time is: it's
    # the selected row's, never the place's (docs/design.md, the drawer).
    desktop.answer("now")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:on")
    desktop.step_in("ret")
    # Now opens on its first row, and the meeting is below the time.
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "For You")
    desktop.down_to("Design Review")
    rows = cast("list[str]", desktop.showing()["rows"])
    for name in reversed(rows[rows.index("Time") : rows.index("Design Review")]):
        desktop.previous_row()
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == name  # noqa: B023
        )
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Time"
            and (shown.get("drawer") or {}).get("out") is False
            and shown["drawer"]["drawn"] == ""
        )
    )
    # Back on the meeting, gone and let go of, the drawer draws the meeting
    # again, though it's given the very details it was given before.
    desktop.down_to("Design Review")
    desktop.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("out") is True
            and shown["drawer"]["drawn"] == "Design Review"
        )
    )
    desktop.close_launcher()


@pytest.mark.usefixtures("google")
def test_a_meeting_further_off_is_shown_by_return_and_joined_from_its_acts(
    desktop: Desktop,
) -> None:
    """Return shows a meeting further off rather than joining it, guests too; its acts join it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Standup starts half an hour from now, on Meet.
    desktop.answer("standup")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Standup"
            and re.fullmatch(r"in (29|30) min", str(shown["selected"].get("side", "")))
        )
    )
    # Its details are beside it in the drawer, as the meeting's own row
    # gives them wherever it shows.
    drawer = desktop.showing()["selected"].get("drawer") or {}
    facts = {fact["label"]: fact["value"] for fact in drawer.get("facts", [])}
    assert drawer.get("title") == "Standup", drawer
    assert facts.get("Where") == "Room 4", drawer
    assert facts.get("Repeats") == "Every weekday", drawer
    assert facts.get("Holds It") == "Ada Lovelace", drawer
    assert facts.get("Asked") == "Grace Hopper \u00b7 not going", drawer
    assert drawer.get("body") == "What we did yesterday,\nwhat we do today.", drawer
    # Found by a search typed at home, it's out too, since the drawer is
    # the selected row's wherever it's found (docs/design.md, "The drawer
    # is the card continued").
    desktop.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("out") is True and shown["drawer"]["title"] == "Standup"
        )
    )
    # Return shows it, a step in named for it, its day and times first,
    # and the launcher stays.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["path"] == ["Standup"]
            and (shown["selected"] or {}).get("side") == "when"
        )
    )
    machine.fail(f"test -s {opened}")
    # Where it is and who holds it come before who else is asked, each by
    # name with how they answered.
    seen = desktop.showing()["rows"]
    named = ["Room 4", "https://meet.google.com/abc-defg-hij", "Ada Lovelace", "Grace Hopper"]
    assert [row for row in seen if row in named] == named, seen
    assert seen[-2] == "What we did yesterday, what we do today.", seen
    desktop.down_to("Grace Hopper")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "not going")
    # Back out, its acts, by Ctrl+], lead with seeing it, Return's own,
    # and then joining, which goes into its call.
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: shown["path"] == [] and (shown["selected"] or {}).get("name") == "Standup"
    )
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Show in Calendar"
    )
    desktop.down_to("Join the Meeting")
    # Joining starts something, so Return writes :meet, and a fresh Return joins, asking
    # nothing, since leaving takes it back.
    desktop.line("meet")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/abc-defg-hij {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_an_invitation_is_answered_from_its_acts_and_the_launcher_stays(
    desktop: Desktop,
) -> None:
    """An invitation is answered from a meeting's acts, the one given noted, and it stays."""
    machine = desktop.machine
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Standup, answered yes, which it hears answered again.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    # Its acts: seeing it, Return's own, and joining first, then the three
    # answers, each by its value, the one Joel gave noted as his.
    desktop.launcher_until(lambda shown: "Not Going" in shown["rows"])
    sides = {thing["is"]: thing["reads"] for thing in desktop.showing()["things"]}
    assert sides["verb:calendar:yes"] == "Going \u00b7 your answer", sides
    assert sides["verb:calendar:maybe"] == "Maybe", sides
    desktop.down_to("Maybe")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Maybe" and shown["footer"] == "going"
        )
    )
    # Answering reaches the one who asked, so Return writes its line, which asks first.
    desktop.line_answered("answer maybe")
    # Its note moves at once, before Google answers.
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "your answer")
    line = json.dumps({"answer": {"id": "standup", "with": "maybe"}}, separators=(",", ":"))
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(line)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays where it was, its foot saying the answer Google took.
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and (shown["selected"] or {}).get("name") == "Maybe"
            and shown["footer"] == "maybe"
        )
    )
    desktop.close_launcher()


@pytest.mark.usefixtures("google")
def test_a_meeting_writes_to_its_guests_and_copies_its_link_last(desktop: Desktop) -> None:
    """A meeting writes to everyone else it names after joining, and copies its link last."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    # Writing to them comes straight after joining, and copying is last.
    desktop.launcher_until(lambda shown: (shown["rows"] or [""])[-1] == "Copy the Link")
    rows = desktop.showing()["rows"]
    assert rows[rows.index("Join the Meeting") + 1] == "Write to the Guests", rows
    desktop.down_to("Copy the Link")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "its call")
    machine.send_key("ret")
    call = "https://meet.google.com/abc-defg-hij"
    machine.wait_until_succeeds(
        desktop.in_session(f'[ "$(wl-paste --no-newline)" = "{call}" ]'),
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Write to the Guests writes to Ada, who holds it, and Grace, asked,
    # about the meeting by its title, in Mail's window.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Write to the Guests" in shown["rows"])
    desktop.down_to("Write to the Guests")
    machine.send_key("ret")
    writing = (
        "https://mail.google.com/mail/u/0/?view=cm&fs=1"
        "&to=ada%40example.com%2Cgrace%40example.com&su=Standup"
    )
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(writing)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meetings_attachments_open_from_inside_it(desktop: Desktop) -> None:
    """A meeting's attachments open from inside it, a document as its own row."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    # Attachments comes before Copy the Link, which is always last.
    desktop.launcher_until(lambda shown: (shown["rows"] or [""])[-1] == "Copy the Link")
    rows = desktop.showing()["rows"]
    assert rows[-2] == "Attachments", rows
    desktop.down_to("Attachments")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Quarterly Plan", "Agenda.pdf"])
    # The plan is the document from Recent: its side says only when he opened it, and its
    # drawer who owns it (VKCF3Z, 2026-11-08).
    plan = desktop.showing()["selected"]
    assert plan["is"] == "gdoc:plan", plan
    assert "Ada Lovelace" not in plan["side"], plan
    facts = (plan.get("drawer") or {}).get("facts") or []
    assert {"label": "Owner", "value": "Ada Lovelace"} in facts, plan
    desktop.down_to("Agenda.pdf")
    machine.send_key("ret")
    agenda = "https://drive.google.com/file/d/agenda/view"
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(agenda)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meetings_call_open_in_meet_is_in_now_named_for_it(desktop: Desktop) -> None:
    """A Meet call in a window of its own is in Now named for its meeting, and goes from it.

    It's the call itself, named for its meeting, saying first whether Joel can be heard and
    how long it's gone on; inside are the microphone, who's in it, then its acts (VKCF3Z,
    2026-11-08). Who's in it and the rest of its words are Meet.js's (tests/calendar.qml).
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # Standup's call, in the window Chrome would open its link in as an
    # app of its own, which kitty stands in for by taking its app id.
    call = "chrome-meet.google.com__abc-defg-hij-Default"
    desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
    machine.wait_until_succeeds(
        desktop.in_session(f'niri msg --json windows | grep -qF \'"app_id":"{call}"\''),
        timeout=STARTING,
    )
    # Another window takes the keys, so Return going to the call is seen.
    other = desktop.opened("Beside the call")
    now_rows(desktop)
    desktop.down_to("Standup")
    desktop.launcher_until(
        lambda shown: shown["selected"]["side"].startswith(("live \u00b7 ", "muted \u00b7 "))
    )
    desktop.step_in()
    inside = ["Microphone", "People", "Go To the Call", "Leave It", "Copy the Link"]
    desktop.launcher_until(lambda shown: shown["rows"] == inside)
    desktop.down_to("Go To the Call")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    machine.wait_until_succeeds(
        desktop.in_session(f'niri msg --json focused-window | grep -qF \'"app_id":"{call}"\''),
        timeout=dt.timedelta(seconds=10),
    )
    # Leave It ends the call, so Return only writes its line, and running
    # that asks first; the call stays until it's answered yes.
    desktop.close(other)
    now_rows(desktop)
    desktop.down_to("Standup")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == inside)
    desktop.down_to("Leave It")
    desktop.line("leave")
    with desktop.told("ask ready"):
        machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["question"] == "Leave Standup? (y/n)")
    machine.succeed(desktop.in_session(f'niri msg --json windows | grep -qF \'"app_id":"{call}"\''))
    with desktop.told("answered yes"):
        machine.send_chars("y")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Its window closed, the call is over, and Now no longer lists it.
    machine.wait_until_fails(
        desktop.in_session(f'niri msg --json windows | grep -qF \'"app_id":"{call}"\''),
        timeout=dt.timedelta(seconds=10),
    )
    # The meeting itself may still be in Now, as it's to come, but not as a call.
    if "Standup" in now_rows(desktop)["rows"]:
        desktop.down_to("Standup")
        side = desktop.showing()["selected"]["side"]
        assert not side.startswith(("live \u00b7 ", "muted \u00b7 ")), desktop.showing()
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_now_lists_what_waits_then_the_time_then_a_call_a_song_and_a_mode(
    desktop: Desktop,
) -> None:
    """Now lists what waits on Joel, then the time and the date, then a call, a song and a mode."""
    asked = f"{MISSION}/waiting/TESTQA.json"
    kept(desktop, asked, {"items": [{"id": "look", "title": "Look", "when": "now"}]})
    # Standup's call, in the window Chrome would open it in, as above.
    call = "chrome-meet.google.com__abc-defg-hij-Default"
    desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
    desktop.as_account("systemctl --user start keep-awake.service")
    desktop.as_account(
        "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
        f" {shlex.quote(desktop.values['track'])}"
    )
    # Each by what it is: what waits first, then the time and the date,
    # Now's own, which never move, then the groups that come and go.
    groups = [
        "on:waitingOnYou",
        "answer:time",
        "answer:today",
        "on:call",
        "on:playing",
        "session:awake",
    ]

    def listed() -> list[str]:
        """Open Now, and give which of those it lists, in its order."""
        shown = now_rows(desktop)
        desktop.close_launcher()
        return [thing["is"] for thing in shown["things"] if thing["is"] in groups]

    try:
        # Opened afresh each time, since rows that come while Now is
        # open go at its end until it's made again (Steady.js).
        retry(lambda _: listed() == groups, timeout=dt.timedelta(seconds=60))
    finally:
        desktop.as_account(f"systemctl --user stop player keep-awake.service; rm -f {asked}")
        desktop.close(desktop.window_titled("Meet"))


@pytest.mark.usefixtures("google")
def test_a_meeting_coming_wants_joel_and_is_said_at_its_reminder_and_as_it_starts(
    desktop: Desktop,
) -> None:
    """A meeting within the hour is in Now, and is said at its reminder and as it starts.

    Twenty minutes off it's in Now alone, counting down, unsaid; a minute off it's said, and
    as it starts the same card says so and is raised again, one card rather than two, and
    Return joins it. Its notice goes by itself, read, once its call is open.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    call = "chrome-meet.google.com__pqr-stuv-wxy-Default"
    try:
        # The stand-in's Design Review twenty minutes on: Now counts it
        # down, and the margin leaves its mark out until ten minutes off.
        desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_SOON=1200000")
        desktop.as_account("systemctl --user restart google.service")
        now_rows(desktop)
        desktop.launcher_until(lambda shown: "Design Review" in shown["rows"])
        desktop.down_to("Design Review")
        desktop.launcher_until(
            lambda shown: re.fullmatch(r"in (19|20) min", shown["selected"]["side"]) is not None
        )
        margin = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep '^holds margin' | tail -n 1"
        )
        assert "meeting" not in margin.split(), margin
        machine.fail(desktop.in_session("niri msg layers | grep -q notifications"))
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # A minute and a half on: said at its reminder, a minute before,
        # once the shell's minute passes it, and as it starts its card,
        # still open, says so in place and is raised again.
        desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_SOON=90000")
        with desktop.told("arrives Design Review", timeout=dt.timedelta(seconds=120)):
            desktop.as_account("systemctl --user restart google.service")
        with desktop.told("raises Design Review now", timeout=dt.timedelta(seconds=120)):
            pass
        # One card, so what's kept of it says the start, and nothing the
        # reminder.
        machine.wait_until_succeeds(
            desktop.in_session(
                'jq -e \'any(.notices[]; .summary == "Design Review now")'
                ' and all(.notices[]; tostring | contains("Design Review in 1 min") | not)\''
                f" {desktop.NOTICES_KEPT}"
            ),
            timeout=dt.timedelta(seconds=10),
        )
        now_rows(desktop)
        desktop.down_to("Design Review")
        desktop.launcher_until(
            lambda shown: shown["selected"]["side"] == "now" and shown["selected"]["verb"] == "join"
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # The call opens in a window of its own, as Chrome would open the
        # link, which kitty stands in for by taking its app id; the meeting
        # no longer wants Joel, so its notice goes, read, untouched.
        desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
        machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_succeeds(
            desktop.notice_kept('.summary == "Design Review now" and .read'),
            timeout=dt.timedelta(seconds=10),
        )
        desktop.close(desktop.window_titled("Meet"))
    finally:
        desktop.as_account("systemctl --user unset-environment GOOGLE_STAND_IN_SOON")


@pytest.mark.usefixtures("google")
def test_a_meeting_joel_holds_is_cancelled_once_he_is_sure(desktop: Desktop) -> None:
    """A meeting Joel holds is cancelled from its acts, a step saying who's told asking first."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Planning, two hours on, is Joel's own, with Grace asked.
    desktop.answer("planning")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Planning")
    desktop.follow()
    desktop.down_to("Cancel the Meeting")
    # Return asks first, saying Google tells the one guest, and cancels nothing yet.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Cancel It Now, Telling 1 Guest"
    )
    machine.fail(f"test -s {asked}")
    # Cancelling tells the guest, so Return writes its line, banged, since the step was the
    # asking; it cancels nothing until a fresh Return there runs it, asking no more.
    desktop.line("cancel!")
    machine.fail(f"test -s {asked}")
    machine.send_key("ret")
    line = json.dumps({"cancel": {"id": "planning"}}, separators=(",", ":"))
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(line)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays, and Google lets the meeting go, so it's found no more.
    assert desktop.showing()["open"]
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.answer("planning")
    desktop.launcher_until(
        lambda shown: (
            not any(thing["is"] == "calendar:meeting:planning" for thing in shown["things"])
        ),
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meeting_google_lets_joel_change_is_moved_to_a_typed_time_once_he_is_sure(
    desktop: Desktop,
) -> None:
    """A meeting Google lets Joel change is moved to a time he types, a step naming who's told."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Standup is Ada's, on her calendar, which Joel may change.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    desktop.down_to("Move the Meeting")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "15 Minutes Later" in shown["rows"])
    # A time typed there is offered first, named by its day and times.
    day = machine.succeed("date -d tomorrow '+%A %-d %B'").strip()
    when = f"{day} \u00b7 10:00 to 10:30"
    desktop.type_in("tomorrow 10:00", selecting=when)
    # Return asks first, naming who Google tells, and moves nothing yet.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == f"Move It to {when}, Telling Ada Lovelace and Grace Hopper"
        )
    )
    machine.fail(f"test -s {asked}")
    machine.send_key("ret")
    moved = '{"move":{"id":"standup","start":'
    machine.wait_until_succeeds(
        f"grep -qF {shlex.quote(moved)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays, as it does once a meeting is cancelled.
    assert desktop.showing()["open"]
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meetings_guests_and_a_mails_sender_step_through_to_the_person(
    desktop: Desktop,
) -> None:
    """A meeting's Guests and a mail's sender are People's rows, each stepping in to the person."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # Standup is Ada's, and Grace is asked: Guests holds both, before
    # Attachments, Ada first as she holds it.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Guests" in shown["rows"])
    rows = desktop.showing()["rows"]
    assert rows.index("Guests") + 1 == rows.index("Attachments"), rows
    desktop.down_to("Guests")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Ada Lovelace", "Grace Hopper"])
    assert desktop.showing()["things"][1]["is"] == "person:grace@example.com"
    desktop.down_to("Grace Hopper")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Write a Mail"])
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Lunch on Friday's sender is Ada, whose row steps in to her.
    desktop.answer("lunch on friday")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Lunch on Friday")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Ada Lovelace" in shown["rows"])
    desktop.down_to("Ada Lovelace")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:5] == ["Write a Mail", "Standup", "Unread", "Recent", "ada@example.com"]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def written_down(desktop: Desktop, folder: str, meta: Json, files: dict[str, str]) -> None:
    """Leave a call in the Calls folder as call-transcripts leaves one: its meta.json and files."""
    folder = f"~/Documents/Calls/{folder}"
    desktop.as_account(f"mkdir -p {folder}")
    for name, text in {"meta.json": json.dumps(meta), **files}.items():
        desktop.as_account(f"printf '%s\\n' {shlex.quote(text)} > {folder}/{name}")


def calls_written_down(desktop: Desktop) -> str:
    """Leave Standup's call, summed up, and one with Ada, and have call-transcripts read them.

    It reads the calls back from their folders as it starts, as it does after a restart, since
    no call can be made here. It gives the key of Ada's.
    """
    now = int(desktop.as_account("date +%s%3N"))
    hour, minute = 3_600_000, 60_000
    transcript = "# Transcript\n\n**Joel**, 0:00: Shall we ship it?\n\n**Others**, 0:03: Friday."
    standup = {
        "key": "call:standup",
        "title": "Standup",
        "started": now - 3 * hour,
        "ended": now - 3 * hour + 15 * minute,
        "event": "standup",
        "person": None,
    }
    ada = {
        "key": f"call:person:ada@example.com:{now - 2 * hour}",
        "title": "Ada Lovelace",
        "started": now - 2 * hour,
        "ended": now - 2 * hour + 32 * minute,
        "event": None,
        "person": "ada@example.com",
    }
    written_down(
        desktop,
        "standup",
        standup,
        {"transcript.md": transcript, "summary.md": "- Ship it on Friday"},
    )
    written_down(desktop, "ada", ada, {"transcript.md": transcript})
    desktop.as_account("systemctl --user restart call-transcripts.service")
    desktop.machine.wait_until_succeeds(
        "test -S /run/user/1000/daemons/calls.sock", timeout=dt.timedelta(seconds=30)
    )
    return str(ada["key"])


def transcripts_listed(desktop: Desktop) -> None:
    """See People's Transcripts list the calls, each its words, its drawer and its acts."""
    # People holds Transcripts, which lists them newest first, each
    # saying what it was and when, and how long it went on.
    desktop.answer("people")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "People")
    desktop.follow()
    desktop.type_in("transcripts", selecting="Transcripts")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Call With Ada Lovelace", "Standup"])
    for name, side, length in [
        ("Call With Ada Lovelace", "call", "32 min"),
        ("Standup", "meeting", "15 min"),
    ]:
        desktop.down_to(name)
        selected = desktop.showing()["selected"]
        assert selected["side"] == side, selected
        # Today is never said as a word: the call's time, else yesterday's.
        when = rf"(yesterday )?\d\d:\d\d \u00b7 {length}"
        assert re.fullmatch(when, selected["note"]), selected
    # Its drawer holds its summary, else how its transcript opens.
    drawer = desktop.showing()["selected"]["drawer"]
    assert drawer["body"] == "\u2022 Ship it on Friday", drawer
    # Inside, what can be done with it, Copy the Path last.
    desktop.follow()
    acts = ["Open the Transcript", "Open Its Folder", "Copy the Path"]
    desktop.launcher_until(lambda shown: shown["rows"] == acts)
    desktop.step_out()
    desktop.down_to("Call With Ada Lovelace")
    drawer = desktop.showing()["selected"]["drawer"]
    assert drawer["body"].startswith("<b>Joel</b>, 0:00: Shall we ship it?"), drawer
    assert drawer["facts"][-1] == {"label": "With", "value": "Ada Lovelace"}, drawer


def found_with_ada_and_in_standup(desktop: Desktop, ada: str) -> None:
    """See Ada's Recent hold the call with her, and Standup offer its transcript."""
    desktop.answer("ada lovelace")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "person:ada@example.com"
    )
    desktop.step_in("ret")
    desktop.down_to("Recent")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Call With Ada Lovelace" in shown["rows"])
    things = desktop.showing()["things"]
    assert any(thing["is"] == f"transcript:{ada}" for thing in things), things
    desktop.as_account("qs ipc call launcher toggle")
    desktop.answer("standup")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "calendar:meeting:standup"
    )
    desktop.follow()
    # Show in Calendar leads, as Return's own act; the transcript comes after the
    # ways to reach the meeting and before Joel's answers.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:5]
            == [
                "Show in Calendar",
                "Join the Meeting",
                "Write to the Guests",
                "Open the Transcript",
                "Going",
            ]
        )
    )


@pytest.mark.usefixtures("google")
def test_the_transcripts_list_each_call_written_down_and_return_opens_it_in_the_reader(
    desktop: Desktop,
) -> None:
    """Each call written down is a transcript, in Transcripts, a person's Recent and its meeting."""
    machine = desktop.machine
    try:
        ada = calls_written_down(desktop)
        transcripts_listed(desktop)
        # Return opens its transcript in the reader, leaving the launcher.
        machine.send_key("ret")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
        reading = desktop.in_session(
            'niri msg --json focused-window | grep -q \'"app_id":"reader"\''
        )
        machine.wait_until_succeeds(reading, timeout=dt.timedelta(seconds=60))
        # What it reads is Ada's call's transcript, as niri's window says by
        # the program it runs.
        window = json.loads(desktop.as_account("niri msg --json focused-window"))
        ran = machine.succeed(f"tr '\\0' ' ' < /proc/{window['pid']}/cmdline")
        assert "/Calls/ada/transcript.md" in ran, ran
        # ZZ closes the reader, as it closes Neovim; q does nothing there.
        machine.send_key("shift-z")
        machine.send_key("shift-z")
        machine.wait_until_fails(reading, timeout=dt.timedelta(seconds=10))
        found_with_ada_and_in_standup(desktop, ada)
    finally:
        # The next test opens the launcher as a toggle, so it's left closed.
        if desktop.showing()["open"]:
            desktop.close_launcher()
        desktop.as_account("rm -rf ~/Documents/Calls/standup ~/Documents/Calls/ada")
        desktop.as_account("systemctl --user restart call-transcripts.service")


@pytest.mark.usefixtures("google")
def test_meet_starts_a_meeting_or_joins_one_by_its_code(desktop: Desktop) -> None:
    """Meet's acts start a new meeting or join one by its code, however it's typed."""
    machine = desktop.machine
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("meet")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Meet")
    desktop.into_program()
    # New Meeting, its desktop entry's, then Join by Code, Meet's own.
    desktop.launcher_until(
        lambda shown: (
            {"New Meeting", "Join by Code"} <= set(shown["rows"])
            and shown["rows"].index("New Meeting") < shown["rows"].index("Join by Code")
        )
    )
    # A code with neither dashes nor lower case is still Meet's.
    desktop.type_in("join by code", selecting="Join by Code")
    desktop.step_in("ret")
    machine.send_chars("ABC DEFG HIJ")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://meet.google.com/abc-defg-hij' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    desktop.close_launcher()


@pytest.mark.usefixtures("google")
def test_meet_copies_a_new_meetings_link_last_without_opening_meet(desktop: Desktop) -> None:
    """Meet's last act has the program make a meeting and copies its link, opening nothing."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {opened} {asked}")
    desktop.answer("meet")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Meet")
    desktop.into_program()
    desktop.launcher_until(lambda shown: (shown["rows"] or [""])[-1] == "Copy a New Meeting's Link")
    desktop.down_to("Copy a New Meeting's Link")
    machine.send_key("ret")
    link = "https://meet.google.com/new-abcd-efg"
    machine.wait_until_succeeds(
        desktop.in_session(f'[ "$(wl-paste --no-newline)" = "{link}" ]'),
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    machine.succeed(f"grep -qxF '{{\"make\":{{}}}}' {asked}")
    machine.fail(f"grep -qF new-abcd-efg {opened}")


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

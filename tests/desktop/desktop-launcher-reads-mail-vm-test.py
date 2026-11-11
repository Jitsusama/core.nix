"""The desktop test's launcher reads mail area.

Here the launcher reads, answers and tidies mail, counts what in Slack wants Joel, and finds
people, docs and Search Slack.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
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

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# brought here.
from desktop_session import (
    LAUNCHER_OPEN,
    SHELL_STARTS,
    STARTING,
    google,  # noqa: F401
    now_rows,
    repeating,
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
def test_a_signed_out_account_wants_joel_to_sign_in_again(desktop: Desktop) -> None:
    """Google signed out wants Joel in Now, unsaid as a notice, and Return signs in in pi."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    desktop.as_account(f"rm -f {opened}")
    desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_ACCOUNT=signedOut")
    try:
        desktop.as_account("systemctl --user restart google.service")
        now_rows(desktop)
        desktop.launcher_until(lambda shown: "Google" in shown["rows"])
        assert "Design Review" not in desktop.showing()["rows"], desktop.showing()
        desktop.down_to("Google")
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "signed out" and shown["selected"]["verb"] == "sign in"
            )
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -qx /google-auth {opened}"),
            timeout=SHELL_STARTS,
        )
        desktop.as_account(f'grep -qx "$HOME " {opened}')
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        machine.fail(desktop.in_session("niri msg layers | grep -q notifications"))
        desktop.as_account(f"rm -f {opened}")
    finally:
        desktop.as_account("systemctl --user unset-environment GOOGLE_STAND_IN_ACCOUNT")


def for_you_rows(desktop: Desktop) -> None:
    """Open the launcher on Now and step into For You, whose Lately band holds Mail and Slack."""
    now_rows(desktop)
    desktop.down_to("For You")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["now", "For You"])


# When the stand-in's mail came or its documents were opened, minutes before it started, as
# a side says it: just now within a minute, on a fast builder, minutes ago within the hour,
# then its time, or yesterday past midnight.
CAME = r"(just now|\d+ min ago|\d\d:\d\d|yesterday)"
# A document's side says that it's when he opened it, its drawer's Changed being another time.
OPENED = rf"opened {CAME}"


@pytest.mark.usefixtures("google")
def test_the_mail_that_wants_joel_is_counted_in_now_and_return_lists_it_as_mail_does(
    desktop: Desktop,
) -> None:
    """The mail that wants Joel is counted in For You, and Return lists it as Mail does.

    Each piece is the same row as in Programs \u203a Mail, with its details beside it, and Show
    the Mail after them goes there.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    lunch = {
        "title": "Lunch on Friday",
        "facts": [
            {"label": "From", "value": "Ada Lovelace <ada@example.com>"},
            {"label": "To", "value": "Joel Gerber"},
        ],
        "body": "Shall we try the place on the corner?\n\nAda",
    }

    def lunch_in_full(shown: dict[str, Any]) -> bool:
        """Whether Lunch on Friday is selected, its row and details as Mail gives them."""
        selected = shown["selected"] or {}
        drawer = selected.get("drawer") or {}
        return (
            selected.get("is") == "mail:lunch"
            and re.fullmatch(rf"Ada Lovelace \u00b7 {CAME}", selected["side"]) is not None
            and drawer.get("title") == lunch["title"]
            and drawer.get("facts", [])[:2] == lunch["facts"]
            and drawer.get("facts", [{}] * 3)[2].get("label") == "Date"
            # The file it carries, by name, after when it came.
            and drawer.get("facts", [{}] * 4)[3:] == [{"label": "Attachment", "value": "menu.pdf"}]
            and drawer.get("body") == lunch["body"]
        )

    for_you_rows(desktop)
    desktop.launcher_until(lambda shown: "Mail" in shown["rows"])
    desktop.down_to("Mail")
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["side"] == "2 to read" and shown["selected"]["verb"] == "show"
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Mail"]
            and shown["rows"] == ["Lunch on Friday", "Quarterly Numbers", "Show the Mail"]
        )
    )
    desktop.launcher_until(lunch_in_full)
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Mail's own place lists the very same row.
    desktop.answer("lunch on friday")
    desktop.launcher_until(lunch_in_full)
    desktop.follow()
    # Its sender, Ada, after Read: People's row for her.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"]
            == [
                "Open in Mail",
                "Reply",
                "Archive",
                "Read",
                "Ada Lovelace",
                "Attachments",
                "Copy the Link",
            ]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Show the Mail, after the mail, goes to Mail in Programs.
    for_you_rows(desktop)
    desktop.down_to("Mail")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: "Show the Mail" in shown["rows"])
    desktop.down_to("Show the Mail")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Mail"]
            and "Lunch on Friday" in shown["rows"]
            and "Show the Mail" not in shown["rows"]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Every mail and meeting row, wherever the walk from home finds one, is
    # made by its one component, never drawn by the surface that lists it.
    said = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
    assert said["uncomposed"] == [], (
        "A row drawn by the surface listing it, though a thing is one row"
        " (docs/design.md, 'A row says its kind, and a thing is one row'):\n"
        + "\n".join(said["uncomposed"])
    )
    assert said["composed"] > 0, "the walk found no mail to ask of"
    # Nor does a mail's or a meeting's drawer only say its row again.
    assert repeating(said) == "", "A drawer only repeats its row:\n" + repeating(said)
    # Nor holds a mail's surface anything for a drawer it never gets.
    assert said["undrawn"] == [], "Held for a drawer, though not a thing:\n" + "\n".join(
        said["undrawn"]
    )


def slack_says(
    desktop: Desktop, state: str, conversations: list[Json], *, recent: list[Json] | None = None
) -> None:
    """Have the stand-in Slack daemon say this, as a round of asking Slack would."""
    slack_tells(desktop, {"state": state, "conversations": conversations, "recent": recent or []})


def slack_tells(desktop: Desktop, said: Json) -> None:
    """Have the stand-in Slack daemon say all of `said`, slack-says.json's whole.

    The stand-in reads it from slack-says.json, as it starts or once told by SIGHUP that it
    changed; a stand-in only just started is never sent one, as it may not hear it yet. With
    messages, it holds every message wanting Joel, as the daemon does once Slack gave them all.
    """
    told = json.dumps(said)
    desktop.as_account(
        f"printf '%s' {shlex.quote(told)} > $XDG_RUNTIME_DIR/slack-says.new"
        " && mv $XDG_RUNTIME_DIR/slack-says.new $XDG_RUNTIME_DIR/slack-says.json"
        " && if systemctl --user is-active --quiet slack-stand-in.service;"
        " then systemctl --user kill -s HUP slack-stand-in.service;"
        " else systemctl --user start slack-stand-in.service; fi"
    )


def slack_holds_each_message(desktop: Desktop, conversations: list[Json]) -> None:
    """With every message held, For You's Slack row lists each; one read in Slack leaves.

    Each is named by who said it, its first words after its name and its side when alone,
    newest first, then Threads, whose replies aren't held (VKCF3Z, 2026-11-08). One read in
    Slack leaves
    by itself and History says so, "read in Slack 14:02", with nothing marked here.
    """
    machine = desktop.machine
    ada = {"channel": "D1", "place": "Ada Lovelace", "who": "Ada Lovelace"}
    grace = {
        "channel": "C1",
        "ts": "1730000003.000100",
        "place": "#deploys",
        "who": "Grace Hopper",
        "first": "@joel ship it?",
        "minutes": 30,
    }
    held = [
        ada | {"ts": "1730000002.000100", "first": "or tea?", "minutes": 2},
        ada | {"ts": "1730000001.000100", "first": "lunch at noon?", "minutes": 3},
        grace,
    ]
    said: Json = {"state": "ok", "conversations": conversations, "recent": []}
    slack_tells(desktop, said | {"messages": held})
    for_you_rows(desktop)
    # Its side counts what it lists: Ada's two, Grace's and Threads' three
    # replies, never the conversations the messages are in.
    desktop.down_to("Slack")
    desktop.launcher_until(
        lambda shown: re.fullmatch(r"6\sto\sread", shown["selected"]["side"]) is not None
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Slack"]
            and shown["rows"]
            == ["Ada Lovelace", "Ada Lovelace", "Grace Hopper", "Threads", "Show Slack"]
            and shown["selected"]["count"] == "or tea?"
            and re.fullmatch(r"\d+\smin\sago", shown["selected"]["side"]) is not None
        )
    )
    desktop.shoot("launcher-for-you-slack-messages")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Open in Slack", "Copy the Link"])
    # The link copied is the message's own, where Slack shows it, and
    # copying closes the launcher, as everywhere.
    closed = desktop.in_session(LAUNCHER_OPEN)
    desktop.down_to("Copy the Link")
    machine.send_key("ret")
    link = "https://slack.example/archives/D1/p1730000002000100"
    machine.wait_until_succeeds(
        desktop.in_session(f'[ "$(wl-paste --no-newline)" = "{link}" ]'),
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Each message row is made by its one component, as a conversation's is.
    actless = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
    assert actless["uncomposed"] == [], "\n".join(actless["uncomposed"])
    # Read in Slack, Grace's message leaves For You by itself, for its History.
    done = {"how": "readInSlack", "minutes": 0}
    slack_tells(desktop, said | {"messages": [*held[:2], grace | {"done": done}]})
    for_you_rows(desktop)
    desktop.down_to("Slack")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Ada Lovelace", "Ada Lovelace", "Threads", "Show Slack"]
    )
    desktop.close_launcher()
    for_you_rows(desktop)
    desktop.down_to("History")
    desktop.step_in("ret")
    desktop.down_to("Grace Hopper")
    desktop.launcher_until(
        lambda shown: (
            re.fullmatch(r"read\sin\sSlack\s\d\d:\d\d", shown["selected"]["side"]) is not None
        )
    )
    desktop.close_launcher()


def slack_goes(desktop: Desktop) -> None:
    """Stop the stand-in Slack daemon, as slack-unreads stopping does, and forget what it said."""
    desktop.as_account(
        "systemctl --user stop slack-stand-in.service; rm -f $XDG_RUNTIME_DIR/slack-says.json"
    )


def slack_is_not_in_now(desktop: Desktop) -> None:
    """Open For You, wait until it holds no Slack row, and close the launcher."""
    for_you_rows(desktop)
    desktop.launcher_until(lambda shown: "Slack" not in shown["rows"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_until(lambda shown: not shown["open"])


def answering(note: str) -> bool:
    """Say whether Slack's program row's note says no trouble, only what it holds, if any."""
    return re.fullmatch(r"(\d+ recent)?", note) is not None


def slack_program_says(
    desktop: Desktop, noted: Callable[[str], bool], *, keep_open: bool = False
) -> None:
    """Find Slack's program row, and wait until its note is as `noted` holds."""
    desktop.answer("slack")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("is") == "program:slack"
            and noted(shown["selected"]["note"])
        )
    )
    if not keep_open:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_until(lambda shown: not shown["open"])


def test_what_in_slack_wants_joel_is_counted_in_now_and_never_an_old_count(
    desktop: Desktop,
) -> None:
    """What in Slack wants Joel is counted in For You, and never an old count.

    Return lists each conversation as Slack's one conversation row, then Show Slack. When
    Slack can't be reached or refused the sign-in, For You has no Slack row, since neither
    wants Joel, and Slack's program row says why, its first act signing in again when signed
    out; with Slack's daemon gone, For You has no Slack row and never a count of nothing.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    conversations = [
        {
            "id": "D1",
            "name": "Ada Lovelace",
            "kind": "dm",
            "count": 2,
            "link": "slack://channel?team=T1&id=D1",
        },
        {
            "id": "C1",
            "name": "#deploys",
            "kind": "mention",
            "count": 1,
            "link": "slack://channel?team=T1&id=C1",
        },
        {
            "id": "threads",
            "name": "Threads",
            "kind": "thread",
            "count": 3,
            "link": "slack://open?team=T1",
        },
    ]
    slack_says(desktop, "ok", conversations)
    try:
        for_you_rows(desktop)
        desktop.launcher_until(lambda shown: "Slack" in shown["rows"])
        desktop.down_to("Slack")
        # Nothing follows its name, and its side is how many alone, as a
        # place's row is (VKCF3Z, 2026-11-09); the conversations wait inside.
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "6 to read"
                and shown["selected"]["count"] == ""
                and shown["selected"]["note"] == ""
            )
        )
        desktop.shoot("launcher-for-you-slack")
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"][-1:] == ["Slack"]
                and shown["rows"] == ["Ada Lovelace", "#deploys", "Threads", "Show Slack"]
                and shown["selected"]["side"] == "2 to read"
                and shown["selected"]["note"] == "a direct message"
            )
        )
        # Its acts are the conversation's, Open in Slack first; with no link
        # to share, there's nothing to copy.
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"] == ["Open in Slack"])
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # Every Slack conversation row is made by its one component.
        said = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
        assert said["uncomposed"] == [], (
            "A row drawn by the surface listing it, though a thing is one row:\n"
            + "\n".join(said["uncomposed"])
        )
        slack_holds_each_message(desktop, conversations)
        # Slack out of reach wants nothing of Joel, so For You has no Slack row;
        # Slack's program row says why.
        slack_says(desktop, "unreachable", [])
        slack_is_not_in_now(desktop)
        slack_program_says(desktop, lambda note: note == "Slack can't be reached")
        # Signed out, the program row says so, and its first act after
        # Return's own is signing in again.
        slack_says(desktop, "signedOut", [])
        slack_is_not_in_now(desktop)
        slack_program_says(desktop, lambda note: note == "signed out", keep_open=True)
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"][:2] == ["New Window", "Sign In"])
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # Slack answering again, the program row says nothing of it.
        slack_says(desktop, "ok", [])
        slack_program_says(desktop, answering)
        # What wants Joel again, and then the daemon gone, as when
        # slack-unreads stops: what it held goes with it.
        slack_says(desktop, "ok", conversations)
        for_you_rows(desktop)
        desktop.launcher_until(lambda shown: "Slack" in shown["rows"])
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    finally:
        slack_goes(desktop)
    for_you_rows(desktop)
    desktop.launcher_until(lambda shown: "Slack" not in shown["rows"])
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def slack_program(desktop: Desktop) -> None:
    """Find Slack's program row, step into it, and wait until it holds Unread and Recent."""
    slack_program_says(desktop, answering, keep_open=True)
    desktop.into_program()
    desktop.launcher_until(lambda shown: "Unread" in shown["rows"] and "Recent" in shown["rows"])


def test_slacks_program_holds_its_unread_then_its_recent_and_now_only_what_wants_joel(
    desktop: Desktop,
) -> None:
    """Slack's program holds Unread and then Recent, and For You only what wants Joel.

    Unread says how many to read and who, and lists every conversation unread, direct
    messages, mentions and Threads first, then the channels merely unread; For You's Slack row
    is its first three groups, so it neither counts nor lists a channel merely unread.
    Recent lists the conversations Joel read last, newest first.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)

    def conversation(id_: str, name: str, kind: str, count: int) -> Json:
        return {
            "id": id_,
            "name": name,
            "kind": kind,
            "count": count,
            "link": f"slack://channel?team=T1&id={id_}",
        }

    unread = [
        conversation("D1", "Ada Lovelace", "dm", 2),
        conversation("C1", "#deploys", "mention", 1),
        conversation("threads", "Threads", "thread", 3),
        conversation("C9", "#random", "unread", 0),
    ]
    recent = [
        conversation("C8", "#launcher", "channel", 0),
        conversation("D1", "Ada Lovelace", "dm", 0),
    ]
    slack_says(desktop, "ok", unread, recent=recent)
    try:
        # For You counts and lists only what wants Joel.
        for_you_rows(desktop)
        desktop.launcher_until(lambda shown: "Slack" in shown["rows"])
        desktop.down_to("Slack")
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "6 to read" and shown["selected"]["count"] == ""
            )
        )
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: shown["rows"] == ["Ada Lovelace", "#deploys", "Threads", "Show Slack"]
        )
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # Slack's program: Unread after its acts, and Recent after Unread.
        slack_program(desktop)
        rows = desktop.showing()["rows"]
        assert rows.index("Unread") < rows.index("Recent"), rows
        assert "Sign In" not in rows, rows
        desktop.down_to("Unread")
        # Nothing after its name, its names in its drawer as a place's are, how many bare
        # at its right, since its name says what's counted, and nothing in the foot.
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "6"
                and shown["selected"]["count"] == ""
                and shown["selected"]["note"] == ""
            )
        )
        desktop.step_in("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"][-1:] == ["Unread"]
                and shown["rows"] == ["Ada Lovelace", "#deploys", "Threads", "#random"]
            )
        )
        desktop.down_to("#random")
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "" and shown["selected"]["note"] == "a channel"
            )
        )
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # Recent, newest first.
        slack_program(desktop)
        desktop.down_to("Recent")
        desktop.step_in("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"][-1:] == ["Recent"] and shown["rows"] == ["#launcher", "Ada Lovelace"]
            )
        )
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # Every Slack conversation row, Unread's and Recent's too, is made by
        # its one component.
        said = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
        assert said["uncomposed"] == [], (
            "A row drawn by the surface listing it, though a thing is one row:\n"
            + "\n".join(said["uncomposed"])
        )
        # Nothing unread, Slack's program holds no Unread.
        slack_says(desktop, "ok", [], recent=recent)
        slack_program_says(desktop, answering, keep_open=True)
        desktop.follow()
        desktop.launcher_until(
            lambda shown: "Recent" in shown["rows"] and "Unread" not in shown["rows"]
        )
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    finally:
        slack_goes(desktop)


@pytest.mark.usefixtures("google")
def test_a_reply_to_mail_is_written_in_mail_to_who_sent_it(desktop: Desktop) -> None:
    """Reply writes a new message in Mail to who sent the mail, its subject marked a reply."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("lunch on friday")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "mail:lunch")
    desktop.follow()
    desktop.down_to("Reply")
    # Reply reaches someone, so Return writes :reply on the command line, and a fresh
    # Return there writes the reply, asking nothing, since nothing is sent yet.
    desktop.line("reply")
    machine.send_key("ret")
    link = (
        "https://mail.google.com/mail/u/0/?view=cm&fs=1"
        "&to=ada%40example.com&su=Re%3A%20Lunch%20on%20Friday"
    )
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(link)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_reply_to_all_is_written_to_who_sent_it_copied_to_the_others(desktop: Desktop) -> None:
    """Reply All, after Reply on mail that went to others too, copies them; Lunch went to Joel."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("quarterly numbers")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "mail:numbers")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: shown["rows"][:4] == ["Open in Mail", "Reply", "Reply All", "Archive"]
    )
    desktop.down_to("Reply All")
    desktop.line("reply all")
    machine.send_key("ret")
    link = (
        "https://mail.google.com/mail/u/0/?view=cm&fs=1"
        "&to=grace%40example.com&cc=ada%40example.com&su=Re%3A%20Quarterly%20Numbers"
    )
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(link)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_file_mail_carries_is_fetched_to_open_or_to_save_only_on_return(
    desktop: Desktop,
) -> None:
    """A file mail carries is fetched only on Return, to open, closing, or to save where picked."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    kept = "/run/user/1000/google-files/lunch/menu.pdf"
    saved = "/home/jitsusama/Downloads/menu.pdf"
    machine.succeed(f"rm -f {asked} {kept} {saved}")

    def into_the_menu() -> None:
        """Step into Lunch on Friday's Attachments, its menu selected."""
        desktop.answer("lunch on friday")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "mail:lunch")
        desktop.follow()
        desktop.launcher_until(lambda shown: "Attachments" in shown["rows"])
        desktop.down_to("Attachments")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"] == ["menu.pdf"])

    into_the_menu()
    selected = desktop.showing()["selected"]
    assert selected["side"] == "PDF \u00b7 4 B", selected
    assert selected["drawer"]["facts"] == [
        {"label": "Kind", "value": "PDF"},
        {"label": "Size", "value": "4 B"},
    ], selected
    # Listing a file fetches nothing; Return fetches it, opens it and closes.
    machine.fail(f"grep -qs fetch {asked}")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"test -f {kept}", timeout=dt.timedelta(seconds=30))
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    def save_a_copy() -> None:
        """Press Save a Copy on the menu, and wait for the picker, in Downloads, naming it."""
        into_the_menu()
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"] == ["Open", "Save a Copy"])
        desktop.down_to("Save a Copy")
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["path"] == ["Save a Copy", "~/Downloads"]
                and (shown["selected"] or {}).get("name") == "menu.pdf"
            ),
            timeout=dt.timedelta(seconds=30),
        )

    # Save a Copy asks where in the launcher's own picker, starting in
    # Downloads with the file's name; Escape, pressed until the launcher
    # closes, picks nothing and saves nothing.
    save_a_copy()
    desktop.close_launcher()
    machine.fail(f"grep -qs save {asked}")
    machine.fail(f"test -e {saved}")
    # Return saves it where the picker stands, and the launcher closes.
    save_a_copy()
    machine.send_key("ret")
    line = json.dumps(
        {"fetch": {"id": "lunch", "file": 0, "how": "save", "to": saved}},
        separators=(",", ":"),
    )
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(line)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_succeeds(f"test -f {saved}", timeout=dt.timedelta(seconds=30))
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_persons_page_still_being_found_stands_dimmed_in_its_place_and_lights_there(
    desktop: Desktop,
) -> None:
    """A person's page still being found stands in its place, dimmed, and lights there."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    pages = "/run/user/1000/pages"

    def say(said: str) -> None:
        # Moved into place whole, since a file written where it's watched
        # can be read empty between its truncating and its writing; and
        # heard before anything is looked for, so a row that never lights
        # isn't taken for a source that never heard.
        since = desktop.mark()
        desktop.as_account(f"echo {said} > {pages}.new && mv {pages}.new {pages}")
        desktop.reported("pages ", lambda rest: rest == said, since)

    # The stand-in Pages source keeps a page for Grace and is still finding
    # it (desktop/people-pages.qml).
    say("looking")
    desktop.answer("grace hopper")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "person:grace@example.com"
    )
    desktop.step_in("ret")
    # From the first rows inside her, Open in Pages stands after Write a
    # Mail, dimmed and saying it's being looked for, rather than arriving
    # at the end once found.
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Write a Mail"])
    shown = desktop.showing()
    assert shown["rows"][:2] == ["Write a Mail", "Open in Pages"], shown["rows"]
    standing = shown["things"][1]
    assert standing["gone"], standing
    assert "looking in Pages" in standing["reads"], standing
    say("found")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:2] == ["Write a Mail", "Open in Pages"]
            and not shown["things"][1]["gone"]
        )
    )
    desktop.as_account(f"rm -f {pages}")
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_search_ends_with_a_sources_last_row_which_lights_in_place_once_found(
    desktop: Desktop,
) -> None:
    """A search ends with what a source ends it with, after all it found, lit once found."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    ends = "/run/user/1000/search-ends"
    further = "Look Further for grace hopper"

    def say(said: str) -> None:
        # Moved into place whole, and heard before anything is looked for.
        since = desktop.mark()
        desktop.as_account(f"echo {said} > {ends}.new && mv {ends}.new {ends}")
        desktop.reported("search ends ", lambda rest: rest == said, since)

    # The stand-in source ends each search with Look Further, still asking
    # (desktop/search-ends.qml), as Search Slack does.
    say("asking")
    # Silenced again however it ends, so no later search ends with it.
    try:
        desktop.answer("grace hopper")
        # After everything found, each row named, and the list still opening
        # on what was found first.
        desktop.launcher_until(
            lambda shown: shown["rows"][-1:] == [further] and shown["count"] == len(shown["rows"])
        )
        shown = desktop.showing()
        assert len(shown["rows"]) > 1, shown["rows"]
        assert (shown["selected"] or {}).get("name") == shown["rows"][0], shown["selected"]
        desktop.down_to(further)
        desktop.follow()
        # Inside, a stand-in dimmed while it's asked, then what was found
        # once the source says so, with nothing typed again. The stand-in
        # stays dimmed where it stood until a key, as any row that leaves
        # does (Steady.js), so nothing moves under the selection.
        desktop.launcher_until(lambda shown: shown["rows"] == ["Still Looking"])
        standing = desktop.showing()["things"][0]
        assert standing["gone"], standing
        say("found")
        desktop.launcher_until(
            lambda shown: (
                shown["rows"] == ["Still Looking", "Found grace hopper"]
                and not shown["things"][1]["gone"]
            )
        )
    finally:
        say("none")
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


# How many people People may hold: those listed and those a search found, a
# search answering at most 50, never the crowd the daemon holds.
PEOPLE_HELD_MOST = 100


def test_a_crowd_the_people_daemon_holds_is_found_by_a_search_and_never_held_by_the_shell(
    desktop: Desktop,
) -> None:
    """A crowd the people daemon holds is found by a search, and never held by the shell.

    People held all of Tuple's 7,731 contacts and made 23,276 rows at each opening on Joel's
    machine, holding his first key a second (2026-10-31). The daemon holds the stand-in's
    8,000 here, so People holds only those listed and those a search asks the daemon for.
    """
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("crowd person 4321")
    desktop.launcher_until(lambda shown: "Crowd Person 4321" in shown["rows"])
    holds = desktop.as_account(
        "journalctl --user --unit=quickshell --output=cat"
        " | grep -a -o 'report\\[people holds [0-9]*' | tail -1"
    )
    held = re.search(r"(\d+)$", holds.strip())
    assert held is not None, "People never said how many it holds"
    assert int(held[1]) <= PEOPLE_HELD_MOST, f"People holds the crowd: {holds.strip()}"
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


def test_slack_is_asked_about_a_person_through_the_people_daemon_alone(
    desktop: Desktop,
) -> None:
    """Slack is asked about a person through the people daemon, with no reader of the shell's.

    The shell ran a slack-people of its own beside the daemon's, so Slack was asked twice
    about everyone People lists (2026-11-02). The daemon runs the stand-in in its place
    (desktop/people-slack.sh), which knows Crowd Person 1234 once it's signed in.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    signed_in = "/run/user/1000/slack-signed-in"
    asked = "/run/user/1000/slack-asked"

    def step_into(number: int) -> None:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in(f"crowd person {number}")
        # Picked by name, since what leads depends on what answered first:
        # Ask has led "crowd person 4323", with the person below it.
        name = f"Crowd Person {number}"
        desktop.launcher_until(lambda shown: name in shown["rows"])
        desktop.down_to(name)
        key = f"person:crowd{number}@crowd.test"
        assert desktop.showing()["selected"]["is"] == key, desktop.showing()["selected"]
        desktop.step_in("ret")

    def close() -> None:
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    def ways(number: int, held: Callable[[list[str]], bool]) -> None:
        # A step-in is what tells Slack's reader of a sign-in come or gone,
        # which the daemon says to the shell as Slack's reader says it.
        step_into(number)
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == [f"Crowd Person {number}"]
                and held([thing["reads"] for thing in shown["things"]])
            )
        )
        close()

    try:
        desktop.as_account(f"touch {signed_in}")
        # Someone Slack doesn't know, once it's signed in.
        ways(4321, lambda reads: "Write in Slack · no one at this address" in reads)
        step_into(1234)
        # Write in Slack leads the ways to reach them from the first rows,
        # dimmed until Slack answers and lit there.
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Write in Slack"])
        desktop.launcher_until(
            lambda shown: shown["rows"][:1] == ["Write in Slack"] and not shown["things"][0]["gone"]
        )
        # Asked of Slack's reader as a step-in, which only a client of the
        # daemon asking for them makes.
        lines = desktop.as_account(f"cat {asked}").split()
        assert "crowd1234@crowd.test" in lines, lines
        close()
    finally:
        # Signed out again, as before this test, for the tests after: a
        # step-in hears so, and the next has no Write in Slack at all.
        desktop.as_account(f"rm -f {signed_in}")
        ways(4322, lambda _: True)
        ways(4323, lambda reads: not any(read.startswith("Write in Slack") for read in reads))


@pytest.mark.usefixtures("google")
def test_a_person_is_found_by_name_and_holds_the_ways_to_reach_them_and_their_recent(
    desktop: Desktop,
) -> None:
    """A person is found by name; inside, the ways to reach them, then what's lately with them."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # Ada holds the stand-in's Standup and wrote Lunch on Friday, so Google
    # knows her by her address, and she's one person for both.
    desktop.answer("ada lovelace")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("is") == "person:ada@example.com"
            and (shown["selected"].get("drawer") or {}).get("facts", [{}])[0]
            == {"label": "Email", "value": "ada@example.com"}
        )
    )
    # Ctrl+] on her goes into her through People, where she lives, as on
    # any row holding parts, though People says itself at home.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["people", "Ada Lovelace"] and shown["rows"][:1] == ["Write a Mail"]
        )
    )
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "person:ada@example.com"
    )
    # Return steps in, never reaching her by itself: Write a Mail first,
    # then her next meeting drawn as a meeting, her mail waiting on Joel,
    # her Recent, and her address to copy last.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:5] == ["Write a Mail", "Standup", "Unread", "Recent", "ada@example.com"]
            and shown["things"][1]["is"] == "calendar:meeting:standup"
        )
    )
    machine.fail(f"test -s {opened}")
    desktop.down_to("Unread")
    desktop.follow()
    # The mail from her, its own component's row; unread, it's never
    # Recent, and a meeting to come isn't either.
    desktop.launcher_until(lambda shown: shown["rows"] == ["Lunch on Friday"])
    desktop.step_out()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Unread")
    desktop.previous_row()
    desktop.previous_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Write a Mail")
    machine.send_key("ret")
    writing = "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com"
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(writing)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_docs_and_drives_recent_list_what_joel_opened_lately_and_return_opens_it(
    desktop: Desktop,
) -> None:
    """Docs' and Drive's Recent list what Joel opened, Drive's Starred too, and Return opens it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Quarterly Plan is Docs' alone; Team Budget is Sheets'.
    # Sheets typed lists its own a depth in under it, and Docs' never.
    desktop.answer("sheets")

    def under_sheets(shown: Json) -> bool:
        """Say whether Sheets is selected, saying how many it keeps, Team Budget under it."""
        rows = cast("list[str]", shown["rows"])
        selected = shown["selected"] or {}
        if selected.get("name") != "Sheets":
            return False
        at = rows.index("Sheets")
        said = str(selected.get("note", ""))
        listed = rows[at + 1 : at + 4]
        return "Team Budget" in listed and "Quarterly Plan" not in listed and said == "1 recent"

    desktop.launcher_until(under_sheets)
    desktop.close_launcher()
    desktop.answer("docs")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Docs")
    desktop.into_program()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Quarterly Plan"])
    plan = desktop.showing()["selected"]
    assert plan["is"] == "gdoc:plan", plan
    # Its side says only when he opened it, and its drawer whose it is,
    # after when it changed.
    assert re.fullmatch(OPENED, plan["side"]), plan
    facts = (plan.get("drawer") or {}).get("facts") or [{}, {}]
    assert facts[0].get("label") == "Changed", plan
    assert facts[1] == {"label": "Owner", "value": "Ada Lovelace"}, plan
    # Return opens it in a window of its own, leaving the launcher.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://docs.google.com/document/d/plan/edit' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Its title finds the same row from anywhere, saying when it was opened.
    desktop.answer("team budget")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("is") == "gdoc:budget"
            and re.fullmatch(OPENED, str(shown["selected"].get("side", ""))) is not None
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    drive_holds_its_places(desktop, opened, closed)


def into_drive(desktop: Desktop) -> None:
    """Open Drive's program from the find, as Joel does.

    Typed, drive leads with the disk's room, drive being one of the disk's words, so the
    selection goes down to the program before Tab goes in.
    """
    desktop.answer("drive")
    desktop.launcher_until(lambda shown: "Drive" in shown["rows"])
    desktop.down_to("Drive")
    desktop.into_program()


def drive_holds_its_places(desktop: Desktop, opened: str, closed: str) -> None:
    """Check Drive's Recent and Starred, as Docs' Recent is checked, Return opening a folder."""
    machine = desktop.machine
    # Drive's Recent holds every kind, the menu he opened last first, each
    # the one row a Drive file is, so the documents are the rows Docs' are.
    into_drive(desktop)
    desktop.launcher_until(lambda shown: "Starred" in shown["rows"])
    desktop.shoot("launcher-drive-program")
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["menu.pdf", "Quarterly Plan", "Team Budget"]
    )
    desktop.shoot("launcher-drive-recent")
    menu = desktop.showing()["selected"]
    assert menu["is"] == "gdoc:menu", menu
    # Its drawer says where it is and its kind, then what changes most
    # first, its id last.
    assert re.fullmatch(OPENED, menu["side"]), menu
    drawer = menu.get("drawer") or {}
    assert drawer.get("eyebrow") == "in My Drive \u00b7 PDF", menu
    labels = [fact["label"] for fact in drawer.get("facts") or []]
    assert labels[:3] == ["Changed", "Changed By", "Owner"], menu
    assert labels[-2:] == ["Size", "ID"], menu
    desktop.close_launcher()
    # Starred says how many it holds, and steps in to them, a folder
    # opening in Drive's window.
    into_drive(desktop)
    desktop.type_in("starred", selecting="Starred")
    assert desktop.showing()["selected"]["side"] == "1"
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Trips"])
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://drive.google.com/drive/folders/trips' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_mail_that_wants_joel_is_found_by_its_subject_and_return_opens_it(
    desktop: Desktop,
) -> None:
    """Mail that wants Joel is found by its subject, says when it came, and Return opens it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Lunch on Friday came two minutes before it started.
    desktop.answer("lunch on friday")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Lunch on Friday"
            and re.fullmatch(rf"Ada Lovelace \u00b7 {CAME}", str(shown["selected"].get("side", "")))
        )
    )
    # Return opens it in Mail, in a window of its own, and the launcher
    # closes, since reading it takes Joel out of it.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://mail.google.com/mail/u/0/#inbox/lunch' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_mail_opened_while_mail_is_open_leaves_one_mail_window(desktop: Desktop) -> None:
    """Mail opened while Mail is open shows in a window beside it, and the old one closes."""
    machine = desktop.machine
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # Mail's own window, and the one Chrome opens a piece of mail in, each
    # named from its link, which kitty stands in for by taking its app id.
    inbox = "chrome-mail.google.com__-Default"
    message = "chrome-mail.google.com__mail_u_0_-Default"
    listed = 'niri msg --json windows | grep -qF \'"app_id":"{}"\''
    focused = 'niri msg --json focused-window | grep -qF \'"app_id":"{}"\''
    desktop.as_account(f"niri msg action spawn -- kitty --class {inbox} --title Gmail")
    machine.wait_until_succeeds(desktop.in_session(listed.format(inbox)), timeout=STARTING)
    # Another window takes the keys, so Mail's coming forward is seen.
    other = desktop.opened("Beside Mail")
    try:
        desktop.answer("lunch on friday")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Lunch on Friday"
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"grep -qxF 'https://mail.google.com/mail/u/0/#inbox/lunch' {opened}",
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_succeeds(
            desktop.in_session(focused.format(inbox)), timeout=dt.timedelta(seconds=10)
        )
        # Its window opens, as Chrome's would, and Mail's old one goes,
        # leaving one Mail window, at the mail.
        desktop.as_account(f"niri msg action spawn -- kitty --class {message} --title Lunch")
        machine.wait_until_fails(
            desktop.in_session(listed.format(inbox)), timeout=dt.timedelta(seconds=10)
        )
        machine.succeed(desktop.in_session(listed.format(message)))
    finally:
        desktop.close(other)
        for window in desktop.windows():
            if window["app_id"] in {inbox, message}:
                desktop.close(window)


def tidy_from_its_acts(desktop: Desktop, subject: str, act: str, line: str, became: str) -> None:
    """Find mail by its subject, do one of its acts by its line, and see it said, kept."""
    desktop.answer(subject.lower())
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == subject)
    desktop.follow()
    # Its acts: opening it, replying, archiving it and whether it's read.
    for _ in range(4):
        if (desktop.showing()["selected"] or {}).get("name") == act:
            break
        before = (desktop.showing()["selected"] or {}).get("name")
        desktop.next_row()
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") != before  # noqa: B023
        )
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == act)
    told = desktop.showing()["selected"]["told"]
    assert told[:1] == [f"\u21b5 :{line}"], told
    # It ends what wants Joel, so Return writes its line, and a fresh Return asks first,
    # in the act's own words, as "Archive Quarterly Numbers?", never the line's :delete.
    desktop.line_answered(line, f"{act} {subject}? (y/n)")
    # The launcher stays where it was, its foot saying what became of it
    # once the program no longer holds it as wanting Joel.
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and (shown["selected"] or {}).get("name") == act
            and shown["footer"] == became
        )
    )


@pytest.mark.usefixtures("google")
def test_mail_is_marked_read_or_archived_from_its_acts_and_the_launcher_stays(
    desktop: Desktop,
) -> None:
    """Mail is marked read, unread again or archived from its acts, said so, the launcher kept."""
    machine = desktop.machine
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")

    def was_asked(line: dict[str, object]) -> None:
        """See that the shell asked the program for `line`."""
        said = json.dumps(line, separators=(",", ":"))
        machine.wait_until_succeeds(
            f"grep -qxF {shlex.quote(said)} {asked}", timeout=dt.timedelta(seconds=10)
        )

    # Read is a switch: it says no, Return marks it read and it says yes.
    tidy_from_its_acts(desktop, "Lunch on Friday", "Read", "delete", "read")
    was_asked({"read": {"id": "lunch"}})
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "yes")
    # Return again, by its line, marks it unread, and it wants Joel once more.
    desktop.line_answered("delete")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Read"
            and shown["selected"].get("side") == "no"
            and shown["footer"] == "unread"
        )
    )
    was_asked({"unread": {"id": "lunch"}})
    desktop.close_launcher()
    tidy_from_its_acts(desktop, "Quarterly Numbers", "Archive", "archive", "archived")
    was_asked({"archive": {"id": "numbers"}})
    desktop.close_launcher()


def test_a_daemon_whose_socket_is_missing_is_asked_only_once_it_appears(desktop: Desktop) -> None:
    """A daemon whose socket isn't there is asked again only once it appears, not every 2 s."""
    machine = desktop.machine
    runtime = "/run/user/1000"
    made = f"{runtime}/daemons/slack.sock"
    hellos = f"{runtime}/slack-hellos"
    missing = "ServerNotFoundError"
    # Slack's daemon as it is before Joel signs in: no socket at all.
    since = desktop.as_account("date +%s").strip()
    desktop.as_account(f"systemctl --user stop slack-stand-in.service; rm -f {made} {hellos}")
    try:
        # A span to show the shell isn't trying: one every 2 s would be 6
        # for each of its two clients of Slack's daemon, Search Slack and
        # what's unread. The socket going may cost each a try or two, as a
        # dropped connection is tried again once and the file going is
        # heard.
        time.sleep(12)
        tries = desktop.as_account(
            f"journalctl --user -u quickshell --since @{since} --no-pager -o cat"
            f" | grep -c {missing}; true"
        ).strip()
        allowed = 4
        assert int(tries) <= allowed, f"the shell tried a missing socket {tries} times in 12 s"
        # Once it's made, the shell says hello, with no clock to wait on.
        desktop.as_account("systemctl --user start slack-stand-in.service")
        machine.wait_until_succeeds(f"test -s {hellos}", timeout=30)
    finally:
        desktop.as_account(f"systemctl --user stop slack-stand-in.service; rm -f {made} {hellos}")


@pytest.fixture
def slack(desktop: Desktop) -> Iterator[str]:
    """Start the stand-in Slack daemon for one test, and stop it after, giving its runtime folder.

    The launcher offers Search Slack only while Slack's daemon answers, so no other test's
    search ends with it. The shell looks for the daemon every 2 seconds, so this yields only
    once the shell has said hello to it.
    """
    runtime = "/run/user/1000"
    kept = f"{runtime}/slack-searched {runtime}/slack-answers {runtime}/slack-hellos"
    desktop.as_account(f"rm -f {kept}")
    desktop.as_account("systemctl --user start slack-stand-in.service")
    desktop.machine.wait_until_succeeds(f"test -s {runtime}/slack-hellos", timeout=30)
    yield runtime
    desktop.as_account("systemctl --user stop slack-stand-in.service")
    desktop.as_account(f"rm -f {kept}")


def test_search_slack_ends_a_search_and_asks_slack_once_on_return_lighting_in_place(
    desktop: Desktop, slack: str
) -> None:
    """Search Slack ends a search; Return asks Slack once, the first message lighting in place."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    searched = f"{slack}/slack-searched"
    search = "Search Slack for launcher"
    desktop.answer("launcher")
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == [search])
    # Nothing is asked of Slack as Joel types, since Slack allows a grid
    # only a few searches a minute.
    machine.fail(f"test -e {searched}")
    desktop.down_to(search)
    desktop.step_in("ret")
    # Return steps in at once, a dimmed Slack Message holding the first
    # message's place while Slack is asked, and asks it once.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Slack Message"]
            and shown["things"][0]["gone"]
            and (shown["selected"] or {}).get("side") == "searching Slack"
        )
    )
    assert machine.succeed(f"cat {searched}") == "launcher\n"
    desktop.as_account(f"touch {slack}/slack-answers")
    # The first lights where the stand-in stood, selected, and the rest
    # follow it, each a Slack message's one row, as For You's are: named by
    # who said it, by the same name, Joel's own as You, its first line after
    # its name, its side when, and where it is in its drawer's eyebrow.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Ada Lovelace", "You"]
            and not shown["things"][0]["gone"]
            and (shown["selected"] or {}).get("name") == "Ada Lovelace"
        )
    )
    selected = desktop.showing()["selected"]
    assert selected["count"] == "The launcher is fast", selected
    # An hour ago is said by its clock, or yesterday just past midnight.
    assert re.fullmatch(r"\d\d:\d\d|yesterday", selected["side"]) is not None, selected
    assert selected["drawer"]["eyebrow"] == "in #launcher \u00b7 message", selected
    assert selected["drawer"]["body"] == "The launcher is fast\nand never waits", selected
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("count") == "Launcher notes for Friday"
    )
    desktop.previous_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Ada Lovelace")
    # Inside, Open in Slack, whose drawer holds the messages around it once
    # asked, only Joel's named, then the link to copy last.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Open in Slack", "Copy the Link"]
            and ((shown["selected"] or {}).get("drawer") or {}).get("body")
            == "You: Is the launcher fast yet?\n\nThe launcher is fast"
        )
    )
    # Stepping out and in again asks Slack nothing more.
    desktop.step_out()
    desktop.step_out()
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == [search])
    desktop.down_to(search)
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Ada Lovelace"])
    assert machine.succeed(f"cat {searched}") == "launcher\n"
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_search_slack_says_when_slack_finds_nothing_or_is_too_busy_to_search(
    desktop: Desktop, slack: str
) -> None:
    """Search Slack says nothing was found in a sentence, and a busy Slack over Try Again."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    desktop.as_account(f"touch {slack}/slack-answers")
    desktop.answer("quokka")
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Search Slack for quokka"])
    desktop.down_to("Search Slack for quokka")
    desktop.step_in("ret")
    # Nothing found is a sentence where the first message would be, dimmed,
    # with no act under it.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Nothing in Slack says quokka."] and shown["things"][0]["gone"]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.answer("busy")
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Search Slack for busy"])
    desktop.down_to("Search Slack for busy")
    desktop.step_in("ret")
    # Too busy says so where the first message would be, over Try Again,
    # which asks Slack again and keeps the selection.
    desktop.launcher_until(
        lambda shown: shown["rows"][-2:] == ["Slack is busy; try again in a minute.", "Try Again"]
    )
    desktop.down_to("Try Again")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"test $(grep -cx busy {slack}/slack-searched) = 2", timeout=dt.timedelta(seconds=30)
    )
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Try Again")
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


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

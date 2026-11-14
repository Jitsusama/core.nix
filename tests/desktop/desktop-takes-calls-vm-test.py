"""The desktop test's takes calls area.

Here Tuple's calls are taken: its daemon starts first, a call shows in the margin, people
coming and going are said, a call that ended is in Tuple's Recent, a ringing call wants Joel,
and its windows find their places.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    FROM_THE_EDGE,
    KEY_PACE,
    MARGIN,
    NEARLY_ALL,
    NOTICES,
    STARTING,
    TUPLE_ASKED,
    drawn_at,
)

if TYPE_CHECKING:
    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


# A call comes before the other apps when the machine is busy.


def test_tuples_daemon_comes_before_the_other_apps(desktop: Desktop) -> None:
    """Tuple's daemon comes before the other apps."""
    comes = desktop.as_account("systemctl --user show tuple --property=CPUWeight --property=Nice")
    assert sorted(comes.split()) == ["CPUWeight=1000", "Nice=-10"]


def in_a_tuple_call(desktop: Desktop, ago: int) -> Json:
    """Give the call Joel's in as Tuple's stream says it, connected so many seconds since.

    Nobody else is in it yet; Tuple lists whoever comes as its participants.
    """
    now = int(desktop.as_account("date +%s%3N"))
    joel = {"is_current_user": True, "connection_state": "connected", "audio_enabled": True}
    return {
        "id": "a-call",
        "local": joel,
        "participants": [],
        "started_at": now - ago * 1000,
    }


# Someone Tuple gives no name, and who isn't one of its contacts.
A_GUEST = {
    "id": 5,
    "connection_state": "connected",
    "audio_enabled": True,
    "share_state": "audio",
    "webcam_enabled": False,
}


def test_a_tuple_call_shows_in_the_margin_while_it_lasts(desktop: Desktop) -> None:
    """A Tuple call shows in the margin while it lasts."""
    machine = desktop.machine
    desktop.tuple_follows()
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    with desktop.told("drawn margin"):
        desktop.tuple_streams(in_a_tuple_call(desktop, 0))
    desktop.shoot("margin-call")
    desktop.tuple_streams(None)
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_people_coming_and_going_in_a_tuple_call_are_said_and_joel_can_leave(
    desktop: Desktop,
) -> None:
    """People coming and going in a Tuple call are said, and Joel can leave."""
    machine = desktop.machine
    # The call before has tuple-call follow the stand-in too, so this one
    # does it itself and runs alone the same.
    desktop.tuple_follows()
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action={0}'"
    # Those there as Joel joins aren't news, so the call connected a
    # while before the guest comes.
    call = in_a_tuple_call(desktop, 10)
    with desktop.told("drawn margin"):
        desktop.tuple_streams(call)
    # The call is Tuple's, a program with no window of its own, whose
    # Return steps in to what can be done in it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("tuple", selecting="Tuple")
    assert desktop.showing()["selected"]["verb"] == "more", desktop.showing()["selected"]
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: "Leave the Call" in shown["rows"])
    desktop.close_launcher()
    with desktop.told("shows card Someone joined the call"):
        desktop.tuple_streams(call | {"participants": [A_GUEST]})
    desktop.shoot("notice-joined")
    with desktop.told("shows card Someone left the call"):
        desktop.tuple_streams(call)
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so the stack is held at once: held, it waits.
    with desktop.told("keys notices"):
        desktop.hold_notices()
    machine.succeed(offered.format("leave"))
    desktop.shoot("notice-alone")
    # Leaving asks tuple-call, which runs tuple call leave, and Tuple's
    # stream says the call's over. :e Notices opens the history on its
    # first row, the newest notice, whose offers are commands from normal
    # mode, as its card's are. Leaving can't be put back, so the line asks
    # first, and y says yes.
    desktop.notices_history()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("Someone left the call")
    )
    desktop.normal()
    machine.send_chars(":leave\n", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: shown["question"].startswith("Leave ") and shown["takesAnswer"]
    )
    machine.send_chars("y")
    # The margin is laid out again as the launcher closes, and goes from
    # niri's layers for a moment while the call's still on, so Tuple asked
    # to leave is what's waited on, and the call's mark going after it.
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -qx 'call leave' {TUPLE_ASKED}"),
        timeout=timedelta(seconds=15),
    )
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=15))
    machine.wait_until_fails(offered.format("leave"), timeout=timedelta(seconds=10))


def test_a_tuple_call_that_ended_is_in_tuples_recent_to_call_again(desktop: Desktop) -> None:
    """A Tuple call that ended is in Tuple's Recent, to call again."""
    machine = desktop.machine
    desktop.tuple_follows()
    # Named by Tuple's call alone, so her row is told apart from the
    # other calls this run's tests had, whose guests Tuple never names.
    ada = A_GUEST | {"id": 7, "full_name": "Ada Brightwater", "email": "ada@example.com"}
    call = in_a_tuple_call(desktop, 9 * 60) | {"participants": [ada]}
    # Ada rings and Joel answers, so tuple-call keeps the call as hers to
    # have rung, which its drawer says first. Her ring's notice going says
    # both were heard, and leaves the margin to the call. It's answered
    # once its card is up, as the shell withdraws a notice by the id it's
    # given only then.
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action=show'"
    with desktop.told("shows card Someone is calling"):
        desktop.tuple_said("received incoming call: 7")
    desktop.tuple_said("accepting call")
    machine.wait_until_fails(offered, timeout=timedelta(seconds=10))
    with desktop.told("drawn margin"):
        desktop.tuple_streams(call)
    desktop.tuple_streams(None)
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    kept = f"grep -q '\"since\":{call['started_at']}' ~/.local/state/tuple-call/recent.jsonl"
    machine.wait_until_succeeds(desktop.in_session(kept), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("tuple", selecting="Tuple")
    desktop.into_program()
    desktop.launcher_until(lambda shown: "Recent" in shown["rows"])
    desktop.down_to("Recent")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["held"][-1:] == ["Recent"] and "Ada Brightwater" in shown["rows"]
    )
    desktop.down_to("Ada Brightwater")
    # The drawer fades in once the selection rests, so the shot waits until
    # it's out and drawing her call, who rang first, over where it was,
    # Tuple named as its desktop entry names it.
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["shown"] is True
            and shown["drawer"]["drawn"] == "Ada Brightwater"
            and shown["drawer"]["eyebrow"] == "on Tuple \u00b7 call"
            and shown["drawer"]["drawnFacts"][:1] == ["Who Rang"]
        )
    )
    desktop.shoot("launcher-tuple-recent")
    row = desktop.showing()["selected"]
    assert row["note"].startswith("9 min \u00b7 "), row
    facts = row["drawer"]["facts"]
    assert facts[0] == {"label": "Who Rang", "value": "Ada Brightwater called"}, row
    # Said once: her name alone is With, and the note says how long and when.
    assert facts[1:] == [{"label": "Email", "value": "ada@example.com"}], row
    # A call rings a person, so Return steps in, calling again first inside.
    assert row["verb"] != "call", row
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Call Ada Brightwater Again"])
    assert desktop.showing()["selected"]["verb"] == "call"
    desktop.shoot("launcher-tuple-call-again")
    desktop.close_launcher()


def test_a_tuple_call_ringing_wants_joel_and_one_not_answered_is_kept_as_missed(
    desktop: Desktop,
) -> None:
    """A Tuple call ringing wants Joel, and one not answered is kept as missed."""
    machine = desktop.machine
    offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action=show'"
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    with desktop.told("shows card Someone is calling"):
        desktop.tuple_said("received incoming call: a caller")
    machine.succeed(offered)
    # The ring's card says it, so the margin's phone waits for it to go.
    desktop.shoot("notice-ringing")
    # The caller gives up: the ring's notice goes, and a missed call stays.
    desktop.tuple_said("Caller.cpp:985|invalidating call")
    machine.wait_until_fails(offered, timeout=timedelta(seconds=10))
    machine.wait_until_succeeds(
        desktop.notice_kept('.summary | contains("Missed a call from someone")'),
        timeout=timedelta(seconds=10),
    )
    # Once it goes by itself it waits among what came lately, and lights no
    # mark: a missed call is an ordinary notice, and For You's tray is the
    # one mark, lit only by what needs Joel.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_a_copied_calls_link_can_be_joined_until_something_else_is_copied(desktop: Desktop) -> None:
    """A copied call's link can be joined until something else is copied."""
    link = "https://tuple.app/c/from-a-chat"
    desktop.as_account(f"echo -n 'pair with me? {link}' | wl-copy > /dev/null 2>&1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("join", selecting="Join the call")
    desktop.close_launcher()

    # A screenshot is copied as an image alone, with no text to read.
    desktop.as_account("printf '\\211PNG\\r\\n' | wl-copy --type image/png > /dev/null 2>&1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("join")

    def joins_nothing(shown: Json) -> bool:
        """Say whether the launcher's selected row joins no Tuple call."""
        row = shown["selected"]
        return row is None or row["is"] != "tuple:join"

    desktop.launcher_until(joins_nothing)
    desktop.close_launcher()


def test_tuples_windows_find_their_places_the_face_in_a_corner_a_shared_screen_wide(
    desktop: Desktop,
) -> None:
    """Tuple's windows find their places: the face in a corner, a shared screen wide."""
    machine = desktop.machine

    # Through X11, as Tuple draws them, so niri starts xwayland-satellite
    # for them, and they reach it titled by their part. Tuple's daemon
    # is a service, started with no token from niri to be let take the
    # keys, which xwayland-satellite would hand on, so neither has one.
    def tuple_opens(part: str) -> Json:
        """Open a window as Tuple's part of this name does, and give it as niri lists it."""
        desktop.as_account(
            "niri msg action spawn -- env -u XDG_ACTIVATION_TOKEN"
            f" {desktop.values['tuple_window']} {part}"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f'niri msg --json windows | grep -q \'"title":"{part}"\''),
            timeout=STARTING,
        )
        return desktop.window_titled(part)

    span = json.loads(desktop.as_account("niri msg --json focused-output"))["logical"]
    # Tuple's own panel floats.
    tuple_panel = tuple_opens("tuple-main")
    assert tuple_panel["is_floating"], tuple_panel
    # Joel is at work in a window of his own when the other's face
    # comes, and keeps the keys. (xwayland-satellite hands a new X11
    # window the keys while another of its windows has them, which no
    # rule of niri's overrides.)
    desktop.as_account("niri msg action spawn -- kitty --title 'at work'")
    machine.wait_until_succeeds(
        desktop.in_session('niri msg --json focused-window | grep -q \'"title":"at work"\''),
        timeout=STARTING,
    )
    tuple_face = tuple_opens("tuple-webcams")
    assert tuple_face["app_id"] == "Tuple"
    assert tuple_face["is_floating"], tuple_face
    assert not tuple_face["is_focused"], tuple_face
    assert desktop.window_titled("at work")["is_focused"], tuple_face
    left, top, width, height = drawn_at(tuple_face)
    assert (width, height) == (384, 216)
    # In the bottom-right corner, a little in from the screen's edges
    # and whatever niri keeps clear along them.
    assert 0 < span["width"] - (left + width) <= FROM_THE_EDGE, tuple_face
    assert 0 < span["height"] - (top + height) <= FROM_THE_EDGE, tuple_face

    tuple_view = tuple_opens("tuple-screen")
    assert not tuple_view["is_floating"], tuple_view
    assert tuple_view["layout"]["window_size"][0] >= span["width"] * NEARLY_ALL, tuple_view
    desktop.shoot("tuple-windows")
    desktop.as_account("pkill -f '[t]uple-window'")
    desktop.as_account(
        f"niri msg action close-window --id {desktop.window_titled('at work')['id']}"
    )
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q 'tuple-\\|at work'"),
        timeout=timedelta(seconds=30),
    )


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

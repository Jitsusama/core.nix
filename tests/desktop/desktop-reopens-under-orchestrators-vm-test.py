"""The desktop test's reopens-under-orchestrators area.

Here sessions come back, or start, on their orchestrator's workspace by themselves: what
restore.json kept of two orchestrations goes back where the shell kept each window, two stacked
in one column included, orchestrators first, and none is begun again when its orchestrator
hands it over too; places kept on workspaces whose
numbers have since gone or been taken by Joel's own puts each orchestration alone on a
workspace of its own, moving nothing on screen; and a session one starts for another's tree
opens with that tree, wherever Joel moved it, in title order after its orchestrator. One pi's host
brought back comes back as its viewer alone, beside the windows kept with it, and ten the host
gave back come back as plain pis where their viewers sat this boot, while a shell started again
opens no viewer kept before it. Joel, after the
21:24 reboot on 2026-11-08 left 31 windows on the wrong workspaces: "i'm so confused, i don't
like my operating system confusing me." The area stays one file. Long because each test lays
out windows and kept places on the one desktop the area boots, with the helpers that read them
back shared between them: split, each part would boot a machine of its own to read the same
shell.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import shlex
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import ARRANGED, PI_PRESENCE, kept

if TYPE_CHECKING:
    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# How long every window of a list may take to come back where it sat, on a loaded builder.
BACK = 240
# Where a window sits: its workspace's number, its column and its tile in that column.
Sits = tuple[int, int, int]


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def sitting(desktop: Desktop, prefix: str) -> dict[str, Sits]:
    """Give each stand-in session's workspace, column and tile, by its session."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    idx = {one["id"]: one["idx"] for one in spaces}
    return {
        # "pi --session <id>", then what it was told, if anything.
        window["title"].split()[2]: (
            idx[window["workspace_id"]],
            *window["layout"]["pos_in_scrolling_layout"],
        )
        for window in desktop.windows()
        if (window["title"] or "").startswith(f"pi --session {prefix}")
        and window["layout"]["pos_in_scrolling_layout"]
    }


def handing(desktop: Desktop, sessions: list[dict[str, Any]]) -> None:
    """Hand the shell these, as `quest restore` and `quest spawn` do, without the focus."""
    handed = shlex.quote(json.dumps({"sessions": [{**one, "focus": False} for one in sessions]}))
    answer = desktop.as_account(f"qs ipc call sessions reopen {handed}").strip()
    assert answer == "taken", answer


def presents(desktop: Desktop, session: str, address: str, said: dict[str, Any]) -> None:
    """Say a reopened stand-in's record, as pi says its own once it starts.

    The stand-in wrote its process and kitty window by its session (tests/pi-stand-in.sh).
    """
    # Its kitty's socket and window when kitty says them, else none: a session is known to
    # run here by its process alone.
    pid, scope, window = [
        *desktop.as_account(f"cat $XDG_RUNTIME_DIR/pi-owner-{session}").split(),
        "",
        "",
    ][:3]
    ticks = desktop.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{pid}/stat")
    record = {
        "session": session,
        "addresses": [address],
        "title": f"pi --session {session}",
        "at": dt.datetime.now(dt.UTC).isoformat(),
        "owner": {
            "pid": int(pid),
            "bootId": desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
            "startTicks": int(ticks),
            "startedAt": int(desktop.as_account("date +%s%3N")),
        },
        "terminal": {"driverId": "kitty", "kind": "kitty-window", "value": window, "scope": scope},
        **said,
    }
    name = address.lower()
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(record))} > {PI_PRESENCE}/.{name}"
        f" && mv {PI_PRESENCE}/.{name} {PI_PRESENCE}/{name}.json"
    )


def kept_before(
    desktop: Desktop, places: dict[str, dict[str, Any]], hosted: frozenset[str] = frozenset()
) -> None:
    """Keep these sessions as the boot before left them, and start the shell anew to read them.

    restore.json says what that boot left, as the harness writes it, those `hosted` ran in pi's
    host, and the agents daemon's arrangement where each sat and whether it led others. The
    shell is stopped while they're written, so nothing it opens moves a window before they're
    read, then started, as after a boot.
    """
    desktop.machine.wait_until_succeeds(
        desktop.in_session("ls -d ~/.local/state/quickshell/by-shell/*"),
        timeout=dt.timedelta(seconds=60),
    )
    desktop.as_account("systemctl --user stop quickshell")
    kept(
        desktop,
        "~/.local/state/pi/restore.json",
        {
            "v": 1,
            "boot": "a-boot-before",
            "sessions": [
                {
                    "id": one,
                    "cwd": "/home/jitsusama/reef",
                    "title": one,
                    **({"hosted": True} if one in hosted else {}),
                }
                for one in places
            ],
        },
    )
    kept(
        desktop,
        ARRANGED,
        {"v": 1, "boots": {"a-boot-before": places}},
    )
    desktop.as_account("systemctl --user start quickshell")


def nothing_kept(desktop: Desktop) -> None:
    """Leave restore.json saying this boot lost nothing, so no later test has any to reopen."""
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})


def test_two_orchestrations_come_back_each_where_it_sat(desktop: Desktop) -> None:
    """What restore.json kept of two orchestrations goes back where the shell kept each window.

    After the 21:24 reboot two windows stacked in one column each went after the other's place
    25 times; and the sessions an orchestrator brought back itself went wherever niri put them,
    since a hand-over never read where they were kept.
    """
    output = json.loads(desktop.as_account("niri msg --json focused-output"))["name"]

    def at(workspace: int, column: int, tile: int = 1, *, leads: bool = False) -> dict[str, Any]:
        """Give a place as the shell keeps one, saying whether its session led others."""
        return {
            "output": output,
            "workspace": workspace,
            "column": column,
            "tile": tile,
            "leads": leads,
        }

    listed = {
        "01d1-lead-a": at(1, 1, leads=True),
        "01d1-a-one": at(1, 2, 1),
        "01d1-a-two": at(1, 2, 2),
        "01d1-lead-b": at(2, 1, leads=True),
        "01d1-b-one": at(2, 2),
        "01d1-b-two": at(2, 3),
    }
    addresses = {"01d1-lead-a": "LEADAA", "01d1-lead-b": "LEADBB"}
    wanted = {
        "01d1-lead-a": (1, 1, 1),
        "01d1-a-one": (1, 2, 1),
        "01d1-a-two": (1, 2, 2),
        "01d1-lead-b": (2, 1, 1),
        "01d1-b-one": (2, 2, 1),
        "01d1-b-two": (2, 3, 1),
    }
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/reef")
    reopen_all = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"
    try:
        kept_before(desktop, listed)
        desktop.machine.wait_until_succeeds(
            desktop.in_session(f"qs ipc call launcher list | grep -qF {shlex.quote(reopen_all)}"),
            timeout=dt.timedelta(seconds=120),
        )
        ran = desktop.as_account(f"qs ipc call launcher run {shlex.quote(reopen_all)}")
        assert '"error"' not in ran, ran
        # Each says its record once it's begun, as pi does, and LEADBB, once back, hands
        # over its own two, as an orchestrator restores its sessions before Reopen Them All
        # reaches them: at 21:24, 40 came back that way.
        # Which a hand-over joins, and where one handed over goes, are held in Qt's engine
        # (tests/places.qml: Places.queuedOnce, Places.keptFor): a VM can't time a hand-over into
        # Reopen Them All's queue. Here each says its record once it's begun, as pi does, and
        # LEADBB, once every window is back, hands its two over again, as an orchestrator
        # restoring its own does: neither is begun twice, since each runs here.
        # Each is said before the places are judged, so one whose window first shows on the
        # poll that finds every place right still says its record before the hand-over.
        said: set[str] = set()
        deadline = time.monotonic() + BACK
        while True:
            back = sitting(desktop, "01d1-")
            for one in set(back) - said:
                lead = addresses.get(one)
                address = lead or f"RO{len(said):04}"
                presents(desktop, one, address, {"orchestrates": lead} if lead else {})
                said.add(one)
            if back == wanted:
                break
            assert time.monotonic() < deadline, f"not each where it sat: {back}"
            time.sleep(0.2)
        since = desktop.mark()
        handing(
            desktop,
            [{"id": each, "cwd": "/home/jitsusama/reef"} for each in ("01d1-b-one", "01d1-b-two")],
        )
        for each in ("01d1-b-one", "01d1-b-two"):
            desktop.machine.wait_until_succeeds(
                desktop.in_session(
                    "journalctl --user --unit=quickshell --no-pager --output=cat"
                    f" --after-cursor={shlex.quote(since)}"
                    f" | grep -qF 'sessions: {each} not reopened, since it runs here'"
                ),
                timeout=dt.timedelta(seconds=60),
            )
        assert sitting(desktop, "01d1-") == wanted, sitting(desktop, "01d1-")
    finally:
        # Reopen Them All ran from the launcher, so one failing with it open fails alone.
        desktop.launcher_left_closed()
        nothing_kept(desktop)
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/pi-stays"
            f" $XDG_RUNTIME_DIR/pi-owner-01d1-* {PI_PRESENCE}/leadaa.json {PI_PRESENCE}/leadbb.json"
            f" {PI_PRESENCE}/ro0*.json; pkill -f 'sleep 60[0]'; true"
        )
        desktop.as_account("rm -rf ~/reef")


def test_a_list_kept_on_numbers_since_taken_comes_back_alone_moving_nothing(
    desktop: Desktop,
) -> None:
    """Each orchestration kept comes back alone on a workspace of its own, and nothing moves.

    At the 08:06 boot on 2026-11-09 the list said workspaces 2 and 4, and Joel's own windows
    and workspaces emptied since had taken those numbers: an orchestrator opened alone while
    another session landed beside Joel's, and 17 windows moved on screen chasing numbers. Joel:
    "we still have bugs with window placement, also had my screen go funky with things moving
    around at one point."
    """
    output = json.loads(desktop.as_account("niri msg --json focused-output"))["name"]
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/reef")
    # Joel's own two windows, on the workspaces the list will name by number.
    notes = desktop.opened("Notes", "sh -c 'sleep 600'")
    mail = desktop.opened("Mail", "sh -c 'sleep 600'")
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    notes_at = next(one["idx"] for one in spaces if one["id"] == notes["workspace_id"])
    desktop.as_account(
        f"niri msg action move-window-to-workspace --window-id {mail['id']}"
        f" --focus false {notes_at + 1}"
    )
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    desktop.machine.wait_until_succeeds(
        desktop.in_session(
            "niri msg --json windows | jq -e '[.[] | select(.id =="
            f" {mail['id']})][0].workspace_id != {notes['workspace_id']}'"
        ),
        timeout=dt.timedelta(seconds=30),
    )
    joels = {
        window["id"]: (window["workspace_id"], window["layout"]["pos_in_scrolling_layout"])
        for window in desktop.windows()
        if window["id"] in (notes["id"], mail["id"])
    }
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    empty = max(one["idx"] for one in spaces if one["output"] == output)

    def at(workspace: int, column: int, *, leads: bool = False) -> dict[str, Any]:
        """Give a place as the shell keeps one, saying whether its session led others."""
        return {
            "output": output,
            "workspace": workspace,
            "column": column,
            "tile": 1,
            "leads": leads,
        }

    # Kept on 2 and 4: 2 is Mail's now, and 3 is gone.
    listed = {
        "01d3-lead-a": at(2, 1, leads=True),
        "01d3-a-one": at(2, 2),
        "01d3-lead-b": at(4, 1, leads=True),
        "01d3-b-one": at(4, 2),
    }
    wanted = {
        "01d3-lead-a": (empty, 1, 1),
        "01d3-a-one": (empty, 2, 1),
        "01d3-lead-b": (empty + 1, 1, 1),
        "01d3-b-one": (empty + 1, 2, 1),
    }
    reopen_all = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"
    try:
        since = desktop.mark()
        kept_before(desktop, listed)
        desktop.machine.wait_until_succeeds(
            desktop.in_session(f"qs ipc call launcher list | grep -qF {shlex.quote(reopen_all)}"),
            timeout=dt.timedelta(seconds=120),
        )
        ran = desktop.as_account(f"qs ipc call launcher run {shlex.quote(reopen_all)}")
        assert '"error"' not in ran, ran
        desktop.machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(since)} | grep -qF 'sessions: reopening settled'"
            ),
            timeout=dt.timedelta(seconds=BACK),
        )
        # A window is placed once niri lays it out, and its stand-in titles it a moment after.
        deadline = time.monotonic() + 60
        while len(back := sitting(desktop, "01d3-")) < len(wanted):
            assert time.monotonic() < deadline, f"not every one titled: {back}"
            time.sleep(0.2)
        assert back == wanted, back
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        # Each opened where it goes, so none was moved on screen after.
        assert " moved from " not in said, said
        mine = {
            window["id"]: (window["workspace_id"], window["layout"]["pos_in_scrolling_layout"])
            for window in desktop.windows()
            if window["id"] in joels
        }
        assert mine == joels, (mine, joels)
    finally:
        desktop.launcher_left_closed()
        nothing_kept(desktop)
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/pi-stays"
            " $XDG_RUNTIME_DIR/pi-owner-01d3-*; pkill -f 'sleep 60[0]'; true"
        )
        desktop.as_account("rm -rf ~/reef")


def test_a_session_started_for_another_tree_opens_with_it_in_title_order(
    desktop: Desktop,
) -> None:
    """One orchestrator starts a session for another's tree: it opens with that tree, in order.

    At 21:30 on 2026-11-08 one started that way opened beside the focused column on its
    starter's workspace and moved every window there along.
    """
    machine = desktop.machine
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/kelp")
    notes = desktop.opened("Notes", "sh -c 'sleep 600'")
    leads = {"LEADAA": "Atoll Orchestrator", "LEADBB": "Bay Orchestrator"}
    try:
        for address, title in leads.items():
            desktop.pi_window(address, title)
            desktop.presence_says(
                {
                    "session": f"01d2-{address.lower()}",
                    "quest": f"QEST-20261108-{address}",
                    "addresses": [address],
                    "orchestrates": address,
                    "questInfo": {"id": f"QEST-20261108-{address}", "title": title},
                },
                address=address,
            )
        # Bay's orchestrator and the one session it has, on the second workspace.
        desktop.pi_window("BRAVQA", "Bravo Checks")
        desktop.presence_says(
            {
                "session": "01d2-bravqa",
                "quest": "QEST-20261108-BRAVQA",
                "addresses": ["BRAVQA"],
                "orchestratedBy": "LEADBB",
                "questInfo": {"id": "QEST-20261108-BRAVQA", "title": "Bravo Checks"},
            },
            address="BRAVQA",
        )
        for title in ("Bay Orchestrator", "Bravo Checks"):
            moving = desktop.window_titled(title)
            desktop.as_account(
                f"niri msg action move-window-to-workspace --window-id {moving['id']}"
                " --focus false 2"
            )
        desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
        lead = desktop.window_titled("Bay Orchestrator")
        since = desktop.mark()
        # Atoll's orchestrator starts it for Bay's tree, as a quest's spawn hands it over.
        handing(
            desktop,
            [
                {
                    "id": "01d2-alpha",
                    "cwd": "/home/jitsusama/kelp",
                    "quest": "QEST-20261108-ALPHQA",
                    "from": "01d2-leadaa",
                    "orchestratedBy": "LEADBB",
                    "title": "Alpha Checks",
                }
            ],
        )
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01d2-alpha'"),
            timeout=dt.timedelta(seconds=300),
        )
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        # Asked to open there, after Bay's orchestrator and before Bravo, and never moved.
        alpha = next(
            window
            for window in desktop.windows()
            if (window["title"] or "").startswith("pi --session 01d2-alpha")
        )
        lead_at = lead["layout"]["pos_in_scrolling_layout"][0]
        asked = f"asked on {sitting_space(desktop, lead)} column {lead_at + 1}"
        assert asked in said, said
        assert f"window {alpha['id']} moved from" not in said, said
        bravo = desktop.window_titled("Bravo Checks")
        assert alpha["workspace_id"] == lead["workspace_id"], (alpha, lead)
        alpha_at = alpha["layout"]["pos_in_scrolling_layout"][0]
        bravo_at = bravo["layout"]["pos_in_scrolling_layout"][0]
        assert (alpha_at, bravo_at) == (lead_at + 1, lead_at + 2), (alpha_at, bravo_at)
        # Joel's window keeps the keys.
        focused = json.loads(desktop.as_account("niri msg --json focused-window"))
        assert focused["id"] == notes["id"], focused
    finally:
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays {PI_PRESENCE}/leadaa.json"
            f" {PI_PRESENCE}/leadbb.json {PI_PRESENCE}/bravqa.json;"
            " pkill -f 'sleep 60[0]'; pkill -f 'sleep infinit[y]'; true"
        )
        desktop.as_account("rm -rf ~/kelp")


def test_a_hosted_session_comes_back_as_its_viewer_beside_its_kin(desktop: Desktop) -> None:
    """Reopen Them All opens a hosted session's viewer alone, beside the window kept with it.

    Its host brings the conversation back as the machine starts, so no pi is begun for it, and
    its viewer opens quietly in its column, taking no focus. After the 11:06 boot on
    2026-11-11 two hosted sessions got no window at all, since none was opened for them.
    """
    machine = desktop.machine
    session = "01d4-hosted"
    output = json.loads(desktop.as_account("niri msg --json focused-output"))["name"]
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/reef")
    notes = desktop.opened("Notes", "sh -c 'sleep 600'")
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    empty = max(one["idx"] for one in spaces if one["output"] == output)
    # The host, standing in for pi's own, and its presence for the conversation, naming the
    # words that view it, as the harness's does. setsid -f returns before its fork runs, so
    # the pid is looked for under a deadline.
    host = desktop.as_account(
        "setsid -f sleep 602 > /dev/null 2>&1;"
        " timeout 10 sh -c \"until pgrep -n -f 'sleep 60[2]'; do sleep 0.1; done\""
    ).strip()
    ticks = desktop.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{host}/stat")
    presence = {
        "session": session,
        "addresses": ["HOSTVW"],
        "title": f"pi --session {session}",
        "at": dt.datetime.now(dt.UTC).isoformat(),
        "owner": {
            "pid": int(host),
            "bootId": desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
            "startTicks": int(ticks),
            "startedAt": int(desktop.as_account("date +%s%3N")),
        },
        "hosted": {
            "host": "harbour-attended",
            "conversation": session,
            "view": ["sleep", "603"],
        },
    }
    reopen_all = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"
    try:
        kept(desktop, f"{PI_PRESENCE}/hostvw.json", presence)
        machine.wait_until_succeeds(
            desktop.in_session(f"mycelium --realm harbour sessions | grep -qF '{session}'"),
            timeout=dt.timedelta(seconds=60),
        )
        since = desktop.mark()
        kept_before(
            desktop,
            {
                "01d4-plain": {"output": output, "workspace": 1, "column": 1, "tile": 1},
                session: {"output": output, "workspace": 1, "column": 2, "tile": 1},
            },
            frozenset({session}),
        )
        machine.wait_until_succeeds(
            desktop.in_session(f"qs ipc call launcher list | grep -qF {shlex.quote(reopen_all)}"),
            timeout=dt.timedelta(seconds=120),
        )
        ran = desktop.as_account(f"qs ipc call launcher run {shlex.quote(reopen_all)}")
        assert '"error"' not in ran, ran
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(since)} | grep -qF 'sessions: reopening settled'"
            ),
            timeout=dt.timedelta(seconds=BACK),
        )
        # Its stand-in titles the plain one's window a moment after niri lays it out.
        deadline = time.monotonic() + 60
        while "01d4-plain" not in (back := sitting(desktop, "01d4-")):
            assert time.monotonic() < deadline, f"the plain one never titled: {back}"
            time.sleep(0.2)
        viewers = [one for one in desktop.windows() if one["app_id"] == f"pi-hosted.{session}"]
        assert len(viewers) == 1, f"not one viewer: {desktop.windows()}"
        idx = {
            one["id"]: one["idx"]
            for one in json.loads(desktop.as_account("niri msg --json workspaces"))
        }
        viewer = viewers[0]
        sits = (idx[viewer["workspace_id"]], *viewer["layout"]["pos_in_scrolling_layout"])
        assert (back["01d4-plain"], sits) == ((empty, 1, 1), (empty, 2, 1)), (back, sits)
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        assert f"reopening {session}'s viewer" in said, said
        assert " moved from " not in said, said
        begun = desktop.machine.execute(f"test -e /run/user/1000/pi-owner-{session}")[0]
        assert begun != 0, "a pi was begun for the hosted session"
        focused = json.loads(desktop.as_account("niri msg --json focused-window"))
        assert focused["id"] == notes["id"], focused
    finally:
        desktop.launcher_left_closed()
        nothing_kept(desktop)
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays $XDG_RUNTIME_DIR/pi-owner-01d4-*"
            f" {PI_PRESENCE}/hostvw.json; pkill -f 'sleep 60[0-3]'; true"
        )
        desktop.as_account("rm -rf ~/reef")


def viewers_sitting(desktop: Desktop, prefix: str) -> dict[str, Sits]:
    """Give each hosted viewer's workspace, column and tile, by its session."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    idx = {one["id"]: one["idx"] for one in spaces}
    return {
        window["app_id"].removeprefix("pi-hosted."): (
            idx[window["workspace_id"]],
            *window["layout"]["pos_in_scrolling_layout"],
        )
        for window in desktop.windows()
        if (window["app_id"] or "").startswith(f"pi-hosted.{prefix}")
        and window["layout"]["pos_in_scrolling_layout"]
    }


def settled_since(desktop: Desktop, since: str) -> None:
    """Wait for the shell to say the walk it began after `since` settled."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)} | grep -qF 'sessions: reopening settled'"
        ),
        timeout=dt.timedelta(seconds=BACK),
    )


# How many sessions the rollback drill gives back, and how many columns their viewers fill on
# each workspace (item 11's run sheet, attachments of QEST-20261109-5KBYD7).
GIVEN = 10
ACROSS = 5
REOPEN_ALL = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"


def test_a_shell_started_again_opens_no_viewer_kept_before_it(desktop: Desktop) -> None:
    """A shell started again, as a switch does, opens no viewer for a windowless hosted session.

    A place kept for its viewer by a move before the shell started stays kept, unopened. At
    13:41:38 on 2026-11-11 a switch restarted the shell, which opened viewers for two hosted
    sessions kept by earlier moves, on a new workspace, while Joel was at the machine, and woke
    one parked in its host.
    """
    machine = desktop.machine
    session = "01d5-kept"
    # A window of Joel's, so niri keeps an empty workspace past his, as his screen had then.
    desktop.opened("Notes", "sh -c 'sleep 608'")
    # The host, standing in for pi's own, as test_a_hosted_session_comes_back_... has it.
    host = desktop.as_account(
        "setsid -f sleep 606 > /dev/null 2>&1;"
        " timeout 10 sh -c \"until pgrep -n -f 'sleep 60[6]'; do sleep 0.1; done\""
    ).strip()
    ticks = desktop.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{host}/stat")
    presence = {
        "session": session,
        "addresses": ["KEPTVW"],
        "title": f"pi --session {session}",
        "at": dt.datetime.now(dt.UTC).isoformat(),
        "owner": {
            "pid": int(host),
            "bootId": desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
            "startTicks": int(ticks),
            "startedAt": int(desktop.as_account("date +%s%3N")),
        },
        "hosted": {"host": "harbour-attended", "conversation": session, "view": ["sleep", "607"]},
    }
    machine.wait_until_succeeds(
        desktop.in_session("ls -d ~/.local/state/quickshell/by-shell/*"),
        timeout=dt.timedelta(seconds=60),
    )
    shell = desktop.as_account("echo ~/.local/state/quickshell/by-shell/*").strip()
    try:
        kept(desktop, f"{PI_PRESENCE}/keptvw.json", presence)
        machine.wait_until_succeeds(
            desktop.in_session(f"mycelium --realm harbour sessions | grep -qF '{session}'"),
            timeout=dt.timedelta(seconds=60),
        )
        desktop.as_account("systemctl --user stop quickshell")
        # Kept on a workspace since gone, as both were at 13:41:38.
        at = int(desktop.as_account("date +%s%3N"))
        kept(
            desktop,
            f"{shell}/kept-places.json",
            {session: {"workspace": 999, "column": 1, "at": at}},
        )
        desktop.as_account("systemctl --user start quickshell")
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc call launcher list > /dev/null"),
            timeout=dt.timedelta(seconds=120),
        )
        # The shell opened those two within seconds of hearing the node, so half a minute
        # with no viewer shows it opens none.
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            viewers = [w for w in desktop.windows() if w["app_id"] == f"pi-hosted.{session}"]
            assert viewers == [], f"a viewer opened as the shell started: {viewers}"
            time.sleep(0.5)
        still = json.loads(desktop.as_account(f"cat {shell}/kept-places.json"))
        assert session in still, still
    finally:
        kept(desktop, f"{shell}/kept-places.json", {})
        desktop.as_account(f"rm -f {PI_PRESENCE}/keptvw.json; pkill -f 'sleep 60[678]'; true")


def hosted_here(desktop: Desktop, sessions: list[str]) -> None:
    """Say these run in one host here, standing in for pi's, each viewed by a sleep."""
    # setsid -f returns before its fork runs, so the pid is looked for under a deadline.
    host = desktop.as_account(
        "setsid -f sleep 604 > /dev/null 2>&1;"
        " timeout 10 sh -c \"until pgrep -n -f 'sleep 60[4]'; do sleep 0.1; done\""
    ).strip()
    owner = {
        "pid": int(host),
        "bootId": desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
        "startTicks": int(
            desktop.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{host}/stat")
        ),
        "startedAt": int(desktop.as_account("date +%s%3N")),
    }
    for at, session in enumerate(sessions):
        kept(
            desktop,
            f"{PI_PRESENCE}/given{at}.json",
            {
                "session": session,
                "addresses": [f"GIVEN{at}"],
                "title": f"pi --session {session}",
                "at": dt.datetime.now(dt.UTC).isoformat(),
                "owner": owner,
                "hosted": {
                    "host": "harbour-attended",
                    "conversation": session,
                    "view": ["sleep", "605"],
                },
            },
        )
    desktop.machine.wait_until_succeeds(
        desktop.in_session(
            "test $(mycelium --realm harbour sessions | grep -o '11r[0-9]-given'"
            f" | sort -u | wc -l) -eq {len(sessions)}"
        ),
        timeout=dt.timedelta(seconds=60),
    )


def reopen_them_all(desktop: Desktop) -> float:
    """Run Reopen Them All from the launcher, once it offers it, and give when it ran."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"qs ipc call launcher list | grep -qF {shlex.quote(REOPEN_ALL)}"),
        timeout=dt.timedelta(seconds=120),
    )
    began = time.monotonic()
    ran = desktop.as_account(f"qs ipc call launcher run {shlex.quote(REOPEN_ALL)}")
    assert '"error"' not in ran, ran
    return began


def kept_this_boot(desktop: Desktop, sessions: list[str]) -> None:
    """Wait for the agents daemon to keep where each of these sits this boot, by its session."""
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    deadline = time.monotonic() + 60
    while not set(sessions) <= set(
        (arranged := json.loads(desktop.as_account(f"cat {ARRANGED} 2>/dev/null || echo {{}}")))
        .get("boots", {})
        .get(boot, {})
    ):
        assert time.monotonic() < deadline, f"not kept this boot: {arranged}"
        time.sleep(0.5)


def given_back(desktop: Desktop, sessions: list[str]) -> None:
    """Have the host give these back with no window open, offered in restore.json.

    Its conversations end and their viewers close, as release --all leaves those whose viewers
    had closed, and each is offered as the harness writes one given back, under a boot that kept
    none of them, since they ran this boot.
    """
    desktop.as_account(f"rm -f {PI_PRESENCE}/given*.json; pkill -f 'sleep 60[45]'; true")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("! niri msg --json windows | grep -qF 'pi-hosted.11r'"),
        timeout=dt.timedelta(seconds=60),
    )
    released = dt.datetime.now(dt.UTC).isoformat()
    kept(
        desktop,
        "~/.local/state/pi/restore.json",
        {
            "v": 1,
            "boot": "a-boot-that-kept-none",
            "sessions": [
                {
                    "id": session,
                    "address": f"GIVEN{at}",
                    "title": session,
                    "cwd": "/home/jitsusama/reef",
                    "lostAt": released,
                    "released": {"from": "harbour-attended", "at": released},
                }
                for at, session in enumerate(sessions)
            ],
        },
    )


def test_ten_given_back_by_their_host_come_back_as_plain_pis_where_their_viewers_sat(
    desktop: Desktop,
) -> None:
    """Reopen Them All puts ten sessions their host gave back where their viewers sat this boot.

    Item 11's rollback drill: ten conversations pi's host ran, viewed in two workspaces of five
    columns, are released to plain pi with no window open, as release --all leaves those whose
    viewers had closed, and restore.json offers each as given back. The boot it names kept none
    of them, since they ran this boot, so only this boot's places say where each goes. Each is
    begun once as plain pi, in its viewer's column, beside the same four, with Joel's window
    keeping the keys. The time from Run to the tenth placed is printed for the drill's sheet.
    """
    sessions = [f"11r{at}-given" for at in range(GIVEN)]
    output = json.loads(desktop.as_account("niri msg --json focused-output"))["name"]
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/reef")
    notes = desktop.opened("Notes", "sh -c 'sleep 600'")
    try:
        hosted_here(desktop, sessions)
        # Their viewers open where the boot before kept them, five to a workspace.
        since = desktop.mark()
        kept_before(
            desktop,
            {
                session: {
                    "output": output,
                    "workspace": 1 + at // ACROSS,
                    "column": 1 + at % ACROSS,
                    "tile": 1,
                }
                for at, session in enumerate(sessions)
            },
            frozenset(sessions),
        )
        reopen_them_all(desktop)
        settled_since(desktop, since)
        viewed = viewers_sitting(desktop, "11r")
        assert sorted(viewed) == sessions, f"not ten viewers: {desktop.windows()}"
        kept_this_boot(desktop, sessions)
        since = desktop.mark()
        given_back(desktop, sessions)
        began = reopen_them_all(desktop)
        # Its stand-in titles each window a moment after niri lays it out.
        deadline = began + BACK
        while sorted(back := sitting(desktop, "11r")) != sessions:
            assert time.monotonic() < deadline, f"not all ten back: {back}"
            time.sleep(0.2)
        took = time.monotonic() - began
        settled_since(desktop, since)
        print(f"ten given back reopened as plain pis in {took:.1f} s")
        back = sitting(desktop, "11r")
        # Each in its viewer's column and tile, beside the same four, on a workspace of their
        # own: the viewers' were dropped once empty, so only the numbering may differ.
        spaces = {viewed[one][0]: back[one][0] for one in sessions}
        assert len(set(spaces.values())) == len(spaces) == GIVEN // ACROSS, (viewed, back)
        assert {one: back[one][1:] for one in sessions} == {
            one: viewed[one][1:] for one in sessions
        }, (viewed, back)
        assert all(spaces[viewed[one][0]] == back[one][0] for one in sessions), (viewed, back)
        assert notes["workspace_id"] not in {
            one["workspace_id"]
            for one in desktop.windows()
            if (one["title"] or "").startswith("pi --session 11r")
        }, desktop.windows()
        titles = [(one["title"] or "").split()[2:3] for one in desktop.windows()]
        assert all(titles.count([one]) == 1 for one in sessions), titles
        focused = json.loads(desktop.as_account("niri msg --json focused-window"))
        assert focused["id"] == notes["id"], focused
    finally:
        desktop.launcher_left_closed()
        nothing_kept(desktop)
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays $XDG_RUNTIME_DIR/pi-owner-11r*"
            f" {PI_PRESENCE}/given*.json; pkill -f 'sleep 60[0-5]'; true"
        )
        desktop.as_account("rm -rf ~/reef")


def sitting_space(desktop: Desktop, window: dict[str, Any]) -> str:
    """Give a window's workspace as the shell's log says one, its screen and its number."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    space = next(one for one in spaces if one["id"] == window["workspace_id"])
    return f"{space['output']} {space['idx']}"

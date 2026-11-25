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
opens no viewer kept before it. New work started from a link starts its session in no window
at all. Joel, after the
21:24 reboot on 2026-11-08 left 31 windows on the wrong workspaces: "i'm so confused, i don't
like my operating system confusing me." The area stays one file. Long because each test lays
out windows and kept places on the one desktop the area boots, with the helpers that read them
back shared between them: split, each part would boot a machine of its own to read the same
shell.

tests/desktop/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import shlex
import time
from typing import TYPE_CHECKING, Any

from desktop_session import (
    ARRANGED,
    KEPT_PLACES,
    PI_PRESENCE,
    kept,
    listed_quest,
    quests_listed,
)

if TYPE_CHECKING:
    from desktop_session import Desktop

# How long every window of a list may take to come back where it sat, on a loaded builder.
BACK = 240
# Where a window sits: its workspace's number, its column and its tile in that column.
Sits = tuple[int, int, int]


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

    The stand-in wrote its process and kitty window by its session
    (tests/stand-ins/pi-stand-in.sh).
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
        # (modules/quickshell/tests/places.qml: Places.queuedOnce, Places.keptFor): a VM can't time
        # a hand-over into Reopen Them All's queue. Here each says its record once it's begun, as pi
        # does, and LEADBB, once every window is back, hands its two over again, as an orchestrator
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
        # The shell's lines can reach the journal after the window, on a busy machine.
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(since)} | grep -qF 'sessions: reopening settled'"
            ),
            timeout=dt.timedelta(seconds=BACK),
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
    try:
        kept(desktop, f"{PI_PRESENCE}/keptvw.json", presence)
        machine.wait_until_succeeds(
            desktop.in_session(f"mycelium --realm harbour sessions | grep -qF '{session}'"),
            timeout=dt.timedelta(seconds=60),
        )
        desktop.as_account("systemctl --user stop quickshell")
        # Kept on a workspace since gone, as both were at 13:41:38, by the agents daemon, which
        # serves it to the shell as it starts.
        at = int(desktop.as_account("date +%s%3N"))
        kept(
            desktop,
            KEPT_PLACES,
            {"v": 1, "places": {session: {"workspace": 999, "column": 1, "at": at}}},
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
        still = json.loads(desktop.as_account(f"cat {KEPT_PLACES}"))
        assert session in still["places"], still
    finally:
        kept(desktop, KEPT_PLACES, {"v": 1, "places": {}})
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


def test_new_work_from_a_link_opens_its_session_focused(desktop: Desktop) -> None:
    """Start New Work starts a session on a link, in its repository, and opens it for Joel.

    Joel, 2026-11-16: starting a session on incoming work took a hand-built chain of a quest,
    a folder and a start. Typed from the find, as his try-line says, From a Link asks for the
    link, and the agents daemon, taking it only from the shell, asks the realm's node for a
    Sprout, which starts pi held in the link's checkout, told to make the work a quest of its
    own. Once its session is up, its window opens and holds the keys, the launcher closed
    for it (Joel, 18 November: "start new work does not seem to work from the launcher",
    when it started in no window and this walk held that in place).
    """
    machine = desktop.machine
    link = "https://github.com/harbour/lighthouse/issues/7"
    folder = "/home/jitsusama/src/github.com/harbour/lighthouse"
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    first = "$XDG_RUNTIME_DIR/pi-first-text"
    # pi stays running, as a real one does, so a window it brought would still be there.
    desktop.as_account(
        f"rm -f {opened} {first} && mkdir -p {folder} && touch $XDG_RUNTIME_DIR/pi-stays"
    )
    before = {window["id"] for window in desktop.windows()}
    noticed = starts_noticed(desktop)
    walked_since = desktop.mark()
    try:
        # Any program of Joel's can ask the shell's catalogue, an agent among them, so From a
        # Link asked that way starts nothing: only his own Return sprouts (#5456's council).
        listed = json.loads(desktop.as_account("qs ipc call launcher list"))
        from_a_link = [one["id"] for one in listed if one["words"] == "From a Link"]
        assert len(from_a_link) == 1, f"From a Link isn't in the catalogue once: {from_a_link}"
        walked = json.loads(
            desktop.as_account(
                f"qs ipc call launcher runWith {shlex.quote(from_a_link[0])} {shlex.quote(link)}"
            )
        )
        assert walked.get("done") is not True, f"a walk from outside sprouted: {walked}"
        # Nor does it put a notice on his screen, which any program could then do: once its
        # end is journalled, when a notice would have been spawned, and a moment for one to land.
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(walked_since)}"
                f" | grep -q {WALK_ENDED}"
            ),
            timeout=dt.timedelta(seconds=30),
        )
        machine.sleep(NOTICE_LANDS)
        assert starts_noticed(desktop) == noticed, "a walk from outside raised a notice"
        # The launcher in a terminal walks the same way, even told --yes.
        status, said = machine.execute(
            desktop.in_session(
                "launcher do agents start-new-work from-a-link --yes"
                f" -- {shlex.quote(link)} </dev/null 2>&1"
            )
        )
        assert status != 0, f"launcher do sprouted: {status} {said}"
        assert "new work starts only from your own Return" in said, (
            f"launcher do wasn't told why: {status} {said}"
        )
        # Watched for a while, since a sprout that went would reach the stand-in in seconds.
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            desktop.as_account(f"test ! -e {opened}")
            time.sleep(0.5)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("new work", selecting="Start New Work")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["From a Link"])
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["mode"] == "insert")
        machine.send_chars(link)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -qx '{folder} ' {opened}"),
            timeout=dt.timedelta(seconds=120),
        )
        # The stand-in writes the first words before pi-opened, so they're there by now.
        told = desktop.as_account(f"cat {first} 2>/dev/null; true")
        assert f"sprouted this session from {link}" in told, f"pi wasn't told the link: {told!r}"
        assert "quest create" in told, f"pi wasn't told to make a quest: {told!r}"
        # Its session says its record once it's up, as pi does, and the node, finding its
        # holder by the process, names the holder's id on it, which the shell opens it by.
        machine.wait_until_succeeds(
            desktop.in_session("test -s $XDG_RUNTIME_DIR/pi-owner-sprouted"),
            timeout=dt.timedelta(seconds=30),
        )
        presents(desktop, "sprouted", "SPROUT", {"cwd": folder})
        opened_focused(desktop, before)
        assert desktop.showing()["open"] is False, "the launcher stayed open over it"
    finally:
        desktop.launcher_left_closed()
        # Its window too, opened or not by the time a failure came, so the next walk has none.
        for window in desktop.windows():
            if window["id"] not in before:
                desktop.close(window)
        # A pattern its own shell's words don't match, so pkill ends only the stand-in.
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays {opened} {first} $XDG_RUNTIME_DIR/pi-owner-sprouted"
            f" {PI_PRESENCE}/sprout.json; pkill -f 'sleep 60[0-5]'; true"
        )
        desktop.as_account("rm -rf ~/src/github.com/harbour")


def test_new_work_that_starts_nothing_is_told_once_the_launcher_is_shut(
    desktop: Desktop,
) -> None:
    """A start that started nothing says so as a notice when it ends with the launcher shut.

    Joel, 2026-11-18 17:15, of a start the launcher said the end of only on its row, in a
    launcher he'd shut five seconds after his Return: "That can never happen, things I send in
    must be seen through." Here the node takes the ask and falls silent, so the end comes after
    he's gone, as the agents daemon's patience runs out; the shell's journal says it too.
    """
    # A repository with no checkout, so the ask the node reads once it runs again starts nothing.
    link = "https://github.com/harbour/nowhere/issues/8"
    since = desktop.mark()
    before = starts_noticed(desktop, UNANSWERED)
    desktop.as_account("systemctl --user kill --signal=STOP mycelium")
    try:
        asked_from_a_link(desktop, link)
        # The daemon gives up on a silent node within 50 s, the shell on the daemon within 60.
        noticed(desktop, UNANSWERED, before, dt.timedelta(seconds=90))
        said = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(since)}"
        )
        assert "launcher: new work's start went unanswered; it may still start" in said, (
            f"its end wasn't journalled: {said}"
        )
        # The test's own reports (report[...]), off on his machine, show the line as typed.
        kept = [line for line in said.splitlines() if "report[" not in line]
        assert not [line for line in kept if link in line], "the journal kept his words"
    finally:
        desktop.as_account("systemctl --user kill --signal=CONT mycelium")
        desktop.launcher_left_closed()


def test_a_link_refused_for_want_of_a_repository_steps_back_to_the_folders(
    desktop: Desktop,
) -> None:
    """A link with no checkout, refused for want of a repository, steps Joel back to pick one.

    Joel, 2026-11-24 12:23, of a link Start New Work refused that way: "the UI was crap". His
    Return empties the field, as the launcher's give does once his words are taken, which is
    no letter of his, so he's still there to be stepped back onto Start New Work's folders,
    an open quest's among them, rather than left in an empty field told to run a command.
    """
    machine = desktop.machine
    link = "https://github.com/harbour/nowhere/issues/9"
    folder = "/home/jitsusama/src/github.com/harbour/lighthouse"
    desktop.as_account(f"mkdir -p {folder}")
    quests_listed(desktop, [listed_quest("QEST-20261124-HARBQA", "Harbour", start=folder)])
    since = desktop.mark()
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("new work", selecting="Start New Work")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["From a Link"])
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["mode"] == "insert")
        machine.send_chars(link)
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                f" --after-cursor={shlex.quote(since)}"
                " | grep -q 'launcher: new work wanted a repository; asking where it starts'"
            ),
            timeout=dt.timedelta(seconds=60),
        )
        desktop.launcher_until(
            lambda shown: shown["open"] is True and shown["rows"][:1] == ["From a Link"]
        )
    finally:
        desktop.launcher_left_closed()
        quests_listed(desktop, [])
        desktop.as_account("rm -rf ~/src/github.com/harbour")


def test_new_work_in_a_zone_by_its_name_begins_at_its_checkouts_top(desktop: Desktop) -> None:
    """Start New Work lists a big repository's zone by its name, and starts in one not out.

    Joel, 2026-11-24 12:23, wanting to start in a zone of a big repository: "the list of things
    to start in was shit as well". The launcher asks the realm's node for its places each time
    it opens, through the agents daemon, and lists each zone a checkout's manifest names. One
    not checked out has no folder yet, so it starts by its name, and the node begins it at its
    checkout's top, telling pi to work in a tree of its own that has the zone.
    """
    machine = desktop.machine
    checkout = "/home/jitsusama/src/github.com/harbour/basin"
    zone = "//system/beacon"
    row = f"In {zone}"
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    first = "$XDG_RUNTIME_DIR/pi-first-text"
    manifest = shlex.quote(json.dumps({zone: {}}))
    # A checkout is a folder holding .git (mycelium's places.rs), its zones its manifest's keys.
    desktop.as_account(
        f"rm -f {opened} {first} && mkdir -p {checkout}/.git {checkout}/.meta"
        f" && printf '%s' {manifest} > {checkout}/.meta/manifest.json"
        " && touch $XDG_RUNTIME_DIR/pi-stays"
    )
    before = {window["id"] for window in desktop.windows()}
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("new work", selecting="Start New Work")
        machine.send_key("ret")
        # The node's places come a moment after the launcher opens.
        desktop.launcher_until(lambda shown: row in shown["rows"])
        desktop.down_to(row)
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["mode"] == "insert")
        machine.send_chars("tidy the beacon's rows")
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -qx '{checkout} ' {opened}"),
            timeout=dt.timedelta(seconds=120),
        )
        # The stand-in writes the first words before pi-opened, so they're there by now.
        told = desktop.as_account(f"cat {first} 2>/dev/null; true")
        assert "tidy the beacon's rows" in told, f"pi wasn't told his words: {told!r}"
        assert f"asked to begin in {zone}, which isn't checked out in {checkout}" in told, (
            f"pi wasn't told where it begins: {told!r}"
        )
        assert "a tree of your own" in told, f"pi wasn't told to work apart: {told!r}"
    finally:
        desktop.launcher_left_closed()
        for window in desktop.windows():
            if window["id"] not in before:
                desktop.close(window)
        # A pattern its own shell's words don't match, so pkill ends only the stand-in.
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays {opened} {first} $XDG_RUNTIME_DIR/pi-owner-sprouted"
            f" {PI_PRESENCE}/sprout.json; pkill -f 'sleep 60[0-5]'; true"
        )
        desktop.as_account("rm -rf ~/src/github.com/harbour")


def test_new_work_is_pictured_from_its_folder_to_its_window(desktop: Desktop) -> None:
    """Pictures each step of Start New Work in a folder, as Joel walks it, to its window.

    Joel, 2026-11-25 14:33, of a start he gave in a folder: "i have no room to type once i
    choose a folder, no confirmation that it took my enter to start the work, no feedback at
    all, and now no clue if i did it once, twice, thrice or not at all". The gallery keeps
    each step for whoever judges the path end to end; the walk itself proves only that the
    session's window opens, as the link's walk does.
    """
    machine = desktop.machine
    folder = "/home/jitsusama/src/github.com/harbour/lighthouse"
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    first = "$XDG_RUNTIME_DIR/pi-first-text"
    desktop.as_account(
        f"rm -f {opened} {first} && mkdir -p {folder} && git -C {folder} init -q"
        " && touch $XDG_RUNTIME_DIR/pi-stays"
    )
    quests_listed(desktop, [listed_quest("QEST-20261124-HARBQA", "Harbour", start=folder)])
    before = {window["id"] for window in desktop.windows()}
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("new work", selecting="Start New Work")
        desktop.shoot("new-work-1-found")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["From a Link"])
        desktop.down_to("In ~/src/github.com/harbour/lighthouse")
        desktop.shoot("new-work-2-folders")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["mode"] == "insert")
        desktop.shoot("new-work-3-asked")
        machine.send_chars(NEW_WORK_SAID)
        desktop.launcher_until(lambda shown: shown["typed"] == NEW_WORK_SAID)
        desktop.shoot("new-work-4-typed")
        machine.send_key("ret")
        # Return is seen to take: the words stay, and the line under them says the start
        # is under way and counts its wait.
        returned = desktop.launcher_until(
            lambda shown: not shown["open"] or shown["start"]["says"] != ""
        )
        if returned["open"]:
            assert returned["typed"] == NEW_WORK_SAID, f"Return took the words: {returned}"
            assert returned["start"]["waited"].endswith(" s"), f"no wait counted: {returned}"
        # The moment after Return is what Joel judges took or not, so it's kept as it is,
        # not once the screen holds still, which a start under way may never do.
        if desktop.values["shots"]:
            machine.sleep(1)  # A beat, as his eye takes in what Return changed.
            machine.screenshot("new-work-5-returned")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -qx '{folder} ' {opened}"),
            timeout=dt.timedelta(seconds=120),
        )
        machine.wait_until_succeeds(
            desktop.in_session("test -s $XDG_RUNTIME_DIR/pi-owner-sprouted"),
            timeout=dt.timedelta(seconds=30),
        )
        presents(desktop, "sprouted", "SPROUT", {"cwd": folder})
        opened_focused(desktop, before)
        desktop.shoot("new-work-6-window")
        # Back to Start New Work, to see what says the start took, and only once.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("new work", selecting="Start New Work")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["From a Link"])
        desktop.shoot("new-work-7-again")
        # The same words again say when they started, and start nothing more.
        desktop.down_to("In ~/src/github.com/harbour/lighthouse")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["mode"] == "insert")
        if desktop.showing()["typed"] != NEW_WORK_SAID:
            machine.send_chars(NEW_WORK_SAID)
            desktop.launcher_until(lambda shown: shown["typed"] == NEW_WORK_SAID)
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["start"]["says"].startswith("started at "))
        desktop.shoot("new-work-8-same")
        starts = desktop.as_account(f"grep -cx '{folder} ' {opened}").strip()
        assert starts == "1", f"the same words started the work again: {starts} starts"
    finally:
        desktop.launcher_left_closed()
        for window in desktop.windows():
            if window["id"] not in before:
                desktop.close(window)
        # A pattern its own shell's words don't match, so pkill ends only the stand-in.
        desktop.as_account(
            f"rm -f $XDG_RUNTIME_DIR/pi-stays {opened} {first} $XDG_RUNTIME_DIR/pi-owner-sprouted"
            f" {PI_PRESENCE}/sprout.json; pkill -f 'sleep 60[0-5]'; true"
        )
        quests_listed(desktop, [])
        desktop.as_account("rm -rf ~/src/github.com/harbour")


# What Joel gives a start in the pictured walk: as long as the words he gave at 14:30 on 25
# November, and a little more, so the field shows how it holds a sentence that wraps.
NEW_WORK_SAID = (
    "see the dms with olga about access to the lighthouse tooling, put together a solution"
    " and an idea of what happened, so she can tell her team today"
)


def test_a_start_the_shell_restarted_under_is_told_by_the_shell_started_again(
    desktop: Desktop,
) -> None:
    """A start whose shell restarts before it ends is told by the shell that comes up.

    The agents daemon keeps each start from his Return until the shell says it told him, so a
    shell started again mid-start, as a switch or a crash does, still says how it ended (Joel,
    2026-11-18: "things I send in must be seen through"). Here the node falls silent, so the
    start ends after the old shell has gone, as the daemon's patience runs out, and the new
    shell tells it once its asker would surely have.
    """
    machine = desktop.machine
    link = "https://github.com/harbour/nowhere/issues/9"
    before = starts_noticed(desktop, UNANSWERED)
    desktop.as_account("systemctl --user kill --signal=STOP mycelium")
    try:
        asked_from_a_link(desktop, link)
        desktop.as_account("systemctl --user restart quickshell")
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc call launcher list > /dev/null"),
            timeout=dt.timedelta(seconds=120),
        )
        noticed(desktop, UNANSWERED, before, dt.timedelta(seconds=ASKER_ANSWERS + 60))
        # The new shell said it told, so the daemon keeps it no more.
        starts_kept(desktop, 0)
        # There's no event for a notice never sent, so a second is waited for.
        machine.sleep(NOTICE_LANDS)
        told = starts_noticed(desktop, UNANSWERED)
        assert told == before + 1, f"a start was noticed {told - before} times"
    finally:
        desktop.as_account("systemctl --user kill --signal=CONT mycelium")
        desktop.launcher_left_closed()


def test_a_start_a_switch_cut_off_is_told_once(desktop: Desktop) -> None:
    """A start cut off by a switch's restarts is told once, as one that may still start.

    A switch stops the agents daemon under the shell as it asks, and hands the shell over to a
    new one held beside it (shell-hands-over.sh) while the start is still kept. The daemon come
    back finds the start with no end and ends it as cut off, since the node may have started it
    and asking again blind could start it twice. The shell that asked, its answer gone with the
    daemon, takes that record as its answer and tells it; the held one, which never asked, waits
    out the asker's patience and finds it told.
    """
    machine = desktop.machine
    link = "https://github.com/harbour/nowhere/issues/10"
    before = starts_noticed(desktop)
    cut = starts_noticed(desktop, CUT_OFF)
    desktop.as_account("systemctl --user kill --signal=STOP mycelium")
    try:
        asked_from_a_link(desktop, link)
        asked = time.monotonic()
        # Kept before the daemon goes, so it's this walk's start the daemon come back finds.
        starts_kept(desktop, 1)
        old = desktop.as_account("systemctl --user show quickshell -p MainPID --value").strip()
        desktop.as_account("systemctl --user stop agents")
        desktop.as_account("systemctl --user reload --no-block quickshell")
        desktop.as_account("systemctl --user start agents")
        # The daemon's words for it, not the shell's for an answer that never came.
        noticed(desktop, CUT_OFF, cut, dt.timedelta(seconds=120))
        starts_kept(desktop, 0)
        machine.wait_until_succeeds(
            desktop.in_session(
                f'test "$(systemctl --user show quickshell -p MainPID --value)" != {old}'
            ),
            timeout=dt.timedelta(seconds=120),
        )
        # The held shell, served it as it came up, would tell it once its asker had surely
        # told it: at its first look every START_SETTLES past that, so as late as a second
        # step on, its notice landing after. There's no event for a notice never sent, so
        # that's waited.
        waited = time.monotonic() - asked
        latest = ASKER_ANSWERS + 2 * START_SETTLES + NOTICE_LANDS
        machine.sleep(max(NOTICE_LANDS, int(latest - waited) + 1))
        told = starts_noticed(desktop)
        assert told == before + 1, f"a cut-off start was noticed {told - before} times"
    finally:
        desktop.as_account("systemctl --user kill --signal=CONT mycelium")
        desktop.launcher_left_closed()


def asked_from_a_link(desktop: Desktop, link: str) -> None:
    """Start New Work on `link` from the launcher with Joel's own Return, then shut it."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("new work", selecting="Start New Work")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["From a Link"])
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["mode"] == "insert")
    machine.send_chars(link)
    machine.send_key("ret")
    # Return empties the field once it's taken the words, and then he leaves.
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_until(lambda shown: shown["open"] is False)


def noticed(desktop: Desktop, body: str | None, before: int, within: dt.timedelta) -> None:
    """Wait `within` for Start New Work to notice `body`, or any, once more than `before`."""
    which = STARTS_NOTICED
    if body is not None:
        which = f"{STARTS_NOTICED} | select(.body == {json.dumps(body)})"
    told = f"[{which}] | length > {before}"
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"jq -e {shlex.quote(told)} {desktop.NOTICES_KEPT}"),
        timeout=within,
    )


def starts_kept(desktop: Desktop, count: int) -> None:
    """Wait for the agents daemon to keep `count` starts on disk, as it writes them."""
    kept = shlex.quote(f"(.starts | length) == {count}")
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"jq -e {kept} {STARTS_KEPT}"),
        timeout=dt.timedelta(seconds=30),
    )


# Start New Work's notices, and what the agents daemon says of a start its node took and fell
# silent on (modules/agents/src/sprout.rs's SILENT).
WALK_ENDED = shlex.quote("launcher: new work wasn't started")
# The starts the agents daemon keeps until the shell says it told Joel (modules/agents/home.nix's
# --kept folder, starts.rs's FILE).
STARTS_KEPT = "~/.local/state/agents/starts.json"
# Seconds for a notice spawned as a walk's end is journalled to reach the notices daemon's
# kept list, waited before saying none came: there's no event for a notice never sent.
NOTICE_LANDS = 3
# Seconds a shell waits before telling a kept start it never asked (Sprouting.js's SETTLE_MS),
# and from the start's asking (its ASKER_ANSWERS_MS).
START_SETTLES = 5
ASKER_ANSWERS = 75
STARTS_NOTICED = '.notices[]? | select(.app == "Agents" and .summary == "Start New Work")'
UNANSWERED = (
    "the node didn't answer in time; it may still start it, so look in Agents before asking again"
)
# What the agents daemon ends a start a restart cut off with (starts.rs's CUT_OFF).
CUT_OFF = (
    "a restart cut it off before the node answered; it may still start it, so look in Agents"
    " before asking again"
)


def starts_noticed(desktop: Desktop, body: str | None = None) -> int:
    """Give how many notices Start New Work has raised, of `body` if named, none at first."""
    which = STARTS_NOTICED
    if body is not None:
        which = f"{STARTS_NOTICED} | select(.body == {json.dumps(body)})"
    program = shlex.quote(f"[{which}] | length")
    return int(
        desktop.as_account(f"jq {program} {desktop.NOTICES_KEPT} 2>/dev/null || echo 0").strip()
    )


def opened_focused(desktop: Desktop, before: set[int]) -> None:
    """Wait for one window more than `before`, a pi session's, holding the keys.

    Up to a little past the shell's minute, after which it opens nothing: it hears a new
    session through the node and the agents daemon before it opens it.
    """
    deadline = time.monotonic() + 65
    while True:
        brought = [one for one in desktop.windows() if one["id"] not in before]
        focused = json.loads(desktop.as_account("niri msg --json focused-window"))
        if (
            len(brought) == 1
            and str(brought[0].get("app_id") or "").startswith("pi-session.")
            and (focused or {}).get("id") == brought[0]["id"]
        ):
            return
        # Each window's id and app_id, the focused one, and where the launcher stood
        # say which part failed: none brought, the wrong one, or the keys left behind.
        assert time.monotonic() < deadline, (
            "its window didn't open focused: brought "
            f"{[(one['id'], one.get('app_id')) for one in brought]}, focused "
            f"{(focused or {}).get('id')} {(focused or {}).get('app_id')}, launcher "
            f"{desktop.showing()}"
        )
        # Each look at niri is cheap; half a second apart sees it open well within the wait.
        time.sleep(0.5)


def sitting_space(desktop: Desktop, window: dict[str, Any]) -> str:
    """Give a window's workspace as the shell's log says one, its screen and its number."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    space = next(one for one in spaces if one["id"] == window["workspace_id"])
    return f"{space['output']} {space['idx']}"

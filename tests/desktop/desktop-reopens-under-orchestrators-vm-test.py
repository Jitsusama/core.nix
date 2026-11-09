"""The desktop test's reopens-under-orchestrators area.

Here sessions come back, or start, on their orchestrator's workspace by themselves: a reboot's
list of two orchestrations puts each window where it sat, two stacked in one column included,
and begins none again when its orchestrator hands it over too; and a session one starts for
another's tree opens with that tree, in title order after its orchestrator. Joel, after the
21:24 reboot on 2026-11-08 left 31 windows on the wrong workspaces: "i'm so confused, i don't
like my operating system confusing me."

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
from desktop_session import PI_PRESENCE, kept

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

    The stand-in wrote its process and kitty window by its session (tests/pi-stand-in.nix).
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


def test_two_orchestrations_come_back_each_where_it_sat(desktop: Desktop) -> None:
    """A reboot's list of two orchestrations puts every window back where it sat.

    After the 21:24 reboot two windows stacked in one column each went after the other's place
    25 times, since the list said no tile; and the sessions an orchestrator brought back itself
    went wherever niri put them, since a hand-over never read the list.
    """
    output = json.loads(desktop.as_account("niri msg --json focused-output"))["name"]

    def at(workspace: int, column: int, tile: int | None = None) -> dict[str, Any]:
        """Give a place as the reboot's list says one, its tile only when said."""
        said = {"output": output, "workspace": workspace, "column": column}
        return said if tile is None else {**said, "tile": tile}

    # The second of the stacked pair says no tile, as the harness's list doesn't yet, so it
    # takes the next in its column, in the order listed.
    listed = {
        "01d1-lead-a": {**at(1, 1), "orchestrates": "LEADAA"},
        "01d1-a-one": at(1, 2, 1),
        "01d1-a-two": at(1, 2),
        "01d1-lead-b": {**at(2, 1), "orchestrates": "LEADBB"},
        "01d1-b-one": at(2, 2),
        "01d1-b-two": at(2, 3),
    }
    wanted = {
        "01d1-lead-a": (1, 1, 1),
        "01d1-a-one": (1, 2, 1),
        "01d1-a-two": (1, 2, 2),
        "01d1-lead-b": (2, 1, 1),
        "01d1-b-one": (2, 2, 1),
        "01d1-b-two": (2, 3, 1),
    }
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/reef")
    now, up = desktop.machine.succeed("date +%s; cut -d. -f1 /proc/uptime").split()
    taken = dt.datetime.fromtimestamp(int(now) - int(up) - 60, dt.UTC)
    kept(
        desktop,
        "~/.local/state/pi/reboot-reopen.json",
        {
            "v": 1,
            "boot": "a-boot-before",
            "taken": taken.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sessions": [
                {"id": one, "cwd": "/home/jitsusama/reef", "title": one, **place}
                for one, place in listed.items()
            ],
        },
    )
    reopen_all = "collection:agents\u203aagents:to-reopen\u203averb:agents:to-reopen:all"
    try:
        # The shell reads the reboot's list as it starts, as after a boot.
        desktop.as_account("systemctl --user restart quickshell")
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
        # (tests/places.qml: Places.joined, Places.keptFor): a VM can't time a hand-over into
        # Reopen Them All's queue. Here each says its record once it's begun, as pi does, and
        # LEADBB, once every window is back, hands its two over again, as an orchestrator
        # restoring its own does: neither is begun twice, since each runs here.
        said: set[str] = set()
        deadline = time.monotonic() + BACK
        while (back := sitting(desktop, "01d1-")) != wanted:
            assert time.monotonic() < deadline, f"not each where it sat: {back}"
            for one in set(back) - said:
                lead = listed[one].get("orchestrates")
                address = lead or f"RO{len(said):04}"
                presents(desktop, one, address, {"orchestrates": lead} if lead else {})
                said.add(one)
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
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/pi-stays ~/.local/state/pi/reboot-reopen.json"
            f" $XDG_RUNTIME_DIR/pi-owner-01d1-* {PI_PRESENCE}/leadaa.json {PI_PRESENCE}/leadbb.json"
            f" {PI_PRESENCE}/ro0*.json; pkill -f 'sleep 60[0]'; true"
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


def sitting_space(desktop: Desktop, window: dict[str, Any]) -> str:
    """Give a window's workspace as the shell's log says one, its screen and its number."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    space = next(one for one in spaces if one["id"] == window["workspace_id"])
    return f"{space['output']} {space['idx']}"

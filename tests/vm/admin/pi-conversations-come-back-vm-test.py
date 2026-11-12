"""A realm's attended pi host gives back the conversations it held, after a kill -9 and a reboot.

pi-conversations-come-back.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), on a machine whose account lingers with the work realm's hosts as their
module renders them, run by the harness's own host program on a pi built with the host's patches.
The machine boots before the harness is there, so neither host starts; the tests lay the harness
out as pi-packages installs it, give pi the harness's scripted model and its park workflow, and
start the attended host holding two conversations, one parked and one idle. The kill and the
reboot each start from the host holding both, so either runs alone with `-k`; the viewer's test
reads the screens after the reboot.
"""

from __future__ import annotations

import base64
import datetime as dt
import hashlib
import json
import re
import shlex
from typing import TYPE_CHECKING, Any, cast

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

UNIT = "pi-host-work-attended.service"
KEY = "work-attended"
SOCKET = f"/run/user/1000/pi-host/{KEY}.sock"
# Where pi-packages keeps the harness: each version in a folder of its own, `current` linking the
# one pi runs, and `loaded`, where each pi says which it loaded, which the host reads.
INSTALL = "/home/alice/.local/share/pi-packages/github.com/Jitsusama/agentic-harness.pi"
AGENT = "/home/alice/.pi/agent"
WORK = "/home/alice/work"
# Naming a conversation here parks it after two minutes idle, as parking is proven on one.
SWITCH = "/home/alice/.local/state/pi/agentic-harness.pi/park-workflow/switch"
ROSTER = f"/home/alice/.local/state/pi/agentic-harness.pi/host-rosters/{KEY}.json"
# What a ready host holding both says, as systemd's status and the journal show it.
SERVING_BOTH = "ready, serving 2 conversations"
# The harness's scripted model, as its test support names it.
PROVIDER = "sealed-faux"
MODEL = "faux-1"
# What each conversation is first told, which its screen shows: words the scripted model
# answers only with "ok", and that no other line of a screen holds.
FIRST_WORDS = {"parked": "the-parked-one-asks", "idle": "the-idle-one-asks"}
# The scripted model's answer as a record keeps it.
ANSWERED = '"text":"ok"'
# A kill's five seconds before the next start, the engine's build and both resumed.
COMING_BACK = dt.timedelta(minutes=2)
# Two minutes idle and the park workflow's look each minute, with room.
PARKING = dt.timedelta(minutes=6)
# A reboot: the host parking what it holds as it stops, a boot, and the host's start.
REBOOTING = dt.timedelta(minutes=6)
# How long a viewer stays attached, long enough to wake a parked conversation and draw it.
VIEWING_SECONDS = 20
ROWS = 40
COLUMNS = 120
# A terminal's control sequences, which the viewer's screen is read without.
CONTROLS = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[@-_]")


class Host:
    """The attended host of alice's work realm, as her user manager runs it."""

    def __init__(self, machine: QemuMachine, values: dict[str, Any]) -> None:
        """Act on the attended host on that machine, with what the tests need from Nix."""
        self.machine = machine
        self.asking = f"{values['node']} {values['ask']} {SOCKET}"
        self.mycelium = str(values["mycelium"])
        self.ids: dict[str, str] = {}
        self.files: dict[str, str] = {}
        self.records: dict[str, bytes] = {}

    def run(self, command: str) -> str:
        """Run a command as alice and give what it printed."""
        return as_account(self.machine, command, account="alice")

    def until(self, command: str, timeout: dt.timedelta = COMING_BACK) -> None:
        """Wait until a command run as alice succeeds."""
        self.machine.wait_until_succeeds(
            "su - alice -c "
            + shlex.quote(f"export XDG_RUNTIME_DIR=/run/user/1000; cd ~; {command}"),
            timeout=timeout,
        )

    def show(self, field: str) -> str:
        """Give one of the unit's properties, as systemd says it."""
        return self.run(f"systemctl --user show -p {field} --value {UNIT}").strip()

    def asked(self, request: dict[str, Any]) -> str:
        """Give the command that sends the host one request through its socket."""
        return f"{self.asking} {shlex.quote(json.dumps({'v': 1, 'id': request['op'], **request}))}"

    def ask(self, request: dict[str, Any]) -> dict[str, Any]:
        """Send the host one request through its socket and give its answer's result."""
        answer = json.loads(self.run(self.asked(request)))
        assert answer.get("ok"), answer
        return cast("dict[str, Any]", answer["result"])

    def states(self) -> dict[str, str]:
        """Give each held conversation's state by its session id, as the host lists them."""
        sessions = self.ask({"op": "list"})["sessions"]
        return {one["sessionId"]: one["state"] for one in sessions}

    def until_serving_both(self, timeout: dt.timedelta = COMING_BACK) -> None:
        """Wait until the host is ready again, saying it holds both."""
        self.until(
            f"systemctl --user show -p StatusText --value {UNIT}"
            f" | grep -qxF {shlex.quote(SERVING_BOTH)}",
            timeout,
        )

    def until_started_again(self, killed: str) -> None:
        """Wait until a process other than the killed one runs the host and says it holds both."""
        # The unit keeps the killed process's status line until another replaces it, so the
        # new process's own ready line is what says it holds both.
        self.until(
            f"pid=$(systemctl --user show -p MainPID --value {UNIT});"
            f' test "$pid" -ne 0 && test "$pid" -ne {killed}',
        )
        started = self.show("MainPID")
        self.machine.wait_until_succeeds(
            f"journalctl --no-pager _PID={started} | grep -qF {shlex.quote(SERVING_BOTH)}",
            timeout=int(COMING_BACK.total_seconds()),
        )

    def record(self, name: str) -> bytes:
        """Give a conversation's record, its session file, as it is now."""
        return base64.b64decode(self.run(f"base64 -w0 {shlex.quote(self.files[name])}"))

    def said(self) -> str:
        """Give what alice's manager, and the host, told the journal about the unit this boot."""
        # --user-unit matches the caller's own uid, which is root's here. The manager's lines
        # about a unit carry USER_UNIT, and the unit's own output _SYSTEMD_USER_UNIT.
        return self.machine.succeed(
            f"journalctl --no-pager -b _UID=1000 USER_UNIT={UNIT}"
            f" + _UID=1000 _SYSTEMD_USER_UNIT={UNIT}",
        )

    def view(self, name: str) -> str:
        """Attach a viewer to a conversation through a terminal and give what it was drawn."""
        drawn = f"/home/alice/viewed-{name}"
        viewer = (
            f"stty rows {ROWS} cols {COLUMNS};"
            f" exec {self.mycelium} --realm work attach --host {KEY} --session {self.ids[name]}"
        )
        # The viewer's keyboard stays open, typing nothing, until the viewer is ended.
        self.run(
            f"sleep {VIEWING_SECONDS + 5} | TERM=xterm-256color timeout {VIEWING_SECONDS}"
            f" script -qfec {shlex.quote(viewer)} {drawn} > /dev/null 2>&1 || true",
        )
        return CONTROLS.sub("", self.machine.succeed(f"cat {drawn}"))


def digest(record: bytes) -> str:
    """Give a record's digest, to say which it is in a failure."""
    return hashlib.sha256(record).hexdigest()


@pytest.fixture(scope="module")
def held(machine: BaseMachine, values: dict[str, Any]) -> Host:
    """Boot, install the harness, and start the host holding a parked and an idle conversation."""
    qemu = cast("QemuMachine", machine)
    qemu.start(allow_reboot=True)
    qemu.wait_for_unit("multi-user.target")
    qemu.wait_for_unit("user@1000.service")
    host = Host(qemu, values)
    host.run(
        f"mkdir -p {INSTALL}/versions {INSTALL}/loaded {AGENT} {WORK}"
        f" && cp -r {values['harness']} {INSTALL}/versions/{values['commit']}"
        f" && ln -sfn versions/{values['commit']} {INSTALL}/current",
    )
    current = f"{INSTALL}/current"
    settings = {
        "defaultProvider": PROVIDER,
        "defaultModel": MODEL,
        "extensions": [
            f"{current}/extensions/park-workflow",
            f"{current}/tests/host/end-to-end/support/scripted-provider.ts",
        ],
        "retry": {"enabled": False},
        "compaction": {"enabled": False},
    }
    for name, value in (("settings", settings), ("trust", {WORK: True})):
        host.run(f"printf %s {shlex.quote(json.dumps(value))} > {AGENT}/{name}.json")
    host.run(f"systemctl --user start {UNIT}")
    host.until(f"systemctl --user is-active {UNIT}")
    for name, words in FIRST_WORDS.items():
        opened = host.ask({"op": "open", "cwd": WORK, "attended": True, "first": words})
        host.ids[name] = opened["sessionId"]
        host.files[name] = opened["file"]
    for name in FIRST_WORDS:
        # The scripted model's answer, once the first turn is written down.
        host.until(f"grep -qF {shlex.quote(ANSWERED)} {shlex.quote(host.files[name])}")
    parked, idle = host.ids["parked"], host.ids["idle"]
    host.run(f"mkdir -p $(dirname {SWITCH}) && echo {parked} > {SWITCH}")
    listed_parked = '"sessionId":"' + parked + '"[^}]*"state":"parked"'
    host.until(f"{host.asked({'op': 'list'})} | grep -qE {shlex.quote(listed_parked)}", PARKING)
    assert host.states() == {parked: "parked", idle: "idle"}, host.states()
    host.until_serving_both()
    host.records = {name: host.record(name) for name in host.ids}
    return host


@pytest.fixture(scope="module")
def rebooted(held: Host) -> Host:
    """Reboot the machine with the host holding both, and wait for the host to say it's ready."""
    held.machine.reboot()
    held.machine.wait_for_unit("multi-user.target", timeout=int(REBOOTING.total_seconds()))
    held.machine.wait_for_unit("user@1000.service")
    held.until_serving_both(REBOOTING)
    return held


def test_a_killed_host_comes_back_holding_both_unchanged(held: Host) -> None:
    """A kill -9 of the host's process: systemd starts it again, and it holds both as they were."""
    host = held
    killed = host.show("MainPID")
    host.run(f"kill -9 {killed}")
    host.until_started_again(killed)
    said = host.said()
    assert "status=9/KILL" in said, said
    assert set(host.states()) == set(host.ids.values()), host.states()
    roster = host.run(f"cat {ROSTER}")
    for name, session in host.ids.items():
        assert session in roster, (name, roster)
        now = host.record(name)
        assert now == host.records[name], (name, digest(host.records[name]), digest(now))


def test_a_rebooted_machine_starts_the_host_and_resumes_both(rebooted: Host) -> None:
    """A reboot: the host starts with the account's manager, ready and holding both."""
    host = rebooted
    assert host.show("ActiveState") == "active"
    said = host.said()
    assert SERVING_BOTH in said, said
    assert set(host.states()) == set(host.ids.values()), host.states()
    roster = host.run(f"cat {ROSTER}")
    for name, session in host.ids.items():
        assert session in roster, (name, roster)
        # A stop parks what the host holds, which may add to a record, never take from it.
        before, now = host.records[name], host.record(name)
        assert now[: len(before)] == before, (name, digest(before), digest(now[: len(before)]))


def test_a_viewer_after_a_reboot_is_drawn_each_conversation(rebooted: Host) -> None:
    """A viewer attaching after a reboot is drawn that conversation's screen, its words on it."""
    for name, words in FIRST_WORDS.items():
        drawn = rebooted.view(name)
        assert words in drawn, (name, drawn[-2000:])

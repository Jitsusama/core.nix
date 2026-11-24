"""Whether a house's reports hold the client's upload under what the house leaves, and only then.

house.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. What the shaper heard of the house each second is
read from its own trace, and the rate it holds the upload to from its status.
"""

from __future__ import annotations

import datetime as dt
import json
import shlex
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
FAR = "203.0.113.3"
# The house's ceiling, and what its other machine sends, in kbit/s.
MOST = 8000
SENDS = 3000
# What the house leaves the client, with room for the reporter's second
# not lining up with the shaper's, and for the house's steady ceiling,
# which may sit as far above for five seconds as the reports swing.
LEFT = MOST - SENDS
HELD = LEFT + 700
# The least and most the reports may read of the house's 3 Mbit/s, past
# its headers and the reports' own seconds: a reporter that counted too
# little or too much, as another interface's count or a wrong unit would,
# reads outside them.
HEARD = 2400
SAID_MOST = 3600
# The least a house holds the client to, where a call still fits.
FLOOR = 4000
# The rate the client climbs past once the house goes quiet, which only a
# hold let go reaches: the connection is 8 Mbit/s.
CLIMBED = HELD + 500
# How many readings a second apart the hold has to keep.
READINGS = 10
# The address the house's other machine moves its route to, as a Mac does
# to its cable's or a new lease, which the client's house names no more than
# its first: its key alone says the reports are its.
MOVED_TO = "192.168.1.12"
# How long the client may take to boot to its first queues, the shaper to
# hear the house and hold, the hold to let go once the reports stop (3 s
# fresh, then the journal's next step), and the upload to climb back.
COMING_UP = dt.timedelta(seconds=60)
HOLDING = dt.timedelta(seconds=60)
QUIETING = dt.timedelta(seconds=15)
CLIMBING = dt.timedelta(seconds=120)
# Where the house's other machine forges its reports from while its
# reporter is stopped: the first address it sent from, which it keeps once
# its route moves. Signed under another key, in step on their own
# clock and saying the house sends 3 Mbit/s, taken they would hold the
# upload as the true ones do.
FORGED_FROM = "192.168.1.2"
WRONG_KEY = "not the house's key, but 32 b!!!"
# What the shaper says of a report whose mac isn't the house's key's.
FORGED = "mac isn't the house's key's"
# The most times the shaper may say forged reports held nothing while they
# come for up to QUIETING and CLIMBING: once a minute, and once more if a
# minute's turn falls in. src/house.rs's tests hold it to once a minute.
REFUSALS_SAID = 4
# The key both machines are moved to, and the file each holds it in.
NEXT_KEY = "the house's next key, 32 bytes!!"
KEY_FILE = "/etc/house.key"


@dataclass
class Rig:
    """The client, shaping its upload, and the house's other machine reporting beside it."""

    client: BaseMachine
    house: BaseMachine

    def up(self) -> int:
        """Give the rate the shaper holds eth1's upload to, in kbit/s, or 0 before it says."""
        status: list[Json] = json.loads(self.client.succeed("cat /run/shaper/status.json"))
        link = next((link for link in status if link["name"] == "eth1"), {})
        return int(link.get("up", 0))

    def traced(self) -> list[Json]:
        """Give each traced second of eth1, oldest first."""
        # Read as it stands, so a missing file can't bury the failure it explains.
        _, text = self.client.execute("cat /var/lib/shaper/heard/*.jsonl")
        seconds = []
        for line in text.splitlines():
            try:
                step: Json = json.loads(line)
            except json.JSONDecodeError:
                continue
            if step.get("link") == "eth1":
                seconds.append(step)
        return seconds

    def heard(self) -> list[Json]:
        """Give each traced second of eth1 that heard the house, oldest first."""
        return [step for step in self.traced() if "house_most" in step]

    def journal(self) -> str:
        """Give what the shaper said."""
        return self.client.succeed("journalctl -u shaper -o cat --no-pager")

    def said(self, words: str) -> int:
        """Give how many times the shaper said `words`, so a wait meets only what's said after."""
        return self.journal().count(words)

    def holds(self) -> list[int]:
        """Give the rate the upload is held to, read a second apart, READINGS times."""
        readings = []
        for _ in range(READINGS):
            readings.append(self.up())
            # Readings a second apart, as the shaper steps.
            self.client.succeed("sleep 1")
        return readings


@pytest.fixture(scope="module")
def rig(machines: dict[str, BaseMachine]) -> Rig:
    """Give the machines once the client's uploads run and the house's reporter is up."""
    for node in machines.values():
        node.start()
    client, house = machines["client"], machines["house"]
    machines["router"].wait_for_unit("connection.service")
    machines["server"].wait_for_unit("sink.service")
    client.wait_until_succeeds(
        'jq -e \'.[] | select(.name == "eth1" and (.network | startswith("link") | not))\''
        " /run/shaper/status.json",
        timeout=COMING_UP,
    )
    house.wait_until_succeeds("systemctl is-active house-report", timeout=COMING_UP)
    client.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=uploads"
        f" {BIN}/bash -c 'for i in 1 2 3 4; do"
        f" (while true; do {BIN}/cat /dev/zero > /dev/tcp/{FAR}/5201; done) & done; wait'"
    )
    house.succeed("systemctl start upload")
    return Rig(client=client, house=house)


def held(rig: Rig) -> None:
    """Wait until the shaper hears the house's upload and holds under what it leaves."""

    def holding(_last_try: bool) -> bool:
        """Say whether the shaper heard the house's upload and holds under what it leaves."""
        # The newest second alone, which leaves the house's figures out
        # while the house holds nothing, so a hold heard before the house
        # went quiet can't pass for one now (house signing round 3's F5).
        last = (rig.traced() or [{}])[-1]
        sent, most = int(last.get("house_sent", 0)), int(last.get("house_most", MOST))
        return sent >= HEARD and most <= HELD

    retry(holding, timeout=HOLDING)


def test_the_house_holds_the_upload_under_what_it_leaves(rig: Rig) -> None:
    """While the house says it sends 3 Mbit/s, the client's upload is held under the 5 left."""
    held(rig)
    readings = rig.holds()
    heard = rig.heard()[-READINGS:]
    said = [(s.get("house_sent"), s.get("house_most"), s.get("up")) for s in heard]
    print(f"held at {readings} kbit/s, the house sending and leaving, and asked: {said}")
    assert max(readings) <= HELD, (
        f"the upload went to {max(readings)} kbit/s, past the {HELD} the house leaves: {said}"
    )
    assert min(readings) >= FLOOR, f"the house held the upload under the floor: {readings}"
    sent = [int(s.get("house_sent", 0)) for s in heard]
    assert min(sent) >= HEARD, f"the reports said the house sent {sent} kbit/s, under its {SENDS}"
    assert max(sent) <= SAID_MOST, (
        f"the reports said the house sent {sent} kbit/s, past its {SENDS}"
    )
    assert "the house's ceiling holds" in rig.journal(), "the shaper held without saying so"


def test_a_house_moved_to_another_address_still_holds(rig: Rig) -> None:
    """Once the reporter's route sends from another address, named nowhere, it still holds."""
    held(rig)
    rig.house.succeed(
        f"ip addr add {MOVED_TO}/24 dev eth1",
        f"ip route replace 192.168.1.0/24 dev eth1 proto kernel scope link src {MOVED_TO}",
    )
    rig.house.wait_until_succeeds(
        f"journalctl -u house-report -o cat --no-pager | grep -q 'by eth1, from {MOVED_TO}'",
        timeout=QUIETING,
    )
    rig.client.wait_until_succeeds(
        "journalctl -u shaper -o cat --no-pager"
        f' | grep -q "the first report from {MOVED_TO} for the house on .* was heard"',
        timeout=QUIETING,
    )
    # The house's word from its first address goes stale, so a hold that
    # stands past it was heard from the new one, where no house is named.
    rig.client.succeed("sleep 5")
    readings = rig.holds()
    print(f"held at {readings} kbit/s after the move")
    assert max(readings) <= HELD, (
        f"the upload went to {max(readings)} kbit/s once the reports came from {MOVED_TO}"
    )
    assert "went quiet" not in rig.journal(), "the house went quiet as its reporter moved"


def climbs(rig: Rig) -> None:
    """Wait until the upload climbs past what the house left, which only a hold let go reaches."""

    def climbed(_last_try: bool) -> bool:
        """Say whether the upload climbed past what the house left."""
        return rig.up() > CLIMBED

    retry(climbed, timeout=CLIMBING)


def test_a_report_forged_without_the_key_holds_nothing(rig: Rig) -> None:
    """In-step reports from the house's first address, under another key, hold nothing."""
    held(rig)
    quiet = rig.said("went quiet")
    rig.house.succeed("systemctl stop house-report")
    rig.client.wait_until_succeeds(
        f'test "$(journalctl -u shaper -o cat --no-pager | grep -c "went quiet")" -gt {quiet}',
        timeout=QUIETING,
    )
    holds, refused = rig.said("the house's ceiling holds"), rig.said(FORGED)
    # The counter climbing 3 Mbit/s, SENDS // 8 bytes a millisecond, under
    # another key (house-forger.sh).
    rig.house.succeed(
        f"systemd-run --unit=forger {BIN}/house-forger"
        f" {shlex.quote(WRONG_KEY)} {SENDS // 8} {FORGED_FROM} 192.168.1.1"
    )
    try:
        # Forgeries are no house's refusals, said apart from the house's own, so
        # a stamp of the house's refused while the clocks settle at boot leaves
        # their first to be said at once (house any round 4's F1, F6, F8 and F12).
        rig.client.wait_until_succeeds(
            f'test "$(journalctl -u shaper -o cat --no-pager | grep -c "{FORGED}")" -gt {refused}',
            timeout=QUIETING,
        )
        climbs(rig)
        journal = rig.journal()
    finally:
        rig.house.succeed("systemctl stop forger || true", "systemctl start house-report")
    assert journal.count("the house's ceiling holds") == holds, (
        "the house held again while only forged reports came"
    )
    refusals = journal.count(FORGED) - refused
    assert refusals <= REFUSALS_SAID, f"the journal said {refusals} refusals of the forgeries"


def swap_key(machine: BaseMachine) -> None:
    """Put the next key in place on a machine, by rename, as a key put there by hand is."""
    machine.succeed(
        f"install -m 0400 /dev/null {KEY_FILE}.next",
        f'printf %s "{NEXT_KEY}" > {KEY_FILE}.next',
        f"mv {KEY_FILE}.next {KEY_FILE}",
    )


def test_a_key_swapped_on_both_machines_is_taken_up(rig: Rig) -> None:
    """A key swapped on both restarts the reporter, whose reports then hold again."""
    held(rig)
    started = rig.house.succeed("systemctl show -p InvocationID --value house-report").strip()
    quiet = rig.said("went quiet")
    # The shaper reads its key again within a second, so its side first:
    # the reports still signed with the old key are refused as forged, and
    # the house goes quiet until its reporter signs with the next.
    swap_key(rig.client)
    rig.client.wait_until_succeeds(
        f'test "$(journalctl -u shaper -o cat --no-pager | grep -c "went quiet")" -gt {quiet}',
        timeout=QUIETING,
    )
    holds = rig.said("the house's ceiling holds")
    swap_key(rig.house)
    rig.house.wait_until_succeeds(
        f'test "$(systemctl show -p InvocationID --value house-report)" != "{started}"',
        timeout=QUIETING,
    )
    rig.client.wait_until_succeeds(
        'test "$(journalctl -u shaper -o cat --no-pager | grep -c "the house\'s ceiling holds")"'
        f" -gt {holds}",
        timeout=HOLDING,
    )
    held(rig)


def test_a_house_gone_quiet_lets_the_upload_go(rig: Rig) -> None:
    """Once the reports stop, the hold lets go, said, and the upload climbs past it."""
    # Run alone, it waits for the hold the other subtest proves.
    held(rig)
    quiet = rig.said("went quiet")
    rig.house.succeed("systemctl stop house-report")
    rig.client.wait_until_succeeds(
        f'test "$(journalctl -u shaper -o cat --no-pager | grep -c "went quiet")" -gt {quiet}',
        timeout=QUIETING,
    )
    climbs(rig)

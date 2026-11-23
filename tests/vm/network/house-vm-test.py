"""Whether a house's reports hold the client's upload under what the house leaves, and only then.

house.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. What the shaper heard of the house each second is
read from its own trace, and the rate it holds the upload to from its status.
"""

from __future__ import annotations

import datetime as dt
import json
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
# to its cable's, which the client's house names beside its first.
MOVED_TO = "192.168.1.12"
# How long the client may take to boot to its first queues, the shaper to
# hear the house and hold, the hold to let go once the reports stop (3 s
# fresh, then the journal's next step), and the upload to climb back.
COMING_UP = dt.timedelta(seconds=60)
HOLDING = dt.timedelta(seconds=60)
QUIETING = dt.timedelta(seconds=15)
CLIMBING = dt.timedelta(seconds=120)
# Where the house's other machine forges its reports from: the first
# address its house names, which it keeps once its route moves. Each says
# it sent nothing, which taken would let the hold go.
FORGED_FROM = "192.168.1.2"
WRONG_KEY = "not the house's key, but 32 b!!!"
# The most times the shaper may say forged reports held nothing while they
# come for under a minute: once, and once more if a minute's turn falls in.
REFUSALS_SAID = 2


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

    def heard(self) -> list[Json]:
        """Give each traced second of eth1 that heard the house, oldest first."""
        # Read as it stands, so a missing file can't bury the failure it explains.
        _, text = self.client.execute("cat /var/lib/shaper/heard/*.jsonl")
        seconds = []
        for line in text.splitlines():
            try:
                step: Json = json.loads(line)
            except json.JSONDecodeError:
                continue
            if step.get("link") == "eth1" and "house_most" in step:
                seconds.append(step)
        return seconds

    def journal(self) -> str:
        """Give what the shaper said."""
        return self.client.succeed("journalctl -u shaper -o cat --no-pager")

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
        last = (rig.heard() or [{}])[-1]
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
    """Once the reporter's route sends from another address the house names, it still holds."""
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
        f' | grep -q "the house\'s first report from {MOVED_TO} was heard"',
        timeout=QUIETING,
    )
    # The first address's last word goes stale, so a hold that stands
    # past it was heard from the new one.
    rig.client.succeed("sleep 5")
    readings = rig.holds()
    print(f"held at {readings} kbit/s after the move")
    assert max(readings) <= HELD, (
        f"the upload went to {max(readings)} kbit/s once the reports came from {MOVED_TO}"
    )
    assert "went quiet" not in rig.journal(), "the house went quiet as its reporter moved"


def test_a_report_forged_without_the_key_holds_nothing(rig: Rig) -> None:
    """Reports from the house's own address, unsigned or under another key, move no hold."""
    held(rig)
    # Ten a second for a while past the readings, each fresh, the house's
    # counter put at nothing, so taken they'd read as a house sending
    # nothing and let the hold go.
    forger = f"""
for i in $(seq 200); do
  ms=$(date +%s%3N)
  text="0 $ms $ms"
  mac=$(printf %s "$text" | openssl dgst -sha256 -hmac "{WRONG_KEY}" -r | cut -c1-32)
  for report in "0 $ms" "$text $mac"; do
    printf %s "$report" | socat -u - UDP:192.168.1.1:47110,bind={FORGED_FROM}
  done
  sleep 0.1
done
"""
    rig.house.succeed(f"cat > /root/forger <<'EOF'\n{forger}EOF")
    rig.house.succeed(f"systemd-run --unit=forger --setenv=PATH={BIN} {BIN}/bash /root/forger")
    rig.client.wait_until_succeeds(
        "journalctl -u shaper -o cat --no-pager"
        f" | grep -q 'a report from {FORGED_FROM} held nothing for the house'",
        timeout=QUIETING,
    )
    readings = rig.holds()
    print(f"held at {readings} kbit/s while reports were forged")
    journal = rig.journal()
    assert max(readings) <= HELD, (
        f"the upload went to {max(readings)} kbit/s while reports were forged: {readings}"
    )
    assert "went quiet" not in journal, "the house went quiet while its reports were forged"
    refusals = journal.count(f"a report from {FORGED_FROM} held nothing")
    assert refusals <= REFUSALS_SAID, f"the journal said {refusals} refusals in under a minute"
    rig.house.succeed("systemctl stop forger || true")


def test_a_house_gone_quiet_lets_the_upload_go(rig: Rig) -> None:
    """Once the reports stop, the hold lets go, said, and the upload climbs past it."""
    # Run alone, it waits for the hold the other subtest proves.
    held(rig)
    rig.house.succeed("systemctl stop house-report")
    rig.client.wait_until_succeeds(
        "journalctl -u shaper -o cat --no-pager | grep -q 'went quiet'", timeout=QUIETING
    )

    def climbed(_last_try: bool) -> bool:
        """Say whether the upload climbed past what the house left."""
        return rig.up() > CLIMBED

    retry(climbed, timeout=CLIMBING)

"""The Wi-Fi test's moves area.

Here the card moves between the access points of one network: on from one that doesn't answer, to
another when an account asks, keeping its neighbours, to another when its point warns it drops the
card, looking first for one it doesn't hear and ending a look under way, free of its point when
the whole network drops it, and free of both when that other doesn't answer, back to the one it
left when the one it was asked to has gone, and free to join any network when it can't get back
either; a network of another name forgets those neighbours.
tests/vm/network/wifi.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The tests run in the order written, each taking the
machine on from where the one before left it, so one run alone with -k starts from Lantern joined
and may not find what an earlier one set up. wifid as the tests reach it is wifi_session.py.
Long because its subtests share the one machine's two points and the card each leaves where the
next begins, which a split would only boot again.
"""

from __future__ import annotations

import json
import re
import time
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import retry
from wifi_session import Wifid, booted

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How many columns a line of wpa_cli's scan results has: its access point, frequency,
# level, flags and network.
SCAN_COLUMNS = 5

# A neighbour on Lantern nothing on the machine asks after, so it stays known only while the
# kernel keeps what it knew, and nothing learning it again hides that it was forgotten.
NEIGHBOUR = "10.42.0.99"
NEIGHBOUR_ADDRESS = "02:00:00:00:42:99"

# How long the line letting a stranded card go says it went back, in milliseconds.
WENT_BACK = re.compile(r"within ([0-9]+) ms, so may join any network")


# Lantern's access points, the first as loud as the second is quiet.
FIRST = "02:00:00:00:00:00"
SECOND = "02:00:00:00:05:00"
# When Lantern's second warns it drops a card, in milliseconds: its 50 beacons of 102.4 ms.
DROPS_IN_MS = 5120


def hears(machine: BaseMachine, wifid: Wifid, bssid: str) -> bool:
    """Have the cards look, and say whether they heard the access point."""
    wifid.ask("Scan")
    return bssid in machine.succeed("wpa_cli -i wlan2 scan_results")


def first_line(said: list[str], words: str, after: int = -1) -> int:
    """Give the place of the first line past `after` holding the words, failing with them."""
    found = next((i for i, line in enumerate(said) if i > after and words in line), None)
    assert found is not None, f"no line says {words!r}"
    return found


def knows_a_neighbour(machine: BaseMachine) -> None:
    """Have the Wi-Fi card know the neighbour, as it knows one it has asked after lately."""
    # The Wi-Fi card keeps what it knows through a moment without its carrier, and a cable
    # doesn't, since it's often plugged into another network. The test's own network and
    # QEMU's are both cables here.
    for setting in ("ipv4/conf/wlan2/arp", "ipv6/conf/wlan2/ndisc"):
        machine.succeed(f"grep -qx 0 /proc/sys/net/{setting}_evict_nocarrier")
    links = machine.succeed("networkctl list --no-legend")
    cables = [line.split()[1] for line in links.splitlines() if line.split()[2] == "ether"]
    assert cables, links
    for cable in cables:
        machine.succeed(f"grep -qx 1 /proc/sys/net/ipv4/conf/{cable}/arp_evict_nocarrier")
    machine.succeed(
        f"ip neighbour replace {NEIGHBOUR} lladdr {NEIGHBOUR_ADDRESS} dev wlan2 nud stale"
    )


def inn_is_on(machine: BaseMachine) -> bool:
    """Say whether the card's Roadside Inn network is on, as wpa_supplicant holds it."""
    networks = machine.succeed("wpa_cli -i wlan2 list_networks").splitlines()
    inns = [row.split("\t")[0] for row in networks if row.split("\t")[1:2] == ["Roadside Inn"]]
    assert len(inns) == 1, f"the Inn is saved once: {networks}"
    return machine.succeed(f"wpa_cli -i wlan2 get_network {inns[0]} disabled").strip() == "0"


def inn_turns_on(machine: BaseMachine) -> None:
    """Wait for the Inn to be on, as wifid says it turns networks on before it does."""
    retry(lambda _: inn_is_on(machine), timeout=timedelta(seconds=10))


def inn_stays_off(machine: BaseMachine) -> bool:
    """Say whether the Inn stays off for two seconds, longer than wifid takes to end a move.

    On the network Joel picked, wifid says nothing as a move ends, so no line marks the moment
    the Inn would be turned on.
    """
    for _ in range(10):
        if inn_is_on(machine):
            return False
        time.sleep(0.2)
    return True


def rejoined_unpicked(machine: BaseMachine) -> None:
    """Have the card come off Lantern and rejoin its first access point with every network on.

    A card joined from the launcher keeps its other networks off while it's on the network Joel
    picked, and is off that pick once it comes off its network. A point just woken can miss the
    look the card joins from, which then takes the Inn and stays, so it's asked again until it
    joins the first point.
    """
    lantern = machine.succeed(
        "wpa_cli -i wlan2 list_networks | awk -F'\\t' '$2 == \"Lantern\" {print $1}'"
    ).strip()
    ranked = machine.succeed(f"wpa_cli -i wlan2 get_network {lantern} priority").strip()

    def rejoined(_: bool) -> bool:
        """Have the card leave and join again, and say whether it joined the first point."""
        machine.succeed(
            "wpa_cli -i wlan2 disconnect"
            " && wpa_cli -i wlan2 enable_network all"
            f" && wpa_cli -i wlan2 set_network {lantern} priority 100"
        )
        machine.wait_until_succeeds(
            "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED",
            timeout=timedelta(seconds=30),
        )
        machine.succeed("wpa_cli -i wlan2 reconnect")
        status, _said = machine.execute(
            "timeout 10 sh -c 'until s=$(wpa_cli -i wlan2 status)"
            ' && echo "$s" | grep -qx wpa_state=COMPLETED'
            ' && echo "$s" | grep -qx bssid=02:00:00:00:00:00; do sleep 0.2; done\''
        )
        if status != 0:
            # wpa_supplicant can keep a look it began as pending for good, after a point
            # stopped under it, refusing every look after ("Reject scan trigger since one is
            # already pending"), so the card can't find the first point again. Taking the
            # link down drops its queued radio work (events.c, EVENT_INTERFACE_DISABLED),
            # and the next try starts clean.
            machine.succeed("ip link set wlan2 down && ip link set wlan2 up")
        return status == 0

    retry(rejoined, timeout=timedelta(seconds=60))
    machine.succeed(f"wpa_cli -i wlan2 set_network {lantern} priority {ranked}")


@pytest.fixture(scope="module")
def wifid(machine: BaseMachine) -> Wifid:
    """Give wifid with the card on Lantern, joined as the shell joins it."""
    wifid = booted(machine)
    wifid.joined("Roadside Inn")
    retry(lambda _: wifid.hears("Lantern"), timeout=timedelta(seconds=60))
    status, _, said = wifid.joining({"name": "Lantern", "password": "glowing in the dark"})
    assert status == 0, said
    wifid.joined("Lantern")
    wifid.holds_address("10.42.0.")
    return wifid


# On a laptop at boot, the access point first picked ignored the first
# join, while another of the same network answered. Trying the silent
# one again failed too, and a full look came before the other was
# tried, six seconds there. Here Lantern has a second, weaker access
# point, so the card picks the first, which stops answering; the card
# goes to the second from its last look, without looking again.
def test_a_card_whose_access_point_doesnt_answer_moves_to_another_of_its_network_at_once(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card whose access point doesn't answer moves to another of its network at once."""
    lantern = machine.succeed(
        "wpa_cli -i wlan2 list_networks | awk -F'\\t' '$2 == \"Lantern\" {print $1}'"
    ).strip()
    ranked = machine.succeed(f"wpa_cli -i wlan2 get_network {lantern} priority").strip()
    machine.succeed("systemctl start lantern-two")
    machine.succeed(
        "wpa_cli -i wlan2 disconnect"
        " && wpa_cli -i wlan2 enable_network all"
        f" && wpa_cli -i wlan2 set_network {lantern} priority 100"
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=timedelta(seconds=30)
    )

    def hears_both(_: bool) -> bool:
        """Say whether the last look heard both of Lantern's, the first louder."""
        if not wifid.hears("Lantern"):
            return False
        levels: dict[str, int] = {}
        for line in machine.succeed("wpa_cli -i wlan2 scan_results").splitlines():
            fields = line.split("\t")
            if len(fields) == SCAN_COLUMNS and fields[4] == "Lantern":
                levels[fields[0]] = int(fields[2])
        first, second = levels.get("02:00:00:00:00:00"), levels.get("02:00:00:00:05:00")
        return first is not None and second is not None and first > second

    retry(hears_both, timeout=timedelta(seconds=60))
    since = wifid.journal_now()
    machine.succeed("systemctl stop lantern && wpa_cli -i wlan2 reconnect")
    machine.wait_until_succeeds(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | grep -q 'wlan2: authentication with 02:00:00:00:00:00 timed out'",
        timeout=timedelta(seconds=30),
    )
    machine.wait_until_succeeds(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:05:00',
        timeout=timedelta(seconds=30),
    )
    logged = machine.succeed(
        f"journalctl -o cat --after-cursor='{since}'"
        " _TRANSPORT=kernel + _PID=$(systemctl show -P MainPID wifid)"
    ).splitlines()
    failed = next(
        i
        for i, line in enumerate(logged)
        if "authentication with 02:00:00:00:00:00 timed out" in line
    )
    completed = next(
        i for i, line in enumerate(logged) if i > failed and line == "wlan2 is completed"
    )
    assert "wlan2 is scanning" not in logged[failed:completed], "\n".join(logged[failed:])
    machine.succeed(
        f"wpa_cli -i wlan2 set_network {lantern} priority {ranked}"
        " && systemctl stop lantern-two && systemctl start lantern"
    )


# Comparing one access point's air with another's needs the card moved
# to each in turn, and only root and wpa_supplicant's group reach its
# bus, so an account asks wifid on the launcher's rows socket
# (2026-11-14). Lantern's second access point is the weaker, which the
# card wouldn't choose itself.
def test_a_card_moves_to_another_access_point_of_its_network_when_asked(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card moves to another access point of its network when asked."""
    machine.succeed("systemctl start lantern-two")
    status, _, said = wifid.joining({"name": "Lantern"})
    assert status == 0, said
    wifid.joined("Lantern")

    retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
    machine.succeed("wpa_cli -i wlan2 status | grep -qx bssid=02:00:00:00:00:00")
    wifid.holds_address("10.42.0.")
    knows_a_neighbour(machine)
    assert not inn_is_on(machine), "joined as the launcher joins, the Inn is off"
    since = wifid.journal_now()
    answer = wifid.acting({"act": "roam", "bssid": "02:00:00:00:05:00"})
    assert answer == {"id": 1, "done": True}, answer
    machine.wait_until_succeeds(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:05:00',
        timeout=timedelta(seconds=30),
    )
    # On the network Joel picked, the card's other networks stay off once a move ends.
    machine.wait_until_succeeds(
        f"journalctl -o cat -u wifid --after-cursor='{since}'"
        " | grep -qE '^wlan2 reached 02:00:00:00:05:00 [0-9]+ ms after it was asked'",
        timeout=timedelta(seconds=30),
    )
    assert inn_stays_off(machine), "still on the network Joel picked, the Inn stays off"
    # The card still knows the neighbour after the move, so it asks no one
    # where the router went, and the router answers through the second.
    still = machine.succeed(f"ip -4 neighbour show {NEIGHBOUR} dev wlan2")
    assert NEIGHBOUR_ADDRESS in still, still
    machine.succeed("ping -c 1 -W 2 10.42.0.1")
    unheard = wifid.acting({"act": "roam", "bssid": "02:00:00:00:09:00"})
    assert unheard["error"]["refused"] == "NotNearby", unheard
    machine.succeed("systemctl stop lantern-two")
    machine.wait_until_succeeds(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:00:00',
        timeout=timedelta(seconds=60),
    )


# The eero asked the laptop to move 27 times from 15 to 20 Nov 2026, and wpa_supplicant said what
# it asked only in its debug log. None warned it would drop the card, which wpa_supplicant can't
# follow for a card wifid holds to a point. Lantern's second asks here, through its own socket.
def held_to(machine: BaseMachine, wifid: Wifid, bssid: str) -> bool:
    """Ask wifid to move the card to the point, and say whether it's held there."""
    # On 23 Nov 2026 the second refused the card's first move (status 53), for a key the card
    # kept from the subtest before, and wpa_supplicant joined it anyway as wifid sent the card
    # back and then let it go. Asked again once there, wifid only holds the card where it is.
    since = wifid.journal_now()
    if wifid.acting({"act": "roam", "bssid": bssid}).get("done") is not True:
        return False
    # A card that doesn't get there is asked again by the caller's retry, rather than failing
    # the subtest from inside it.
    status, _ = machine.execute(
        "timeout 30 sh -c 'until s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        f' && echo "$s" | grep -qx bssid={bssid}; do sleep 0.2; done\''
    )
    if status != 0:
        return False
    said = machine.succeed(f"journalctl -o cat -u wifid --after-cursor='{since}'")
    return "goes back to" not in said and "may join any access point" not in said


def held_nowhere(machine: BaseMachine) -> bool:
    """Say whether Lantern joins at any of its points, as wpa_supplicant holds it."""
    lantern = machine.succeed(
        "wpa_cli -i wlan2 list_networks | awk -F'\\t' '$2 == \"Lantern\" {print $1}'"
    ).strip()
    # wpa_supplicant answers FAIL for a network's address once it's held to none.
    return machine.succeed(f"wpa_cli -i wlan2 get_network {lantern} bssid").strip() == "FAIL"


def warning(machine: BaseMachine, wifid: Wifid) -> str:
    """Hold the card to Lantern's second, and give the ask that warns it drops the card there.

    The ask names Lantern's first on channel 6, op class 81, which is 2437 MHz.
    """
    retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
    retry(lambda _: held_to(machine, wifid, SECOND), timeout=timedelta(seconds=60))
    station = machine.succeed("wpa_cli -i wlan2 status | sed -n 's/^address=//p'").strip()
    return (
        f"hostapd_cli -p /run/hostapd-lantern-two -i wlan5 bss_tm_req {station}"
        f" pref=1 valid_int=200 neighbor={FIRST},0x0000,81,6,7,0301ff"
        " disassoc_imminent=1 disassoc_timer=50"
    )


def reached_in_time(machine: BaseMachine, since: str, *said: str) -> None:
    """Check wifid said each line in order and the card reached the first before its drop."""
    reached = f"^wlan2 reached {FIRST} [0-9]+ ms after it was asked"
    machine.wait_until_succeeds(
        f"journalctl -o cat -u wifid --after-cursor='{since}' | grep -qE '{reached}'",
        timeout=timedelta(seconds=10),
    )
    timed = said_since(machine, since)
    lines = [line for _, line in timed]
    place = -1
    for words in said:
        place = first_line(lines, words, place)
    heard = next((at for at, line in timed if "warning it drops the card" in line), None)
    landed = next((at for at, line in timed if line.startswith(f"wlan2 reached {FIRST} ")), None)
    assert heard is not None, f"wifid never said the warning in:\n{lines}"
    assert landed is not None, f"wifid never said the card reached {FIRST} in:\n{lines}"
    took = (landed - heard) // 1000
    assert took < DROPS_IN_MS, f"the card took {took} ms, past the {DROPS_IN_MS} ms warned of"
    machine.succeed(f"wpa_cli -i wlan2 status | grep -qx bssid={FIRST}")


def said_since(machine: BaseMachine, since: str) -> list[tuple[int, str]]:
    """Give each line wifid said since the cursor, with when, in microseconds since boot."""
    lines = machine.succeed(f"journalctl -o json -u wifid --after-cursor='{since}'")
    entries = [json.loads(line) for line in lines.splitlines() if line.strip()]
    return [(int(entry["__MONOTONIC_TIMESTAMP"]), str(entry["MESSAGE"])) for entry in entries]


def test_a_held_card_says_its_points_asks_and_moves_when_one_warns_it_drops_it(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A held card says its point's asks to move, and moves when one warns it drops the card."""
    machine.succeed("systemctl start lantern-two")
    # Stopped whatever fails, so the subtests after this one aren't left a second point, and
    # the card held to it.
    try:
        retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
        retry(lambda _: held_to(machine, wifid, SECOND), timeout=timedelta(seconds=60))
        station = machine.succeed("wpa_cli -i wlan2 status | sed -n 's/^address=//p'").strip()
        ask = (
            f"hostapd_cli -p /run/hostapd-lantern-two -i wlan5 bss_tm_req {station}"
            f" pref=1 valid_int=200 neighbor={FIRST},0x0000,81,6,7,0301ff"
        )
        since = wifid.journal_now()
        machine.succeed(f"{ask} | grep -qx OK")
        asked = f"wlan2 was asked by {SECOND} to move to {FIRST} at 255, for 20480 ms"
        plain = f"{asked}, warning nothing"
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}' | grep -qxF '{plain}'",
            timeout=timedelta(seconds=10),
        )
        # wifid says what it makes of the ask once it has decided, so a move for a plain ask
        # would be said by then.
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qxF 'wlan2 stays where it is, as the ask warns nothing'",
            timeout=timedelta(seconds=10),
        )
        said = machine.succeed(f"journalctl -o cat -u wifid --after-cursor='{since}'")
        assert "before it's dropped" not in said, f"wifid moved the card for a plain ask:\n{said}"
        # Whether wpa_supplicant left the point on its own is read by the warned ask below:
        # the second answers OK only for a card still joined to it, and a status read here
        # could come before a move wpa_supplicant began after the signal.
        since = wifid.journal_now()
        machine.succeed(f"{ask} disassoc_imminent=1 disassoc_timer=50 | grep -qx OK")
        warned = f"{asked}, warning it drops the card in {DROPS_IN_MS} ms"
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            f" | grep -qxF '{warned}'"
            f" && journalctl -o cat -u wifid --after-cursor='{since}'"
            f" | grep -qE '^wlan2 reached {FIRST} [0-9]+ ms after it was asked'",
            timeout=timedelta(seconds=10),
        )
        # Gone before the time it was warned of, the card was never the point's to drop. Timed
        # from the warning as wifid read it, so a move asked late counts its wait.
        timed = said_since(machine, since)
        heard = next((at for at, line in timed if line == warned), None)
        reached = next(
            (at for at, line in timed if line.startswith(f"wlan2 reached {FIRST} ")), None
        )
        assert heard is not None, f"wifid never said the warning in:\n{timed}"
        assert reached is not None, f"wifid never said the card reached {FIRST} in:\n{timed}"
        took = (reached - heard) // 1000
        assert took < DROPS_IN_MS, f"the card took {took} ms, past the {DROPS_IN_MS} ms warned of"
        machine.succeed(f"wpa_cli -i wlan2 status | grep -qx bssid={FIRST}")
    finally:
        machine.succeed("systemctl stop lantern-two")


# wpa_supplicant forgets a point three minutes after it last heard it, and on a strong link the
# laptop looks once an hour, so most warnings name points the card doesn't hear. It can't move to
# one it hasn't heard, so it looks over the channels the warning names, a fraction of a second
# where a look over every channel takes 5 to 6.
def test_a_warned_card_that_hears_none_of_the_points_named_looks_on_their_channels(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A warned card that hears none of the points named looks over their channels, and moves."""
    machine.succeed("systemctl start lantern-two")
    since = wifid.journal_now()
    try:
        ask = warning(machine, wifid)
        # Forgets every point but the one the card is on, and says so to wifid ahead of the
        # warning, on the same bus.
        machine.succeed("wpa_cli -i wlan2 bss_flush 0 | grep -qx OK")
        assert FIRST not in machine.succeed("wpa_cli -i wlan2 scan_results"), "still heard"
        since = wifid.journal_now()
        machine.succeed(f"{ask} | grep -qx OK")
        reached_in_time(
            machine,
            since,
            "warning it drops the card in 5120 ms",
            "wlan2 hears none of the points asked, so looks over 2437 MHz",
            f"wlan2 moves to {FIRST} before it's dropped",
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        machine.succeed("systemctl stop lantern-two")


# A look over every channel takes 5 to 6 s, as long as a warning gives, so a warned move ends a
# look under way rather than wait for it.
def test_a_warned_card_stops_looking_to_move_before_it_s_dropped(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card warned while it looks over every channel stops looking, and moves in time."""
    machine.succeed("systemctl start lantern-two")
    since = wifid.journal_now()
    try:
        ask = warning(machine, wifid)
        since = wifid.journal_now()
        machine.succeed(f"wpa_cli -i wlan2 scan | grep -qx OK && {ask} | grep -qx OK")
        reached_in_time(
            machine,
            since,
            "warning it drops the card in 5120 ms",
            "wlan2 stops looking to move before it's dropped",
            f"wlan2 moves to {FIRST} before it's dropped",
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        machine.succeed("systemctl stop lantern-two")


# A look under way may be over other channels than a warning names, so a card hearing none of them
# stops it and looks over theirs once it's said to end. What the stopped look heard before it ended
# may hold the first already, so the card's own look over 2437 MHz is left unchecked: the move
# comes only once the stopped look's end is heard.
def test_a_warned_card_hearing_none_named_stops_a_look_under_way_to_look_for_them(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A warned card hearing none named, mid-look, stops that look, then finds them and moves."""
    machine.succeed("systemctl start lantern-two")
    since = wifid.journal_now()
    try:
        ask = warning(machine, wifid)
        machine.succeed("wpa_cli -i wlan2 bss_flush 0 | grep -qx OK")
        assert FIRST not in machine.succeed("wpa_cli -i wlan2 scan_results"), "still heard"
        since = wifid.journal_now()
        machine.succeed(f"wpa_cli -i wlan2 scan | grep -qx OK && {ask} | grep -qx OK")
        reached_in_time(
            machine,
            since,
            "warning it drops the card in 5120 ms",
            "wlan2 stops looking to look over 2437 MHz",
            f"wlan2 moves to {FIRST} before it's dropped",
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        machine.succeed("systemctl stop lantern-two")


# An access point that says its whole network drops the card names no point to go to, so the card
# is let join any point, and any network, rather than held to one that drops it.
def test_a_card_whose_whole_network_drops_it_may_join_any_point_again(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card warned its whole network drops it is let go of its point."""
    machine.succeed("systemctl start lantern-two")
    since = wifid.journal_now()
    try:
        warning(machine, wifid)
        station = machine.succeed("wpa_cli -i wlan2 status | sed -n 's/^address=//p'").strip()
        since = wifid.journal_now()
        machine.succeed(
            "hostapd_cli -p /run/hostapd-lantern-two -i wlan5"
            f" ess_disassoc {station} 50 https://api.lantern/ | grep -qx OK"
        )
        said = (
            f"wlan2 was asked by {SECOND} to move, naming no point,"
            f" warning its whole network drops the card in {DROPS_IN_MS} ms"
        )
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}' | grep -qxF '{said}'"
            f" && journalctl -o cat -u wifid --after-cursor='{since}'"
            ' | grep -qxF "wlan2\'s whole network drops it, so it may join any point again"',
            timeout=timedelta(seconds=10),
        )
        # The line is said before the hold is let go, so the hold is read until it goes.
        retry(lambda _: held_nowhere(machine), timeout=timedelta(seconds=10))
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        machine.succeed("systemctl stop lantern-two")


def test_a_warned_card_whose_way_out_doesnt_answer_is_let_go_rather_than_sent_back(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A warned card whose way out doesn't answer is let go, never sent back to its point."""
    machine.succeed("systemctl start lantern-two")
    since = wifid.journal_now()
    try:
        retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
        retry(lambda _: held_to(machine, wifid, SECOND), timeout=timedelta(seconds=60))
        station = machine.succeed("wpa_cli -i wlan2 status | sed -n 's/^address=//p'").strip()
        # Stopped, hostapd answers no join, while the kernel still beacons for it, so the card
        # still hears the first and its move there falls short.
        machine.succeed("systemctl kill -s STOP lantern")
        machine.succeed(
            f"hostapd_cli -p /run/hostapd-lantern-two -i wlan5 bss_tm_req {station}"
            f" pref=1 valid_int=200 neighbor={FIRST},0x0000,81,6,7,0301ff"
            " disassoc_imminent=1 disassoc_timer=50 | grep -qx OK"
        )
        let_go = f", and {SECOND} drops it, so it may join any point again"
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}' | grep -qF '{let_go}'",
            timeout=timedelta(seconds=30),
        )
        said = machine.succeed(f"journalctl -o cat -u wifid --after-cursor='{since}'")
        sent_back = f"so goes back to {SECOND}"
        assert sent_back not in said, f"sent back to the point that drops it:\n{said}"
        # The line is said before the hold is let go, so the hold is read as well, while the
        # first is still frozen: a card left held to it would be stranded off its network.
        retry(lambda _: held_nowhere(machine), timeout=timedelta(seconds=10))
    finally:
        try:
            # Logged first, so a clean-up that fails can't hide why.
            machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        finally:
            # Lantern goes on, and the second off, whatever fails, so the subtests after this
            # one aren't left a frozen point or a second. wpa_supplicant ignores the first a
            # while for the joins it refused, which would keep the card off it below.
            # Each is run whether or not the one before it fails, and all are judged together.
            cleaned = {
                step: machine.execute(step)[0]
                for step in (
                    "systemctl kill -s CONT lantern",
                    "wpa_cli -i wlan2 bssid_ignore clear | grep -qx OK",
                    "systemctl stop lantern-two",
                )
            }
            failed = [step for step, status in cleaned.items() if status != 0]
            assert not failed, f"the clean-up failed at: {failed}"


# On 16 Nov 2026 a move found its access point gone, and the card scanned
# 18 s on no network before it landed anywhere.
def test_a_move_to_an_access_point_gone_since_it_was_heard_goes_back_to_the_one_it_left(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A move to an access point gone since it was heard goes back to the one it left."""
    # Off the network Joel picked, the card has the Inn on, and this move is the one to turn it
    # off and on again. Rejoined here, the subtest runs alone as well as after the one before.
    rejoined_unpicked(machine)
    machine.succeed("systemctl start lantern-two")

    retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
    assert inn_is_on(machine), "the Inn is on before the move"
    # wpa_supplicant keeps an access point through two looks that don't hear it, so the move
    # still finds the second after its own look over the channel, and is asked into nothing.
    # The card gives up on it within tens of milliseconds, and wpa_supplicant was on its way
    # to the Roadside Inn before wifid heard it gave up (2026-11-20), so the card moves with
    # its other networks off, and is back within wifid's 2 s, never on the Inn. A card that
    # left the first for the second without a word finds the first still holding it as
    # joined, and bidding it come back once it has asked whether the card is still there,
    # which the 2 s leave room for.
    machine.succeed("systemctl stop lantern-two")
    since = wifid.journal_now()
    answer = wifid.acting({"act": "roam", "bssid": "02:00:00:00:05:00"})
    assert answer == {"id": 1, "done": True}, answer
    # wifid answers once it has turned the Inn off and asked the move, and says it turns the
    # networks on again before it does, so an Inn found on before that line is one the move
    # never turned off. The journal is read after the Inn, so a way back ending between the
    # two reads passes rather than racing.
    inn_on = inn_is_on(machine)
    on_again = machine.succeed(
        f"journalctl -o cat -u wifid --after-cursor='{since}'"
        " | grep -x 'wlan2.s other networks are on again' || true"
    )
    assert not inn_on or on_again, "the Inn is off while the card moves"
    # The way back is timed by its journal, kept whether it comes or not. A card sent back only
    # once its time is up is waited for too, so it fails on its time below, not as a timeout.
    try:
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 (came off its network short of|didn.t reach)"
            " 02:00:00:00:05:00.*, so goes back to 02:00:00:00:00:00$'",
            timeout=timedelta(seconds=30),
        )
        # Back past its 2 s, wifid lets the card go, so being back unstranded is its bound
        # here, and whichever comes first is waited for so a stranded card fails as one. A
        # card wifid finds back only by asking as its 2 s end fails too: its arrival went
        # unheard. In five runs on the builders (2026-11-21) the card was back 1.17 to 1.22 s
        # after it was sent, 1.024 s of it the point's bidden wait, so the bound leaves 0.8 s.
        # One ended on a point wifid can't name fails too, in its own words: hwsim's are named.
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 (reached 02:00:00:00:00:00 [0-9]+ ms after it was asked"
            "|was found at 02:00:00:00:00:00 as its [0-9]+ ms ran out"
            "|(joined an access point|was found at a point) wifid can.t name"
            "|didn.t get back to 02:00:00:00:00:00 within [0-9]+ ms, so may join any network$)'",
            timeout=timedelta(seconds=30),
        )
        behind = machine.succeed(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -E 'so may join any network|as its [0-9]+ ms ran out"
            "|(joined an access point|was found at a point) wifid can.t name' || true"
        )
        # Whether the point bade the card wait to come back, as a WPA3 one does for 1024 ms,
        # says whether a late way back is the point's or wifid's.
        bade = machine.succeed(
            f"journalctl -o cat -k --after-cursor='{since}' | grep -i comeback || true"
        )
        assert not behind, f"not back in time: {behind}; the point bade: {bade or 'no wait'}"
        # Moved with its other networks off, it has them on again once back.
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qx 'wlan2.s other networks are on again'",
            timeout=timedelta(seconds=30),
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
    said = machine.succeed(
        f"journalctl -o short-unix -u wifid -u wpa_supplicant --after-cursor='{since}'"
    ).splitlines()
    moved = said[first_line(said, "wlan2 moved to 02:00:00:00:05:00, as asked")]
    sent = said[first_line(said, ", so goes back to 02:00:00:00:00:00")]
    late = "didn't reach" in sent
    # wpa_supplicant's own line times the way back for the log alone, so its wording failing
    # to match fails nothing: wifid's line above already says the card got back.
    back = next(
        (
            line
            for line in said
            if "wlan2: CTRL-EVENT-CONNECTED - Connection to 02:00:00:00:00:00" in line
        ),
        None,
    )
    # The gone point refuses the card's join within its first second, so it comes off its
    # network and goes back at once, not once its move's time is up: about 0.1 s after the move
    # in the runs so far. Only the path is judged, the figure said beside it.
    off = float(sent.split()[0]) - float(moved.split()[0])
    assert not late, f"sent back only once its time was up, {off:.3f} s after the move"
    bidden = machine.succeed(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | sed -n 's/^wlan2: .* rejected association temporarily;.* (\\([0-9]*\\) ms)$/\\1/p'"
    ).split()
    # wifid gives a way back 2 s, room for one such wait and the join after it; how often the
    # point bade one is the point's and hwsim's doing, so it's logged, not judged.
    waited = "+".join(bidden) or "no"
    # The plan's bar is back within a second past any wait, judged on the laptop; a guest's
    # clock stretches on a loaded builder, so here the card is held to wifid's own time for a
    # way back, and the figure is logged against the bar.
    if back is None:
        machine.log(
            f"back at a time wpa_supplicant didn't say, bidden wait {waited} ms, bar 1 s past it"
        )
    else:
        took = float(back.split()[0]) - float(moved.split()[0])
        returned = float(back.split()[0]) - float(sent.split()[0])
        machine.log(
            f"back {took:.3f} s after the move was asked and {returned:.3f} s after it was sent,"
            f" bidden wait {waited} ms, bar 1 s past it"
        )
    # Turned off, as wifid says it couldn't otherwise or that it had no point to go back to,
    # where it turns nothing off, the Inn is never tried.
    stayed_on = [
        line
        for line in said
        if re.search(r"wlan2 moves with .* on", line) or "no point known to go back to" in line
    ]
    assert not stayed_on, f"moved with its other networks on: {stayed_on}"
    tried = [line for line in said if "SSID='Roadside Inn'" in line]
    assert not tried, f"tried the Inn on the way back: {tried}"
    inn_turns_on(machine)
    machine.succeed(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:00:00'
    )
    wifid.holds_address("10.42.0.")


def test_a_card_that_cant_get_back_either_may_join_any_network(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card that can't get back to the point it left either may join any network."""
    rejoined_unpicked(machine)
    machine.succeed("systemctl start lantern-two")

    retry(lambda _: hears(machine, wifid, SECOND), timeout=timedelta(seconds=60))
    # Stopped, hostapd answers no join, while the kernel still beacons for it and keeps the
    # card joined, so the card leaves the first point for a gone second and can't get back.
    since = wifid.journal_now()
    try:
        machine.succeed("systemctl stop lantern-two && systemctl kill -s STOP lantern")
        answer = wifid.acting({"act": "roam", "bssid": "02:00:00:00:05:00"})
        assert answer == {"id": 1, "done": True}, answer
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 didn.t get back to 02:00:00:00:00:00 within [0-9]+ ms,"
            " so may join any network$'",
            timeout=timedelta(seconds=30),
        )
        # Let go with its other networks on, it isn't left held to a point that won't answer.
        wifid.joined("Roadside Inn")
    finally:
        try:
            # Logged first, so a clean-up that fails can't hide why.
            machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        finally:
            # Lantern goes on whether or not the log could be read, so the subtests after
            # this one aren't left with a frozen point. wpa_supplicant ignores the first point a
            # while for the joins it refused, which would keep the card off it below.
            machine.succeed("systemctl kill -s CONT lantern && wpa_cli -i wlan2 bssid_ignore clear")
    said = machine.succeed(
        f"journalctl -o short-unix -u wifid -u wpa_supplicant --after-cursor='{since}'"
    ).splitlines()
    sent = first_line(said, ", so goes back to 02:00:00:00:00:00")
    let_go = first_line(said, "so may join any network")
    # Held to its way back for wifid's 2 s, it's let go then, and never tries the Inn
    # meanwhile. When its 2 s are up is arrive.rs's to prove; how late the guest woke wifid
    # past them is only logged, as a loaded builder decides it. The hold lifted only after the
    # line letting it go shows it wasn't let go before.
    went = WENT_BACK.search(said[let_go])
    assert went, f"the line letting it go says no time: {said[let_go]}"
    machine.log(f"let go {int(went.group(1))} ms after it was sent back")
    unheld = first_line(said, "wlan2 may join any access point of its network again", sent)
    assert unheld > let_go, f"its hold let go before its 2 s were up: {said[unheld]}"
    # wifid's lines come down a stream and wpa_supplicant's through syslog, each stamped as
    # journald reads it, which a loaded builder does late and in either order. So the Inn's
    # tries are judged by their stamps, with 0.5 s for that: one tried while it was held would
    # be up to 2 s early.
    let_go_at = float(said[let_go].split()[0])
    inn = [line for line in said if "SSID='Roadside Inn'" in line]
    tried = [line for line in inn if float(line.split()[0]) < let_go_at - 0.5]
    assert not tried, f"tried the Inn before it was let go at {let_go_at}: {tried}"
    # The Inn's tries are found by these words, as they are once it's let go.
    assert inn, "the Inn's tries are said as searched for"
    machine.log(f"the Inn first tried {float(inn[0].split()[0]) - let_go_at:.3f} s after")
    # Let go while elsewhere, a stranded card isn't pulled off the network it takes. wifid
    # marks a card to look again only when wpa_supplicant is already joining elsewhere as its
    # hold is lifted, so this catches that coming back only in the runs where the Inn's join
    # has begun by then; main.rs's decisions want unit cases for the rest.
    looked = [line for line in said if "looks again, having taken another network" in line]
    assert not looked, f"looked again off the Inn: {looked}"


def test_joining_a_network_of_another_name_forgets_the_neighbours_of_the_last(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Joining a network of another name forgets the neighbours of the last.

    A neighbour kept from Lantern could carry the Inn's traffic to a router that isn't there,
    were the two to give their routers one address, as many networks do.
    """
    # Set up here rather than by the subtest before, so one that fails there, leaving the card
    # on the Inn, doesn't fail this one too. The neighbour is known only once Lantern's address
    # is held, so the Inn's address going can't forget it before the join it's judged by.
    rejoined_unpicked(machine)
    wifid.holds_address("10.42.0.")
    knows_a_neighbour(machine)
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")
    gone = machine.succeed(f"ip -4 neighbour show {NEIGHBOUR} dev wlan2")
    assert NEIGHBOUR_ADDRESS not in gone, gone

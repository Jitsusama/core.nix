"""The Wi-Fi test's moves area.

Here the card moves between the access points of one network: on from one that doesn't answer, to
another when an account asks, keeping its neighbours, back to the one it left when the one it was
asked to has gone, and free to join any network when it can't get back either; a network of
another name forgets those neighbours.
tests/vm/network/wifi.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The tests run in the order written, each taking the
machine on from where the one before left it, so one run alone with -k starts from Lantern joined
and may not find what an earlier one set up. wifid as the tests reach it is wifi_session.py.
"""

from __future__ import annotations

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

    def hears_the_second(_: bool) -> bool:
        """Have the cards look, and say whether they heard Lantern's second."""
        wifid.ask("Scan")
        return "02:00:00:00:05:00" in machine.succeed("wpa_cli -i wlan2 scan_results")

    retry(hears_the_second, timeout=timedelta(seconds=60))
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

    def hears_the_second(_: bool) -> bool:
        """Have the cards look, and say whether they heard Lantern's second."""
        wifid.ask("Scan")
        return "02:00:00:00:05:00" in machine.succeed("wpa_cli -i wlan2 scan_results")

    retry(hears_the_second, timeout=timedelta(seconds=60))
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
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 (reached 02:00:00:00:00:00 [0-9]+ ms after it was asked"
            "|was found at 02:00:00:00:00:00 as its [0-9]+ ms ran out"
            "|didn.t get back to 02:00:00:00:00:00 within [0-9]+ ms, so may join any network$)'",
            timeout=timedelta(seconds=30),
        )
        behind = machine.succeed(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -E 'so may join any network|as its [0-9]+ ms ran out' || true"
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

    def hears_the_second(_: bool) -> bool:
        """Have the cards look, and say whether they heard Lantern's second."""
        wifid.ask("Scan")
        return "02:00:00:00:05:00" in machine.succeed("wpa_cli -i wlan2 scan_results")

    retry(hears_the_second, timeout=timedelta(seconds=60))
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
    rejoined_unpicked(machine)


def test_joining_a_network_of_another_name_forgets_the_neighbours_of_the_last(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Joining a network of another name forgets the neighbours of the last.

    A neighbour kept from Lantern could carry the Inn's traffic to a router that isn't there,
    were the two to give their routers one address, as many networks do.
    """
    knows_a_neighbour(machine)
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")
    gone = machine.succeed(f"ip -4 neighbour show {NEIGHBOUR} dev wlan2")
    assert NEIGHBOUR_ADDRESS not in gone, gone

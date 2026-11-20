"""The Wi-Fi test's moves area.

Here the card moves between the access points of one network: on from one that doesn't answer, to
another when an account asks, keeping its neighbours, to another at once when its echoes are lost
during a call, and back to the one it left when the one it was asked to has gone; a network of
another name forgets those neighbours.
tests/vm/network/wifi.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The tests run in the order written, each taking the
machine on from where the one before left it, so one run alone with -k starts from Lantern joined
and may not find what an earlier one set up. wifid as the tests reach it is wifi_session.py.
"""

from __future__ import annotations

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

# How near, in dB, Lantern's two are heard when set alike, as hwsim gives the same air.
ALIKE = 3


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
    answer = wifid.acting({"act": "roam", "bssid": "02:00:00:00:05:00"})
    assert answer == {"id": 1, "done": True}, answer
    machine.wait_until_succeeds(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:05:00',
        timeout=timedelta(seconds=30),
    )
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


# On 17 Nov 2026 a call's link faded for four minutes before the card
# moved, once a minute had lost over a quarter of its echoes. During a
# call wifid now gives way as soon as ten seconds lose two echoes.
def test_a_card_losing_its_echoes_during_a_call_gives_way_to_another_at_once(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card losing its echoes during a call gives way to another of its network at once."""
    machine.succeed("systemctl start lantern-two")
    # Both heard alike, since a lossy link gives way to none heard over 6 dB fainter.
    for radio in ("wlan0", "wlan5"):
        machine.succeed(f"ip netns exec air iw dev {radio} set txpower fixed 2000")

    def hears_both_alike(_: bool) -> bool:
        """Have the cards look, and say whether the look heard the second as loud."""
        # The second is kept from the subtest before, heard then at -45 dBm, so only a look
        # since its power went up says it's near enough: on 20 Nov 2026 the card gave way
        # only once a look that ended 16 s into the loss heard it at -30. What the last look
        # heard is read before another is asked, so none is under way as the call begins:
        # one begun then ran 19 s as the link lost every frame.
        levels: dict[str, int] = {}
        for line in machine.succeed("wpa_cli -i wlan2 scan_results").splitlines():
            fields = line.split("\t")
            if len(fields) == SCAN_COLUMNS and fields[4] == "Lantern":
                levels[fields[0]] = int(fields[2])
        first, second = levels.get("02:00:00:00:00:00"), levels.get("02:00:00:00:05:00")
        alike = first is not None and second is not None and abs(first - second) <= ALIKE
        if not alike:
            wifid.ask("Scan")
        return alike

    retry(hears_both_alike, timeout=timedelta(seconds=90))
    machine.log(machine.succeed("wpa_cli -i wlan2 scan_results | grep Lantern"))
    machine.succeed("wpa_cli -i wlan2 status | grep -qx bssid=02:00:00:00:00:00")
    # The looks just asked lost echoes as they took the card off its channel, and are left
    # out: counted, they moved the card as the call began (2026-11-20), before any loss.
    answer = wifid.acting({"act": "calling", "on": True})
    assert answer == {"id": 1, "done": True}, answer
    # Nothing the first sends the card arrives, its echoes' answers among it, while what
    # manages the link passes beside the queue.
    machine.succeed("ip netns exec air tc qdisc add dev wlan0 root netem loss 100%")
    since = wifid.journal_now()
    try:
        # Two lost come within about three seconds, an echo waited on a second, where the
        # minute's choice would hold the card until two minutes had lost a tenth.
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 gives way to 02:00:00:00:05:00, heard at -[0-9]+ dBm,"
            " during a call, as 02:00:00:00:00:00 lost [0-9]+ echoes in 10 s$'",
            timeout=timedelta(seconds=20),
        )
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 reached 02:00:00:00:05:00 [0-9]+ ms after it was asked'",
            timeout=timedelta(seconds=30),
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
        machine.succeed("ip netns exec air tc qdisc del dev wlan0 root")
        answer = wifid.acting({"act": "calling", "on": False})
        assert answer == {"id": 1, "done": True}, answer
    wifid.holds_address("10.42.0.")
    machine.succeed("ping -c 1 -W 2 10.42.0.1")
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
    machine.succeed("systemctl start lantern-two")

    def hears_the_second(_: bool) -> bool:
        """Have the cards look, and say whether they heard Lantern's second."""
        wifid.ask("Scan")
        return "02:00:00:00:05:00" in machine.succeed("wpa_cli -i wlan2 scan_results")

    retry(hears_the_second, timeout=timedelta(seconds=60))
    # The fixture joined Lantern as the launcher does, which keeps the card's other networks
    # off while it's on the network Joel picked; the subtest before took the card off it, so
    # the Inn is on, and this move is the one to turn it off and on again.
    networks = machine.succeed("wpa_cli -i wlan2 list_networks").splitlines()
    inns = [row.split("\t")[0] for row in networks if row.split("\t")[1:2] == ["Roadside Inn"]]
    assert len(inns) == 1, f"the Inn is saved once: {networks}"
    inn = inns[0]
    disabled = f"wpa_cli -i wlan2 get_network {inn} disabled"
    assert machine.succeed(disabled).strip() == "0", "the Inn is on before the move"
    # wpa_supplicant keeps an access point through two looks that don't hear it, so the move
    # still finds the second after its own look over the channel, and is asked into nothing.
    # The card gives up on it within tens of milliseconds, and wpa_supplicant was on its way
    # to the Roadside Inn before wifid heard it gave up (2026-11-20), so the card moves with
    # its other networks off, and is back within a second, never on the Inn. A card that left
    # the first for the second without a word finds the first still holding it as joined,
    # and bidding it come back once it has asked whether the card is still there, so the
    # second is counted beyond that wait.
    machine.succeed("systemctl stop lantern-two")
    since = wifid.journal_now()
    answer = wifid.acting({"act": "roam", "bssid": "02:00:00:00:05:00"})
    assert answer == {"id": 1, "done": True}, answer
    # The way back is timed by its journal, kept whether it comes or not.
    try:
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qxF 'wlan2 came off its network short of 02:00:00:00:05:00,"
            " so goes back to 02:00:00:00:00:00'",
            timeout=timedelta(seconds=30),
        )
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qE '^wlan2 reached 02:00:00:00:00:00 [0-9]+ ms after it was asked'",
            timeout=timedelta(seconds=30),
        )
        # Moved with its other networks off, it has them on again once back.
        machine.wait_until_succeeds(
            f"journalctl -o cat -u wifid --after-cursor='{since}'"
            " | grep -qx 'wlan2 may join its other networks again'",
            timeout=timedelta(seconds=30),
        )
    finally:
        machine.log(machine.succeed(f"journalctl -o short-precise --after-cursor='{since}'"))
    said = machine.succeed(
        f"journalctl -o short-unix -u wifid -u wpa_supplicant --after-cursor='{since}'"
    ).splitlines()
    moved = next(line for line in said if "wlan2 moved to 02:00:00:00:05:00, as asked" in line)
    back = next(
        line
        for line in said
        if "wlan2: CTRL-EVENT-CONNECTED - Connection to 02:00:00:00:00:00" in line
    )
    took = float(back.split()[0]) - float(moved.split()[0])
    bidden = machine.succeed(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | sed -n 's/^wlan2: .* rejected association temporarily;.* (\\([0-9]*\\) ms)$/\\1/p'"
    ).split()
    # wifid gives a way back 2 s, room for one such wait and the join after it.
    assert len(bidden) <= 1, f"bidden wait more than once: {bidden} ms"
    waited = sum(int(ms) for ms in bidden) / 1000
    assert took < 1 + waited, f"back {took:.3f} s after the move was asked, bidden wait {waited} s"
    tried = [line for line in said if "SSID='Roadside Inn'" in line]
    assert not tried, f"tried the Inn on the way back: {tried}"
    assert machine.succeed(disabled).strip() == "0", "the Inn is on again once back"
    machine.succeed(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:00:00'
    )
    wifid.holds_address("10.42.0.")


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

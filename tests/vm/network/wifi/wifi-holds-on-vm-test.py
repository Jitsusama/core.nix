"""The Wi-Fi test's holds-on area.

Here nothing that should leave the card on its network takes it off, and what's forgotten goes
from everywhere. tests/vm/network/wifi.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The tests run in
the order written, each taking the machine on from where the one before left it, so one run
alone with -k starts from Lantern joined and may not find what an earlier one set up. wifid as
the tests reach it is wifi_session.py.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import retry
from wifi_session import Wifid, booted, given, saved

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def wifid(machine: BaseMachine) -> Wifid:
    """Give wifid with the card on Lantern, joined as the shell joins it, the Inn's noted."""
    wifid = booted(machine)
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")
    wifid.inn_address = wifid.shown()
    retry(lambda _: wifid.hears("Lantern"), timeout=timedelta(seconds=60))
    status, _, said = wifid.joining({"name": "Lantern", "password": "glowing in the dark"})
    assert status == 0, said
    wifid.joined("Lantern")
    wifid.holds_address("10.42.0.")
    return wifid


def test_another_programs_routing_rule_survives_a_reconfigure(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Another program's routing rule survives a reconfigure."""
    # WARP's own rule, at its own priority, for both families.
    for family in ("-4", "-6"):
        machine.succeed(f"ip {family} rule add not fwmark 0x100cf lookup 65743 priority 32765")
    machine.succeed("networkctl reconfigure wlan2")
    wifid.holds_address("10.42.0.")
    wifid.rules_stand()


def test_it_survives_moving_to_another_network_when_one_goes_away(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """It survives moving to another network when one goes away."""
    machine.systemctl("stop lantern")
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")
    wifid.on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
    wifid.rules_stand()
    wifid.answers("front-desk.inn", "10.43.0.1")
    wifid.answers("probe.corp.internal", "10.99.0.7")


def test_a_network_sees_the_same_address_every_time(wifid: Wifid) -> None:
    """A network sees the same address every time."""
    assert wifid.shown() == wifid.inn_address, wifid.shown()


def test_wifid_starting_again_leaves_the_card_on_its_network(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Starting wifid again leaves the card on its network."""
    garbled = "/var/lib/wifid/networks/garbled.network"
    machine.succeed(f"printf '[\\377]' > {garbled}")
    since = wifid.journal_now()
    machine.succeed("systemctl restart wifid")
    wifid.says(f"{garbled}: stream did not contain valid UTF-8")
    wifid.says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
    wifid.stayed_on(since)
    machine.succeed(f"rm {garbled}")


def test_another_card_coming_and_going_leaves_the_first_on_its_network(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Another card coming and going leaves the first on its network."""
    supplicant = machine.succeed("systemctl show -P MainPID wpa_supplicant")
    since = wifid.journal_now()
    radio = machine.succeed(
        "ip netns exec drawer iw dev wlan3 info | sed -n 's/^.*wiphy /phy/p'"
    ).strip()
    machine.succeed(f"ip netns exec drawer iw phy {radio} set netns 1")
    machine.wait_until_succeeds(
        "wpa_cli -i wlan3 ping | grep -qx PONG", timeout=timedelta(seconds=30)
    )
    # It's given every network, their passwords opened without systemd.
    retry(
        lambda _: "lighthouse-8" in machine.succeed("wpa_cli -i wlan3 list_networks"),
        timeout=timedelta(seconds=30),
    )
    machine.fail(f"journalctl -o cat --after-cursor='{since}' | grep systemd-creds@")
    # It looks for networks from addresses its driver makes up, so its
    # link stays up as it looks, as it wouldn't were its own changed.
    held = machine.succeed(
        "busctl call fi.w1.wpa_supplicant1 /fi/w1/wpa_supplicant1"
        " fi.w1.wpa_supplicant1 GetInterface s wlan3"
    ).split('"')[1]
    machine.wait_until_succeeds(
        f"busctl get-property fi.w1.wpa_supplicant1 {held}"
        " fi.w1.wpa_supplicant1.Interface MACAddressRandomizationMask | grep -q scan",
        timeout=timedelta(seconds=30),
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan3 status | grep -qx wpa_state=COMPLETED", timeout=timedelta(seconds=30)
    )
    # Joining takes the link down once, to give the card the address it
    # has for the Inn; looking first takes it down no more.
    drops = machine.succeed(
        f"journalctl -o cat --after-cursor='{since}' -u systemd-networkd"
        " | grep -c 'wlan3: Link DOWN' || true"
    )
    assert int(drops) <= 1, drops
    machine.succeed(f"iw phy {radio} set netns name drawer")
    machine.wait_until_succeeds("! test -e /sys/class/net/wlan3", timeout=timedelta(seconds=30))
    assert machine.succeed("systemctl show -P MainPID wpa_supplicant") == supplicant
    wifid.stayed_on(since)
    wifid.joined("Roadside Inn")


def test_and_networkd_restarting(machine: BaseMachine, wifid: Wifid) -> None:
    """And networkd restarting."""
    machine.succeed("systemctl restart systemd-networkd")
    wifid.holds_address("10.43.0.")
    wifid.rules_stand()
    wifid.answers("probe.corp.internal", "10.99.0.7")


def test_wifid_adds_the_card_again_when_wpa_supplicant_restarts(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The card is added again when wpa_supplicant restarts."""
    machine.succeed("systemctl restart wpa_supplicant")
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 ping | grep -qx PONG", timeout=timedelta(seconds=30)
    )
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")


def test_the_check_asks_the_networks_own_dns_server_while_a_vpns_stand_in(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The check asks the network's own DNS server while a VPN's stand in."""
    # WARP points the link at its own servers while it's connected.
    machine.succeed("resolvectl dns wlan2 192.0.2.53")
    machine.succeed("resolvectl dns wlan2 | grep -q '192.0.2.53$'")
    machine.succeed("systemctl restart wifid")
    wifid.says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
    # The sign-in window looks names up the same way, through wifid.
    found = wifid.ask("Lookup", {"card": "wlan2", "name": "front-desk.inn"})
    assert found == {"addresses": ["10.43.0.1"]}, found
    wifid.refused("Lookup", {"card": "wlan9", "name": "front-desk.inn"}, "NotJoined")
    machine.succeed("resolvectl revert wlan2")


def test_forgetting_lantern_takes_it_from_everywhere_and_the_card_stays_on_the_inn(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Forgetting Lantern takes it from everywhere, and the card stays on the Inn."""
    since = wifid.journal_now()
    wifid.ask("Forget", {"name": "Lantern"})
    for gone in (saved, given, "/var/lib/wifid/networks/lantern.sealed"):
        machine.succeed(f"! test -e {gone}")
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "Roadside Inn" in held, held
    assert "Lantern" not in held, held
    assert wifid.saved_as("Lantern") == [], wifid.saved_as("Lantern")
    wifid.stayed_on(since)
    wifid.refused("Forget", {"name": "Lantern"}, "NotSaved")
    wifid.refused("Forget", {"name": "Roadside Inn"}, "Declared")
    wifid.joined("Roadside Inn")


@pytest.mark.usefixtures("wifid")
def test_networkds_file_for_a_network_no_longer_saved_is_taken_away(machine: BaseMachine) -> None:
    """The file networkd was given for a network no longer saved is taken away."""
    gone = "/run/systemd/network/50-wifi-gone.network"
    machine.succeed(
        f"printf '# Written by wifid from /var/lib/wifid/networks/gone.network\\n' > {gone}"
    )
    machine.succeed("systemctl restart wifid")
    machine.wait_for_unit("wifid.service")
    machine.wait_until_succeeds(f"! test -e {gone}", timeout=timedelta(seconds=30))

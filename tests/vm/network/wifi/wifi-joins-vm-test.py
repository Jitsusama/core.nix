"""The Wi-Fi test's joins area.

Here the card joins the networks it's given, wifid tells the shell what it finds, and a network
saved by the launcher is chosen first. tests/vm/network/wifi.nix runs this through pytest inside the
driver (tests/lib/vm_pytest.py), and its opening comment says what the test proves. The tests
run in the order written, each taking the machine on from where the one before left it, so one
run alone with -k starts from the boot and may not find what an earlier one set up. wifid as the
tests reach it is wifi_session.py.
"""

from __future__ import annotations

import json
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import retry
from wifi_session import Wifid, booted, given, saved, wifi

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine
    from wifi_session import Json

# Lantern as the launcher saved it before wifid had a vault: only
# what's its own, with its password sealed to this machine by systemd.
launcher_lantern = [
    "[Match]",
    "SSID=Lantern",
    "",
    "[X-WiFi]",
    "Security=personal",
    "Secret=/var/lib/wifid/networks/lantern.cred",
    "Priority=10",
]

# What the lowest two bits of an address's first byte are when the address is one a card
# made up for itself, and is no other's: locally administered, and for one card alone.
MADE_UP = 0b10

# The Inn's channel, 11, in MHz.
INN_FREQUENCY = 2462

# The most bars wifid gives a network's strength.
STRONGEST = 4


@pytest.fixture(scope="module")
def wifid(machine: BaseMachine) -> Wifid:
    """Give wifid once the machine has booted and its networks are up."""
    return booted(machine)


@pytest.mark.usefixtures("wifid")
def test_the_boot_waits_on_wifid_for_the_network_not_on_networkd(machine: BaseMachine) -> None:
    """The boot waits on wifid for the network, not on networkd."""
    machine.wait_for_unit("needs-network.service")
    machine.succeed("systemctl is-active wifid-online.service")
    waited = int(
        machine.succeed("systemctl show -P ActiveEnterTimestampMonotonic wifid-online.service")
    )
    started = int(
        machine.succeed("systemctl show -P ExecMainStartTimestampMonotonic needs-network.service")
    )
    assert 0 < waited <= started, (waited, started)
    wanted = machine.succeed("systemctl show -P Wants network-online.target").split()
    assert "wifid-online.service" in wanted, wanted
    assert "systemd-networkd-wait-online.service" not in wanted, wanted


def test_wifid_gives_wpa_supplicant_the_card_which_joins_nixs_network(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The card goes from wifid to wpa_supplicant, which joins Nix's network."""
    machine.wait_for_unit("wifid.service")
    wifid.joined("Roadside Inn")
    wifid.holds_address("10.43.0.")
    wifid.on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
    wifid.answers("front-desk.inn", "10.43.0.1")


def test_wifid_finds_the_inn_wants_a_sign_in_and_where(wifid: Wifid) -> None:
    """The Inn is found to want a sign-in, and where."""
    wifid.says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")


def test_and_tells_the_shell_which_only_an_account_of_the_machines_may_ask(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """And tells the shell, which only an account of the machine's may ask."""
    machine.succeed(f"varlinkctl introspect {wifi} io.jitsusama.Wifi > /tmp/wifi.varlink")
    machine.succeed("varlinkctl validate-idl /tmp/wifi.varlink")
    machine.succeed("varlinkctl list-registry | grep -q io.jitsusama.Wifi")
    described = wifid.ask("Describe")
    assert described["cards"] == [
        {
            "name": "wlan2",
            "network": "Roadside Inn",
            "reach": "signIn",
            "signIn": "http://front-desk.inn/sign-in",
            "trouble": None,
        }
    ], described
    inn = wifid.saved_as("Roadside Inn")
    assert inn[0]["declared"], inn
    assert inn[0]["security"] == "open", inn
    machine.fail(f"runuser -u stranger -- varlinkctl call {wifi} io.jitsusama.Wifi.Describe '{{}}'")


def test_and_whats_in_the_air_looking_again_when_asked(machine: BaseMachine, wifid: Wifid) -> None:
    """And what's in the air, looking again when asked."""
    nearby = {network["name"]: network for network in wifid.ask("Describe")["nearby"]}
    assert nearby["Roadside Inn"]["security"] == "open", nearby
    assert nearby["Lantern"]["security"] == "personal", nearby
    assert all(1 <= network["strength"] <= STRONGEST for network in nearby.values()), nearby
    # On a network this strong, the card looks again only hourly by
    # itself, so a scan finishing now is the one asked for.
    machine.succeed(
        "busctl monitor --match \"type='signal',member='ScanDone'\" > /tmp/scans 2>&1 &"
    )
    machine.wait_until_succeeds("grep -q Monitoring /tmp/scans", timeout=timedelta(seconds=10))
    wifid.ask("Scan")
    machine.wait_until_succeeds("grep -q ScanDone /tmp/scans", timeout=timedelta(seconds=30))
    machine.succeed("pkill -f 'busctl monitor'")


def test_and_one_network_in_detail_with_the_link_of_the_card_on_it_as_it_changes(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """And one network in detail, with the link of the card on it as it changes."""
    # Asked for as it changes, wifid answers again as the card reads
    # its link, every two seconds, until the call ends.
    streaming = wifid.calling("Detail", {"name": "Roadside Inn"}).replace(
        "varlinkctl call", "varlinkctl call --more --json=short"
    )
    machine.succeed(f"timeout 7 {streaming} > /tmp/detail || [ $? -eq 124 ]")
    answered = machine.succeed("cat /tmp/detail").splitlines()
    answered = [line for line in answered if line]
    detailed = json.loads(answered[-1])
    heard = detailed["points"][0]
    unread = machine.succeed('journalctl -u wifid -o cat | grep "link: " || true')
    assert detailed["joined"], (answered, unread)
    assert len(detailed["points"]) == 1, detailed
    where_heard = (heard["band"], heard["channel"], heard["frequency"])
    assert where_heard == ("ghz2", 11, INN_FREQUENCY), heard
    assert (heard["width"], heard["protection"], heard["joined"]) == (20, None, True), heard
    assert heard["estimate"] > 0, heard
    assert detailed["bands"] == [{"band": "ghz2", "points": 1}], detailed
    assert detailed["best"]["bssid"] == heard["bssid"], detailed
    reading = detailed["joined"][0]
    assert (reading["card"], reading["bssid"]) == ("wlan2", heard["bssid"]), reading
    assert reading["signal"] < 0, reading
    assert reading["frequency"] == INN_FREQUENCY, reading
    # Each way on its own, from the rate to what's been carried.
    for way in (reading["receive"], reading["send"]):
        assert way["mbps"] > 0, reading
        assert way["bytes"] > 0, reading
        assert way["packets"] > 0, reading

    # And once nobody asks, the card stops reading it.
    def unpolled(_: bool) -> bool:
        """Say whether wifid no longer reads the Inn's link."""
        return not wifid.ask("Detail", {"name": "Roadside Inn"})["joined"]

    retry(unpolled, timeout=timedelta(seconds=30))
    # Though it still says which access point the card is joined to.
    once = wifid.ask("Detail", {"name": "Roadside Inn"})["points"][0]
    assert once["joined"], once
    # As it does after a restart, which leaves the card where it is.
    machine.succeed("systemctl restart wifid")

    def points() -> list[Json]:
        """Give the Inn's access points, as wifid details them."""
        found: list[Json] = wifid.ask("Detail", {"name": "Roadside Inn"})["points"]
        return found

    retry(lambda _: any(point["joined"] for point in points()), timeout=timedelta(seconds=30))
    # Lantern's beacon says how it's protected.
    guarded = wifid.ask("Detail", {"name": "Lantern"})["points"][0]["protection"]
    assert guarded["standard"] == "WPA2/WPA3", guarded
    assert guarded["ways"] == ["wpa-psk", "sae"], guarded
    assert guarded["management"] == "optional", guarded


def test_the_network_sees_an_address_the_card_chose_for_it(wifid: Wifid) -> None:
    """The network sees an address the card chose for it."""
    wifid.inn_address = wifid.shown()
    assert wifid.inn_address != wifid.card, wifid.inn_address
    assert int(wifid.inn_address[:2], 16) & 0b11 == MADE_UP, wifid.inn_address


def test_a_network_saved_from_the_launcher_is_laid_over_the_defaults_and_chosen_first(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A network saved from the launcher is laid over the defaults and chosen first."""
    lines = " ".join(f"'{line}'" for line in launcher_lantern)
    machine.succeed(f"printf '%s\\n' {lines} > {saved}")
    machine.succeed(
        "printf 'glowing in the dark' | systemd-creds encrypt --name=lantern"
        " - /var/lib/wifid/networks/lantern.cred"
    )
    machine.succeed("systemctl restart wifid")
    # Moved into the vault as wifid starts, and opened from it after.
    wifid.says("moved 1 saved password into wifid's vault")
    machine.succeed("! test -e /var/lib/wifid/networks/lantern.cred")
    held = machine.succeed("cat /var/lib/wifid/networks/lantern.sealed")
    assert held, held
    assert "glowing" not in held, held
    machine.succeed(f"grep -qx 'Secret=/var/lib/wifid/networks/lantern.sealed' {saved}")
    machine.succeed("test -s /var/lib/wifid/vault.cred")
    # The card stays where it is until it next chooses, as the
    # launcher's join will have it do.
    machine.wait_until_succeeds(f"test -e {given}", timeout=timedelta(seconds=30))
    wifid.joined("Roadside Inn")
    machine.succeed("wpa_cli -i wlan2 reassociate")
    wifid.joined("Lantern")
    wifid.holds_address("10.42.0.")
    wifid.on_file(given)
    written = machine.succeed(f"cat {given}")
    # Moving its password wrote the file again, with its name quoted.
    for line in ("Type=wlan", 'SSID="Lantern"', "IPv6PrivacyExtensions=yes", "[X-WiFi]"):
        assert line in written, written
    wifid.answers("router.lantern", "10.42.0.1")
    # networkd knows the network by name, which a saved network's file
    # is matched on.
    machine.succeed("networkctl status wlan2 | grep -q 'Wi-Fi access point: Lantern '")
    wifid.answers("probe.corp.internal", "10.99.0.7")
    lantern_address = wifid.shown()
    assert lantern_address not in (wifid.inn_address, wifid.card), lantern_address


def test_and_finds_lantern_reaches_beyond_itself(wifid: Wifid) -> None:
    """And finds Lantern reaches beyond itself."""
    wifid.says("Lantern on wlan2 reaches beyond itself")


@pytest.mark.usefixtures("wifid")
def test_and_keeps_the_channel_it_joined_lantern_on_and_nothing_of_where(
    machine: BaseMachine,
) -> None:
    """And keeps the channel it joined Lantern on, and nothing of where."""
    seen = json.loads(machine.succeed("cat /var/lib/wifid/seen"))
    assert seen.get("Lantern") == [2437], seen
    assert "02:00" not in json.dumps(seen), seen
    machine.succeed('test "$(stat -c %a /var/lib/wifid/seen)" = 600')


def test_and_asks_lantern_again_as_the_session_it_gave_ends(wifid: Wifid) -> None:
    """And asks Lantern again as the session it gave ends."""
    wifid.says(
        "Lantern on wlan2 wants a sign-in at https://router.lantern/renew",
        timeout=timedelta(seconds=45),
    )


@pytest.mark.usefixtures("wifid")
def test_a_strangers_network_gets_no_llmnr_or_mdns(machine: BaseMachine) -> None:
    """A stranger's network gets no LLMNR or mDNS."""
    resolving = machine.succeed("resolvectl status wlan2")
    assert "-LLMNR" in resolving, resolving
    assert "-mDNS" in resolving, resolving


def test_trusting_it_from_the_shell_turns_on_mdns_and_the_card_stays_on_it(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Trusting it from the shell turns on mDNS, and the card stays on it."""
    since = wifid.journal_now()
    wifid.ask("Trust", {"name": "Lantern", "trusted": True})
    machine.wait_until_succeeds(
        "resolvectl status wlan2 | grep -qF +mDNS", timeout=timedelta(seconds=30)
    )
    machine.succeed(f"grep -qx MulticastDNS=yes {saved}")
    assert wifid.saved_as("Lantern")[0]["trusted"], wifid.saved_as("Lantern")
    wifid.ask("Trust", {"name": "Lantern", "trusted": False})
    machine.wait_until_succeeds(
        "resolvectl status wlan2 | grep -qF -- -mDNS", timeout=timedelta(seconds=30)
    )
    assert not wifid.saved_as("Lantern")[0]["trusted"], wifid.saved_as("Lantern")
    wifid.holds_address("10.42.0.")
    wifid.stayed_on(since)
    wifid.refused("Trust", {"name": "Roadside Inn", "trusted": True}, "Declared")
    wifid.refused("Trust", {"name": "Nowhere", "trusted": True}, "NotSaved")

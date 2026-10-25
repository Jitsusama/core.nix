"""Wi-Fi networks joined in a virtual machine the way a laptop meets them, through wifid.

tests/wifi.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. The tests run in the order written, each taking the
machine on from where the one before left it, so one run alone with -k starts from the boot and
may not find what an earlier one set up. The script needs no value from Nix: the stand-in
access points, networks and accounts are all named in the machine wifi.nix builds.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as wifid gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

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
saved = "/var/lib/wifid/networks/lantern.network"
given = "/run/systemd/network/50-wifi-lantern.network"

# What the lowest two bits of an address's first byte are when the address is one a card
# made up for itself, and is no other's: locally administered, and for one card alone.
MADE_UP = 0b10
# The Inn's channel, 11, in MHz.
INN_FREQUENCY = 2462
# How many columns a line of wpa_cli's scan results has: its access point, frequency,
# level, flags and network.
SCAN_COLUMNS = 5
# The most bars wifid gives a network's strength.
STRONGEST = 4

wifi = "/run/wifid/io.jitsusama.Wifi"
sealed = "/var/lib/wifid/networks/lantern.sealed"


class Wifid:
    """wifid on the machine, as the shell and the test reach it, and what the test noted."""

    def __init__(self, machine: BaseMachine, card: str) -> None:
        """Reach wifid on that machine, whose card has that address of its own."""
        self.machine = machine
        self.card = card
        # The address the card showed the Inn, and the machine's cables, noted by one
        # test for those after it.
        self.inn_address = ""
        self.cables: list[str] = []

    def holds_address(self, prefix: str) -> None:
        """Wait until the card holds an address starting with the prefix, and is routable."""
        self.machine.wait_until_succeeds(
            f"ip -4 -o address show dev wlan2 | grep -q 'inet {prefix}'",
            timeout=timedelta(seconds=60),
        )
        self.machine.wait_until_succeeds(
            "networkctl status wlan2 | grep -q 'State: routable'", timeout=timedelta(seconds=30)
        )

    def on_file(self, file: str) -> None:
        """Wait until networkd configures the card from the file."""
        self.machine.wait_until_succeeds(
            f"networkctl status wlan2 | grep -q 'Network File: {file}'",
            timeout=timedelta(seconds=30),
        )

    def joined(self, ssid: str) -> None:
        """Wait until the card is on the network."""
        self.machine.wait_until_succeeds(
            f"wpa_cli -i wlan2 status | grep -qx 'ssid={ssid}'", timeout=timedelta(seconds=60)
        )

    def answers(self, name: str, address: str) -> None:
        """Wait until the name resolves to the address."""
        self.machine.wait_until_succeeds(
            f"resolvectl query --legend=no {name} | grep -qF {address}",
            timeout=timedelta(seconds=30),
        )

    def rules_stand(self) -> None:
        """Check another program's routing rule stands, for both families."""
        for family in ("-4", "-6"):
            self.machine.succeed(
                f"ip {family} rule show | grep -q 'not from all fwmark 0x100cf lookup 65743'"
            )

    def says(self, line: str, timeout: timedelta = timedelta(seconds=30)) -> None:
        """Wait for wifid, as it runs now, to say line in the journal."""
        self.machine.wait_until_succeeds(
            "journalctl -o cat _PID=$(systemctl show -P MainPID wifid)"
            f" | grep -qxF {shlex.quote(line)}",
            timeout=timeout,
        )

    def shown(self) -> str:
        """Give the address the card is showing the network it's on."""
        return self.machine.succeed("cat /sys/class/net/wlan2/address").strip()

    def calling(self, method: str, parameters: Json) -> str:
        """Give how an account of the machine's asks wifid something."""
        return (
            f"runuser -u guest -- varlinkctl call {wifi}"
            f" io.jitsusama.Wifi.{method} '{json.dumps(parameters)}'"
        )

    def ask(self, method: str, parameters: Json | None = None) -> Json:
        """Ask wifid something as the account would, and read the answer."""
        answer: Json = json.loads(
            self.machine.succeed(self.calling(method, parameters or {})) or "{}"
        )
        return answer

    def refused(self, method: str, parameters: Json, error: str) -> None:
        """Check wifid refuses, saying why."""
        said = self.machine.fail(f"{self.calling(method, parameters)} 2>&1")
        assert f"failed: io.jitsusama.Wifi.{error}" in said, said

    def saved_as(self, name: str) -> list[Json]:
        """Give the saved network of that name, as wifid tells the shell."""
        return [network for network in self.ask("Describe")["saved"] if network["name"] == name]

    def journal_now(self) -> str:
        """Give where the journal is, to read on from."""
        return self.machine.succeed(
            "journalctl -n1 --show-cursor -o cat | sed -n 's/^-- cursor: //p'"
        ).strip()

    def stayed_on(self, since: str) -> None:
        """Check wpa_supplicant hasn't taken wlan2 off its network since."""
        self.machine.fail(
            f"journalctl -o cat -u wpa_supplicant --after-cursor='{since}'"
            " | grep 'wlan2: CTRL-EVENT-DISCONNECTED'"
        )

    def hears(self, name: str) -> bool:
        """Have the cards look, and say whether they hear the network."""
        self.ask("Scan")
        return any(network["name"] == name for network in self.ask("Describe")["nearby"])

    def joining(self, parameters: Json) -> tuple[int, list[str], str]:
        """Join as the launcher does, giving each step wifid said, or why not."""
        # With more, and no timeout of varlinkctl's own: wifid's is the join's.
        call = self.calling("Join", parameters).replace("call", "call -E --json=short", 1)
        status, said = self.machine.execute(f"{call} 2>&1")
        # An error's parameters are printed too, and have no stage.
        replies = [json.loads(line) for line in said.splitlines() if line.startswith("{")]
        stages = [reply["stage"] for reply in replies if "stage" in reply]
        return status, stages, said

    def on_network(self, name: str) -> bool:
        """Say whether wifid tells the shell a card is on the network."""
        return any(card["network"] == name for card in self.ask("Describe")["cards"])

    # The kernel's stand-in battery has two chargers, mains and USB, and the machine is on
    # battery only with both pulled.
    def chargers(self, plugged: str) -> None:
        """Plug both of the stand-in battery's chargers in, or pull them."""
        for supply in ["ac", "usb"]:
            self.machine.succeed(
                f"echo {plugged} > /sys/module/test_power/parameters/{supply}_online"
            )

    def dozes(self, answer: str) -> None:
        """Wait until the laptop's card says whether it dozes, on or off."""
        self.machine.wait_until_succeeds(
            f"iw dev wlan2 get power_save | grep -qx 'Power save: {answer}'",
            timeout=timedelta(seconds=30),
        )


@pytest.fixture(scope="module")
def wifid(machine: BaseMachine) -> Wifid:
    """Give wifid once the machine has booted and its networks are up."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("networks.service")
    machine.wait_for_unit("private-dns.service")
    card = machine.succeed("cat /sys/class/net/wlan2/phy80211/macaddress").strip()
    return Wifid(machine, card)


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


def test_signing_in_at_the_inn_is_noticed_when_asked_and_the_shell_told(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Signing in at the Inn is noticed when asked, and the shell told."""
    machine.succeed(
        f"runuser -u guest -- varlinkctl -E --json=short call {wifi}"
        " io.jitsusama.Wifi.Describe '{}' > /tmp/described 2>&1 &"
    )
    machine.wait_until_succeeds("grep -q signIn /tmp/described", timeout=timedelta(seconds=10))
    machine.succeed("curl -sf --interface wlan2 http://front-desk.inn/sign-in")
    wifid.ask("Check")
    # Well before the half-minute the Inn would otherwise wait.
    wifid.says("Roadside Inn on wlan2 reaches beyond itself", timeout=timedelta(seconds=15))
    machine.wait_until_succeeds(
        'tail -n1 /tmp/described | grep -qF \'"reach":"online"\'', timeout=timedelta(seconds=10)
    )


def test_a_network_letting_only_the_plain_check_through_reaches_no_further(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A network letting only the plain check through reaches no further."""
    machine.succeed("systemctl restart wifid")
    wifid.says("Roadside Inn on wlan2 reaches no further")


def test_joining_lantern_from_the_launcher_with_the_wrong_password_saves_nothing(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Joining Lantern from the launcher with the wrong password saves nothing."""
    machine.systemctl("start lantern")
    retry(lambda _: wifid.hears("Lantern"), timeout=timedelta(seconds=60))
    wifid.refused("Join", {"name": "Lantern"}, "NeedsPassword")
    wifid.refused("Join", {"name": "Nowhere"}, "NotNearby")
    status, stages, said = wifid.joining({"name": "Lantern", "password": "not the password"})
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.WrongPassword" in said, said
    assert stages[:1] == ["joining"], said
    assert "online" not in stages, said
    assert wifid.saved_as("Lantern") == [], wifid.saved_as("Lantern")
    machine.succeed(f"! test -e {sealed}")
    wifid.joined("Roadside Inn")


def test_and_with_the_right_one_its_saved_sealed_and_said_joined_step_by_step(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """And with the right one, it's saved sealed and said joined step by step."""
    since = wifid.journal_now()
    status, stages, said = wifid.joining({"name": "Lantern", "password": "glowing in the dark"})
    assert status == 0, said
    # Lantern's session ran out long ago, so it wants a sign-in again.
    assert stages[0] == "joining", said
    assert stages[-1] == "signIn", said
    wifid.joined("Lantern")
    # By WPA3, which the card prefers where the network offers both.
    machine.succeed("wpa_cli -i wlan2 status | grep -qx key_mgmt=SAE")
    wifid.holds_address("10.42.0.")
    lantern_saved = wifid.saved_as("Lantern")
    assert lantern_saved, lantern_saved
    assert not lantern_saved[0]["declared"], lantern_saved
    held = machine.succeed(f"cat {sealed}")
    assert "glowing" not in held, held
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "[DISABLED]" not in held, held
    # Sealed and opened by the vault, so nothing asked systemd's
    # credential service, which takes the TPM seconds a password.
    machine.fail(f"journalctl -o cat --after-cursor='{since}' | grep systemd-creds@")


def test_joining_the_inn_again_moves_the_card_and_says_how_far_the_inn_reaches(
    wifid: Wifid,
) -> None:
    """Joining the Inn again moves the card, and says how far the Inn reaches."""
    status, stages, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    assert stages[-1] == "limited", said
    wifid.joined("Roadside Inn")


def test_the_office_is_joined_with_an_account_its_server_first_proving_itself(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The Office is joined with an account, its server first proving itself."""
    retry(lambda _: wifid.hears("Office"), timeout=timedelta(seconds=60))
    nearby = {network["name"]: network for network in wifid.ask("Describe")["nearby"]}
    assert nearby["Office"]["security"] == "enterprise", nearby
    wifid.refused("Join", {"name": "Office", "password": "hunter22"}, "NeedsAccount")
    # A server naming another domain is never given the password.
    elsewhere = {"method": "peap", "identity": "joel", "serverDomain": "elsewhere.test"}
    status, _, said = wifid.joining(
        {"name": "Office", "password": "hunter22", "account": elsewhere}
    )
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.ServerUnproven" in said, said
    joel = {"method": "peap", "identity": "joel", "serverDomain": "radius.office.test"}
    status, _, said = wifid.joining({"name": "Office", "password": "not it", "account": joel})
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.WrongPassword" in said, said
    assert wifid.saved_as("Office") == [], wifid.saved_as("Office")
    status, _, said = wifid.joining({"name": "Office", "password": "hunter22", "account": joel})
    assert status == 0, said
    wifid.joined("Office")
    wifid.holds_address("10.44.0.")
    kept = machine.succeed("cat /var/lib/wifid/networks/office.network")
    assert "ServerDomain=radius.office.test" in kept, kept
    assert "hunter22" not in kept, kept
    # The steps after this start on the Inn.
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    wifid.joined("Roadside Inn")


def test_leaving_the_inn_from_the_shell_keeps_the_card_off_it_until_its_joined(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Leaving the Inn from the shell keeps the card off it until it's joined."""
    wifid.ask("Leave", {"name": "Roadside Inn"})
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=timedelta(seconds=30)
    )
    retry(lambda _: not wifid.on_network("Roadside Inn"), timeout=timedelta(seconds=30))
    wifid.refused("Leave", {"name": "Roadside Inn"}, "NotJoined")
    machine.sleep(timedelta(seconds=10))
    machine.succeed("wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED")
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    wifid.joined("Roadside Inn")


def test_the_shell_is_told_whether_a_cable_reaches_beyond_the_machine(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The shell is told whether a cable reaches beyond the machine."""
    # The test's own network and QEMU's are both cables here.
    links = machine.succeed("networkctl list --no-legend")
    wifid.cables = [line.split()[1] for line in links.splitlines() if line.split()[2] == "ether"]
    assert "routable" in links, links
    assert wifid.cables, links
    retry(lambda _: wifid.ask("Describe")["wired"] is True, timeout=timedelta(seconds=30))
    for cable in wifid.cables:
        machine.succeed(f"ip link set {cable} down")
    retry(lambda _: wifid.ask("Describe")["wired"] is False, timeout=timedelta(seconds=30))


def test_wifid_says_its_settled_once_the_cards_online_or_wi_fis_off(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The wait for wifid to settle ends once the card's online, or Wi-Fi's off."""
    settled = wifid.calling("Settled", {}).replace("call", "call --timeout=5", 1)
    machine.succeed(settled)
    wifid.ask("Leave", {"name": "Roadside Inn"})
    retry(lambda _: not wifid.on_network("Roadside Inn"), timeout=timedelta(seconds=30))
    # The Inn is still heard, so the card has more to join, and wifid
    # answers everything else while the wait goes on.
    machine.succeed(
        "systemd-run --unit=settling --uid=guest --property=RemainAfterExit=yes"
        " $(command -v varlinkctl) call --timeout=5 /run/wifid/io.jitsusama.Wifi"
        " io.jitsusama.Wifi.Settled '{}'"
    )
    assert wifid.ask("Describe")["cards"], wifid.ask("Describe")
    machine.wait_until_succeeds("systemctl is-failed -q settling", timeout=timedelta(seconds=15))
    machine.succeed("journalctl -u settling -o cat | grep -q 'Timer expired'")
    machine.succeed("systemctl reset-failed settling")
    wifid.ask("Radio", {"on": False})
    machine.succeed(settled)
    wifid.ask("Radio", {"on": True})
    retry(lambda _: ",UP" in machine.succeed("ip link show wlan2"), timeout=timedelta(seconds=30))
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    machine.succeed(settled)


def test_and_the_cables_come_back(machine: BaseMachine, wifid: Wifid) -> None:
    """And the cables come back."""
    for cable in wifid.cables:
        machine.succeed(f"ip link set {cable} up")
    retry(lambda _: wifid.ask("Describe")["wired"] is True, timeout=timedelta(seconds=60))


def test_turning_wi_fi_off_from_the_shell_switches_the_radios_off_and_on_again(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """Turning Wi-Fi off from the shell switches the radios off, and on again."""
    assert wifid.ask("Describe")["radio"], wifid.ask("Describe")
    # Off its network first, so nothing but wifid has networkd set the
    # card's link up again once the radios are back.
    wifid.ask("Leave", {"name": "Roadside Inn"})
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=timedelta(seconds=30)
    )
    wifid.ask("Radio", {"on": False})
    machine.wait_until_succeeds(
        "rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=timedelta(seconds=10)
    )
    retry(lambda _: wifid.ask("Describe")["radio"] is False, timeout=timedelta(seconds=30))
    wifid.ask("Radio", {"on": True})
    machine.wait_until_succeeds(
        "! rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=timedelta(seconds=10)
    )
    retry(lambda _: wifid.ask("Describe")["radio"] is True, timeout=timedelta(seconds=30))
    retry(lambda _: ",UP" in machine.succeed("ip link show wlan2"), timeout=timedelta(seconds=30))
    status, _, said = wifid.joining({"name": "Roadside Inn"})
    assert status == 0, said
    wifid.joined("Roadside Inn")


def test_a_network_saved_from_the_shell_without_joining_it_is_kept_sealed(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A network saved from the shell without joining it is kept, sealed."""
    since = wifid.journal_now()
    hall = {
        "name": "Hidden Hall",
        "security": "personal",
        "password": "under the stairs",
        "hidden": True,
        "priority": 5,
        "address": "card",
    }
    wifid.ask("Save", {"network": hall})
    assert wifid.saved_as("Hidden Hall") == [
        {
            "name": "Hidden Hall",
            "security": "personal",
            "declared": False,
            "trusted": False,
            "priority": 5,
            "hidden": True,
        }
    ], wifid.saved_as("Hidden Hall")
    held = machine.succeed("cat /var/lib/wifid/networks/hidden-hall.sealed")
    assert held, held
    assert "stairs" not in held, held
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "Hidden Hall" in held, held
    wifid.stayed_on(since)
    inn_network = {"name": "Roadside Inn", "security": "open"}
    wifid.refused("Save", {"network": inn_network}, "Declared")
    nameless = {"name": "Nameless", "security": "personal"}
    wifid.refused("Save", {"network": nameless}, "NeedsPassword")


# A radio here never hangs a look, so the card is stopped where it
# stands, which wpa_supplicant then leaves alone, and said to be
# scanning, in the signal wifid hears.
@pytest.mark.usefixtures("wifid")
def test_a_card_scanning_too_long_with_a_saved_network_in_range_is_told_to_join_it(
    machine: BaseMachine,
) -> None:
    """A card scanning too long with a saved network in range is told to join it."""
    machine.succeed("wpa_cli -i wlan2 disconnect")
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=timedelta(seconds=30)
    )
    held = machine.succeed(
        "busctl call fi.w1.wpa_supplicant1 /fi/w1/wpa_supplicant1"
        " fi.w1.wpa_supplicant1 GetInterface s wlan2"
    ).split('"')[1]
    machine.succeed(
        f"busctl emit {held} org.freedesktop.DBus.Properties PropertiesChanged"
        " 'sa{sv}as' fi.w1.wpa_supplicant1.Interface 1 State s scanning 0"
    )
    # Nothing else joins it: a card stopped so stays stopped.
    machine.sleep(timedelta(seconds=10))
    machine.succeed("wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED")
    # Which saved network it hears best varies from run to run among
    # the several here, and the card joins whichever it can.
    machine.wait_until_succeeds(
        "journalctl -o cat _PID=$(systemctl show -P MainPID wifid) | grep -qxE"
        " 'wlan2 looked for networks for 20 s with .+ in range; asking it to join'",
        timeout=timedelta(seconds=30),
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=COMPLETED", timeout=timedelta(seconds=30)
    )


# On a laptop at boot, the access point wpa_supplicant picked didn't
# answer, and the card then sat off every network, with nothing pending,
# until Joel joined by hand. Here the card starts off every network, as
# at boot, with every saved one on and Lantern ranked first, and Lantern
# stops answering while the card's last look still holds it.
def test_a_card_whose_access_point_doesnt_answer_moves_on_to_another_network(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A card whose access point doesn't answer moves on to another network."""
    lantern = machine.succeed(
        "wpa_cli -i wlan2 list_networks | awk -F'\\t' '$2 == \"Lantern\" {print $1}'"
    ).strip()
    ranked = machine.succeed(f"wpa_cli -i wlan2 get_network {lantern} priority").strip()
    machine.succeed(
        "wpa_cli -i wlan2 disconnect"
        " && wpa_cli -i wlan2 enable_network all"
        f" && wpa_cli -i wlan2 set_network {lantern} priority 100"
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=timedelta(seconds=30)
    )
    retry(lambda _: wifid.hears("Lantern"), timeout=timedelta(seconds=60))
    since = wifid.journal_now()
    machine.succeed("systemctl stop lantern && wpa_cli -i wlan2 reconnect")
    machine.wait_until_succeeds(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | grep -q 'wlan2: authentication with 02:00:00:00:00:00 timed out'",
        timeout=timedelta(seconds=30),
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=COMPLETED", timeout=timedelta(seconds=30)
    )
    machine.succeed(
        f"wpa_cli -i wlan2 set_network {lantern} priority {ranked} && systemctl start lantern"
    )


@pytest.mark.usefixtures("wifid")
def test_and_says_each_step_of_the_join_and_each_look_for_a_join_that_dawdles(
    machine: BaseMachine,
) -> None:
    """And says each step of the join and each look, for a join that dawdles."""
    for said in ["wlan2 is scanning", "wlan2 is completed", "wlan2 is done looking"]:
        machine.wait_until_succeeds(
            f"journalctl -o cat _PID=$(systemctl show -P MainPID wifid) | grep -qx '{said}'",
            timeout=timedelta(seconds=30),
        )


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


def test_the_card_stays_awake_on_mains_power_and_dozes_on_battery(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """The card stays awake on mains power, and dozes on battery."""
    wifid.chargers("on")
    wifid.dozes("off")
    wifid.chargers("off")
    wifid.dozes("on")
    wifid.chargers("on")
    wifid.dozes("off")
    machine.succeed("journalctl -t wifi-power-save -o cat | grep -qx 'wlan2: power save on'")
    # As after a switch, which reloads udev's rules without running them: a card
    # left dozing on mains power is woken by the service the switch starts.
    machine.succeed("iw dev wlan2 set power_save on", "systemctl restart wifi-power-save.service")
    wifid.dozes("off")

"""Wi-Fi networks joined in a virtual machine the way a laptop meets them, through wifid.

tests/wifi.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the boot and the helpers, runs
whichever subtests are chosen. The script needs no value from Nix: the stand-in access
points, networks and accounts are all named in the machine wifi.nix builds.
"""

import json
import shlex
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest

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


def holds_address(prefix: str) -> None:
    """Wait until the card holds an address starting with the prefix, and is routable."""
    machine.wait_until_succeeds(
        f"ip -4 -o address show dev wlan2 | grep -q 'inet {prefix}'", timeout=60
    )
    machine.wait_until_succeeds("networkctl status wlan2 | grep -q 'State: routable'", timeout=30)


def on_file(file: str) -> None:
    """Wait until networkd configures the card from the file."""
    machine.wait_until_succeeds(
        f"networkctl status wlan2 | grep -q 'Network File: {file}'", timeout=30
    )


def joined(ssid: str) -> None:
    """Wait until the card is on the network."""
    machine.wait_until_succeeds(f"wpa_cli -i wlan2 status | grep -qx 'ssid={ssid}'", timeout=60)


def answers(name: str, address: str) -> None:
    """Wait until the name resolves to the address."""
    machine.wait_until_succeeds(
        f"resolvectl query --legend=no {name} | grep -qF {address}", timeout=30
    )


def rules_stand() -> None:
    """Check another program's routing rule stands, for both families."""
    for family in ("-4", "-6"):
        machine.succeed(
            f"ip {family} rule show | grep -q 'not from all fwmark 0x100cf lookup 65743'"
        )


def says(line: str, timeout: int = 30) -> None:
    """Wait for wifid, as it runs now, to say line in the journal."""
    machine.wait_until_succeeds(
        "journalctl -o cat _PID=$(systemctl show -P MainPID wifid)"
        f" | grep -qxF {shlex.quote(line)}",
        timeout=timeout,
    )


def shown() -> str:
    """Give the address the card is showing the network it's on."""
    return machine.succeed("cat /sys/class/net/wlan2/address").strip()


wifi = "/run/wifid/io.jitsusama.Wifi"


def calling(method: str, parameters: Json) -> str:
    """Give how an account of the machine's asks wifid something."""
    return (
        f"runuser -u guest -- varlinkctl call {wifi}"
        f" io.jitsusama.Wifi.{method} '{json.dumps(parameters)}'"
    )


def ask(method: str, parameters: Json | None = None) -> Json:
    """Ask wifid something as the account would, and read the answer."""
    answer: Json = json.loads(machine.succeed(calling(method, parameters or {})) or "{}")
    return answer


def refused(method: str, parameters: Json, error: str) -> None:
    """Check wifid refuses, saying why."""
    said = machine.fail(f"{calling(method, parameters)} 2>&1")
    assert f"failed: io.jitsusama.Wifi.{error}" in said, said


def saved_as(name: str) -> list[Json]:
    """Give the saved network of that name, as wifid tells the shell."""
    return [network for network in ask("Describe")["saved"] if network["name"] == name]


def journal_now() -> str:
    """Give where the journal is, to read on from."""
    return machine.succeed(
        "journalctl -n1 --show-cursor -o cat | sed -n 's/^-- cursor: //p'"
    ).strip()


def stayed_on(since: str) -> None:
    """Check wpa_supplicant hasn't taken wlan2 off its network since."""
    machine.fail(
        f"journalctl -o cat -u wpa_supplicant --after-cursor='{since}'"
        " | grep 'wlan2: CTRL-EVENT-DISCONNECTED'"
    )


machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("networks.service")
machine.wait_for_unit("private-dns.service")
card = machine.succeed("cat /sys/class/net/wlan2/phy80211/macaddress").strip()

with subtest("the boot waits on wifid for the network, not on networkd"):
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

with subtest("wifid gives wpa_supplicant the card, which joins Nix's network"):
    machine.wait_for_unit("wifid.service")
    joined("Roadside Inn")
    holds_address("10.43.0.")
    on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
    answers("front-desk.inn", "10.43.0.1")

with subtest("wifid finds the Inn wants a sign-in, and where"):
    says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")

with subtest("and tells the shell, which only an account of the machine's may ask"):
    machine.succeed(f"varlinkctl introspect {wifi} io.jitsusama.Wifi > /tmp/wifi.varlink")
    machine.succeed("varlinkctl validate-idl /tmp/wifi.varlink")
    machine.succeed("varlinkctl list-registry | grep -q io.jitsusama.Wifi")
    described = ask("Describe")
    assert described["cards"] == [
        {
            "name": "wlan2",
            "network": "Roadside Inn",
            "reach": "signIn",
            "signIn": "http://front-desk.inn/sign-in",
            "trouble": None,
        }
    ], described
    inn = saved_as("Roadside Inn")
    assert inn[0]["declared"], inn
    assert inn[0]["security"] == "open", inn
    machine.fail(f"runuser -u stranger -- varlinkctl call {wifi} io.jitsusama.Wifi.Describe '{{}}'")

with subtest("and what's in the air, looking again when asked"):
    nearby = {network["name"]: network for network in ask("Describe")["nearby"]}
    assert nearby["Roadside Inn"]["security"] == "open", nearby
    assert nearby["Lantern"]["security"] == "personal", nearby
    assert all(1 <= network["strength"] <= STRONGEST for network in nearby.values()), nearby
    # On a network this strong, the card looks again only hourly by
    # itself, so a scan finishing now is the one asked for.
    machine.succeed(
        "busctl monitor --match \"type='signal',member='ScanDone'\" > /tmp/scans 2>&1 &"
    )
    machine.wait_until_succeeds("grep -q Monitoring /tmp/scans", timeout=10)
    ask("Scan")
    machine.wait_until_succeeds("grep -q ScanDone /tmp/scans", timeout=30)
    machine.succeed("pkill -f 'busctl monitor'")

with subtest("and one network in detail, with the link of the card on it as it changes"):
    # Asked for as it changes, wifid answers again as the card reads
    # its link, every two seconds, until the call ends.
    streaming = calling("Detail", {"name": "Roadside Inn"}).replace(
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
        return not ask("Detail", {"name": "Roadside Inn"})["joined"]

    retry(unpolled, timeout_seconds=30)
    # Though it still says which access point the card is joined to.
    once = ask("Detail", {"name": "Roadside Inn"})["points"][0]
    assert once["joined"], once
    # As it does after a restart, which leaves the card where it is.
    machine.succeed("systemctl restart wifid")

    def points() -> list[Json]:
        """Give the Inn's access points, as wifid details them."""
        found: list[Json] = ask("Detail", {"name": "Roadside Inn"})["points"]
        return found

    retry(lambda _: any(point["joined"] for point in points()), timeout_seconds=30)
    # Lantern's beacon says how it's protected.
    guarded = ask("Detail", {"name": "Lantern"})["points"][0]["protection"]
    assert guarded["standard"] == "WPA2/WPA3", guarded
    assert guarded["ways"] == ["wpa-psk", "sae"], guarded
    assert guarded["management"] == "optional", guarded

with subtest("the network sees an address the card chose for it"):
    inn_address = shown()
    assert inn_address != card, inn_address
    assert int(inn_address[:2], 16) & 0b11 == MADE_UP, inn_address

with subtest("a network saved from the launcher is laid over the defaults and chosen first"):
    lines = " ".join(f"'{line}'" for line in launcher_lantern)
    machine.succeed(f"printf '%s\\n' {lines} > {saved}")
    machine.succeed(
        "printf 'glowing in the dark' | systemd-creds encrypt --name=lantern"
        " - /var/lib/wifid/networks/lantern.cred"
    )
    machine.succeed("systemctl restart wifid")
    # Moved into the vault as wifid starts, and opened from it after.
    says("moved 1 saved password into wifid's vault")
    machine.succeed("! test -e /var/lib/wifid/networks/lantern.cred")
    held = machine.succeed("cat /var/lib/wifid/networks/lantern.sealed")
    assert held, held
    assert "glowing" not in held, held
    machine.succeed(f"grep -qx 'Secret=/var/lib/wifid/networks/lantern.sealed' {saved}")
    machine.succeed("test -s /var/lib/wifid/vault.cred")
    # The card stays where it is until it next chooses, as the
    # launcher's join will have it do.
    machine.wait_until_succeeds(f"test -e {given}", timeout=30)
    joined("Roadside Inn")
    machine.succeed("wpa_cli -i wlan2 reassociate")
    joined("Lantern")
    holds_address("10.42.0.")
    on_file(given)
    written = machine.succeed(f"cat {given}")
    # Moving its password wrote the file again, with its name quoted.
    for line in ("Type=wlan", 'SSID="Lantern"', "IPv6PrivacyExtensions=yes", "[X-WiFi]"):
        assert line in written, written
    answers("router.lantern", "10.42.0.1")
    # networkd knows the network by name, which a saved network's file
    # is matched on.
    machine.succeed("networkctl status wlan2 | grep -q 'Wi-Fi access point: Lantern '")
    answers("probe.corp.internal", "10.99.0.7")
    lantern_address = shown()
    assert lantern_address not in (inn_address, card), lantern_address

with subtest("and finds Lantern reaches beyond itself"):
    says("Lantern on wlan2 reaches beyond itself")

with subtest("and keeps the channel it joined Lantern on, and nothing of where"):
    seen = json.loads(machine.succeed("cat /var/lib/wifid/seen"))
    assert seen.get("Lantern") == [2437], seen
    assert "02:00" not in json.dumps(seen), seen
    machine.succeed('test "$(stat -c %a /var/lib/wifid/seen)" = 600')

with subtest("and asks Lantern again as the session it gave ends"):
    says("Lantern on wlan2 wants a sign-in at https://router.lantern/renew", timeout=45)

with subtest("a stranger's network gets no LLMNR or mDNS"):
    resolving = machine.succeed("resolvectl status wlan2")
    assert "-LLMNR" in resolving, resolving
    assert "-mDNS" in resolving, resolving

with subtest("trusting it from the shell turns on mDNS, and the card stays on it"):
    since = journal_now()
    ask("Trust", {"name": "Lantern", "trusted": True})
    machine.wait_until_succeeds("resolvectl status wlan2 | grep -qF +mDNS", timeout=30)
    machine.succeed(f"grep -qx MulticastDNS=yes {saved}")
    assert saved_as("Lantern")[0]["trusted"], saved_as("Lantern")
    ask("Trust", {"name": "Lantern", "trusted": False})
    machine.wait_until_succeeds("resolvectl status wlan2 | grep -qF -- -mDNS", timeout=30)
    assert not saved_as("Lantern")[0]["trusted"], saved_as("Lantern")
    holds_address("10.42.0.")
    stayed_on(since)
    refused("Trust", {"name": "Roadside Inn", "trusted": True}, "Declared")
    refused("Trust", {"name": "Nowhere", "trusted": True}, "NotSaved")

# WARP's own rule, at its own priority, for both families.
for family in ("-4", "-6"):
    machine.succeed(f"ip {family} rule add not fwmark 0x100cf lookup 65743 priority 32765")

with subtest("another program's routing rule survives a reconfigure"):
    machine.succeed("networkctl reconfigure wlan2")
    holds_address("10.42.0.")
    rules_stand()

with subtest("it survives moving to another network when one goes away"):
    machine.systemctl("stop lantern")
    joined("Roadside Inn")
    holds_address("10.43.0.")
    on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
    rules_stand()
    answers("front-desk.inn", "10.43.0.1")
    answers("probe.corp.internal", "10.99.0.7")

with subtest("a network sees the same address every time"):
    assert shown() == inn_address, shown()

with subtest("wifid starting again leaves the card on its network"):
    garbled = "/var/lib/wifid/networks/garbled.network"
    machine.succeed(f"printf '[\\377]' > {garbled}")
    since = journal_now()
    machine.succeed("systemctl restart wifid")
    says(f"{garbled}: stream did not contain valid UTF-8")
    says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
    stayed_on(since)
    machine.succeed(f"rm {garbled}")

with subtest("another card coming and going leaves the first on its network"):
    supplicant = machine.succeed("systemctl show -P MainPID wpa_supplicant")
    since = journal_now()
    radio = machine.succeed(
        "ip netns exec drawer iw dev wlan3 info | sed -n 's/^.*wiphy /phy/p'"
    ).strip()
    machine.succeed(f"ip netns exec drawer iw phy {radio} set netns 1")
    machine.wait_until_succeeds("wpa_cli -i wlan3 ping | grep -qx PONG", timeout=30)
    # It's given every network, their passwords opened without systemd.
    retry(
        lambda _: "lighthouse-8" in machine.succeed("wpa_cli -i wlan3 list_networks"),
        timeout_seconds=30,
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
        timeout=30,
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan3 status | grep -qx wpa_state=COMPLETED", timeout=30
    )
    # Joining takes the link down once, to give the card the address it
    # has for the Inn; looking first takes it down no more.
    drops = machine.succeed(
        f"journalctl -o cat --after-cursor='{since}' -u systemd-networkd"
        " | grep -c 'wlan3: Link DOWN' || true"
    )
    assert int(drops) <= 1, drops
    machine.succeed(f"iw phy {radio} set netns name drawer")
    machine.wait_until_succeeds("! test -e /sys/class/net/wlan3", timeout=30)
    assert machine.succeed("systemctl show -P MainPID wpa_supplicant") == supplicant
    stayed_on(since)
    joined("Roadside Inn")

with subtest("and networkd restarting"):
    machine.succeed("systemctl restart systemd-networkd")
    holds_address("10.43.0.")
    rules_stand()
    answers("probe.corp.internal", "10.99.0.7")

with subtest("wifid adds the card again when wpa_supplicant restarts"):
    machine.succeed("systemctl restart wpa_supplicant")
    machine.wait_until_succeeds("wpa_cli -i wlan2 ping | grep -qx PONG", timeout=30)
    joined("Roadside Inn")
    holds_address("10.43.0.")

with subtest("the check asks the network's own DNS server while a VPN's stand in"):
    # WARP points the link at its own servers while it's connected.
    machine.succeed("resolvectl dns wlan2 192.0.2.53")
    machine.succeed("resolvectl dns wlan2 | grep -q '192.0.2.53$'")
    machine.succeed("systemctl restart wifid")
    says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
    # The sign-in window looks names up the same way, through wifid.
    found = ask("Lookup", {"card": "wlan2", "name": "front-desk.inn"})
    assert found == {"addresses": ["10.43.0.1"]}, found
    refused("Lookup", {"card": "wlan9", "name": "front-desk.inn"}, "NotJoined")
    machine.succeed("resolvectl revert wlan2")

with subtest("forgetting Lantern takes it from everywhere, and the card stays on the Inn"):
    since = journal_now()
    ask("Forget", {"name": "Lantern"})
    for gone in (saved, given, "/var/lib/wifid/networks/lantern.sealed"):
        machine.succeed(f"! test -e {gone}")
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "Roadside Inn" in held, held
    assert "Lantern" not in held, held
    assert saved_as("Lantern") == [], saved_as("Lantern")
    stayed_on(since)
    refused("Forget", {"name": "Lantern"}, "NotSaved")
    refused("Forget", {"name": "Roadside Inn"}, "Declared")
    joined("Roadside Inn")

with subtest("networkd's file for a network no longer saved is taken away"):
    gone = "/run/systemd/network/50-wifi-gone.network"
    machine.succeed(
        f"printf '# Written by wifid from /var/lib/wifid/networks/gone.network\\n' > {gone}"
    )
    machine.succeed("systemctl restart wifid")
    machine.wait_for_unit("wifid.service")
    machine.wait_until_succeeds(f"! test -e {gone}", timeout=30)

with subtest("signing in at the Inn is noticed when asked, and the shell told"):
    machine.succeed(
        f"runuser -u guest -- varlinkctl -E --json=short call {wifi}"
        " io.jitsusama.Wifi.Describe '{}' > /tmp/described 2>&1 &"
    )
    machine.wait_until_succeeds("grep -q signIn /tmp/described", timeout=10)
    machine.succeed("curl -sf --interface wlan2 http://front-desk.inn/sign-in")
    ask("Check")
    # Well before the half-minute the Inn would otherwise wait.
    says("Roadside Inn on wlan2 reaches beyond itself", timeout=15)
    machine.wait_until_succeeds(
        'tail -n1 /tmp/described | grep -qF \'"reach":"online"\'', timeout=10
    )

with subtest("a network letting only the plain check through reaches no further"):
    machine.succeed("systemctl restart wifid")
    says("Roadside Inn on wlan2 reaches no further")


def hears(name: str) -> bool:
    """Have the cards look, and say whether they hear the network."""
    ask("Scan")
    return any(network["name"] == name for network in ask("Describe")["nearby"])


def joining(parameters: Json) -> tuple[int, list[str], str]:
    """Join as the launcher does, giving each step wifid said, or why not."""
    # With more, and no timeout of varlinkctl's own: wifid's is the join's.
    call = calling("Join", parameters).replace("call", "call -E --json=short", 1)
    status, said = machine.execute(f"{call} 2>&1")
    # An error's parameters are printed too, and have no stage.
    replies = [json.loads(line) for line in said.splitlines() if line.startswith("{")]
    stages = [reply["stage"] for reply in replies if "stage" in reply]
    return status, stages, said


sealed = "/var/lib/wifid/networks/lantern.sealed"

with subtest("joining Lantern from the launcher with the wrong password saves nothing"):
    machine.systemctl("start lantern")
    retry(lambda _: hears("Lantern"), timeout_seconds=60)
    refused("Join", {"name": "Lantern"}, "NeedsPassword")
    refused("Join", {"name": "Nowhere"}, "NotNearby")
    status, stages, said = joining({"name": "Lantern", "password": "not the password"})
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.WrongPassword" in said, said
    assert stages[:1] == ["joining"], said
    assert "online" not in stages, said
    assert saved_as("Lantern") == [], saved_as("Lantern")
    machine.succeed(f"! test -e {sealed}")
    joined("Roadside Inn")

with subtest("and with the right one, it's saved sealed and said joined step by step"):
    since = journal_now()
    status, stages, said = joining({"name": "Lantern", "password": "glowing in the dark"})
    assert status == 0, said
    # Lantern's session ran out long ago, so it wants a sign-in again.
    assert stages[0] == "joining", said
    assert stages[-1] == "signIn", said
    joined("Lantern")
    # By WPA3, which the card prefers where the network offers both.
    machine.succeed("wpa_cli -i wlan2 status | grep -qx key_mgmt=SAE")
    holds_address("10.42.0.")
    lantern_saved = saved_as("Lantern")
    assert lantern_saved, lantern_saved
    assert not lantern_saved[0]["declared"], lantern_saved
    held = machine.succeed(f"cat {sealed}")
    assert "glowing" not in held, held
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "[DISABLED]" not in held, held
    # Sealed and opened by the vault, so nothing asked systemd's
    # credential service, which takes the TPM seconds a password.
    machine.fail(f"journalctl -o cat --after-cursor='{since}' | grep systemd-creds@")

with subtest("joining the Inn again moves the card, and says how far the Inn reaches"):
    status, stages, said = joining({"name": "Roadside Inn"})
    assert status == 0, said
    assert stages[-1] == "limited", said
    joined("Roadside Inn")

with subtest("the Office is joined with an account, its server first proving itself"):
    retry(lambda _: hears("Office"), timeout_seconds=60)
    nearby = {network["name"]: network for network in ask("Describe")["nearby"]}
    assert nearby["Office"]["security"] == "enterprise", nearby
    refused("Join", {"name": "Office", "password": "hunter22"}, "NeedsAccount")
    # A server naming another domain is never given the password.
    elsewhere = {"method": "peap", "identity": "joel", "serverDomain": "elsewhere.test"}
    status, _, said = joining({"name": "Office", "password": "hunter22", "account": elsewhere})
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.ServerUnproven" in said, said
    joel = {"method": "peap", "identity": "joel", "serverDomain": "radius.office.test"}
    status, _, said = joining({"name": "Office", "password": "not it", "account": joel})
    assert status != 0, said
    assert "failed: io.jitsusama.Wifi.WrongPassword" in said, said
    assert saved_as("Office") == [], saved_as("Office")
    status, stages, said = joining({"name": "Office", "password": "hunter22", "account": joel})
    assert status == 0, said
    joined("Office")
    holds_address("10.44.0.")
    kept = machine.succeed("cat /var/lib/wifid/networks/office.network")
    assert "ServerDomain=radius.office.test" in kept, kept
    assert "hunter22" not in kept, kept
    # The steps after this start on the Inn.
    status, _, said = joining({"name": "Roadside Inn"})
    assert status == 0, said
    joined("Roadside Inn")


def on_network(name: str) -> bool:
    """Say whether wifid tells the shell a card is on the network."""
    return any(card["network"] == name for card in ask("Describe")["cards"])


with subtest("leaving the Inn from the shell keeps the card off it until it's joined"):
    ask("Leave", {"name": "Roadside Inn"})
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
    )
    retry(lambda _: not on_network("Roadside Inn"), timeout_seconds=30)
    refused("Leave", {"name": "Roadside Inn"}, "NotJoined")
    machine.sleep(10)
    machine.succeed("wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED")
    status, stages, said = joining({"name": "Roadside Inn"})
    assert status == 0, said
    joined("Roadside Inn")

with subtest("the shell is told whether a cable reaches beyond the machine"):
    # The test's own network and QEMU's are both cables here.
    links = machine.succeed("networkctl list --no-legend")
    cables = [line.split()[1] for line in links.splitlines() if line.split()[2] == "ether"]
    assert "routable" in links, links
    assert cables, links
    retry(lambda _: ask("Describe")["wired"] is True, timeout_seconds=30)
    for cable in cables:
        machine.succeed(f"ip link set {cable} down")
    retry(lambda _: ask("Describe")["wired"] is False, timeout_seconds=30)

with subtest("wifid says it's settled once the card's online, or Wi-Fi's off"):
    settled = calling("Settled", {}).replace("call", "call --timeout=5", 1)
    machine.succeed(settled)
    ask("Leave", {"name": "Roadside Inn"})
    retry(lambda _: not on_network("Roadside Inn"), timeout_seconds=30)
    # The Inn is still heard, so the card has more to join, and wifid
    # answers everything else while the wait goes on.
    machine.succeed(
        "systemd-run --unit=settling --uid=guest --property=RemainAfterExit=yes"
        " $(command -v varlinkctl) call --timeout=5 /run/wifid/io.jitsusama.Wifi"
        " io.jitsusama.Wifi.Settled '{}'"
    )
    assert ask("Describe")["cards"], ask("Describe")
    machine.wait_until_succeeds("systemctl is-failed -q settling", timeout=15)
    machine.succeed("journalctl -u settling -o cat | grep -q 'Timer expired'")
    machine.succeed("systemctl reset-failed settling")
    ask("Radio", {"on": False})
    machine.succeed(settled)
    ask("Radio", {"on": True})
    retry(lambda _: ",UP" in machine.succeed("ip link show wlan2"), timeout_seconds=30)
    status, stages, said = joining({"name": "Roadside Inn"})
    assert status == 0, said
    machine.succeed(settled)

with subtest("and the cables come back"):
    for cable in cables:
        machine.succeed(f"ip link set {cable} up")
    retry(lambda _: ask("Describe")["wired"] is True, timeout_seconds=60)

with subtest("turning Wi-Fi off from the shell switches the radios off, and on again"):
    assert ask("Describe")["radio"], ask("Describe")
    # Off its network first, so nothing but wifid has networkd set the
    # card's link up again once the radios are back.
    ask("Leave", {"name": "Roadside Inn"})
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
    )
    ask("Radio", {"on": False})
    machine.wait_until_succeeds("rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=10)
    retry(lambda _: ask("Describe")["radio"] is False, timeout_seconds=30)
    ask("Radio", {"on": True})
    machine.wait_until_succeeds("! rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=10)
    retry(lambda _: ask("Describe")["radio"] is True, timeout_seconds=30)
    retry(lambda _: ",UP" in machine.succeed("ip link show wlan2"), timeout_seconds=30)
    status, stages, said = joining({"name": "Roadside Inn"})
    assert status == 0, said
    joined("Roadside Inn")

with subtest("a network saved from the shell without joining it is kept, sealed"):
    since = journal_now()
    hall = {
        "name": "Hidden Hall",
        "security": "personal",
        "password": "under the stairs",
        "hidden": True,
        "priority": 5,
        "address": "card",
    }
    ask("Save", {"network": hall})
    assert saved_as("Hidden Hall") == [
        {
            "name": "Hidden Hall",
            "security": "personal",
            "declared": False,
            "trusted": False,
            "priority": 5,
            "hidden": True,
        }
    ], saved_as("Hidden Hall")
    held = machine.succeed("cat /var/lib/wifid/networks/hidden-hall.sealed")
    assert held, held
    assert "stairs" not in held, held
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "Hidden Hall" in held, held
    stayed_on(since)
    inn_network = {"name": "Roadside Inn", "security": "open"}
    refused("Save", {"network": inn_network}, "Declared")
    nameless = {"name": "Nameless", "security": "personal"}
    refused("Save", {"network": nameless}, "NeedsPassword")

# A radio here never hangs a look, so the card is stopped where it
# stands, which wpa_supplicant then leaves alone, and said to be
# scanning, in the signal wifid hears.
with subtest("a card scanning too long with a saved network in range is told to join it"):
    machine.succeed("wpa_cli -i wlan2 disconnect")
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
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
    machine.sleep(10)
    machine.succeed("wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED")
    # Which saved network it hears best varies from run to run among
    # the several here, and the card joins whichever it can.
    machine.wait_until_succeeds(
        "journalctl -o cat _PID=$(systemctl show -P MainPID wifid) | grep -qxE"
        " 'wlan2 looked for networks for 20 s with .+ in range; asking it to join'",
        timeout=30,
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=COMPLETED", timeout=30
    )

# On a laptop at boot, the access point wpa_supplicant picked didn't
# answer, and the card then sat off every network, with nothing pending,
# until Joel joined by hand. Here the card starts off every network, as
# at boot, with every saved one on and Lantern ranked first, and Lantern
# stops answering while the card's last look still holds it.
with subtest("a card whose access point doesn't answer moves on to another network"):
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
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
    )
    retry(lambda _: hears("Lantern"), timeout_seconds=60)
    since = journal_now()
    machine.succeed("systemctl stop lantern && wpa_cli -i wlan2 reconnect")
    machine.wait_until_succeeds(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | grep -q 'wlan2: authentication with 02:00:00:00:00:00 timed out'",
        timeout=30,
    )
    machine.wait_until_succeeds(
        "wpa_cli -i wlan2 status | grep -qx wpa_state=COMPLETED", timeout=30
    )
    machine.succeed(
        f"wpa_cli -i wlan2 set_network {lantern} priority {ranked} && systemctl start lantern"
    )

with subtest("and says each step of the join and each look, for a join that dawdles"):
    for said in ["wlan2 is scanning", "wlan2 is completed", "wlan2 is done looking"]:
        machine.wait_until_succeeds(
            f"journalctl -o cat _PID=$(systemctl show -P MainPID wifid) | grep -qx '{said}'",
            timeout=30,
        )

# On a laptop at boot, the access point first picked ignored the first
# join, while another of the same network answered. Trying the silent
# one again failed too, and a full look came before the other was
# tried, six seconds there. Here Lantern has a second, weaker access
# point, so the card picks the first, which stops answering; the card
# goes to the second from its last look, without looking again.
with subtest("a card whose access point doesn't answer moves to another of its network at once"):
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
        "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
    )

    def hears_both(_: bool) -> bool:
        """Say whether the last look heard both of Lantern's, the first louder."""
        if not hears("Lantern"):
            return False
        levels: dict[str, int] = {}
        for line in machine.succeed("wpa_cli -i wlan2 scan_results").splitlines():
            fields = line.split("\t")
            if len(fields) == SCAN_COLUMNS and fields[4] == "Lantern":
                levels[fields[0]] = int(fields[2])
        first, second = levels.get("02:00:00:00:00:00"), levels.get("02:00:00:00:05:00")
        return first is not None and second is not None and first > second

    retry(hears_both, timeout_seconds=60)
    since = journal_now()
    machine.succeed("systemctl stop lantern && wpa_cli -i wlan2 reconnect")
    machine.wait_until_succeeds(
        f"journalctl -k -o cat --after-cursor='{since}'"
        " | grep -q 'wlan2: authentication with 02:00:00:00:00:00 timed out'",
        timeout=30,
    )
    machine.wait_until_succeeds(
        "s=$(wpa_cli -i wlan2 status)"
        ' && echo "$s" | grep -qx wpa_state=COMPLETED'
        ' && echo "$s" | grep -qx bssid=02:00:00:00:05:00',
        timeout=30,
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


# The kernel's stand-in battery has two chargers, mains and USB, and the machine is on
# battery only with both pulled.
def chargers(plugged: str) -> None:
    """Plug both of the stand-in battery's chargers in, or pull them."""
    for supply in ["ac", "usb"]:
        machine.succeed(f"echo {plugged} > /sys/module/test_power/parameters/{supply}_online")


def dozes(answer: str) -> None:
    """Wait until the laptop's card says whether it dozes, on or off."""
    machine.wait_until_succeeds(
        f"iw dev wlan2 get power_save | grep -qx 'Power save: {answer}'", timeout=30
    )


with subtest("the card stays awake on mains power, and dozes on battery"):
    chargers("on")
    dozes("off")
    chargers("off")
    dozes("on")
    chargers("on")
    dozes("off")
    machine.succeed("journalctl -t wifi-power-save -o cat | grep -qx 'wlan2: power save on'")

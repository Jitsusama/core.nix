"""The Wi-Fi test's joins-when-asked area.

Here the shell has wifid join, leave, sign in and save, the radios go off and on, a card stuck or
unanswered moves on, and the card dozes only on battery. tests/vm/network/wifi.nix runs this through
pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the test
proves. The tests run in the order written, each taking the machine on from where the one before
left it, so one run alone with -k starts from the boot and may not find what an earlier one set up.
wifid as the tests reach it is wifi_session.py.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import retry
from wifi_session import Wifid, booted, sealed, wifi

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How many columns a line of wpa_cli's scan results has: its access point, frequency,
# level, flags and network.
SCAN_COLUMNS = 5


@pytest.fixture(scope="module")
def wifid(machine: BaseMachine) -> Wifid:
    """Give wifid once the card is on the Inn, which wants a sign-in."""
    wifid = booted(machine)
    wifid.joined("Roadside Inn")
    wifid.says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
    return wifid


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
    # Lantern gives a session afresh to a card it hasn't seen, so it reaches beyond itself.
    assert stages[0] == "joining", said
    assert stages[-1] == "online", said
    wifid.joined("Lantern")
    # By WPA3, which the card prefers where the network offers both.
    machine.succeed("wpa_cli -i wlan2 status | grep -qx key_mgmt=SAE")
    wifid.holds_address("10.42.0.")
    lantern_saved = wifid.saved_as("Lantern")
    assert lantern_saved, lantern_saved
    assert not lantern_saved[0]["declared"], lantern_saved
    held = machine.succeed(f"cat {sealed}")
    assert "glowing" not in held, held
    # Sealed and opened by the vault, so nothing asked systemd's
    # credential service, which takes the TPM seconds a password.
    machine.fail(f"journalctl -o cat --after-cursor='{since}' | grep systemd-creds@")


def test_a_network_joined_from_the_launcher_keeps_the_card_until_it_goes(
    machine: BaseMachine, wifid: Wifid
) -> None:
    """A network joined from the launcher keeps the card as it looks, until the network goes.

    The card's others stay off while it's on Lantern, so a look can't move it to the Inn, ranked
    the same, and come on again once Lantern goes, so the card joins the Inn by itself.
    """
    since = wifid.journal_now()
    assert wifid.hears("Roadside Inn")
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    inn = [line for line in held.splitlines() if "\tRoadside Inn\t" in line]
    assert inn, held
    assert "[DISABLED]" in inn[0], held
    wifid.joined("Lantern")
    wifid.stayed_on(since)
    machine.succeed("systemctl stop lantern")
    wifid.joined("Roadside Inn")
    held = machine.succeed("wpa_cli -i wlan2 list_networks")
    assert "[DISABLED]" not in held, held
    machine.succeed("systemctl start lantern")
    retry(lambda _: wifid.hears("Lantern"), timeout=timedelta(seconds=60))
    # The steps after this start on Lantern, joined from the launcher.
    status, _, said = wifid.joining({"name": "Lantern"})
    assert status == 0, said
    wifid.joined("Lantern")


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

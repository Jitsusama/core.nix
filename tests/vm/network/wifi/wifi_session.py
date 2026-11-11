"""wifid on the Wi-Fi test's machine, as the shell and the test reach it.

Every area of the Wi-Fi test (tests/vm/network/wifi/) imports this, a module of the driver's Python
that tests/vm/network/wifi.nix gives it. Each area boots a machine of its own, so what one area sets
up the others never see.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as wifid gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

# Where Lantern is saved, where wifid gives networkd its file, and where its password is sealed.
saved = "/var/lib/wifid/networks/lantern.network"
given = "/run/systemd/network/50-wifi-lantern.network"
sealed = "/var/lib/wifid/networks/lantern.sealed"
# Where wifid answers.
wifi = "/run/wifid/io.jitsusama.Wifi"


class Wifid:
    """wifid on the machine, as the shell and the test reach it, and what the test noted."""

    def __init__(self, machine: BaseMachine, card: str) -> None:
        """Reach wifid on that machine, whose card has that address of its own."""
        self.machine = machine
        self.card = card
        # The address the card showed the Inn, and the machine's cables, noted once for
        # the tests after.
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


def booted(machine: BaseMachine) -> Wifid:
    """Give wifid once the machine has booted and its networks are up."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("networks.service")
    machine.wait_for_unit("private-dns.service")
    card = machine.succeed("cat /sys/class/net/wlan2/phy80211/macaddress").strip()
    return Wifid(machine, card)

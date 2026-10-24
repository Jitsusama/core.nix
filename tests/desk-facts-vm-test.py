"""desk-facts follows a USB sound card plugged in, pulled out and plugged in again.

tests/desk-facts.nix runs this, on a machine with an xHCI controller named desk and nothing
plugged into it. QEMU's monitor plugs its USB audio device in and takes it out, and each time
sound.json, which desk-facts rewrites only on what it hears, must name the card or no longer
name it. The script needs no value from Nix.
"""

import json
from typing import TYPE_CHECKING

from vm_helpers import as_account

if TYPE_CHECKING:
    from driver_names import machine, subtest

FACTS = "/run/user/1000/desk/sound.json"
PLUG = "device_add usb-audio,id=speakers,bus=desk.0,audiodev=nowhere"


def cards() -> list[str]:
    """Give the cards sound.json names, by their ALSA id and direction."""
    said: dict[str, object] = json.loads(as_account(machine, f"cat {FACTS}"))
    return sorted(said)


def until(*, wanted: bool, what: str) -> list[str]:
    """Wait until sound.json names a card, or names none, and give what it names then."""
    machine.wait_until_succeeds(
        f"test -s {FACTS} && {'' if wanted else '! '}grep -q ':out' {FACTS}", timeout=30
    )
    named = cards()
    machine.log(f"{what}: {named}")
    return named


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
machine.wait_for_unit("desk-facts.service", "joel")

with subtest("nothing plugged in, sound.json names no card"):
    assert until(wanted=False, what="at start") == [], cards()

with subtest("a USB sound card plugged in is named, with what it offers"):
    machine.send_monitor_command(PLUG)
    named = until(wanted=True, what="plugged in")
    assert len(named) == 1, named
    said = json.loads(as_account(machine, f"cat {FACTS}"))[named[0]]
    facts = {fact["name"]: fact["value"] for fact in said}
    assert "offers" in facts, facts

with subtest("pulled out, it's no longer named"):
    machine.send_monitor_command("device_del speakers")
    assert until(wanted=False, what="pulled out") == [], cards()

with subtest("plugged in again, it's named again"):
    # QEMU frees the id only once the guest has let the device go.
    machine.wait_until_succeeds("! ls /proc/asound/card[0-9]", timeout=30)
    machine.send_monitor_command(PLUG)
    assert len(until(wanted=True, what="plugged in again")) == 1, cards()
    as_account(machine, "systemctl --user is-active desk-facts.service", bus=True)

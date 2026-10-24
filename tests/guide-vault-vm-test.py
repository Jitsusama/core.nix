"""A stick's vault sealed to a YubiKey, opened with its PIN, and without it by the passphrase.

tests/guide-vault.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the boot and the helpers, runs
whichever subtests are chosen. What the script needs from Nix, the key's PIN and the stick's
recovery passphrase, comes in one JSON file, whose path guide-vault.nix writes in place of the
marker at the top.
"""

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest

values = json.loads(Path("@values@").read_text())
pin: str = values["pin"]
passphrase: str = values["passphrase"]


def screen() -> str:
    """Give what the guide shows on the first console."""
    return machine.get_tty_text("1")


def on_screen(pattern: str, timeout: int = 60) -> None:
    """Wait until the guide's screen matches the pattern, saying what was running if it never does.

    It fails at once if the guide stopped.
    """

    def shown(_: bool) -> bool:
        """Say whether the screen matches the pattern, or fail if the guide stopped."""
        text = screen()
        if re.search(r"(?i)(try again)", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, text) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout)
        except Exception:
            print("SCREEN\n" + screen())
            _, processes = machine.execute(
                "ps -eo pid,stat,wchan:20,args | grep -i -e guide -e age-plugin"
            )
            print("PROCESSES\n" + processes)
            _, log = machine.execute("tail -n 40 /run/guide/log")
            print("LOG\n" + log)
            raise


def type_in(text: str) -> None:
    """Type the text at the guide, and Return."""
    machine.send_chars(text + "\n")


machine.wait_for_unit("multi-user.target")
machine.wait_until_succeeds('test -n "$(fido2-token -L)"')

with subtest("the key gets a PIN and a recipient, as a person makes one"):
    machine.succeed(f"set-pin {pin}")
    recipient = machine.succeed(f"make-recipient {pin}").strip()
    assert recipient.startswith("age1fido2-hmac1"), recipient

with subtest("the guide seals a stick's vault to it, needing no touch"):
    # A small image with an ESP and one more partition, as core's has.
    machine.succeed(
        "truncate -s 16M /root/image.raw && "
        "printf 'label: gpt\\nsize=4M, type=uefi, name=esp\\nsize=4M, name=store\\n'"
        " | sfdisk /root/image.raw"
    )
    machine.succeed("mkdir -p /root/payload/cache && touch /root/payload/layout.sh")
    target = {
        "machine": "laptop",
        "disk": "/dev/disk/by-id/nvme-the-laptops",
        "disk_name": "system",
        "install_key": "/run/guide/install-key",
        "layout_packages": [],
        "system": "/nix/store/abc",
        "account": "alice",
        "yubikeys": ["the Nano"],
        "recipients": [recipient],
    }
    machine.succeed(f"echo '{json.dumps(target)}' > /root/payload/target.json")
    machine.succeed("truncate -s 120M /root/stick.raw")
    print(
        machine.succeed(
            f"echo '{passphrase}' | guide write-stick /root/image.raw /root/stick.raw"
            " --install /root/payload"
        )
    )
    machine.succeed("losetup --find --partscan --show /root/stick.raw && udevadm settle")
    machine.succeed("test -e /dev/disk/by-partlabel/vault")

with subtest("on the stick, the key and its PIN open the vault"):
    machine.wait_until_succeeds("pgrep -u root -t tty1 bash")
    type_in("guide install")
    on_screen("Check the keyboard")
    type_in("keyboard")
    on_screen("Or plug in the Nano")
    type_in(pin)
    on_screen("vault is open")

with subtest("with the key taken out, the recovery passphrase opens it"):
    machine.send_key("ctrl-c")
    machine.succeed("rm /run/guide/vault-key")
    machine.send_monitor_command("device_del key")
    machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=30)
    type_in("guide install")
    on_screen("the stick's passphrase")
    type_in(passphrase)
    on_screen("vault is open")

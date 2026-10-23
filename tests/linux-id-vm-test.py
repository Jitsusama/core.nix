"""Passkeys and security keys made in a virtual machine's TPM through linux-id, each after a yes.

tests/linux-id.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the boot and the helpers, runs
whichever subtests are chosen. The account's runtime folder is linux-id.nix's, whose askpass
writes there too, so it comes from the JSON file it writes, whose path it puts in place of
the marker below.
"""

import json
import shlex
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# Joel's runtime folder, where the askpass writes down what it's asked, as linux-id.nix has it.
values = json.loads(Path("@values@").read_text())
runtime: str = values["runtime"]


def as_joel(command: str, *, succeed: bool = True) -> str:
    """Run a command as Joel, in his home, and give what it said, failing as it's told to."""
    environment = f"export XDG_RUNTIME_DIR={runtime}; cd ~; "
    run = machine.succeed if succeed else machine.fail
    return run("su - joel -c " + shlex.quote(environment + command))


def asked() -> list[str]:
    """Give what the askpass was asked since this was last read."""
    return as_joel(f"cat {runtime}/asked 2>/dev/null; rm -f {runtime}/asked").splitlines()


def found() -> None:
    """Wait for the security key linux-id makes, as libfido2 finds it."""
    machine.wait_until_succeeds("su - joel -c 'fido2-token -L' | grep -q ^/dev/hidraw", timeout=60)


machine.wait_for_unit("multi-user.target")
machine.wait_for_file(f"{runtime}/bus")
machine.wait_for_unit("default.target", "joel")

with subtest("only the account at the seat may make the device"):
    found()
    t.assertIn("user:joel:rw-", machine.succeed("getfacl -p /dev/uhid"))

with subtest("a refused yes makes no passkey"):
    as_joel(f"touch {runtime}/refuse")
    as_joel("passkey make refused example.com passkey", succeed=False)
    t.assertEqual(asked(), ["confirm FIDO2 Register: example.com"])
    as_joel(f"rm {runtime}/refuse")

with subtest("a passkey is made in the TPM and signs in after a yes"):
    as_joel("passkey make passkey example.com passkey")
    t.assertEqual(asked(), ["confirm FIDO2 Register: example.com"])
    # The site keeps it on the key, so it signs in with no name given.
    as_joel("test -s ~/.config/linux-id/creds.json")
    # linux-id takes a yes as good for five seconds.
    machine.sleep(6)
    as_joel("passkey sign passkey example.com passkey")
    t.assertEqual(asked(), ["confirm FIDO2 Authenticate: example.com"])

with subtest("a security key is added and signs in, asked without the site"):
    as_joel("passkey make security example.org security")
    as_joel("passkey sign security example.org security")
    t.assertEqual(asked(), ["confirm FIDO Confirm Register", "confirm FIDO Confirm Auth"])

with subtest("a site that wants to know who's there is refused, unasked"):
    as_joel("passkey make verified example.net verified", succeed=False)
    t.assertEqual(asked(), [])

with subtest("the keys outlive linux-id, since the TPM holds them"):
    as_joel("systemctl --user restart linux-id")
    # The old device is gone once udev has heard, so the next one found
    # is the new one's.
    machine.succeed("udevadm settle")
    found()
    as_joel("passkey sign passkey example.com passkey")
    as_joel("passkey sign security example.org security")

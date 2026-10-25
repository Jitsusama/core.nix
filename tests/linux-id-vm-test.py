"""Passkeys and security keys made in a virtual machine's TPM through linux-id, each after a yes.

tests/linux-id.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test starts with nothing asked yet, and makes
the keys it signs in with, so any runs alone. The account's runtime folder is linux-id.nix's,
whose askpass writes there too, so it comes from the `values` fixture.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# How long libfido2 may take to find the security key linux-id makes.
FINDING = timedelta(minutes=1)
# linux-id takes a yes as good for five seconds, so a sign-in after this is asked again.
YES_LASTS = timedelta(seconds=6)


class Joel:
    """Joel at the seat, signing in through linux-id, and what his askpass was asked."""

    def __init__(self, machine: BaseMachine, runtime: str) -> None:
        """Act as Joel on that machine, with the runtime folder his askpass writes to."""
        self.machine = machine
        self.runtime = runtime

    def run(self, command: str, *, succeed: bool = True) -> str:
        """Run a command as Joel, in his home, and give what it said; unless succeed, it fails."""
        return as_account(self.machine, command, runtime=self.runtime, succeed=succeed)

    def asked(self) -> list[str]:
        """Give what the askpass was asked since this was last read."""
        return self.run(f"cat {self.runtime}/asked 2>/dev/null; rm -f {self.runtime}/asked").split(
            "\n"
        )[:-1]

    def found(self) -> None:
        """Wait for the security key linux-id makes, as libfido2 finds it."""
        self.machine.wait_until_succeeds(
            "su - joel -c 'fido2-token -L' | grep -q ^/dev/hidraw", timeout=FINDING
        )


@pytest.fixture(scope="module")
def seated(machine: BaseMachine, values: dict[str, Any]) -> Joel:
    """Give Joel signed in at the seat, his services up and linux-id's key found."""
    runtime: str = values["runtime"]
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_file(f"{runtime}/bus")
    machine.wait_for_unit("default.target", "joel")
    joel = Joel(machine, runtime)
    joel.found()
    return joel


@pytest.fixture
def joel(seated: Joel) -> Joel:
    """Give Joel at the seat with nothing asked of him yet."""
    seated.asked()
    return seated


@pytest.fixture
def refusing(joel: Joel) -> Iterator[Joel]:
    """Give Joel refusing every yes he's asked for; he gives them again at the end."""
    joel.run(f"touch {joel.runtime}/refuse")
    yield joel
    joel.run(f"rm -f {joel.runtime}/refuse")


def test_only_the_account_at_the_seat_may_make_the_device(joel: Joel) -> None:
    """Only the account at the seat may make the device."""
    assert "user:joel:rw-" in joel.machine.succeed("getfacl -p /dev/uhid")


def test_a_refused_yes_makes_no_passkey(refusing: Joel) -> None:
    """A refused yes makes no passkey."""
    refusing.run("passkey make refused example.com passkey", succeed=False)
    assert refusing.asked() == ["confirm FIDO2 Register: example.com"]


def test_a_passkey_is_made_in_the_tpm_and_signs_in_after_a_yes(joel: Joel) -> None:
    """A passkey is made in the TPM, and signs in after a yes of its own."""
    joel.run("passkey make passkey example.com passkey")
    assert joel.asked() == ["confirm FIDO2 Register: example.com"]
    # The site keeps it on the key, so it signs in with no name given.
    joel.run("test -s ~/.config/linux-id/creds.json")
    joel.machine.sleep(duration=YES_LASTS)
    joel.run("passkey sign passkey example.com passkey")
    assert joel.asked() == ["confirm FIDO2 Authenticate: example.com"]


def test_a_security_key_is_added_and_signs_in_asked_without_the_site(joel: Joel) -> None:
    """A security key is added and signs in, asked without the site."""
    joel.run("passkey make security example.org security")
    joel.run("passkey sign security example.org security")
    assert joel.asked() == ["confirm FIDO Confirm Register", "confirm FIDO Confirm Auth"]


def test_a_site_that_wants_to_know_whos_there_is_refused_unasked(joel: Joel) -> None:
    """A site that wants to know who's there is refused, unasked."""
    joel.run("passkey make verified example.net verified", succeed=False)
    assert joel.asked() == []


def test_the_keys_outlive_linux_id_since_the_tpm_holds_them(joel: Joel) -> None:
    """The keys outlive linux-id restarting, since the TPM holds them.

    A passkey is found by its site alone, so the one kept here has a site of its own.
    """
    joel.run("passkey make kept example.edu passkey")
    joel.run("passkey make kept-security example.org security")
    joel.run("systemctl --user restart linux-id")
    # The old device is gone once udev has heard, so the next one found is the new one's.
    joel.machine.succeed("udevadm settle")
    joel.found()
    joel.run("passkey sign kept example.edu passkey")
    joel.run("passkey sign kept-security example.org security")

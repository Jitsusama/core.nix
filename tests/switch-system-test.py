"""The machine's administrators switch it with no password, and nobody else does.

tests/switch-system.nix runs this. The machine's system has a specialisation
that adds /etc/switched, so switching to it and back is a switch to another
system and to the first again. An account in wheel switches; an account
outside it, and anything that isn't a NixOS system in the store, is refused,
and the machine is left as it was.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Callable
    from contextlib import AbstractContextManager

    from test_driver.machine import QemuMachine

# The driver runs the script with its machine and its subtests among the
# script's globals.
machine = cast("QemuMachine", globals()["machine"])
subtest = cast("Callable[[str], AbstractContextManager[None]]", globals()["subtest"])


def as_account(account: str, command: str) -> str:
    """Run a command as an account, outside any session, as an agent would."""
    return f"runuser -u {account} -- {command}"


def running() -> str:
    """Give the system the machine is running."""
    return machine.succeed("readlink -f /run/current-system").strip()


def profile() -> str:
    """Give the system the machine boots next."""
    return machine.succeed("readlink -f /nix/var/nix/profiles/system").strip()


machine.wait_for_unit("multi-user.target")
first = running()
switched = machine.succeed(f"readlink -f {first}/specialisation/switched").strip()

with subtest("an administrator switches to another system, with no password"):
    machine.succeed(as_account("someone", f"switch-system {switched}"))
    machine.succeed("test -e /etc/switched")
    assert running() == switched, "it isn't running the system it switched to"
    assert profile() == switched, "it won't boot the system it switched to"

with subtest("an administrator switches back"):
    machine.succeed(as_account("someone", f"switch-system {first}"))
    machine.fail("test -e /etc/switched")
    assert profile() == first, "it won't boot the system it switched back to"

with subtest("an account outside wheel is refused"):
    status, said = machine.execute(as_account("other", f"switch-system {switched} 2>&1"))
    assert status != 0, "an account outside wheel switched"
    assert "authentication required" in said.lower(), f"it wasn't refused by polkit: {said}"
    assert running() == first, "the machine changed"

with subtest("what isn't a NixOS system in the store is refused"):
    bash = machine.succeed("readlink -f /run/current-system/sw/bin/bash").strip()
    package = bash.removesuffix("/bin/bash")
    for target, refusal in [
        (package, "not a NixOS system"),
        (bash, "not a path in the Nix store"),
        ("/etc", "not a path in the Nix store"),
    ]:
        status, said = machine.execute(as_account("someone", f"switch-system {target} 2>&1"))
        assert status != 0, f"it switched to {target}"
        assert refusal in said, f"{target} wasn't refused as {refusal!r}: {said}"
    assert running() == first, "the machine changed"
    assert profile() == first, "the next boot changed"

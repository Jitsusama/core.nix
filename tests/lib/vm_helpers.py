"""What VM tests' scripts share, so each says only what its own test is about.

A test's .nix gives this to its driver through `extraPythonPackages` (vm-helpers.nix), and
mypy finds it on the path core's shared linters set, as it finds driver_names.pyi. Each helper
takes the machine it acts on, since a module can't see the driver's globals.
"""

import shlex
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# Where systemd keeps the first account's runtime folder, its user manager's and PipeWire's
# sockets among them.
FIRST_RUNTIME = "/run/user/1000"


def as_account(  # noqa: PLR0913 # Each keyword is a part of the account's surroundings.
    machine: BaseMachine,
    command: str,
    *,
    account: str = "joel",
    runtime: str = FIRST_RUNTIME,
    bus: bool = False,
    succeed: bool = True,
) -> str:
    """Run a command as an account, in its home, and give what it printed.

    The command gets the account's runtime folder, so it reaches the account's systemd and
    PipeWire, and with bus its session's D-Bus too. Unless succeed, it must fail.
    """
    environment = f"XDG_RUNTIME_DIR={runtime}"
    if bus:
        environment += f" DBUS_SESSION_BUS_ADDRESS=unix:path={runtime}/bus"
    run = machine.succeed if succeed else machine.fail
    return run(f"su - {account} -c " + shlex.quote(f"export {environment}; cd ~; {command}"))


def journal_cursor(said: str) -> str:
    """Give the cursor `journalctl --show-cursor` printed last, to read only what comes after."""
    marker = "-- cursor: "
    if marker not in said:
        message = f"journalctl printed no cursor: {said!r}"
        raise ValueError(message)
    return said.rsplit(marker, 1)[1].strip()

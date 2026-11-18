"""A boot's firmware tables kept where an account reads them, and the missing one left out.

firmware-errors.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The table's body is
the line that file gives QEMU, so the tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# What QEMU's table carries after its header, as firmware-errors.nix gives it.
BODY = "firmware-errors stand-in"
# An account with no say over the machine, reading the copies.
ACCOUNT = "alice"


def kept(machine: BaseMachine) -> str:
    """Give the folder holding this boot's copies, named as systemd's %b names the boot."""
    machine.wait_for_unit("firmware-errors.service")
    boot = machine.succeed("tr -d '-' < /proc/sys/kernel/random/boot_id").strip()
    return f"/var/lib/firmware-errors/{boot}"


def test_an_account_reads_the_boots_health_table(machine: BaseMachine) -> None:
    folder = kept(machine)
    # The header between them holds a length and a checksum, bytes that needn't be text.
    copy = f"{folder}/phat-table"
    signature = machine.succeed(f"su {ACCOUNT} -c 'head -c 4 {copy}'")
    body = machine.succeed(f"su {ACCOUNT} -c 'tail -c {len(BODY)} {copy}'")
    assert signature == "PHAT", f"the copy isn't the table: {signature!r}"
    assert body == BODY, f"the copy lost the table's body: {body!r}"


def test_a_table_the_firmware_lacks_is_left_out(machine: BaseMachine) -> None:
    folder = kept(machine)
    machine.succeed("systemctl is-active firmware-errors.service")
    machine.fail(f"test -e {folder}/bert-region || test -e {folder}/bert-table")

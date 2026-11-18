"""A boot's firmware tables kept where an administrator reads them, and no other account.

firmware-errors.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The health table's
body and the error region's length are what that file gives QEMU, so the tests need no value from
Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# What QEMU's health table carries after its header, as firmware-errors.nix gives it.
BODY = "firmware-errors stand-in"
# How long the error region is, as firmware-errors.nix's BERT says.
REGION = 64
# An administrator, in wheel, and an account that isn't one.
ADMINISTRATOR = "alice"
ACCOUNT = "bob"


def kept(machine: BaseMachine) -> str:
    """Give the folder holding this boot's copies, named as the kernel's boot id is, unhyphened."""
    machine.wait_for_unit("firmware-errors.service")
    boot = machine.succeed("tr -d '-' < /proc/sys/kernel/random/boot_id").strip()
    return f"/var/lib/firmware-errors/{boot}"


def test_an_administrator_reads_the_boots_health_table(machine: BaseMachine) -> None:
    copy = f"{kept(machine)}/phat-table"
    # The header between them holds a length and a checksum, bytes that needn't be text.
    signature = machine.succeed(f"su {ADMINISTRATOR} -c 'head -c 4 {copy}'")
    body = machine.succeed(f"su {ADMINISTRATOR} -c 'tail -c {len(BODY)} {copy}'")
    assert signature == "PHAT", f"the copy isn't the table: {signature!r}"
    assert body == BODY, f"the copy lost the table's body: {body!r}"


def test_an_administrator_reads_the_boots_error_record(machine: BaseMachine) -> None:
    folder = kept(machine)
    signature = machine.succeed(f"su {ADMINISTRATOR} -c 'head -c 4 {folder}/bert-table'")
    region = machine.succeed(f"su {ADMINISTRATOR} -c 'wc -c < {folder}/bert-region'").strip()
    assert signature == "BERT", f"the copy isn't the table: {signature!r}"
    assert region == str(REGION), f"the region the table points at wasn't kept whole: {region}"


def test_an_account_that_isnt_an_administrator_reads_none(machine: BaseMachine) -> None:
    folder = kept(machine)
    machine.fail(f"su {ACCOUNT} -c 'ls {folder}'")
    machine.fail(f"su {ACCOUNT} -c 'head -c 4 {folder}/bert-region'")

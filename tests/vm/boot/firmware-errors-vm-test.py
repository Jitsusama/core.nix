"""A boot's firmware tables kept where an administrator reads them, and no other account.

firmware-errors.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The health table's
body and the error region's length are what that file gives QEMU, so the tests need no value from
Nix.
"""

from __future__ import annotations

import datetime as dt
import json
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# What QEMU's health table carries after its header, as firmware-errors.nix gives it.
BODY = "firmware-errors stand-in"
# How long the error region is, as firmware-errors.nix's BERT says.
REGION = 64
# The machine's serial number, as firmware-errors.nix gives QEMU.
SERIAL = "stand-in-serial"
# An administrator, in wheel, and an account that isn't one.
ADMINISTRATOR = "alice"
ACCOUNT = "bob"
# What docs/commands.md has a command exit with when it may not do what it's asked.
REFUSED = 77
# An error region's status block, before its entries, and where it gives their length.
STATUS_BLOCK = 20
ENTRIES_LENGTH = 12


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
    # What is refused has to be there, and the account able to read what
    # anyone may, or a missing folder or a failing su would pass alone.
    machine.succeed(f"test -s {folder}/bert-region")
    machine.succeed(f"su {ACCOUNT} -c 'ls /var/lib'")
    listed = machine.fail(f"su {ACCOUNT} -c 'ls {folder}' 2>&1")
    read = machine.fail(f"su {ACCOUNT} -c 'head -c 4 {folder}/bert-region' 2>&1")
    assert "Permission denied" in listed, f"listing failed for another reason: {listed!r}"
    assert "Permission denied" in read, f"reading failed for another reason: {read!r}"


def test_the_machines_serial_is_kept_for_an_administrator_alone(machine: BaseMachine) -> None:
    copy = f"{kept(machine)}/product-serial"
    serial = machine.succeed(f"su {ADMINISTRATOR} -c 'cat {copy}'").strip()
    assert serial == SERIAL, f"the copy isn't the machine's serial: {serial!r}"
    machine.fail(f"su {ACCOUNT} -c 'cat {copy}'")


# The kernel works btime out as the clock less the time since boot, each read, and a guest's
# clock settles against the host's in its first seconds: the start the unit kept early in boot
# was 2 s from a later read on 21 November. A few seconds don't change which boot is which.
BTIME_DRIFT = 10


def began(machine: BaseMachine) -> int:
    """Give when the boot began, in seconds since 1970, as the kernel says."""
    return int(machine.succeed("sed -n 's/^btime //p' /proc/stat").strip())


def test_a_later_run_puts_right_when_the_boot_began(machine: BaseMachine) -> None:
    copy = f"{kept(machine)}/began"
    first = machine.succeed(f"cat {copy}").strip()
    kernel = began(machine)
    assert abs(int(first) - kernel) <= BTIME_DRIFT, (
        f"the boot's start was kept as {first}, where the kernel says {kernel}"
    )
    # A switch runs the unit again, and a start kept while the clock was
    # wrong is put right from what the kernel says now. A start no kernel
    # would give stands in for the wrong one.
    machine.succeed(f"echo 1 >{copy}")
    machine.succeed("systemctl restart firmware-errors.service")
    again = int(machine.succeed(f"cat {copy}").strip())
    kernel = began(machine)
    assert abs(again - kernel) <= BTIME_DRIFT, (
        f"a later run left the boot's start as {again}, where the kernel says {kernel}"
    )


def test_an_administrator_is_told_how_each_boot_before_ended(machine: BaseMachine) -> None:
    folder = kept(machine)
    boot = folder.rsplit("/", 1)[1]
    said = machine.succeed(f"su {ADMINISTRATOR} -c 'firmware-errors --format json'")
    answer = json.loads(said)
    assert answer["schema"] == "firmware-errors.boots/1", answer
    current = [kept_boot for kept_boot in answer["boots"] if kept_boot["current"]]
    assert [kept_boot["boot"] for kept_boot in current] == [boot], answer
    this = current[0]
    # It says the start the unit kept, which the test above holds to the kernel's.
    kept_start = int(machine.succeed(f"cat {folder}/began").strip())
    start = dt.datetime.fromtimestamp(kept_start, dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    assert this["began"] == start, f"it says the boot began at {this['began']}, not {start}"
    # The stand-in's health table holds no records, so it names no reset,
    # and the answer says it was kept rather than lost.
    assert this["reset"] is None, this
    assert this["health_table"] == "read", f"the kept health table was said lost: {this!r}"
    # The region is the BIOS's own bytes as the kernel laid them out, so
    # what it holds is read from the copy: no entries is no error, and
    # entries it can't hold are a region cut short. Neither is a reset.
    entries = int(
        machine.succeed(
            f"su {ADMINISTRATOR} -c 'od -A n -t u4 -j {ENTRIES_LENGTH} -N 4 {folder}/bert-region'"
        ).strip()
    )
    found = (this["crash_log"]["state"], this["end"])
    if entries == 0:
        assert found == ("none", "unknown"), f"a region with no entries read as {found}"
    elif STATUS_BLOCK + entries > REGION:
        assert found == ("unreadable", "unreadable"), f"{entries} bytes in {REGION}: {found}"
    else:
        # Entries the region holds whole, which the BIOS's bytes aren't.
        assert found in (("other", "other-error"), ("unreadable", "unreadable")), (
            f"{entries} bytes of the BIOS's own read as {found}"
        )
    assert this["weight"] != "reset", f"the stand-in's bytes read as a reset: {this}"
    listed = machine.succeed(f"su {ADMINISTRATOR} -c 'firmware-errors --format text'")
    assert f"{boot[:8]} now" in listed, listed
    # The text names the boot by the minute it began, on the account's own clock.
    minute = machine.succeed(
        f"su {ADMINISTRATOR} -c \"date -d @{kept_start} '+%Y-%m-%d %H:%M'\""
    ).strip()
    assert minute in listed, f"the listing doesn't say the boot began at {minute}: {listed}"


def test_an_account_that_isnt_an_administrator_is_refused(machine: BaseMachine) -> None:
    kept(machine)
    status, said = machine.execute(f"su {ACCOUNT} -c 'firmware-errors' 2>&1")
    assert status == REFUSED, f"it exited {status}, not {REFUSED}: {said!r}"
    assert "wheel group" in said, f"the refusal doesn't say who may read: {said!r}"
    # It parses what the firmware wrote, so it never sends anyone to root.
    assert "run0" not in said, f"the refusal sends the account to root: {said!r}"


def test_one_boot_is_told_in_full(machine: BaseMachine) -> None:
    boot = kept(machine).rsplit("/", 1)[1]
    said = machine.succeed(f"su {ADMINISTRATOR} -c 'firmware-errors {boot[:8]}'")
    assert boot in said, said
    # Every table was kept, and the machine has no watchdog, so nothing
    # reads as lost: QEMU's health table names no reset and says so.
    assert "not kept" not in said, f"a fact of a boot kept whole reads as lost: {said!r}"
    assert "names no reset" in said, f"the health table naming no reset isn't said: {said!r}"

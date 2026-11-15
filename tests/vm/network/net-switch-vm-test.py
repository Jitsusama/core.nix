"""net's switch reader, run as the machine runs it, against a stand-in switch.

net-switch.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what stands in for the switch. The tests run in order: the reader
first meets no credential, then one is written, as ISXEG8 writes the real one, and the reader
is started again. The password is the test's own, given as the `values` fixture, so every test
looks for the same one.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long the reader may take to write its report once started: it reads at once, each page
# with a five-second limit.
READING = timedelta(minutes=1)
REPORT = "/run/net-switch/report.json"
CREDENTIAL = "/etc/credstore.encrypted/switch-password"


@pytest.fixture(scope="module")
def reader(machines: dict[str, BaseMachine]) -> BaseMachine:
    """Give the machine once its reader and the stand-in switch both run."""
    machines["switch"].start()
    machines["switch"].wait_for_unit("switch-stand-in.service")
    # The stand-in serves on the switch's own address, as a real switch
    # does, which the test's hosts file names after the node.
    machines["switch"].wait_for_open_port(80, "switch")
    reader = machines["machine"]
    reader.start()
    reader.wait_for_unit("net-switch.service")
    return reader


def report(machine: BaseMachine, state: str) -> dict[str, Any]:
    """Wait for the reader's report to say the state, and give it."""
    machine.wait_until_succeeds(f'grep -Eq \'"state": ?"{state}"\' {REPORT}', timeout=READING)
    read: dict[str, Any] = json.loads(machine.succeed(f"cat {REPORT}"))
    return read


def sign_ins(machines: dict[str, BaseMachine], values: dict[str, Any]) -> int:
    """Give how many times the stand-in switch was asked to sign in."""
    return int(machines["switch"].succeed(f"cat {values['counted']}"))


def test_without_a_credential_it_reads_signed_out(
    reader: BaseMachine, machines: dict[str, BaseMachine], values: dict[str, Any]
) -> None:
    """With no credential written, the report says signed out, and it never signs in."""
    read = report(reader, "signed-out")
    assert read["reason"] == "no password is set for it", read
    assert read["status"] is None, read
    assert sign_ins(machines, values) == 0


def test_with_the_credential_it_signs_in_once_and_reads_every_port(
    reader: BaseMachine, machines: dict[str, BaseMachine], values: dict[str, Any]
) -> None:
    """Once the credential is written, a start signs in once and reads the switch's ports."""
    reader.succeed(
        "install -d -m 0700 /etc/credstore.encrypted",
        f"printf %s {shlex.quote(values['password'])}"
        f" | systemd-creds encrypt --name=switch-password - {CREDENTIAL}",
        f"rm -f {REPORT}",
        "systemctl restart net-switch.service",
    )
    read = report(reader, "answering")
    ports = read["status"]["ports"]
    assert [port["port"] for port in ports] == list(range(1, 9)), read
    assert sign_ins(machines, values) == 1


def test_joels_account_reads_the_report_but_not_the_password(reader: BaseMachine) -> None:
    """Uid 1000 reads the report, and neither the credential nor the reader's copy of it."""
    as_joel = "runuser -u joel -- "
    reader.succeed(f"{as_joel} cat {REPORT}")
    reader.fail(f"{as_joel} systemd-creds decrypt {CREDENTIAL} -")
    copy = "/run/credentials/net-switch.service/switch-password"
    reader.succeed(f"test -s {copy}")
    reader.fail(f"{as_joel} cat {copy}")


def test_the_password_is_in_no_report_or_journal_line(
    reader: BaseMachine, values: dict[str, Any]
) -> None:
    """The password shows nowhere the reader writes, nor in any unit's journal."""
    password = shlex.quote(values["password"])
    reader.fail(f"grep -rF {password} /run/net-switch")
    reader.fail(f"journalctl -b --no-pager -o cat | grep -F {password}")

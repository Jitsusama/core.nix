"""How the kernel shares a disk named with a model, in a virtual machine.

measured-disk.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
Each test starts from the machine with app.slice and session.slice held open, since a slice's
cgroup exists only while something runs in it, and the one that stops the disk's sharing comes
last. The sleep the holders run is measured-disk.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

ACCOUNT = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
# The slices the kernel could hold a latency target for, had the disk named none.
TARGETED = [f"{ACCOUNT}/app.slice", f"{ACCOUNT}/session.slice", "/sys/fs/cgroup/user.slice"]
# How long a held slice's unit may take to start.
APPEARING = dt.timedelta(seconds=30)


def kernel(machine: BaseMachine, path: str) -> str:
    """Give what the kernel's file holds, without the trailing newline."""
    return machine.succeed(f"cat {path}").strip()


def cost(machine: BaseMachine, name: str) -> list[str]:
    """Give the settings io.cost holds for the disk, from its model or qos file."""
    vda = kernel(machine, "/sys/class/block/vda/dev")
    lines = kernel(machine, f"/sys/fs/cgroup/io.cost.{name}").splitlines()
    return next(line.split()[1:] for line in lines if line.startswith(f"{vda} "))


@pytest.fixture(scope="module")
def measured(machine: BaseMachine, values: dict[str, Any]) -> BaseMachine:
    """Give the machine once its disk is shared, with app.slice and session.slice held open."""
    machine.wait_for_unit("user@1000.service")
    for name in ["app", "session"]:
        machine.succeed(
            f"systemd-run -M alice@ --user --slice={name}.slice --unit=held-{name}"
            f" {values['sleep']} infinity"
        )
        machine.wait_until_succeeds(
            f"systemctl -M alice@ --user is-active held-{name}.service", timeout=APPEARING
        )
    machine.wait_for_unit("disk-order-vda.service")
    return machine


def test_a_measured_disk_is_shared_by_its_model(measured: BaseMachine) -> None:
    """A measured disk is shared by weight, by the model it was given."""
    model = cost(measured, "model")
    for setting in ["ctrl=user", "rbps=1000000000", "rrandiops=50000", "wseqiops=80000"]:
        assert setting in model, f"io.cost's model lacks {setting}: {model}"
    qos = cost(measured, "qos")
    for setting in ["enable=1", "ctrl=user", "rlat=5000", "wlat=5000"]:
        assert setting in qos, f"io.cost's qos lacks {setting}: {qos}"


def test_with_it_no_slice_holds_a_latency_target(measured: BaseMachine) -> None:
    """With it, no slice holds a latency target."""
    for path in TARGETED:
        assert kernel(measured, f"{path}/io.latency") == "", f"{path} holds a latency target"


def test_old_latency_targets_go_as_it_turns_on(measured: BaseMachine) -> None:
    """Latency targets left from before the disk was measured go as it turns on."""
    # A switch from a machine that had targets keeps them: systemd doesn't write over a
    # target it no longer manages, so they stay in the kernel until a reboot.
    vda = kernel(measured, "/sys/class/block/vda/dev")
    for path in TARGETED:
        measured.succeed(f"echo '{vda} target=1000' > {path}/io.latency")
    measured.succeed("systemctl restart disk-order-vda.service")
    for path in TARGETED:
        assert kernel(measured, f"{path}/io.latency") == "", f"{path} kept its old target"


def test_stopping_it_hands_the_disk_back(measured: BaseMachine) -> None:
    """Stopping it hands the disk back to first come, first served.

    It turns the disk's sharing off, so it comes last.
    """
    measured.succeed("systemctl stop disk-order-vda.service")
    qos = cost(measured, "qos")
    assert "enable=0" in qos, f"io.cost's qos still on: {qos}"

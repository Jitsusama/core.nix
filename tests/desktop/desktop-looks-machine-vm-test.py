"""The desktop test's looks area for Machine and Machines.

Here Machine's things are shot at Joel's two scales, on a screen as large as the laptop's, as the
looks area shoots the launcher's own: 1.5, as the HP draws it, and 2, as the laptop does. Every
figure is the machine daemon's (modules/machine), read from the guest's kernel, or, for Machines,
the realm's mycelium node's record of this machine, so this is where the design looks at what
each gives. What the guest lacks, a USB-C charger, an update offered and a sleep that failed, is
said by stand-ins: a copy of the kernel's folder bound over it, and the stand-in fwupd. It's an
area of its own, rather than more of looks, since the
pyramid keeps a VM test to twenty subtests (docs/testing.md).

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# The scales Joel's screens are drawn at, as niri spells them.
JOELS_SCALES = ("1.5", "2")

# Machine's things, each with a part the machine daemon alone gives, and that part's own parts,
# as the daemon reads the guest's kernel: what moves most leads and the model ends
# (VKCF3Z, 2026-11-07).
MACHINES_PARTS = [
    ("processor", "Time Spent", "Programs"),
    ("memory", "Held By", "Programs"),
    ("disk", "Speed", "Reading"),
    ("uptime", "Asleep", None),
]


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> None:
    """Set the screen's scale, with the launcher closed, so it opens laid out for it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")


def test_machines_parts_are_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot the processor's, memory's, disk's and uptime's parts, and a part's own."""
    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        for asked, part, inside in MACHINES_PARTS:
            desktop.answer(asked)
            desktop.launcher_until(lambda shown: shown["kind"] == "answer")
            desktop.step_in()
            desktop.launcher_until(lambda shown: part in shown["rows"])  # noqa: B023
            desktop.down_to(part)
            desktop.shoot(f"launcher-machine-{asked}-at-{named}")
            if inside is not None:
                desktop.step_in()
                desktop.launcher_until(lambda shown: inside in shown["rows"])  # noqa: B023
                held = part.lower().replace(" ", "-")
                desktop.shoot(f"launcher-machine-{asked}-{held}-at-{named}")
            desktop.close_launcher()


def test_machines_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Machines, this machine's row first, saying how it stands from its node's record.

    The node here links no other machine, so this one is the realm's only row. It has no
    drawer, since Return takes it to the Machine place, whose own places hold its figures;
    another machine's drawer, and one out of reach dimmed, are tests/machines.qml's.
    """
    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("machines", selecting="Machines")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == ["machine"]
                and "NixOS" in shown["selected"]["note"]
                and shown["selected"]["side"] == "this machine"
                and shown["drawer"]["out"] is False
            )
        )
        desktop.shoot(f"launcher-machines-at-{named}")
        desktop.close_launcher()


# A USB-C port charging the machine, as a laptop's UCSI driver says one: the guest has none, so
# a copy of its supplies with this beside them stands over theirs while it's shot.
USB_C_PORT = "ucsi-source-psy-USBC000:001"
USB_C_SAYS = {
    "type": "USB",
    "online": "1",
    "scope": "System",
    "status": "Charging",
    "charge_type": "Trickle",
    "usb_type": "C [PD] PD_PPS",
    "voltage_now": "20000000",
    "voltage_min": "5000000",
    "voltage_max": "20000000",
    "current_now": "2250000",
    "current_max": "3000000",
}
# Sleep's counts after two failures freezing, as Linux keeps them, standing over the guest's.
SLEEP_SAYS = {
    "success": "12",
    "fail": "2",
    "failed_freeze": "2",
    "last_failed_dev": "0000:00:14.0",
    "last_failed_step": "freeze",
    "last_failed_errno": "-16",
    "last_hw_sleep": "3600000000",
    "total_hw_sleep": "25200000000",
    "max_hw_sleep": "523986009990",
}
# An update offered to the BIOS, as fwupdmgr's --json says one, for the stand-in fwupd.
FIRMWARE_OFFERED = {
    "Devices": [
        {
            "DeviceId": "bios",
            "Releases": [
                {
                    "Version": "1.13.0",
                    "Summary": "System Firmware Update",
                    "Description": (
                        "<p>Fixes the keyboard's light.</p><ul><li>Wakes faster</li></ul>"
                    ),
                    "Urgency": 3,
                    "Size": 26518575,
                    "Created": 1788150135,
                    "InstallDuration": 120,
                }
            ],
        }
    ]
}
# Where the stand-in fwupd reads what it answers, in the account's runtime folder.
FWUPD_SAYS = "$XDG_RUNTIME_DIR/fwupd"


def over(folder: str) -> str:
    """Give where a copy stood over one of the kernel's folders is kept."""
    return f"/run/over{folder.replace('/', '-')}"


def stand_over(desktop: Desktop, folder: str, held: str, says: dict[str, str]) -> None:
    """Stand a copy of a folder of the kernel's over it, `held` in it saying `says`.

    `held` is a folder of the copy's, or "" for the copy itself. The copy keeps what the
    folder held, its links read where the folder is, so the rest reads as it did.
    """
    machine = desktop.machine
    copy = over(folder)
    # Some of the kernel's files can't be read, which leaves them out of the copy.
    machine.execute(f"mkdir -p {copy} && cp -a {folder}/. {copy}/")
    into = shlex.quote(f"{copy}/{held}" if held else copy)
    machine.succeed(f"mkdir -p {into}")
    for file, value in says.items():
        machine.succeed(f"echo {shlex.quote(value)} > {into}/{file}")
    machine.succeed(f"mount --bind {copy} {folder}")


def stand_down(desktop: Desktop, folder: str) -> None:
    """Take a copy stood over a folder away, so the kernel's own shows again."""
    desktop.machine.succeed(f"umount {folder} && rm -rf {over(folder)}")


def followed(desktop: Desktop, asked: str, until: Callable[[Json], bool]) -> None:
    """Ask for an answer and follow it into its place, until the place shows what's waited on."""
    desktop.answer(asked)
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.follow()
    desktop.launcher_until(until)


def drawer_says(shown: Json, fact: str) -> bool:
    """Give whether the drawer out beside the selection names the fact."""
    drawer = shown.get("drawer") or {}
    return drawer.get("out") is True and fact in (drawer.get("facts") or [])


def test_heat_and_a_chips_drawer_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Heat's answer, and a chip's drawer once it's stepped into."""
    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        desktop.answer("heat")
        desktop.launcher_until(lambda shown: shown["kind"] == "answer")
        desktop.shoot(f"launcher-heat-at-{named}")
        desktop.step_in()
        desktop.launcher_until(lambda shown: (shown.get("drawer") or {}).get("out") is True)
        desktop.shoot(f"launcher-heat-chip-at-{named}")
        desktop.close_launcher()


def test_a_usb_c_charger_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot the Charger's USB-C port, and its drawer, as a laptop's UCSI driver says one."""
    stand_over(desktop, "/sys/class/power_supply", USB_C_PORT, USB_C_SAYS)
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            followed(desktop, "charger", lambda shown: shown["rows"][:1] == ["Charger"])
            desktop.step_in()
            desktop.launcher_until(lambda shown: drawer_says(shown, "Offered"))
            desktop.shoot(f"launcher-power-usb-c-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        stand_down(desktop, "/sys/class/power_supply")


def test_an_offered_update_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Firmware with an update offered to the BIOS, and its drawer."""
    offered = shlex.quote(json.dumps(FIRMWARE_OFFERED))
    desktop.as_account(f"mkdir -p {FWUPD_SAYS} && echo {offered} > {FWUPD_SAYS}/get-updates.json")
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            followed(
                desktop,
                "firmware",
                lambda shown: (
                    shown["rows"][:1] == ["System Firmware"] and drawer_says(shown, "Urgency")
                ),
            )
            desktop.shoot(f"launcher-firmware-update-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -rf {FWUPD_SAYS}")


def test_sleep_that_failed_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Sleep after two failures freezing, its drawer saying what Linux counted."""
    stand_over(desktop, "/sys/power/suspend_stats", "", SLEEP_SAYS)
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer("sleep")
            desktop.launcher_until(
                lambda shown: (
                    (shown["selected"] or {}).get("name") == "Sleep"
                    and drawer_says(shown, "Failed")
                )
            )
            desktop.shoot(f"launcher-sleep-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        stand_down(desktop, "/sys/power/suspend_stats")

"""core's install, Secure Boot and measured boot walked through in a virtual machine with a TPM.

secure-boot.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Each state a test
starts from is a fixture, and each fixture that boots the machine again asks for the one before
it, in the order the boots come: the keys enrolled at the first boot, the real system installed,
the disk bound to the TPM, its next boot, and a generation passed over. A fixture's work is done
once a run, so any test runs alone with `-k`, booting only as far as it needs. What the tests
need from Nix, the disk image, the systems to install and the secrets typed at the install, are
secure-boot.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import QemuMachine

# A JSON object as bootctl gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

DISK = "/dev/disk/by-partlabel/system"
# How long a unit may take to settle after the boot that starts it.
SETTLE = timedelta(seconds=60)


# Two password agents print each prompt, so an unlock waits for the disk
# to open before anything looks for the next one.
def unlock(machine: QemuMachine, secret: str) -> None:
    """Type the secret at the disk's prompt, and wait for the disk to open."""
    machine.wait_for_console_text("Please enter")
    # The agent that wrote the prompt to the serial console reads it
    # there, where tty1's may not be reading keys yet.
    machine.send_console(secret + "\n")
    # Perhaps coloured; the driver matches across lines joined with nothing
    # between them, so the pattern mustn't span one.
    machine.wait_for_console_text(r"Finished (?:\x1b\[[0-9;]*m)*Cryptography Setup for system")


def entries(machine: QemuMachine) -> list[Json]:
    """Give the boot loader's entries, as bootctl lists them."""
    listed: list[Json] = json.loads(machine.succeed("bootctl list --json=short"))
    return listed


def told(machine: QemuMachine, state: str) -> None:
    """Wait until the session's notice of a fallen-back boot is in the state."""
    machine.wait_until_succeeds(
        'test "$(systemctl --user -M alice@ show -P ActiveState'
        f' boot-fell-back.service)" = {state}',
        timeout=SETTLE,
    )


@pytest.fixture(scope="module")
def overlay(values: dict[str, Any]) -> Generator[str]:
    """Give the overlay the machine writes to, the store's disk image beneath it.

    secure-boot.nix reads it from the environment as QEMU starts, so it's made before the
    machine is.
    """
    with tempfile.NamedTemporaryFile(suffix=".qcow2") as disk:
        # qemu-img and the image are Nix's own store paths, so nothing untrusted reaches
        # the call.
        backing = ["-b", values["image"], "-F", "qcow2"]
        subprocess.run(  # noqa: S603
            [values["qemuImg"], "create", "-f", "qcow2", *backing, disk.name],
            check=True,
        )
        os.environ["NIX_DISK_IMAGE"] = disk.name
        yield disk.name


@pytest.fixture(scope="module")
def enrolled(machine: QemuMachine, overlay: str, values: dict[str, Any]) -> QemuMachine:
    """Boot the machine for the first time, through the reboot that enrols its keys."""
    del overlay
    machine.start(allow_reboot=True)
    unlock(machine, values["passphrase"])
    # Every boot's shell is connected in turn, or a later connection
    # reads an earlier boot's.
    machine.connect()
    # lanzaboote reboots once its keys are on the ESP; systemd-boot then
    # enrols them and the firmware comes back with Secure Boot on.
    machine.wait_for_console_text("Exporting as auth files")
    machine.wait_for_console_text("Linux version")
    # The driver can't see a reboot the machine started itself.
    machine.connected = False
    unlock(machine, values["passphrase"])
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    return machine


@pytest.fixture(scope="module")
def real(enrolled: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Install the real system, and boot it."""
    machine, real = enrolled, values["real"]
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {real}")
    print(machine.succeed(f"{real}/bin/switch-to-configuration boot 2>&1"))
    # Installed with its three tries to count.
    machine.succeed("ls /boot/EFI/Linux/nixos-generation-2-*+3.efi")
    machine.reboot()
    unlock(machine, values["passphrase"])
    machine.wait_for_unit("multi-user.target")
    return machine


@pytest.fixture(scope="module")
def bound(real: QemuMachine, values: dict[str, Any]) -> str:
    """Bind the disk to the TPM and a PIN as the runbook does, and give its recovery key."""
    passphrase, pin = values["passphrase"], values["pin"]
    real.wait_for_unit("systemd-pcrlock-make-policy.service")
    real.succeed(
        f"PASSWORD={passphrase} NEWPIN={pin} systemd-cryptenroll --tpm2-device=auto"
        f" --tpm2-with-pin=yes --tpm2-pcrlock=/var/lib/systemd/pcrlock.json {DISK}"
    )
    recovery = (
        real.succeed(f"PASSWORD={passphrase} systemd-cryptenroll --recovery-key {DISK} 2>/dev/null")
        .strip()
        .split()[-1]
    )
    real.succeed(f"PASSWORD={passphrase} systemd-cryptenroll --wipe-slot=password {DISK}")
    return recovery


@pytest.fixture(scope="module")
def sealed(real: QemuMachine, bound: str, values: dict[str, Any]) -> QemuMachine:
    """Boot the machine again, its disk opened by the TPM and the PIN."""
    del bound
    real.reboot()
    unlock(real, values["pin"])
    real.wait_for_unit("multi-user.target")
    return real


@pytest.fixture(scope="module")
def fallen_back(sealed: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Install a later generation with its tries spent, and boot again past it."""
    machine, later = sealed, values["later"]
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {later}")
    machine.succeed(f"{later}/bin/switch-to-configuration boot")
    # Its three tries spent, as three boots that never came up leave it.
    machine.succeed(
        "cd /boot/EFI/Linux && f=$(ls nixos-generation-3-*+3.efi) && mv $f ${f%+3.efi}+0-3.efi"
    )
    machine.reboot()
    unlock(machine, values["pin"])
    machine.wait_for_unit("multi-user.target")
    return machine


def test_first_boot_makes_the_keys_and_enrols_them(enrolled: QemuMachine) -> None:
    """The first boot makes the keys and reboots to enrol them."""
    status = enrolled.succeed("bootctl status")
    print(status)
    assert "Secure Boot: enabled (user)" in status, status
    print(enrolled.succeed("sbctl verify"))


def test_keys_files_leave_the_esp_once_the_firmware_enforces(enrolled: QemuMachine) -> None:
    """Once the firmware enforces, the keys' files leave the ESP, and none are written again."""
    # Left there, a firmware cleared for a fresh install would have the
    # keys put back by systemd-boot before any disk asked for a PIN.
    enrolled.wait_until_succeeds("test ! -e /boot/loader/keys/auto", timeout=SETTLE)
    enrolled.succeed("journalctl -b -u secure-boot-enrolled.service | grep -q 'Finished'")
    # As a reinstall's new ESP leaves it, its keys carried and enrolled.
    boot = enrolled.succeed("cat /proc/sys/kernel/random/boot_id")
    enrolled.succeed("systemctl start prepare-sb-auto-enroll.service")
    enrolled.succeed("journalctl -b -u prepare-sb-auto-enroll.service | grep -q 'unmet condition'")
    enrolled.succeed("test ! -e /boot/loader/keys/auto")
    enrolled.sleep(timedelta(seconds=5))
    assert enrolled.succeed("cat /proc/sys/kernel/random/boot_id") == boot


def test_option_rom_stops_enrolment_without_checksums(
    enrolled: QemuMachine, values: dict[str, Any]
) -> None:
    """Without checksums, a firmware option ROM stops enrolment."""
    code, output = enrolled.execute(f"{values['optionRomCheck']} 2>&1")
    print(output)
    assert code != 0, output
    assert "Found OptionROM" in output, output
    assert "includeChecksumsFromTPM" in output, output


def test_real_system_installs_with_measured_boot(real: QemuMachine) -> None:
    """The real system installs with measured boot."""
    assert "Secure Boot: enabled (user)" in real.succeed("bootctl status")
    # The boot counts only once the services are up.
    real.wait_for_unit("systemd-bless-boot.service")
    up, blessed = (
        int(real.succeed(f"systemctl show -P {when} {unit}"))
        for when, unit in [
            ("ActiveEnterTimestampMonotonic", "multi-user.target"),
            ("ExecMainStartTimestampMonotonic", "systemd-bless-boot.service"),
        ]
    )
    assert blessed > up, "the boot was blessed before its services were up"


def test_measured_boot_policy_covers_pcrs_0_4_and_7(real: QemuMachine) -> None:
    """The measured-boot policy covers PCRs 0, 4 and 7."""
    real.wait_for_unit("systemd-pcrlock-make-policy.service")
    policy = json.loads(real.succeed("cat /var/lib/systemd/pcrlock.json"))
    pcrs = sorted(value["pcr"] for value in policy["pcrValues"])
    assert pcrs == [0, 4, 7], pcrs


def test_runbook_binds_the_disk_to_the_tpm_and_a_pin(
    real: QemuMachine, bound: str, values: dict[str, Any]
) -> None:
    """The runbook binds the disk to the TPM and a PIN."""
    tokens = real.succeed(f"cryptsetup luksDump --dump-json-metadata {DISK}")
    kinds = sorted(token["type"] for token in json.loads(tokens)["tokens"].values())
    print(kinds)
    assert kinds == ["systemd-recovery", "systemd-tpm2"], kinds
    real.fail(f"echo -n {values['passphrase']} | cryptsetup open --test-passphrase {DISK}")
    real.succeed(f"echo -n {bound} | cryptsetup open --test-passphrase {DISK}")


def test_disk_opens_with_the_tpm_and_the_pin_alone(sealed: QemuMachine) -> None:
    """The disk opens with the TPM and the PIN alone."""
    print(sealed.succeed("journalctl -b -o cat -u systemd-cryptsetup@system.service"))


def test_no_nvpcr_is_set_up_and_a_login_measures_nothing(sealed: QemuMachine) -> None:
    """No NvPCR is set up, and a login measures nothing."""
    # By its name rather than its unit: it exits before journald can
    # tell which unit its lines came from.
    setup = sealed.succeed("journalctl -b -o cat -t systemd-tpm2-setup")
    print(setup)
    assert "SRK already stored in the TPM." in setup, setup
    assert "NvPCR" not in setup, setup
    sealed.fail("ls /boot/loader/credentials/nvpcr-anchor.*.cred")
    sealed.fail("test -s /run/systemd/nvpcr/nvpcr-anchor.cred")
    # Lingering has logind start the user as a first login does.
    sealed.succeed("loginctl enable-linger alice")
    sealed.wait_for_unit("user@1000.service")
    state = sealed.succeed("systemctl show -P LoadState systemd-pcrlogin@1000.service")
    assert state.strip() == "masked", state
    logind = sealed.succeed("journalctl -b -o cat -u systemd-logind.service")
    print(logind)
    assert "measurement" not in logind, logind


def test_generation_counted_from_its_install_is_blessed(sealed: QemuMachine) -> None:
    """A generation counted from its install is blessed once it's up."""
    booted = [entry for entry in entries(sealed) if entry.get("isSelected")]
    print(booted)
    assert len(booted) == 1, booted
    assert "nixos-generation-2-" in booted[0]["id"], booted
    assert "triesLeft" not in booted[0], "the booted generation is still being counted"
    sealed.wait_for_unit("boot-fell-back.service")
    sealed.fail("test -e /run/boot-fell-back")


def test_generation_that_never_boots_is_passed_over(
    fallen_back: QemuMachine, values: dict[str, Any]
) -> None:
    """A generation that never boots is passed over, and the session is told."""
    machine = fallen_back
    assert machine.succeed("readlink -f /run/booted-system").strip() == values["real"]
    machine.wait_for_unit("boot-fell-back.service")
    said = machine.succeed("cat /run/boot-fell-back").strip()
    assert said == "Generation 3 failed to boot 3 times, so this is 2.", said
    # The session hears the file and waits for something to show
    # notices, then tells it.
    machine.succeed("loginctl enable-linger alice")
    machine.wait_for_unit("user@1000.service")
    told(machine, "activating")
    machine.succeed(
        "systemd-run --user -M alice@ --unit notices"
        " /run/current-system/sw/bin/stand-in-notices /home/alice/told"
    )
    told(machine, "active")
    assert machine.succeed("cat /home/alice/told") == (
        f"Boot\nBooted the last good generation\n{said}\n"
    )


FREED = "fwupd has a firmware update waiting"


def policy_pcrs(machine: QemuMachine) -> list[int]:
    """Give the PCRs the TPM's policy covers."""
    policy = json.loads(machine.succeed("cat /var/lib/systemd/pcrlock.json"))
    return sorted({value["pcr"] for value in policy["pcrValues"]})


@pytest.fixture(scope="module")
def updated(fallen_back: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Restart with BootNext on fwupd's entry, as fwupd leaves it with an update waiting."""
    machine = fallen_back
    # No boot so far had an update waiting, so none freed PCR 0.
    machine.fail(f"journalctl -o cat -u tpm-follows-a-firmware-update --grep '{FREED}'")
    # Named as fwupd names its own, and starting the signed systemd-boot
    # rather than fwupd's program, so the restart boots the machine.
    part = machine.succeed("lsblk -no PARTN /dev/disk/by-partlabel/system-boot").strip()
    machine.succeed(
        f"efibootmgr --create --disk /dev/vda --part {part} --label 'Linux Firmware Updater'"
        " --loader '\\EFI\\systemd\\systemd-bootx64.efi'"
    )
    # Boot0004* Linux Firmware Updater<tab>HD(...)
    listed = machine.succeed("efibootmgr").splitlines()
    number = next(line[4:8] for line in listed if "* Linux Firmware Updater\t" in line)
    machine.succeed(f"efibootmgr --bootnext {number}")
    machine.reboot()
    unlock(machine, values["pin"])
    machine.wait_for_unit("multi-user.target")
    machine.succeed(f"efibootmgr --bootnum {number} --delete-bootnum")
    return machine


def test_a_firmware_update_frees_pcr_0_until_the_next_boot(updated: QemuMachine) -> None:
    """Shut down with a firmware update waiting, PCR 0 leaves the policy, and comes back."""
    said = updated.succeed("journalctl -b -1 -o cat -u tpm-follows-a-firmware-update")
    print(said)
    assert FREED in said, said
    # The firmware here doesn't change, so the disk would open either way;
    # the policy made at shutdown is what shows PCR 0 left it.
    assert "PCRs dropped from protection mask: 0 (" in said, said
    # Opened with the PIN alone, then locked to this boot's firmware again.
    print(updated.succeed("journalctl -b -o cat -u systemd-cryptsetup@system.service"))
    updated.wait_for_unit("systemd-pcrlock-make-policy.service")
    assert policy_pcrs(updated) == [0, 4, 7], policy_pcrs(updated)


# Last, since the PCR it changes stays changed until the next boot, and
# after every boot the others ask for.
def test_tpm_refuses_once_a_measured_pcr_changes(
    fallen_back: QemuMachine, values: dict[str, Any]
) -> None:
    """The TPM refuses once a measured PCR changes."""
    machine, pin = fallen_back, values["pin"]
    # A scratch volume bound to the same policy, since the system's own
    # is already open.
    machine.succeed(
        "truncate -s 32M /root/scratch.img && "
        "echo -n scratch | cryptsetup luksFormat --batch-mode /root/scratch.img - && "
        f"PASSWORD=scratch NEWPIN={pin} systemd-cryptenroll --tpm2-device=auto"
        " --tpm2-with-pin=yes --tpm2-pcrlock=/var/lib/systemd/pcrlock.json /root/scratch.img"
    )
    attach = (
        f"PIN={pin} systemd-cryptsetup attach scratch /root/scratch.img -"
        " tpm2-device=auto,headless=yes,password-echo=no"
    )
    machine.succeed(attach)
    machine.succeed("systemd-cryptsetup detach scratch")
    machine.succeed("tpm2_pcrextend 7:sha256=" + "00" * 32)
    machine.fail(attach)

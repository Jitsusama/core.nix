"""A machine installed from its stick by the guide, driven by its keys, in a virtual machine.

guide-install.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. One machine plays
the whole of it, as a laptop does, so each state a test starts from is a fixture that walks the
machine one stage further than the fixture it asks for, in the order a person meets them: the
stick booted, its vault opened, the disk confirmed, each key enrolled, the recovery key kept,
the install stopped and carried on, the system installed, the machine's first boots, the TPM
sealed, the account's keys and spares made, the TPM cleared, a stick written on a host, the
reinstall from the machine's own stick, and the firmware cleared of its keys. A fixture's work
is done once a run, so any test runs alone with `-k`, walking only as far as it needs. What a
screen shows only in passing is checked where it shows, in the fixture that reaches it; what
stays is checked by the tests. What the tests need from Nix, the keys' state, the sticks'
images, the payload, the programs run beside the driver and the secrets typed at the guide, are
guide-install.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import base64
import contextlib
import json
import os
import re
import shlex
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from collections.abc import Generator

    from test_driver.machine import QemuMachine

# systemd saying the disk opened, perhaps coloured. The driver matches across lines joined with
# nothing between them, so nothing here may span one.
OPENED = r"Finished (?:\x1b\[[0-9;]*m)*Cryptography Setup for system"
# How many words a recovery key has, which are all its own screen shows.
RECOVERY_KEY_WORDS = 7
# The range systemd picks a policy's NV index from (src/shared/tpm2-util.h).
UNASSIGNED = range(0x01800000, 0x01BFFFFF + 1)
VOLUME = "/dev/disk/by-partlabel/system"
# The priority LUKS gives a slot tried before the others.
PREFERRED = 2

# Before its keyboard screen, the stick copies its whole system into memory through QEMU's
# USB stick, which on a host busy with other machines' disks took past five minutes. The test
# proves what the screens say and in what order, not how fast.
COPIED = timedelta(minutes=15)
# How long a screen may take to come, a step that works to show the next, a key or the stick
# to come or go, and the system to install.
SCREEN = timedelta(seconds=120)
STEP = timedelta(seconds=300)
PLUGGED = timedelta(seconds=30)
INSTALLED = timedelta(minutes=30)


@dataclass(frozen=True)
class Rig:
    """What QEMU's devices read, made before the machine starts.

    The keys' state files, the overlay the install's stick writes to, and a blank stick that
    stands where it was once it's out.
    """

    keys: str
    stick: str
    blank: str


def screen(machine: QemuMachine) -> str:
    """Give what the guide's console shows."""
    return machine.get_tty_text("1")


# The screen as a person reads it, whatever's capitalised and however
# its lines wrap. A step that stops says so at once, rather than after
# the wait for what it would have shown.
def on_screen(
    machine: QemuMachine,
    pattern: str,
    timeout: timedelta = SCREEN,
    stopped: str = r"try again|look again",
) -> None:
    """Wait until the pattern is on the guide's screen, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the pattern is on the screen, failing if the guide says it stopped."""
        text = screen(machine)
        if re.search(f"(?i)({stopped})", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, " ".join(text.split())) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout=timeout)
        except Exception:
            # What the machine was at when the wait ran out: the
            # screen, the guide's raw log, and the journal's last.
            print("SCREEN\n" + screen(machine))
            for where in ["/run/guide/log", "/var/lib/guide/log"]:
                print(f"LOG {where}\n" + machine.execute(f"tail -n 60 {where}")[1])
            print("JOURNAL\n" + machine.execute("journalctl -b -n 80 -o cat")[1])
            raise


def enter(machine: QemuMachine, text: str) -> None:
    """Type the text at the guide and press Return."""
    machine.send_chars(text + "\n")


def plug(machine: QemuMachine, rig: Rig, name: str) -> None:
    """Take the key out and plug in the one whose state file is named."""
    machine.send_monitor_command("device_del key")
    machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=PLUGGED)
    machine.send_monitor_command(f"device_add canokey,id=key,bus=xhci.0,file={rig.keys}/{name}")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=PLUGGED)


# The stick out of its port and back, as on a laptop with one port
# both it and a YubiKey fit. QEMU drops a drive with its device, so
# the drive is made again before the stick goes back.
def unplug_stick(machine: QemuMachine) -> None:
    """Take the stick out, and wait until its vault is gone."""
    machine.send_monitor_command("device_del stickdev")
    machine.wait_until_succeeds("test ! -e /dev/disk/by-partlabel/vault", timeout=PLUGGED)


def plug_stick_back(machine: QemuMachine, rig: Rig) -> None:
    """Put the stick back in, and wait until its vault is there."""
    print(
        machine.send_monitor_command(f"drive_add 0 if=none,id=stick,format=qcow2,file={rig.stick}")
    )
    print(machine.send_monitor_command("device_add usb-storage,bus=xhci.0,drive=stick,id=stickdev"))
    machine.wait_until_succeeds("test -e /dev/disk/by-partlabel/vault", timeout=PLUGGED)


def fixture(machine: QemuMachine, name: str, command: str) -> None:
    """Print what the command prints on the machine, on one line, as the named fixture.

    It's what the tools print here, for the guide's fixtures (qemu-canokey).
    """
    print(f"FIXTURE {name} " + machine.succeed(command).replace("\n", "\\n"))


# The recovery key, kept as a person keeps it on their phone: the
# steps, then its seven words alone on the screen for the phone's
# camera, nothing else there for it to copy, then typed back.
def keep_the_recovery_key(machine: QemuMachine) -> str:
    """Read the recovery key off its own screen, type it back, and give it."""
    on_screen(machine, "Keep the recovery key", timeout=STEP)
    on_screen(machine, "Copy All")
    enter(machine, "")
    retry(
        lambda _: len(screen(machine).split()) == RECOVERY_KEY_WORDS,
        timeout=timedelta(seconds=30),
    )
    words = " ".join(screen(machine).split())
    enter(machine, "")
    on_screen(machine, "reading it off your phone")
    # The firmware's check and the keys' step each say their own line,
    # though a short screen may have let the earlier go.
    assert screen(machine).count("Secure Boot keys are in place") <= 1, screen(machine)
    enter(machine, words)
    return words


# systemd-cryptsetup asks through libcryptsetup's token plugin first and
# then its own FIDO2 code, and a key that's not there yet at the first
# leaves the second asking, so each prompt is answered until the disk
# opens.
def unlock(machine: QemuMachine, secret: str) -> None:
    """Answer the disk's prompts with the secret until it opens."""
    machine.wait_for_console_text("Please enter")
    for _ in range(4):
        machine.send_console(secret + "\n")
        # A prompt left asking times the wait out, and the next try answers it.
        with contextlib.suppress(Exception):
            machine.wait_for_console_text(OPENED, timeout=timedelta(seconds=30))
            return
    message = "the disk never opened with the key's PIN"
    raise RuntimeError(message)


def unlock_as_asked(machine: QemuMachine, pin: str) -> str:
    """Answer each of the disk's prompts with the one PIN, and say which key took it."""
    asked = r"Please enter (?:LUKS2 token|TPM2|security token) PIN"
    since = len(machine.get_console_log())
    for _ in range(8):
        machine.wait_for_console_text(f"{asked}|{OPENED}")
        boot = machine.get_console_log()[since:]
        if re.search(OPENED, boot):
            # systemd asks a YubiKey only once the TPM has refused.
            return "the key" if "Asking FIDO2 token" in boot else "the TPM"
        machine.send_console(pin + "\n")
    message = "the disk never opened with the one PIN"
    raise RuntimeError(message)


def prompts(boot: str) -> list[str]:
    """Give each prompt the disk put up in a boot's console."""
    return re.findall(r"Please enter [^:(]*:", boot)


def getcap(machine: QemuMachine, values: dict[str, Any], what: str) -> str:
    """Give what the TPM says of this capability, through tpm2-tools in the store."""
    tool = Path(values["tpm2Clear"]).parent / "tpm2_getcap"
    return machine.succeed(f"{tool} -T device:/dev/tpmrm0 {what}")


def policies(machine: QemuMachine, values: dict[str, Any]) -> list[int]:
    """Give the NV indices in the range systemd-pcrlock picks a policy's from."""
    listed = getcap(machine, values, "handles-nv-index")
    # Nothing but systemd-pcrlock defines an index in that range on the
    # VM's TPM, so each is a policy.
    indices = [int(index, 16) for index in re.findall(r"0x[0-9A-Fa-f]+", listed)]
    return [index for index in indices if index in UNASSIGNED]


def lockouts(machine: QemuMachine, values: dict[str, Any]) -> int:
    """Give how many failed authorisations the TPM is counting towards its lockout."""
    shown = getcap(machine, values, "properties-variable")
    counted = re.search(r"TPM2_PT_LOCKOUT_COUNTER: (0x[0-9A-Fa-f]+)", shown)
    if counted is None:
        message = "the TPM didn't say its lockout counter:\n" + shown
        raise RuntimeError(message)
    return int(counted.group(1), 16)


def own_policy(machine: QemuMachine) -> int:
    """Give the NV index this disk's policy file names."""
    policy: int = json.loads(machine.succeed("cat /var/lib/systemd/pcrlock.json"))["nvIndex"]
    return policy


def spare(machine: QemuMachine, name: str) -> str:
    """Give the public half of alice's spare from the YubiKey with this file name."""
    return machine.succeed(f"cat /home/alice/.ssh/yubikey-{name}.pub")


def header(machine: QemuMachine) -> dict[str, Any]:
    """Give the disk's LUKS header, as cryptsetup dumps it."""
    dumped: dict[str, Any] = json.loads(
        machine.succeed(f"cryptsetup luksDump --dump-json-metadata {VOLUME}")
    )
    return dumped


def overlay(qemu_img: str, file: str, *backing: str, size: str | None = None) -> None:
    """Make a qcow2 overlay over a stick's image, or a blank one of this size."""
    # qemu-img and the images are Nix's own store paths or the test's own files, so
    # nothing untrusted reaches the call.
    subprocess.run(  # noqa: S603
        [qemu_img, "create", "-f", "qcow2", *backing, file, *([size] if size else [])],
        check=True,
    )


@pytest.fixture(scope="module")
def rig(values: dict[str, Any]) -> Generator[Rig]:
    """Give the keys' state files and the sticks' overlays, made before QEMU starts.

    guide-install.nix reads each from the environment as QEMU starts. The machine writes to
    the stick's overlay, since its image is in the store.
    """
    with (
        tempfile.TemporaryDirectory() as keys,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as stick,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as blank,
    ):
        for name in ["nano", "fivec"]:
            shutil.copy(f"{values['canokeys']}/{name}", f"{keys}/{name}")
            Path(f"{keys}/{name}").chmod(0o600)
        os.environ["KEYS_DIR"] = keys
        overlay(values["qemuImg"], stick.name, "-b", values["written"], "-F", "raw")
        os.environ["STICK_IMAGE"] = stick.name
        # A blank stick stands where it was once it's out, which the firmware passes over.
        overlay(values["qemuImg"], blank.name, size="64M")
        yield Rig(keys, stick.name, blank.name)


@pytest.fixture(scope="module")
def booted(machine: QemuMachine, rig: Rig) -> QemuMachine:
    """Boot the stick, until its guide checks the keyboard."""
    del rig
    machine.start(allow_reboot=True)
    on_screen(machine, "Check the keyboard types what you mean", timeout=COPIED)
    # Done before the keyboard, and said so above it.
    on_screen(machine, "The stick's system runs from memory")
    return machine


@pytest.fixture(scope="module")
def vault_opened(booted: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Type the keyboard's word, then the stick's passphrase, as named on the phone."""
    enter(booted, "keyboard")
    # This stick was written before any key was enrolled, so only the
    # passphrase opens it; the vault test has a stick keys open too.
    on_screen(
        booted,
        "the stick's passphrase, the seven words in 1Password on your phone named "
        '"NixOS Install Phrase"',
    )
    enter(booted, values["passphrase"])
    return booted


@pytest.fixture(scope="module")
def confirmed(vault_opened: QemuMachine) -> QemuMachine:
    """Type the machine's name back at the disk it names, to be wiped."""
    machine = vault_opened
    on_screen(machine, "Confirm the disk to wipe")
    on_screen(machine, "wipes")
    # The whole install walks again the steps the vault was opened by,
    # and each is listed once.
    assert screen(machine).count("The stick's vault is open") == 1, screen(machine)
    fixture(machine, "lsblk-blank.json", "lsblk --json --output-all --bytes")
    enter(machine, "machine")
    return machine


@pytest.fixture(scope="module")
def nano_enrolled(confirmed: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Enrol the Nano by name, the stick taken out for it."""
    machine = confirmed
    on_screen(machine, "Plug in the Nano", timeout=STEP)
    on_screen(machine, "take the stick out first")
    fixture(machine, "lsblk-laid-out.json", "lsblk --json --output-all --bytes")
    fixture(machine, "fido2-token-L.txt", "fido2-token -L")
    unplug_stick(machine)
    machine.succeed(f"set-pin {values['pin']}")
    enter(machine, "")
    on_screen(machine, "the Nano's PIN")
    enter(machine, values["pin"])
    return machine


@pytest.fixture(scope="module")
def fivec_enrolled(nano_enrolled: QemuMachine, rig: Rig, values: dict[str, Any]) -> QemuMachine:
    """Enrol the 5C by name, swapped in for the Nano with the stick still out."""
    machine = nano_enrolled
    on_screen(machine, "Plug in the 5C", timeout=STEP)
    on_screen(machine, "take the stick out first")
    plug(machine, rig, "fivec")
    machine.succeed(f"set-pin {values['pin']}")
    enter(machine, "")
    on_screen(machine, "the 5C's PIN")
    enter(machine, values["pin"])
    return machine


@pytest.fixture(scope="module")
def recovery_key(fivec_enrolled: QemuMachine) -> str:
    """Keep the recovery key, and give its words."""
    return keep_the_recovery_key(fivec_enrolled)


@pytest.fixture(scope="module")
def stick_back(fivec_enrolled: QemuMachine, rig: Rig, recovery_key: str) -> QemuMachine:
    """Put the stick back once the guide asks for it, and wait for the copy off it to start."""
    del recovery_key
    machine = fivec_enrolled
    on_screen(machine, "Put the stick back in", timeout=STEP, stopped="try again")
    plug_stick_back(machine, rig)
    # The card asking for the stick says "look again" until the guide
    # next looks, up to two seconds after the stick is back.
    on_screen(
        machine,
        r"Copy 1 of 2, the system packed off the stick: [0-9.]+ [KMG]B of ",
        timeout=STEP,
        stopped="try again",
    )
    return machine


@pytest.fixture(scope="module")
def stopped(stick_back: QemuMachine) -> QemuMachine:
    """Press Ctrl+C partway through the copy, and wait for the guide to say how to go on."""
    machine = stick_back
    on_screen(machine, "Ctrl\\+C stop")
    machine.send_key("ctrl-c")
    machine.wait_until_tty_matches("1", "Stopped. Start the guide again", timeout=SCREEN)
    machine.wait_until_tty_matches("1", "type guide install, then press Return", timeout=SCREEN)
    return machine


# A restart as the guide meets it: what the stick held in memory gone, the
# install's key and journal with it, and the disk let go. The system's own
# copies in /run/stick stay, as a restarted stick makes them again.
@pytest.fixture(scope="module")
def carried_on(stopped: QemuMachine, rig: Rig, values: dict[str, Any]) -> QemuMachine:
    """Restart the guide with what the stick held gone, and carry the install on, stick out."""
    machine = stopped
    machine.succeed("umount --recursive /mnt && cryptsetup close system")
    machine.succeed("find /run/guide -maxdepth 1 -type f -delete")
    enter(machine, "guide install")
    on_screen(machine, "An install stopped part way")
    on_screen(machine, "Carry it on from where it stopped")
    enter(machine, "")
    on_screen(machine, "Check the keyboard types what you mean")
    enter(machine, "keyboard")
    on_screen(machine, "the stick's passphrase")
    enter(machine, values["passphrase"])
    on_screen(machine, "Open the install that stopped")
    on_screen(machine, "Plug in the Nano or the 5C, which opened the disk")
    # The stick out for the key, as on a laptop whose one port takes either,
    # so the disk opens from what the guide held when the vault opened.
    unplug_stick(machine)
    enter(machine, "")
    on_screen(machine, "its PIN")
    enter(machine, values["pin"])
    on_screen(machine, "Put the stick back in", timeout=STEP, stopped="try again")
    plug_stick_back(machine, rig)
    on_screen(
        machine,
        r"Copy 1 of 2, the system packed off the stick: [0-9.]+ [KMG]B of ",
        timeout=STEP,
        stopped="try again",
    )
    return machine


@pytest.fixture(scope="module")
def installed(carried_on: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Let the system install, and give the account its password, twice."""
    machine = carried_on
    on_screen(machine, "a password for alice", timeout=INSTALLED)
    enter(machine, values["password"])
    on_screen(machine, "the same password again")
    enter(machine, values["password"])
    # The reboot card's own key, which nothing before it shows.
    on_screen(machine, "shut down", timeout=STEP)
    fixture(
        machine,
        "luks-dump-installed.json",
        f"cryptsetup luksDump --dump-json-metadata {VOLUME}",
    )
    return machine


@pytest.fixture(scope="module")
def shut_down(installed: QemuMachine) -> QemuMachine:
    """Read the reboot card, and press Return to shut the machine down."""
    on_screen(installed, "security token PIN")
    enter(installed, "")
    installed.wait_for_shutdown()
    return installed


@pytest.fixture(scope="module")
def first_boot(shut_down: QemuMachine, rig: Rig, values: dict[str, Any]) -> QemuMachine:
    """Boot the machine's own disk with the stick out, opening it with a key's PIN."""
    machine = shut_down
    os.environ["STICK_IMAGE"] = rig.blank
    machine.start(allow_reboot=True)
    # systemd-boot enrols the keys the stick readied and restarts before
    # any kernel starts, so the disk asks only once, with Secure Boot on.
    unlock(machine, values["pin"])
    machine.connect()
    # The guide, at boot, holds multi-user.target back until it's done.
    machine.wait_until_succeeds(
        "systemctl show -P ActiveState guide-installed | grep -qx activating"
    )
    return machine


@pytest.fixture(scope="module")
def tpm_sealed(first_boot: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Let a YubiKey open the disk for the guide to seal it to the TPM, with the one PIN."""
    machine = first_boot
    on_screen(machine, "Plug in a YubiKey, which opens the disk for the TPM")
    # The units still starting don't write across it.
    assert machine.succeed("systemctl show -P ShowStatus").strip() == "no"
    # Held awake while the guide works: sleep, idling and the lid.
    held = machine.succeed("systemd-inhibit --list --no-legend --no-pager")
    assert re.search(
        r"guide .*sleep:idle:handle-lid-switch +finishing this machine +block", held
    ), held
    # Whatever writes over the screen as it waits, as the kernel does
    # when a device comes or goes, is undone within seconds.
    machine.succeed("for _ in $(seq 40); do echo scribbled; done > /dev/tty1")
    on_screen(
        machine,
        "Plug in a YubiKey, which opens the disk for the TPM",
        timeout=timedelta(seconds=10),
    )
    assert "scribbled" not in screen(machine), screen(machine)
    enter(machine, "")
    on_screen(machine, "which the TPM takes too")
    enter(machine, values["pin"])
    # Sealed, the guide goes on to the account's keys.
    on_screen(machine, "a PIN for alice's SSH keys", timeout=STEP)
    return machine


@pytest.fixture(scope="module")
def ssh_keys(tpm_sealed: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Give alice's SSH keys their PIN, twice."""
    enter(tpm_sealed, values["keyPin"])
    on_screen(tpm_sealed, "the same PIN again")
    enter(tpm_sealed, values["keyPin"])
    # Made, the guide goes on to the YubiKeys' spares.
    on_screen(tpm_sealed, "Plug in the Nano", timeout=STEP)
    return tpm_sealed


@pytest.fixture(scope="module")
def spares(ssh_keys: QemuMachine, rig: Rig, values: dict[str, Any]) -> str:
    """Keep a spare on each YubiKey, the Nano refused as the 5C, and give the last screen."""
    machine, pin = ssh_keys, values["pin"]
    plug(machine, rig, "nano")
    enter(machine, "")
    on_screen(machine, "the Nano's PIN")
    enter(machine, pin)
    on_screen(machine, "Plug in the 5C", timeout=STEP)
    # The Nano still in, which the guide met as the Nano.
    enter(machine, "")
    on_screen(machine, "the 5C's PIN")
    enter(machine, pin)
    on_screen(machine, "this key is the Nano, not the 5C", timeout=STEP, stopped="look again")
    plug(machine, rig, "fivec")
    enter(machine, "")
    # Asked again, as the guide looks again after Return; until it has,
    # the refusal stays on the screen, "try again" and all.
    on_screen(machine, "Plug in the 5C, and only the 5C", stopped="look again")
    enter(machine, "")
    on_screen(machine, "the 5C's PIN")
    enter(machine, pin)
    on_screen(machine, "machine is finished", timeout=STEP)
    # Root from the boot, with no password, and the sign-in waits.
    machine.succeed("pgrep -u root -xf '.*/guide installed'")
    assert machine.succeed("systemctl show -P ActiveState getty@tty1").strip() == "inactive"
    return " ".join(screen(machine).split()).lower()


@pytest.fixture(scope="module")
def signed_in(ssh_keys: QemuMachine, spares: str) -> QemuMachine:
    """Press Return on the last screen, which signs in rather than restarting."""
    del spares
    enter(ssh_keys, "")
    ssh_keys.wait_for_unit("getty@tty1.service")
    return ssh_keys


@pytest.fixture(scope="module")
def restarted(signed_in: QemuMachine, values: dict[str, Any]) -> str:
    """Restart the machine, and give what its console said as it booted."""
    machine = signed_in
    before = len(machine.get_console_log())
    machine.reboot()
    unlock(machine, values["pin"])
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    # With nothing to do, the guide showed nothing and left.
    machine.wait_for_unit("getty@tty1.service")
    return machine.get_console_log()[before:]


@dataclass(frozen=True)
class Cleared:
    """What the TPM held before it was cleared, and the boot after the guide sealed it again."""

    nano: str
    before: dict[str, str]
    boot: str


TPM_KEYS = ["/home/alice/.ssh/id_ecdsa", "/home/alice/.ssh/agent-signing"]


@pytest.fixture(scope="module")
def cleared(signed_in: QemuMachine, restarted: str, rig: Rig, values: dict[str, Any]) -> Cleared:
    """Clear the TPM and the Nano's spare, let the guide put both back, then restart."""
    del restarted
    machine, pin, key_pin = signed_in, values["pin"], values["keyPin"]
    # The guide sees nothing to do, until the TPM is cleared, which
    # takes the NV index the disk's pcrlock policy lives in, and the seed
    # alice's keys were sealed under. The Nano's spare goes too, for the
    # guide to bring back off the Nano.
    nano = spare(machine, "nano")
    before = {file: machine.succeed(f"cat {file}.pub") for file in TPM_KEYS}
    machine.succeed("rm /home/alice/.ssh/yubikey-nano /home/alice/.ssh/yubikey-nano.pub")
    machine.succeed(f"{values['tpm2Clear']} -T device:/dev/tpmrm0 -c l")
    machine.reboot()
    unlock(machine, pin)
    machine.connect()
    on_screen(machine, "Plug in a YubiKey, which opens the disk for the TPM")
    enter(machine, "")
    on_screen(machine, "which the TPM takes too")
    enter(machine, pin)
    # The TPM refuses the keys it sealed before, so the guide makes them
    # again, saying why.
    on_screen(machine, "a PIN for alice's new SSH keys", timeout=STEP)
    assert "the tpm was cleared" in " ".join(screen(machine).split()).lower(), screen(machine)
    enter(machine, key_pin)
    on_screen(machine, "the same PIN again")
    enter(machine, key_pin)
    # The 5C is in from its spare, so the Nano goes in for its own.
    on_screen(machine, "Plug in the Nano", timeout=STEP)
    plug(machine, rig, "nano")
    enter(machine, "")
    on_screen(machine, "the Nano's PIN")
    enter(machine, pin)
    on_screen(machine, "machine is finished", timeout=STEP)
    # Nothing the guide loaded the keys in is left.
    machine.fail("test -e /run/guide/tpm-key")
    enter(machine, "")
    machine.wait_for_unit("getty@tty1.service")
    since = len(machine.get_console_log())
    machine.reboot()
    unlock(machine, pin)
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    return Cleared(nano, before, machine.get_console_log()[since:])


@dataclass(frozen=True)
class HostStick:
    """The rescue bundle the install sealed, as a host carried it onto a stick, and its owner."""

    owner: str
    installed: dict[str, Any]
    own: dict[str, Any]


@pytest.fixture(scope="module")
def host_stick(
    signed_in: QemuMachine,
    cleared: Cleared,
    recovery_key: str,
    values: dict[str, Any],
) -> Generator[HostStick]:
    """Write a stick on the host from the install's bundle, boot it, then shut down."""
    del cleared
    machine, payload = signed_in, values["payload"]
    # The bundle the install made, taken off as alice can, sealed.
    sealed = base64.b64decode(
        machine.succeed("su alice -c 'base64 -w0 /var/lib/guide/rescue-bundle.json'")
    )
    assert b"PRIVATE KEY" not in sealed
    installed = json.loads(sealed)
    owner = machine.succeed("cat /var/lib/sbctl/GUID").strip()
    # A machine makes one with a passphrase of its own too, as root,
    # holding the same keys.
    target = base64.b64encode(Path(f"{payload}/target.json").read_bytes()).decode()
    machine.succeed(f"echo {target} | base64 -d > /root/target.json")
    said = machine.succeed(
        f"echo {values['bundlePassphrase']} | guide seal-keys /root/target.json /root/rescue.json"
    )
    assert owner in said, said
    machine.succeed("test $(stat -c %a /root/rescue.json) = 600")
    own = json.loads(machine.succeed("cat /root/rescue.json"))
    with (
        tempfile.TemporaryDirectory() as folder,
        tempfile.NamedTemporaryFile(suffix=".qcow2") as written_on_host,
    ):
        bundle = Path(folder) / "rescue.json"
        bundle.write_bytes(sealed)
        again = Path(folder) / "reinstall.raw"
        with again.open("wb") as stick_file:
            stick_file.truncate(Path(values["written"]).stat().st_size)
        # The stick's passphrase, then the disk's recovery key, which
        # opens the bundle the install sealed to it. The guide, its images
        # and payload are Nix's own store paths, and the rest the test's
        # own files, so nothing untrusted reaches the call.
        sources = ["--install", payload, "--rescue-bundle", bundle]
        subprocess.run(  # noqa: S603
            [values["guide"], "write-stick", values["image"], again, *sources],
            input=(values["passphrase"] + "\n" + recovery_key + "\n").encode(),
            check=True,
        )
        overlay(values["qemuImg"], written_on_host.name, "-b", str(again), "-F", "raw")
        machine.shutdown()
        os.environ["STICK_IMAGE"] = written_on_host.name
        # The firmware, enforcing the machine's keys, boots it.
        machine.start(allow_reboot=True)
        on_screen(machine, "A machine is already here")
        machine.shutdown()
        yield HostStick(owner, installed, own)


@pytest.fixture(scope="module")
def reinstalled(
    signed_in: QemuMachine, host_stick: HostStick, rig: Rig, values: dict[str, Any]
) -> dict[str, Any]:
    """Reinstall from the install's own stick, and give the bundle the reinstall sealed."""
    machine, pin, password = signed_in, values["pin"], values["password"]
    # The stick the machine was installed from, which it made its own.
    os.environ["STICK_IMAGE"] = rig.stick
    machine.start(allow_reboot=True)
    # The disk holds the machine, so its stick asks first.
    on_screen(machine, "A machine is already here")
    assert "install machine on it again" in " ".join(screen(machine).split()), screen(machine)
    machine.send_key("down")
    enter(machine, "")
    # The stick copies its system into memory first, as on the first boot.
    on_screen(machine, "Check the keyboard types what you mean", timeout=COPIED)
    enter(machine, "keyboard")
    on_screen(machine, "the stick's passphrase")
    enter(machine, values["passphrase"])
    on_screen(machine, "Confirm the disk to wipe")
    enter(machine, "machine")
    for name, state in [("the Nano", "nano"), ("the 5C", "fivec")]:
        on_screen(machine, f"Plug in {name}", timeout=STEP)
        plug(machine, rig, state)
        enter(machine, "")
        on_screen(machine, f"{name}'s PIN")
        enter(machine, pin)
    keep_the_recovery_key(machine)
    on_screen(machine, "a password for alice", timeout=INSTALLED)
    enter(machine, password)
    on_screen(machine, "the same password again")
    enter(machine, password)
    on_screen(machine, "shut down", timeout=STEP)
    machine.succeed(f"grep -qx {host_stick.owner} /mnt/var/lib/sbctl/GUID")
    remade: dict[str, Any] = json.loads(
        machine.succeed("cat /mnt/var/lib/guide/rescue-bundle.json")
    )
    # The stick carries the new one on.
    machine.succeed("mkdir -p /run/vault && mount -o ro /dev/disk/by-partlabel/vault /run/vault")
    machine.succeed(
        "cmp /run/vault/machine-rescue-bundle.json /mnt/var/lib/guide/rescue-bundle.json"
    )
    machine.succeed("umount /run/vault")
    enter(machine, "")
    machine.wait_for_shutdown()
    return remade


@pytest.fixture(scope="module")
def reinstalled_boot(
    signed_in: QemuMachine, reinstalled: dict[str, Any], rig: Rig, values: dict[str, Any]
) -> QemuMachine:
    """Boot the reinstalled machine, leaving the TPM's seal for another boot with Ctrl+C."""
    del reinstalled
    machine = signed_in
    os.environ["STICK_IMAGE"] = rig.blank
    machine.start(allow_reboot=True)
    unlock(machine, values["pin"])
    machine.connect()
    # Its new disk isn't sealed to the TPM yet, so the guide asks;
    # Ctrl+C leaves that for another boot.
    on_screen(machine, "Plug in a YubiKey, which opens the disk for the TPM")
    machine.send_key("ctrl-c")
    machine.wait_for_unit("multi-user.target")
    return machine


@pytest.fixture(scope="module")
def policy_freed(reinstalled_boot: QemuMachine, values: dict[str, Any]) -> list[int]:
    """Restart, and let the guide seal the TPM; give the policies there were before."""
    machine = reinstalled_boot
    # The first install's policy outlived its wiped disk, beside the one
    # this boot made.
    before = policies(machine, values)
    machine.reboot()
    unlock(machine, values["pin"])
    machine.connect()
    on_screen(machine, "Plug in a YubiKey, which opens the disk for the TPM")
    enter(machine, "")
    on_screen(machine, "which the TPM takes too")
    enter(machine, values["pin"])
    # Sealed, and freed before the guide asks for the account's keys.
    on_screen(machine, "a PIN for alice's SSH keys", timeout=STEP)
    return before


@dataclass(frozen=True)
class Refused:
    """How the restart refused with an unsigned program on the ESP, and the TPM's count."""

    said: str
    lockouts: int
    unsigned: str


UNSIGNED = "/boot/EFI/Linux/unsigned.efi"


@pytest.fixture(scope="module")
def firmware_cleared(
    reinstalled_boot: QemuMachine, policy_freed: list[int], values: dict[str, Any]
) -> Refused:
    """Clear the firmware's keys with an unsigned program on the ESP, and boot."""
    del policy_freed
    machine = reinstalled_boot
    # A program the firmware would start, left unsigned on the ESP, which
    # would leave the machine unable to start once the keys enforce.
    machine.succeed(
        f"cp /run/current-system/systemd/lib/systemd/boot/efi/systemd-bootx64.efi {UNSIGNED}"
    )
    before = lockouts(machine, values)
    # A firmware's setup clearing its keys leaves it as a fresh copy of
    # OVMF's variables is: in setup mode, with no keys and no boot
    # entries, so it starts systemd-boot from the disk's fallback path.
    machine.shutdown()
    machine.efi_vars_path.unlink()
    machine.start(allow_reboot=True)
    # Without Secure Boot the TPM's policy doesn't hold, so it refuses
    # the PIN for the boot, and a YubiKey opens the disk with the same
    # one, so neither counts a wrong try.
    assert unlock_as_asked(machine, values["pin"]) == "the key"
    machine.connect()
    # lanzaboote readies the keys again, from the machine's own, and
    # fails only at predicting the TPM's policy, which systemd-pcrlock
    # won't rewrite from a boot that policy doesn't cover. The restart
    # goes by what's on the ESP instead, and finds the unsigned program.
    machine.wait_until_succeeds(
        "systemctl show -P Result secure-boot-enrol-restart | grep -qx exit-code",
        timeout=STEP,
    )
    result = machine.succeed("systemctl show -P Result prepare-sb-auto-enroll").strip()
    assert result == "exit-code", result
    said = machine.succeed("journalctl -b -o cat -u secure-boot-enrol-restart")
    assert "Secure Boot: disabled" in machine.succeed("bootctl status")
    # Nor are they left for systemd-boot to enrol at whatever restart
    # comes next.
    machine.succeed("test ! -e /boot/loader/keys/auto")
    return Refused(said, before, UNSIGNED)


@pytest.fixture(scope="module")
def keys_back(
    reinstalled_boot: QemuMachine, firmware_cleared: Refused, values: dict[str, Any]
) -> QemuMachine:
    """Take the unsigned program away, and restart for the keys to be enrolled again."""
    del firmware_cleared
    machine, pin = reinstalled_boot, values["pin"]
    machine.succeed(f"rm {UNSIGNED}")
    machine.reboot()
    rebooted = len(machine.get_console_log())
    assert unlock_as_asked(machine, pin) == "the key"
    machine.wait_for_console_text("Custom Secure Boot keys successfully enrolled")
    # A boot that restarted itself may have greeted the driver first, and
    # the driver takes the first greeting it reads for the boot it's in.
    greeted = "Started backdoor.service" in machine.get_console_log()[rebooted:]
    # Enforcing with the same keys, the TPM's policy, which the boot
    # without them couldn't rewrite, holds again, and its PIN alone opens
    # the disk.
    assert unlock_as_asked(machine, pin) == "the TPM"
    machine.connect()
    if greeted:
        machine.connected = False
        machine.connect()
    return machine


def test_stick_boots_into_the_guide_from_memory(booted: QemuMachine) -> None:
    """The stick boots into the guide, which runs its system from memory and checks the keyboard."""
    table = booted.succeed("dmsetup table usr")
    assert re.search(r" verity 1 7:\d+ 7:\d+ ", table), "the store reads from the copies"
    # Held awake while the guide works: sleep, idling and the lid.
    held = booted.succeed("systemd-inhibit --list --no-legend --no-pager")
    assert re.search(
        r"guide .*sleep:idle:handle-lid-switch +installing this machine +block", held
    ), held


def test_vault_opens_with_the_sticks_passphrase(vault_opened: QemuMachine) -> None:
    """The vault opens with the stick's passphrase, named as it's kept on the phone."""
    on_screen(vault_opened, "The stick's vault is open")


def test_disk_is_shown_and_the_name_typed_back(confirmed: QemuMachine) -> None:
    """The disk is shown and the machine's name typed back."""
    on_screen(confirmed, "is machine's, to wipe", timeout=STEP)


def test_nano_is_enrolled_with_the_stick_out(nano_enrolled: QemuMachine) -> None:
    """The Nano is enrolled by name with the stick out, and opens the disk."""
    on_screen(nano_enrolled, "The Nano opens the disk", timeout=STEP)


def test_5c_is_enrolled_with_the_stick_out(fivec_enrolled: QemuMachine) -> None:
    """The 5C is enrolled by name with the stick out, and opens the disk."""
    on_screen(fivec_enrolled, "The 5C opens the disk", timeout=STEP)


def test_recovery_key_is_typed_back_and_opens_the_disk(
    fivec_enrolled: QemuMachine, recovery_key: str
) -> None:
    """The recovery key is shown, typed back, and opens the disk."""
    assert len(recovery_key.split()) == RECOVERY_KEY_WORDS, recovery_key
    # Said above the card asking for the stick back, which says "look again"
    # until the guide next looks.
    on_screen(fivec_enrolled, "The recovery key opens the disk", timeout=STEP, stopped="try again")


def test_install_asks_for_the_stick_back_and_goes_on(stick_back: QemuMachine) -> None:
    """The install asks for the stick back, goes on once it's in, and says how far along it is."""
    stick_back.succeed("test -e /dev/disk/by-partlabel/vault")


def test_ctrl_c_stops_the_guide_partway(stopped: QemuMachine) -> None:
    """Ctrl+C partway through a step stops the guide there, and it carries on once started again."""
    # The journal's copy on the disk's boot partition, from the layout on.
    stopped.succeed("grep -q '\"keep-recovery-key\"' /mnt/boot/guide/install-journal")


def test_install_a_restart_stopped_is_carried_on(
    carried_on: QemuMachine, recovery_key: str
) -> None:
    """An install a restart stopped is carried on, opened by a key it enrolled."""
    # What only the journal knew came back with its copy: the recovery key
    # typed back stays done, the same words opening the disk.
    carried_on.succeed("grep -q '\"keep-recovery-key\"' /run/guide/journal")
    carried_on.succeed(
        f"printf %s {shlex.quote(recovery_key)} | cryptsetup open --test-passphrase"
        f" --key-file=- {VOLUME}"
    )
    # The new install key, made through the key, opens the disk.
    carried_on.succeed(
        f"cryptsetup open --test-passphrase --key-file=/run/guide/install-key {VOLUME}"
    )


def test_system_installs_and_the_account_gets_its_password(
    installed: QemuMachine, values: dict[str, Any]
) -> None:
    """The system installs from the vault, and the account gets its password."""
    system = installed.succeed("readlink -f /mnt/nix/var/nix/profiles/system").strip()
    assert system == values["toplevel"], system


def test_only_the_keys_and_recovery_key_open_the_disk(installed: QemuMachine) -> None:
    """Only the keys and the recovery key open the disk once it's done."""
    installed.fail("test -e /run/guide/install-key")
    # Nothing is left to carry on, so the journal's copy is gone.
    installed.fail("test -e /mnt/boot/guide/install-journal")
    installed.succeed("test -s /mnt/var/lib/sbctl/keys/db/db.key")
    # The rescue bundle, sealed, readable without root.
    made = json.loads(installed.succeed("cat /mnt/var/lib/guide/rescue-bundle.json"))
    assert made["machine"] == "machine", made["machine"]
    assert made["opened_by"][-1] == "the disk's recovery key", made["opened_by"]
    mode = installed.succeed("stat -c %a /mnt/var/lib/guide/rescue-bundle.json").strip()
    assert mode == "644", mode
    # The stick is the machine's own now: its bundle beside the
    # vault's sealed files, the keys sealed among them.
    installed.succeed("mkdir -p /run/vault && mount -o ro /dev/disk/by-partlabel/vault /run/vault")
    installed.succeed(
        "cmp /run/vault/machine-rescue-bundle.json /mnt/var/lib/guide/rescue-bundle.json"
    )
    installed.fail("grep -rq 'PRIVATE KEY' /run/vault")
    installed.succeed("umount /run/vault")


def test_card_says_what_the_disk_asks_and_shuts_down(shut_down: QemuMachine) -> None:
    """The card says what the disk will ask, and the machine shuts down."""
    assert not shut_down.booted


def test_machine_boots_its_own_disk_with_a_keys_pin(first_boot: QemuMachine) -> None:
    """The stick comes out, and the machine boots its own disk with a key's PIN."""
    log = first_boot.get_console_log()
    enrolled = log.rfind("Custom Secure Boot keys successfully enrolled")
    assert enrolled != -1
    assert log[enrolled:].count("Linux version") == 1
    first_boot.succeed("journalctl -b -u prepare-sb-auto-enroll | grep -q 'unmet condition'")
    status = first_boot.succeed("bootctl status")
    print(status)
    assert "Secure Boot: enabled (user)" in status, status
    first_boot.succeed("grep -q '^alice:\\$' /etc/shadow")


def test_guide_seals_the_disk_to_the_tpm(tpm_sealed: QemuMachine, values: dict[str, Any]) -> None:
    """The guide seals the disk to the TPM with a PIN, a YubiKey opening it to do so."""
    dumped = header(tpm_sealed)
    (sealed,) = [
        token["keyslots"][0]
        for token in dumped["tokens"].values()
        if token["type"] == "systemd-tpm2"
    ]
    assert dumped["keyslots"][sealed].get("priority") == PREFERRED
    print("FIXTURE luks-dump-sealed.json " + json.dumps(dumped))
    assert policies(tpm_sealed, values) == [own_policy(tpm_sealed)]


def test_guide_makes_alices_ssh_keys_in_the_tpm(
    ssh_keys: QemuMachine, values: dict[str, Any]
) -> None:
    """The guide makes alice's SSH keys in the TPM, as alice, one with her PIN."""
    machine, keygen = ssh_keys, values["keygen"]
    for file in TPM_KEYS:
        for half in [".tpm", ".pub"]:
            assert machine.succeed(f"stat -c %U {file}{half}").strip() == "alice", file + half
        public = machine.succeed(f"cat {file}.pub")
        assert machine.succeed(f"{keygen} --print-pubkey {file}.tpm") == public
    assert machine.succeed("stat -c %a /home/alice/.ssh").strip() == "700"
    assert "alice@machine agents" in machine.succeed("cat /home/alice/.ssh/agent-signing.pub")
    # The TPM holds the login key under alice's PIN, whole: it opens with it,
    # spaces inside and all, and not with another. ssh-tpm-keygen 0.9.0's -p
    # hands its -N to the TPM as the old PIN and its -P as the new one
    # (cmd/ssh-tpm-keygen/main.go's ChangeAuth call swaps them), so each
    # try gives the same PIN as both, which holds whichever way round.
    login = "/home/alice/.ssh/id_ecdsa.tpm"
    for pin_tried, opens in [(values["keyPin"], True), ("not her PIN", False)]:
        quoted = shlex.quote(pin_tried)
        command = f"runuser -u alice -- {keygen} -p -f {login} -P {quoted} -N {quoted}"
        if opens:
            machine.succeed(command)
        else:
            machine.fail(command)


def test_guide_keeps_a_spare_on_each_yubikey(spares: str, signed_in: QemuMachine) -> None:
    """The guide keeps a spare SSH key on each YubiKey, refusing the one it met as another."""
    machine = signed_in
    assert "ssh keys id_ecdsa, agent-signing" in spares, spares
    assert "the nano holds alice's spare ssh key" in spares, spares
    assert "the 5c holds alice's spare ssh key" in spares, spares
    for name in ["nano", "5c"]:
        file = f"/home/alice/.ssh/yubikey-{name}"
        for half in ["", ".pub"]:
            assert machine.succeed(f"stat -c %U {file}{half}").strip() == "alice", file + half
        assert machine.succeed(f"stat -c %a {file}").strip() == "600"
        public = spare(machine, name)
        assert public.startswith("sk-ssh-ed25519@openssh.com "), public
        assert public.strip().endswith(f"alice@yubikey {name}"), public
        read = machine.succeed(f"ssh-keygen -y -f {file}")
        assert read.split()[:2] == public.split()[:2]
    assert spare(machine, "nano").split()[1] != spare(machine, "5c").split()[1]
    # Nothing the guide made them in is left.
    machine.fail("test -e /run/guide/spare")
    # Return signed in, rather than restarting a boot not yet counted.
    assert machine.succeed("systemctl show -P Result guide-installed").strip() == "success"
    # And they show again once it's gone, as the machine had them.
    assert machine.succeed("systemctl show -P ShowStatus").strip() == "yes"


def test_after_the_restart_the_tpm_opens_the_disk(signed_in: QemuMachine, restarted: str) -> None:
    """After the restart the disk asks once, and the TPM opens it."""
    assert "Finish machine" not in screen(signed_in), screen(signed_in)
    assert signed_in.succeed("systemctl show -P ShowStatus").strip() == "yes"
    assert signed_in.succeed("systemctl show -P Result guide-installed").strip() == "success"
    # One prompt, and the TPM opened the disk: systemd asks a YubiKey
    # only once the TPM has refused.
    assert prompts(restarted) == ["Please enter LUKS2 token PIN:"], prompts(restarted)
    assert "Asking FIDO2 token" not in restarted
    # Started again in the same boot, as a switch does, it leaves the first
    # console, and the session signed in there, alone.
    signed_in.succeed("systemctl start guide-installed")
    assert signed_in.succeed("systemctl show -P ConditionResult guide-installed").strip() == "no"
    assert signed_in.succeed("systemctl show -P Result guide-installed").strip() == "success"


def test_cleared_tpm_is_sealed_again(
    signed_in: QemuMachine, cleared: Cleared, values: dict[str, Any]
) -> None:
    """A cleared TPM leaves a YubiKey to open the disk, and the guide seals it again."""
    machine, keygen = signed_in, values["keygen"]
    # Downloaded from the Nano, the same key, under its own comment again.
    assert spare(machine, "nano") == cleared.nano
    # New keys in place of the old, which this TPM loads.
    for file in TPM_KEYS:
        public = machine.succeed(f"cat {file}.pub")
        assert public.split()[1] != cleared.before[file].split()[1], file
        assert machine.succeed(f"stat -c %U {file}.tpm").strip() == "alice", file
        assert machine.succeed(f"{keygen} --print-pubkey {file}.tpm") == public
    # Changing its PIN to itself loads the login key, as only this TPM can.
    quoted = shlex.quote(values["keyPin"])
    machine.succeed(f"runuser -u alice -- {keygen} -p -f {TPM_KEYS[0]}.tpm -P {quoted} -N {quoted}")
    sealed = [
        token for token in header(machine)["tokens"].values() if token["type"] == "systemd-tpm2"
    ]
    assert len(sealed) == 1, sealed
    assert prompts(cleared.boot) == ["Please enter LUKS2 token PIN:"], prompts(cleared.boot)
    assert "Asking FIDO2 token" not in cleared.boot


def test_stick_carrying_the_machines_keys_is_written_on_a_host(host_stick: HostStick) -> None:
    """A stick carrying the machine's own Secure Boot keys is written on a host."""
    assert host_stick.installed["owner"] == host_stick.owner
    own, installed = host_stick.own, host_stick.installed
    assert (own["owner"], own["db"]) == (installed["owner"], installed["db"])
    # Named as the phone keeps it, after the machine.
    assert 'named "Machine Rescue Phrase"' in own["opened_by"][-1], own["opened_by"]


def test_machines_own_stick_reinstalls_with_the_keys_put_back(
    host_stick: HostStick, reinstalled: dict[str, Any]
) -> None:
    """The install's own stick boots, enforcing, and reinstalls with the keys put back."""
    # A new bundle for the same keys, sealed to the new recovery key.
    assert reinstalled["db"] == host_stick.installed["db"]
    assert reinstalled["to_passphrase"] != host_stick.installed["to_passphrase"]


def test_reinstalled_machine_enforces_with_the_same_keys(
    reinstalled_boot: QemuMachine, host_stick: HostStick
) -> None:
    """The reinstalled machine enforces with the same keys, the firmware never visited."""
    machine = reinstalled_boot
    # The firmware already enforces with the keys it carried, so the
    # first boot leaves nothing for systemd-boot to enrol and doesn't
    # restart: one PIN reached the guide, where a restart would have
    # asked again.
    machine.succeed("journalctl -b -u prepare-sb-auto-enroll | grep -q 'unmet condition'")
    machine.succeed("test ! -e /boot/loader/keys/auto")
    assert "Secure Boot: enabled (user)" in machine.succeed("bootctl status")
    assert host_stick.owner in machine.succeed("sbctl status")


def test_reinstalled_machines_guide_frees_the_first_installs_policy(
    reinstalled_boot: QemuMachine, policy_freed: list[int], values: dict[str, Any]
) -> None:
    """The reinstalled machine's guide frees the TPM of the first install's policy."""
    machine = reinstalled_boot
    own = own_policy(machine)
    # Before the guide sealed it, the first install's policy was beside
    # this one's; once it had, only this one's is left.
    assert len([index for index in policy_freed if index != own]) == 1, policy_freed
    assert policies(machine, values) == [own]


def test_cleared_firmware_gets_its_keys_back_once_all_is_signed(
    keys_back: QemuMachine,
    firmware_cleared: Refused,
    host_stick: HostStick,
    values: dict[str, Any],
) -> None:
    """A firmware cleared of its keys gets them back once the machine's are all signed."""
    machine = keys_back
    refused = firmware_cleared.said
    assert "isn't signed, so the keys aren't enrolled" in refused, refused
    assert firmware_cleared.unsigned in refused, refused
    log = machine.get_console_log()
    enrolled = log.rfind("Custom Secure Boot keys successfully enrolled")
    assert log[enrolled:].count("Linux version") == 1
    # Refusing for the boot, not the PIN, cost the TPM no tries.
    assert lockouts(machine, values) == firmware_cleared.lockouts
    # The guide finds the TPM still sealed, and goes on to the keys.
    on_screen(machine, "a PIN for alice's SSH keys", timeout=STEP)
    assert "Secure Boot: enabled (user)" in machine.succeed("bootctl status")
    assert host_stick.owner in machine.succeed("sbctl status")
    # And the files it enrolled from go, so a later clearing can't have
    # them enrolled again behind the person's back.
    machine.wait_until_succeeds("test ! -e /boot/loader/keys/auto")

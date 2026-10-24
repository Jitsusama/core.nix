"""core's install, Secure Boot and measured boot walked through in a virtual machine with a TPM.

tests/secure-boot.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the overlay, the boot and the helpers,
runs whichever subtests are chosen. What the script needs from Nix, the disk image, the systems
to install and the secrets typed at the install, comes in one JSON file, whose path
secure-boot.nix writes in place of the marker at the top.
"""

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# A JSON object as bootctl gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

values = json.loads(Path("@values@").read_text())
qemu_img: str = values["qemuImg"]
image: str = values["image"]
option_rom_check: str = values["optionRomCheck"]
real: str = values["real"]
later: str = values["later"]
passphrase: str = values["passphrase"]
pin: str = values["pin"]

# The machine writes to an overlay, since the image is in the store. The overlay is deleted once
# it's closed, so it's held open for the whole script rather than in a with block, which would
# take every subtest off the top level.
overlay = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
# qemu-img and the image are Nix's own store paths, so nothing untrusted reaches the call.
subprocess.run(  # noqa: S603
    [qemu_img, "create", "-f", "qcow2", "-b", image, "-F", "qcow2", overlay.name],
    check=True,
)
os.environ["NIX_DISK_IMAGE"] = overlay.name


# Two password agents print each prompt, so an unlock waits for the disk
# to open before anything looks for the next one.
def unlock(secret: str) -> None:
    """Type the secret at the disk's prompt, and wait for the disk to open."""
    machine.wait_for_console_text("Please enter")
    # The agent that wrote the prompt to the serial console reads it
    # there, where tty1's may not be reading keys yet.
    machine.send_console(secret + "\n")
    # Perhaps coloured; the driver matches across lines joined with nothing
    # between them, so the pattern mustn't span one.
    machine.wait_for_console_text(r"Finished (?:\x1b\[[0-9;]*m)*Cryptography Setup for system")


machine.start(allow_reboot=True)

with subtest("the first boot makes the keys and reboots to enrol them"):
    unlock(passphrase)
    # Every boot's shell is connected in turn, or a later connection
    # reads an earlier boot's.
    machine.connect()
    # lanzaboote reboots once its keys are on the ESP; systemd-boot then
    # enrols them and the firmware comes back with Secure Boot on.
    machine.wait_for_console_text("Exporting as auth files")
    machine.wait_for_console_text("Linux version")
    # The driver can't see a reboot the machine started itself.
    machine.connected = False
    unlock(passphrase)
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    status = machine.succeed("bootctl status")
    print(status)
    t.assertIn("Secure Boot: enabled (user)", status)
    print(machine.succeed("sbctl verify"))

with subtest(
    "once the firmware enforces, the keys' files leave the ESP, and none are written again"
):
    # Left there, a firmware cleared for a fresh install would have the
    # keys put back by systemd-boot before any disk asked for a PIN.
    machine.wait_until_succeeds("test ! -e /boot/loader/keys/auto", timeout=60)
    machine.succeed("journalctl -b -u secure-boot-enrolled.service | grep -q 'Finished'")
    # As a reinstall's new ESP leaves it, its keys carried and enrolled.
    boot = machine.succeed("cat /proc/sys/kernel/random/boot_id")
    machine.succeed("systemctl start prepare-sb-auto-enroll.service")
    machine.succeed("journalctl -b -u prepare-sb-auto-enroll.service | grep -q 'unmet condition'")
    machine.succeed("test ! -e /boot/loader/keys/auto")
    machine.sleep(5)
    t.assertEqual(machine.succeed("cat /proc/sys/kernel/random/boot_id"), boot)

with subtest("without checksums, a firmware option ROM stops enrolment"):
    code, output = machine.execute(f"{option_rom_check} 2>&1")
    print(output)
    t.assertNotEqual(code, 0)
    t.assertIn("Found OptionROM", output)
    t.assertIn("includeChecksumsFromTPM", output)

with subtest("the real system installs with measured boot"):
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {real}")
    print(machine.succeed(f"{real}/bin/switch-to-configuration boot 2>&1"))
    # Installed with its three tries to count.
    machine.succeed("ls /boot/EFI/Linux/nixos-generation-2-*+3.efi")
    machine.reboot()
    unlock(passphrase)
    machine.wait_for_unit("multi-user.target")
    t.assertIn("Secure Boot: enabled (user)", machine.succeed("bootctl status"))
    # The boot counts only once the services are up.
    machine.wait_for_unit("systemd-bless-boot.service")
    up, blessed = (
        int(machine.succeed(f"systemctl show -P {when} {unit}"))
        for when, unit in [
            ("ActiveEnterTimestampMonotonic", "multi-user.target"),
            ("ExecMainStartTimestampMonotonic", "systemd-bless-boot.service"),
        ]
    )
    t.assertGreater(blessed, up, "the boot was blessed before its services were up")

with subtest("the measured-boot policy covers PCRs 0, 4 and 7"):
    machine.wait_for_unit("systemd-pcrlock-make-policy.service")
    policy = json.loads(machine.succeed("cat /var/lib/systemd/pcrlock.json"))
    pcrs = sorted(value["pcr"] for value in policy["pcrValues"])
    t.assertEqual(pcrs, [0, 4, 7])

disk = "/dev/disk/by-partlabel/system"
with subtest("the runbook binds the disk to the TPM and a PIN"):
    machine.succeed(
        f"PASSWORD={passphrase} NEWPIN={pin} systemd-cryptenroll --tpm2-device=auto"
        f" --tpm2-with-pin=yes --tpm2-pcrlock=/var/lib/systemd/pcrlock.json {disk}"
    )
    recovery = (
        machine.succeed(
            f"PASSWORD={passphrase} systemd-cryptenroll --recovery-key {disk} 2>/dev/null"
        )
        .strip()
        .split()[-1]
    )
    machine.succeed(f"PASSWORD={passphrase} systemd-cryptenroll --wipe-slot=password {disk}")
    tokens = machine.succeed(f"cryptsetup luksDump --dump-json-metadata {disk}")
    kinds = sorted(token["type"] for token in json.loads(tokens)["tokens"].values())
    print(kinds)
    t.assertEqual(kinds, ["systemd-recovery", "systemd-tpm2"])
    machine.fail(f"echo -n {passphrase} | cryptsetup open --test-passphrase {disk}")
    machine.succeed(f"echo -n {recovery} | cryptsetup open --test-passphrase {disk}")

with subtest("the disk opens with the TPM and the PIN alone"):
    machine.reboot()
    unlock(pin)
    machine.wait_for_unit("multi-user.target")
    print(machine.succeed("journalctl -b -o cat -u systemd-cryptsetup@system.service"))

with subtest("no NvPCR is set up, and a login measures nothing"):
    # By its name rather than its unit: it exits before journald can
    # tell which unit its lines came from.
    setup = machine.succeed("journalctl -b -o cat -t systemd-tpm2-setup")
    print(setup)
    t.assertIn("SRK already stored in the TPM.", setup)
    t.assertNotIn("NvPCR", setup)
    machine.fail("ls /boot/loader/credentials/nvpcr-anchor.*.cred")
    machine.fail("test -s /run/systemd/nvpcr/nvpcr-anchor.cred")
    # Lingering has logind start the user as a first login does.
    machine.succeed("loginctl enable-linger alice")
    machine.wait_for_unit("user@1000.service")
    state = machine.succeed("systemctl show -P LoadState systemd-pcrlogin@1000.service")
    t.assertEqual(state.strip(), "masked")
    logind = machine.succeed("journalctl -b -o cat -u systemd-logind.service")
    print(logind)
    t.assertNotIn("measurement", logind)


def entries() -> list[Json]:
    """Give the boot loader's entries, as bootctl lists them."""
    listed: list[Json] = json.loads(machine.succeed("bootctl list --json=short"))
    return listed


with subtest("a generation counted from its install is blessed once it's up"):
    booted = [entry for entry in entries() if entry.get("isSelected")]
    print(booted)
    t.assertEqual(len(booted), 1)
    t.assertIn("nixos-generation-2-", booted[0]["id"])
    t.assertNotIn("triesLeft", booted[0], "the booted generation is still being counted")
    machine.wait_for_unit("boot-fell-back.service")
    machine.fail("test -e /run/boot-fell-back")


def told(state: str) -> None:
    """Wait until the session's notice of a fallen-back boot is in the state."""
    machine.wait_until_succeeds(
        'test "$(systemctl --user -M alice@ show -P ActiveState'
        f' boot-fell-back.service)" = {state}',
        timeout=60,
    )


with subtest("a generation that never boots is passed over, and the session is told"):
    machine.succeed(f"nix-env -p /nix/var/nix/profiles/system --set {later}")
    machine.succeed(f"{later}/bin/switch-to-configuration boot")
    # Its three tries spent, as three boots that never came up leave it.
    machine.succeed(
        "cd /boot/EFI/Linux && f=$(ls nixos-generation-3-*+3.efi) && mv $f ${f%+3.efi}+0-3.efi"
    )
    machine.reboot()
    unlock(pin)
    machine.wait_for_unit("multi-user.target")
    t.assertEqual(machine.succeed("readlink -f /run/booted-system").strip(), real)
    machine.wait_for_unit("boot-fell-back.service")
    said = machine.succeed("cat /run/boot-fell-back").strip()
    t.assertEqual(said, "Generation 3 failed to boot 3 times, so this is 2.")
    # The session hears the file and waits for something to show
    # notices, then tells it.
    machine.succeed("loginctl enable-linger alice")
    machine.wait_for_unit("user@1000.service")
    told("activating")
    machine.succeed(
        "systemd-run --user -M alice@ --unit notices"
        " /run/current-system/sw/bin/stand-in-notices /home/alice/told"
    )
    told("active")
    t.assertEqual(
        machine.succeed("cat /home/alice/told"),
        f"Boot\nBooted the last good generation\n{said}\n",
    )

# Last, since the PCR it changes stays changed until the next boot.
with subtest("the TPM refuses once a measured PCR changes"):
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

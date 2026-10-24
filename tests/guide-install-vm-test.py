"""A machine installed from its stick by the guide, driven by its keys, in a virtual machine.

tests/guide-install.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every subtest
stays at the top level under its own name, and what sits outside them, the keys, the stick, the
boot and the helpers, runs whichever subtests are chosen; the two keys' subtests share one loop,
so they run with what sits outside. What the script needs from Nix, the keys' state, the sticks'
images, the payload, the programs run beside the driver and the secrets typed at the guide,
comes in one JSON file, whose path guide-install.nix writes in place of the marker at the top.
"""

import base64
import contextlib
import json
import os
import re
import shlex
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest, t

values = json.loads(Path("@values@").read_text())
# The keys' state files, made before QEMU starts, and the stick as `guide write-stick` wrote it.
canokeys: str = values["canokeys"]
written: str = values["written"]
# The stick's own system image, and the machine's payload, which a host writes a stick from.
image: str = values["image"]
payload: str = values["payload"]
# The system the guide should install.
toplevel: str = values["toplevel"]
# The programs the script runs: qemu-img and the guide beside the driver, tpm2_clear on the
# machine.
qemu_img: str = values["qemuImg"]
guide: str = values["guide"]
tpm2_clear: str = values["tpm2Clear"]
keygen: str = values["keygen"]
# What's typed at the guide: the keys' PIN, the TPM's, the account's password, the stick's
# passphrase and the rescue bundle's.
pin: str = values["pin"]
tpm_pin: str = values["tpmPin"]
key_pin: str = values["keyPin"]
password: str = values["password"]
passphrase: str = values["passphrase"]
bundle_passphrase: str = values["bundlePassphrase"]

# How many words a recovery key has, which are all its own screen shows.
RECOVERY_KEY_WORDS = 7

keys = tempfile.mkdtemp()
for name in ["nano", "fivec"]:
    shutil.copy(f"{canokeys}/{name}", f"{keys}/{name}")
    Path(f"{keys}/{name}").chmod(0o600)
os.environ["KEYS_DIR"] = keys
# The machine writes to the stick's overlay, since its image is in the store. The overlay is
# deleted once it's closed, so it's held open for the whole script rather than in a with
# block, which would take every subtest off the top level.
stick = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
# qemu-img and the image are Nix's own store paths, so nothing untrusted reaches the call.
subprocess.run(  # noqa: S603
    [qemu_img, "create", "-f", "qcow2", "-b", written, "-F", "raw", stick.name],
    check=True,
)
os.environ["STICK_IMAGE"] = stick.name


def screen() -> str:
    """Give what the guide's console shows."""
    return machine.get_tty_text("1")


# The screen as a person reads it, whatever's capitalised and however
# its lines wrap. A step that stops says so at once, rather than after
# the wait for what it would have shown.
def on_screen(pattern: str, timeout: int = 120, stopped: str = r"try again|look again") -> None:
    """Wait until the pattern is on the guide's screen, failing at once if the guide stopped."""

    def shown(_: bool) -> bool:
        """Say whether the pattern is on the screen, failing if the guide says it stopped."""
        text = screen()
        if re.search(f"(?i)({stopped})", text):
            message = "the guide stopped:\n" + text
            raise RuntimeError(message)
        return re.search("(?i)" + pattern, " ".join(text.split())) is not None

    with machine.nested(f"waiting for {pattern} on the guide's screen"):
        try:
            retry(shown, timeout)
        except Exception:
            # What the machine was at when the wait ran out: the
            # screen, the guide's raw log, and the journal's last.
            print("SCREEN\n" + screen())
            for where in ["/run/guide/log", "/var/lib/guide/log"]:
                print(f"LOG {where}\n" + machine.execute(f"tail -n 60 {where}")[1])
            print("JOURNAL\n" + machine.execute("journalctl -b -n 80 -o cat")[1])
            raise


def enter(text: str) -> None:
    """Type the text at the guide and press Return."""
    machine.send_chars(text + "\n")


def plug(name: str) -> None:
    """Take the key out and plug in the one whose state file is named."""
    machine.send_monitor_command("device_del key")
    machine.wait_until_succeeds('test -z "$(fido2-token -L)"', timeout=30)
    machine.send_monitor_command(f"device_add canokey,id=key,bus=xhci.0,file={keys}/{name}")
    machine.wait_until_succeeds('test -n "$(fido2-token -L)"', timeout=30)


# The stick out of its port and back, as on a laptop with one port
# both it and a YubiKey fit. QEMU drops a drive with its device, so
# the drive is made again before the stick goes back.
def unplug_stick() -> None:
    """Take the stick out, and wait until its vault is gone."""
    machine.send_monitor_command("device_del stickdev")
    machine.wait_until_succeeds("test ! -e /dev/disk/by-partlabel/vault", timeout=30)


def plug_stick_back() -> None:
    """Put the stick back in, and wait until its vault is there."""
    print(
        machine.send_monitor_command(
            f"drive_add 0 if=none,id=stick,format=qcow2,file={os.environ['STICK_IMAGE']}"
        )
    )
    print(machine.send_monitor_command("device_add usb-storage,bus=xhci.0,drive=stick,id=stickdev"))
    machine.wait_until_succeeds("test -e /dev/disk/by-partlabel/vault", timeout=30)


# Before its keyboard screen, the stick copies its whole system into memory through QEMU's
# USB stick, which on a host busy with other machines' disks took past five minutes. The test
# proves what the screens say and in what order, not how fast.
COPIED = 900

machine.start(allow_reboot=True)

with subtest(
    "the stick boots into the guide, which runs its system from memory and checks the keyboard"
):
    on_screen("Check the keyboard types what you mean", timeout=COPIED)
    # Done before the keyboard, and said so above it.
    on_screen("The stick's system runs from memory")
    table = machine.succeed("dmsetup table usr")
    t.assertRegex(table, r" verity 1 7:\d+ 7:\d+ ", "the store reads from the copies")
    enter("keyboard")

with subtest("the vault opens with the stick's passphrase, named as it's kept on the phone"):
    # This stick was written before any key was enrolled, so only the
    # passphrase opens it; the vault test has a stick keys open too.
    on_screen(
        "the stick's passphrase, the seven words in 1Password on your phone named "
        '"machine stick passphrase"'
    )
    enter(passphrase)


# What the tools print here, for the guide's fixtures (qemu-canokey).
def fixture(name: str, command: str) -> None:
    """Print what the command prints on the machine, on one line, as the named fixture."""
    print(f"FIXTURE {name} " + machine.succeed(command).replace("\n", "\\n"))


with subtest("the disk is shown and the machine's name typed back"):
    on_screen("Confirm the disk to wipe")
    on_screen("wipes")
    fixture("lsblk-blank.json", "lsblk --json --output-all --bytes")
    enter("machine")

for name, state in [("the Nano", "nano"), ("the 5C", "fivec")]:
    with subtest(f"{name} is enrolled by name with the stick out, and opens the disk"):
        on_screen(f"Plug in {name}", timeout=300)
        on_screen("take the stick out first")
        if state == "nano":
            fixture("lsblk-laid-out.json", "lsblk --json --output-all --bytes")
            fixture("fido2-token-L.txt", "fido2-token -L")
            unplug_stick()
        else:
            plug(state)
        machine.succeed(f"set-pin {pin}")
        enter("")
        on_screen(f"{name}'s PIN")
        enter(pin)


# The recovery key, kept as a person keeps it on their phone: the
# steps, then its seven words alone on the screen for the phone's
# camera, nothing else there for it to copy, then typed back.
def keep_the_recovery_key() -> str:
    """Read the recovery key off its own screen, type it back, and give it."""
    on_screen("Keep the recovery key", timeout=300)
    on_screen("Copy All")
    enter("")
    retry(lambda _: len(screen().split()) == RECOVERY_KEY_WORDS, 30)
    words = " ".join(screen().split())
    enter("")
    on_screen("reading it off your phone")
    enter(words)
    return words


with subtest("the recovery key is shown, typed back, and opens the disk"):
    key = keep_the_recovery_key()

with subtest(
    "the install asks for the stick back, goes on once it's in, and says how far along it is"
):
    on_screen("Put the stick back in", timeout=300, stopped="try again")
    plug_stick_back()
    # The card asking for the stick says "look again" until the guide
    # next looks, up to two seconds after the stick is back.
    on_screen(r"Copying it off the stick: [0-9.]+ [KMG]B of ", timeout=300, stopped="try again")

with subtest(
    "Ctrl+C partway through a step stops the guide there, and it carries on once started again"
):
    on_screen("Ctrl\\+C stop")
    machine.send_key("ctrl-c")
    machine.wait_until_tty_matches("1", "Stopped. Start the guide again", timeout=60)
    machine.wait_until_tty_matches("1", "type guide install, then press Return", timeout=60)
    enter("guide install")

with subtest("the system installs from the vault, and the account gets its password"):
    on_screen("a password for alice", timeout=1800)
    enter(password)
    on_screen("the same password again")
    enter(password)

with subtest("only the keys and the recovery key open the disk once it's done"):
    # The reboot card's own key, which nothing before it shows.
    on_screen("shut down", timeout=300)
    fixture(
        "luks-dump-installed.json",
        "cryptsetup luksDump --dump-json-metadata /dev/disk/by-partlabel/system",
    )
    system = machine.succeed("readlink -f /mnt/nix/var/nix/profiles/system").strip()
    assert system == toplevel, system
    machine.fail("test -e /run/guide/install-key")
    machine.succeed("test -s /mnt/var/lib/sbctl/keys/db/db.key")
    # The rescue bundle, sealed, readable without root.
    made = json.loads(machine.succeed("cat /mnt/var/lib/guide/rescue-bundle.json"))
    t.assertEqual(made["machine"], "machine")
    t.assertEqual(made["opened_by"][-1], "the disk's recovery key")
    t.assertEqual(
        machine.succeed("stat -c %a /mnt/var/lib/guide/rescue-bundle.json").strip(), "644"
    )
    # The stick is the machine's own now: its bundle beside the
    # vault's sealed files, the keys sealed among them.
    machine.succeed("mkdir -p /run/vault && mount -o ro /dev/disk/by-partlabel/vault /run/vault")
    machine.succeed(
        "cmp /run/vault/machine-rescue-bundle.json /mnt/var/lib/guide/rescue-bundle.json"
    )
    machine.fail("grep -rq 'PRIVATE KEY' /run/vault")
    machine.succeed("umount /run/vault")

with subtest("the card says what the disk will ask, and the machine shuts down"):
    on_screen("security token PIN")
    enter("")
    machine.wait_for_shutdown()

with subtest("the stick comes out, and the machine boots its own disk with a key's PIN"):
    # A blank stick stands where it was, which the firmware passes over. Like the stick's
    # overlay, it's deleted once it's closed, and later subtests boot with it in, so it's held
    # open for the rest of the script.
    blank = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
    # qemu-img is Nix's own store path, so nothing untrusted reaches the call.
    subprocess.run(  # noqa: S603
        [qemu_img, "create", "-f", "qcow2", blank.name, "64M"], check=True
    )
    os.environ["STICK_IMAGE"] = blank.name

    # systemd-cryptsetup asks through libcryptsetup's token plugin
    # first and then its own FIDO2 code, and a key that's not there
    # yet at the first leaves the second asking, so each prompt is
    # answered until the disk opens.
    def unlock(secret: str = pin) -> None:
        """Answer the disk's prompts with the secret until it opens."""
        machine.wait_for_console_text("Please enter")
        for _ in range(4):
            machine.send_console(secret + "\n")
            # A prompt left asking times the wait out, and the next try answers it.
            with contextlib.suppress(Exception):
                machine.wait_for_console_text(
                    "Finished .*Cryptography Setup for system", timeout=30
                )
                return
        message = "the disk never opened with the key's PIN"
        raise RuntimeError(message)

    machine.start(allow_reboot=True)
    # systemd-boot enrols the keys the stick readied and restarts before
    # any kernel starts, so the disk asks only once, with Secure Boot on.
    unlock()
    machine.connect()
    log = machine.get_console_log()
    enrolled = log.rfind("Custom Secure Boot keys successfully enrolled")
    t.assertNotEqual(enrolled, -1)
    t.assertEqual(log[enrolled:].count("Linux version"), 1)
    machine.succeed("journalctl -b -u prepare-sb-auto-enroll | grep -q 'unmet condition'")
    # The guide, at boot, holds multi-user.target back until it's done.
    machine.wait_until_succeeds(
        "systemctl show -P ActiveState guide-installed | grep -qx activating"
    )
    status = machine.succeed("bootctl status")
    print(status)
    t.assertIn("Secure Boot: enabled (user)", status)
    machine.succeed("grep -q '^alice:\\$' /etc/shadow")


# The range systemd picks a policy's NV index from (src/shared/tpm2-util.h).
UNASSIGNED = range(0x01800000, 0x01BFFFFF + 1)


def policies() -> list[int]:
    """Give the NV indices in the range systemd-pcrlock picks a policy's from."""
    getcap = Path(tpm2_clear).parent / "tpm2_getcap"
    listed = machine.succeed(f"{getcap} -T device:/dev/tpmrm0 handles-nv-index")
    # Nothing but systemd-pcrlock defines an index in that range on the
    # VM's TPM, so each is a policy.
    indices = [int(index, 16) for index in re.findall(r"0x[0-9A-Fa-f]+", listed)]
    return [index for index in indices if index in UNASSIGNED]


def own_policy() -> int:
    """Give the NV index this disk's policy file names."""
    policy: int = json.loads(machine.succeed("cat /var/lib/systemd/pcrlock.json"))["nvIndex"]
    return policy


with subtest("the guide seals the disk to the TPM with a PIN, a YubiKey opening it to do so"):
    on_screen("a PIN for the TPM")
    # The units still starting don't write across it.
    t.assertEqual(machine.succeed("systemctl show -P ShowStatus").strip(), "no")
    enter(tpm_pin)
    on_screen("the same PIN again")
    enter(tpm_pin)
    on_screen("Plug in a YubiKey")
    enter("")
    on_screen("the YubiKey's PIN")
    enter(pin)
    # Sealed, the guide goes on to the account's keys.
    on_screen("a PIN for alice's SSH keys", timeout=300)
    header = json.loads(
        machine.succeed("cryptsetup luksDump --dump-json-metadata /dev/disk/by-partlabel/system")
    )
    (sealed,) = [t["keyslots"][0] for t in header["tokens"].values() if t["type"] == "systemd-tpm2"]
    t.assertEqual(header["keyslots"][sealed].get("priority"), 2)
    print("FIXTURE luks-dump-sealed.json " + json.dumps(header))
    t.assertEqual(policies(), [own_policy()])

with subtest("the guide makes alice's SSH keys in the TPM, as alice, one with her PIN"):
    enter(key_pin)
    on_screen("the same PIN again")
    enter(key_pin)
    # Made, the guide goes on to the YubiKeys' spares.
    on_screen("Plug in the Nano", timeout=300)
    for file in ["/home/alice/.ssh/id_ecdsa", "/home/alice/.ssh/agent-signing"]:
        for half in [".tpm", ".pub"]:
            t.assertEqual(machine.succeed(f"stat -c %U {file}{half}").strip(), "alice", file + half)
        public = machine.succeed(f"cat {file}.pub")
        t.assertEqual(machine.succeed(f"{keygen} --print-pubkey {file}.tpm"), public)
    t.assertEqual(machine.succeed("stat -c %a /home/alice/.ssh").strip(), "700")
    t.assertIn("alice@machine agents", machine.succeed("cat /home/alice/.ssh/agent-signing.pub"))
    # The TPM holds the login key under alice's PIN, whole: it opens with it,
    # spaces inside and all, and not with another. ssh-tpm-keygen 0.9.0's -p
    # hands its -N to the TPM as the old PIN and its -P as the new one
    # (cmd/ssh-tpm-keygen/main.go's ChangeAuth call swaps them), so each
    # try gives the same PIN as both, which holds whichever way round.
    login = "/home/alice/.ssh/id_ecdsa.tpm"
    for pin_tried, opens in [(key_pin, True), ("not her PIN", False)]:
        quoted = shlex.quote(pin_tried)
        command = f"runuser -u alice -- {keygen} -p -f {login} -P {quoted} -N {quoted}"
        if opens:
            machine.succeed(command)
        else:
            machine.fail(command)


def spare(name: str) -> str:
    """Give the public half of alice's spare from the YubiKey with this file name."""
    return machine.succeed(f"cat /home/alice/.ssh/yubikey-{name}.pub")


with subtest("the guide keeps a spare SSH key on each YubiKey, refusing the one it met as another"):
    plug("nano")
    enter("")
    on_screen("the Nano's PIN")
    enter(pin)
    on_screen("Plug in the 5C", timeout=300)
    # The Nano still in, which the guide met as the Nano.
    enter("")
    on_screen("the 5C's PIN")
    enter(pin)
    on_screen("this key is the Nano, not the 5C", timeout=300, stopped="look again")
    plug("fivec")
    enter("")
    # Asked again, as the guide looks again after Return; until it has,
    # the refusal stays on the screen, "try again" and all.
    on_screen("Plug in the 5C, and only the 5C", stopped="look again")
    enter("")
    on_screen("the 5C's PIN")
    enter(pin)
    on_screen("machine is finished", timeout=300)
    shown = " ".join(screen().split()).lower()
    t.assertIn("ssh keys id_ecdsa, agent-signing", shown)
    t.assertIn("the nano holds alice's spare ssh key", shown)
    t.assertIn("the 5c holds alice's spare ssh key", shown)
    for name in ["nano", "5c"]:
        file = f"/home/alice/.ssh/yubikey-{name}"
        for half in ["", ".pub"]:
            t.assertEqual(machine.succeed(f"stat -c %U {file}{half}").strip(), "alice", file + half)
        t.assertEqual(machine.succeed(f"stat -c %a {file}").strip(), "600")
        public = spare(name)
        t.assertTrue(public.startswith("sk-ssh-ed25519@openssh.com "), public)
        t.assertTrue(public.strip().endswith(f"alice@yubikey {name}"), public)
        read = machine.succeed(f"ssh-keygen -y -f {file}")
        t.assertEqual(read.split()[:2], public.split()[:2])
    t.assertNotEqual(spare("nano").split()[1], spare("5c").split()[1])
    # Nothing the guide made them in is left.
    machine.fail("test -e /run/guide/spare")
    # Root from the boot, with no password, and the sign-in waits.
    machine.succeed("pgrep -u root -xf '.*/guide installed'")
    t.assertEqual(machine.succeed("systemctl show -P ActiveState getty@tty1").strip(), "inactive")
    # Return signs in, rather than restarting a boot not yet counted.
    enter("")
    machine.wait_for_unit("getty@tty1.service")
    t.assertEqual(machine.succeed("systemctl show -P Result guide-installed").strip(), "success")
    # And they show again once it's gone, as the machine had them.
    t.assertEqual(machine.succeed("systemctl show -P ShowStatus").strip(), "yes")

with subtest("after the restart the disk asks once, and opens with the TPM's PIN"):
    before = len(machine.get_console_log())
    machine.reboot()
    unlock(tpm_pin)
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    # With nothing to do, the guide showed nothing and left.
    machine.wait_for_unit("getty@tty1.service")
    t.assertNotIn("Finish machine", screen())
    t.assertEqual(machine.succeed("systemctl show -P ShowStatus").strip(), "yes")
    t.assertEqual(machine.succeed("systemctl show -P Result guide-installed").strip(), "success")
    # One prompt, which the TPM's PIN answered: no YubiKey's PIN is
    # that, so only the TPM's slot could have opened the disk.
    prompts = re.findall(r"Please enter [^:(]*:", machine.get_console_log()[before:])
    t.assertEqual(prompts, ["Please enter LUKS2 token PIN:"])

with subtest("a cleared TPM leaves a YubiKey to open the disk, and the guide seals it again"):
    # The guide sees nothing to do, until the TPM is cleared, which
    # takes the NV index the disk's pcrlock policy lives in, and the seed
    # alice's keys were sealed under. The Nano's spare goes too, for the
    # guide to bring back off the Nano.
    nano = spare("nano")
    tpm_keys = ["/home/alice/.ssh/id_ecdsa", "/home/alice/.ssh/agent-signing"]
    before_clear = {file: machine.succeed(f"cat {file}.pub") for file in tpm_keys}
    machine.succeed("rm /home/alice/.ssh/yubikey-nano /home/alice/.ssh/yubikey-nano.pub")
    machine.succeed(f"{tpm2_clear} -T device:/dev/tpmrm0 -c l")
    machine.reboot()
    unlock()
    machine.connect()
    on_screen("a PIN for the TPM")
    enter(tpm_pin)
    on_screen("the same PIN again")
    enter(tpm_pin)
    on_screen("Plug in a YubiKey")
    enter("")
    on_screen("the YubiKey's PIN")
    enter(pin)
    # The TPM refuses the keys it sealed before, so the guide makes them
    # again, saying why.
    on_screen("a PIN for alice's new SSH keys", timeout=300)
    t.assertIn("the tpm was cleared", " ".join(screen().split()).lower())
    enter(key_pin)
    on_screen("the same PIN again")
    enter(key_pin)
    # The 5C is in from its spare, so the Nano goes in for its own.
    on_screen("Plug in the Nano", timeout=300)
    plug("nano")
    enter("")
    on_screen("the Nano's PIN")
    enter(pin)
    on_screen("machine is finished", timeout=300)
    # Downloaded from the Nano, the same key, under its own comment again.
    t.assertEqual(spare("nano"), nano)
    # New keys in place of the old, which this TPM loads, and nothing the
    # guide loaded them in is left.
    for file in tpm_keys:
        public = machine.succeed(f"cat {file}.pub")
        t.assertNotEqual(public.split()[1], before_clear[file].split()[1], file)
        t.assertEqual(machine.succeed(f"stat -c %U {file}.tpm").strip(), "alice", file)
        t.assertEqual(machine.succeed(f"{keygen} --print-pubkey {file}.tpm"), public)
    # Changing its PIN to itself loads the login key, as only this TPM can.
    quoted = shlex.quote(key_pin)
    machine.succeed(f"runuser -u alice -- {keygen} -p -f {tpm_keys[0]}.tpm -P {quoted} -N {quoted}")
    machine.fail("test -e /run/guide/tpm-key")
    header = json.loads(
        machine.succeed("cryptsetup luksDump --dump-json-metadata /dev/disk/by-partlabel/system")
    )
    t.assertEqual(len([t for t in header["tokens"].values() if t["type"] == "systemd-tpm2"]), 1)
    enter("")
    machine.wait_for_unit("getty@tty1.service")
    before = len(machine.get_console_log())
    machine.reboot()
    unlock(tpm_pin)
    machine.connect()
    machine.wait_for_unit("multi-user.target")
    prompts = re.findall(r"Please enter [^:(]*:", machine.get_console_log()[before:])
    t.assertEqual(prompts, ["Please enter LUKS2 token PIN:"])

with subtest("a stick carrying the machine's own Secure Boot keys is written on a host"):
    # The bundle the install made, taken off as alice can, sealed.
    sealed = base64.b64decode(
        machine.succeed("su alice -c 'base64 -w0 /var/lib/guide/rescue-bundle.json'")
    )
    t.assertNotIn(b"PRIVATE KEY", sealed)
    installed = json.loads(sealed)
    owner = machine.succeed("cat /var/lib/sbctl/GUID").strip()
    t.assertEqual(installed["owner"], owner)
    bundle = Path(tempfile.mkdtemp()) / "rescue.json"
    bundle.write_bytes(sealed)
    # A machine makes one with a passphrase of its own too, as root,
    # holding the same keys.
    target = base64.b64encode(Path(f"{payload}/target.json").read_bytes()).decode()
    machine.succeed(f"echo {target} | base64 -d > /root/target.json")
    t.assertIn(
        owner,
        machine.succeed(
            f"echo {bundle_passphrase} | guide seal-keys /root/target.json /root/rescue.json"
        ),
    )
    machine.succeed("test $(stat -c %a /root/rescue.json) = 600")
    own = json.loads(machine.succeed("cat /root/rescue.json"))
    t.assertEqual((own["owner"], own["db"]), (installed["owner"], installed["db"]))
    t.assertEqual(own["opened_by"][-1], "the recovery passphrase")
    again = Path(tempfile.mkdtemp()) / "reinstall.raw"
    with again.open("wb") as stick_file:
        stick_file.truncate(Path(written).stat().st_size)
    # The stick's passphrase, then the disk's recovery key, which
    # opens the bundle the install sealed to it. The guide, its images
    # and payload are Nix's own store paths, and the rest the test's
    # own files, so nothing untrusted reaches the call.
    subprocess.run(  # noqa: S603
        [
            guide,
            "write-stick",
            image,
            again,
            "--install",
            payload,
            "--rescue-bundle",
            bundle,
        ],
        input=(passphrase + "\n" + key + "\n").encode(),
        check=True,
    )
    # Held open until the script ends, as the stick's overlay is.
    written_on_host = tempfile.NamedTemporaryFile(suffix=".qcow2")  # noqa: SIM115
    # qemu-img is Nix's own store path, and the stick the test's own file.
    subprocess.run(  # noqa: S603
        [qemu_img, "create", "-f", "qcow2", "-b", again, "-F", "raw", written_on_host.name],
        check=True,
    )
    machine.shutdown()
    os.environ["STICK_IMAGE"] = written_on_host.name
    # The firmware, enforcing the machine's keys, boots it.
    machine.start(allow_reboot=True)
    on_screen("A machine is already here")
    machine.shutdown()

with subtest("the install's own stick boots, enforcing, and reinstalls with the keys put back"):
    # The stick the machine was installed from, which it made its own.
    os.environ["STICK_IMAGE"] = stick.name
    machine.start(allow_reboot=True)
    # The disk holds the machine, so its stick asks first.
    on_screen("A machine is already here")
    t.assertIn("install machine on it again", " ".join(screen().split()))
    machine.send_key("down")
    enter("")
    # The stick copies its system into memory first, as on the first boot.
    on_screen("Check the keyboard types what you mean", timeout=COPIED)
    enter("keyboard")
    on_screen("the stick's passphrase")
    enter(passphrase)
    on_screen("Confirm the disk to wipe")
    enter("machine")
    for name, state in [("the Nano", "nano"), ("the 5C", "fivec")]:
        on_screen(f"Plug in {name}", timeout=300)
        plug(state)
        enter("")
        on_screen(f"{name}'s PIN")
        enter(pin)
    key = keep_the_recovery_key()
    on_screen("a password for alice", timeout=1800)
    enter(password)
    on_screen("the same password again")
    enter(password)
    on_screen("shut down", timeout=300)
    machine.succeed(f"grep -qx {owner} /mnt/var/lib/sbctl/GUID")
    # A new bundle for the same keys, sealed to the new recovery key.
    remade = json.loads(machine.succeed("cat /mnt/var/lib/guide/rescue-bundle.json"))
    t.assertEqual(remade["db"], installed["db"])
    t.assertNotEqual(remade["to_passphrase"], installed["to_passphrase"])
    # The stick carries the new one on.
    machine.succeed("mkdir -p /run/vault && mount -o ro /dev/disk/by-partlabel/vault /run/vault")
    machine.succeed(
        "cmp /run/vault/machine-rescue-bundle.json /mnt/var/lib/guide/rescue-bundle.json"
    )
    machine.succeed("umount /run/vault")
    enter("")
    machine.wait_for_shutdown()

with subtest("the reinstalled machine enforces with the same keys, the firmware never visited"):
    os.environ["STICK_IMAGE"] = blank.name
    machine.start(allow_reboot=True)
    unlock()
    machine.connect()
    # Its new disk isn't sealed to the TPM yet, so the guide asks;
    # Ctrl+C leaves that for another boot.
    on_screen("a PIN for the TPM")
    machine.send_key("ctrl-c")
    machine.wait_for_unit("multi-user.target")
    # The firmware already enforces with the keys it carried, so the
    # first boot leaves nothing for systemd-boot to enrol and doesn't
    # restart: one PIN reached the guide, where a restart would have
    # asked again.
    machine.succeed("journalctl -b -u prepare-sb-auto-enroll | grep -q 'unmet condition'")
    machine.succeed("test ! -e /boot/loader/keys/auto")
    t.assertIn("Secure Boot: enabled (user)", machine.succeed("bootctl status"))
    t.assertIn(owner, machine.succeed("sbctl status"))

with subtest("the reinstalled machine's guide frees the TPM of the first install's policy"):
    # The first install's policy outlived its wiped disk, beside the one
    # this boot made.
    earlier = [index for index in policies() if index != own_policy()]
    t.assertEqual(len(earlier), 1)
    machine.reboot()
    unlock()
    machine.connect()
    on_screen("a PIN for the TPM")
    enter(tpm_pin)
    on_screen("the same PIN again")
    enter(tpm_pin)
    on_screen("Plug in a YubiKey")
    enter("")
    on_screen("the YubiKey's PIN")
    enter(pin)
    # Sealed, and freed before the guide asks for the account's keys.
    on_screen("a PIN for alice's SSH keys", timeout=300)
    t.assertEqual(policies(), [own_policy()])

with subtest("a firmware cleared of its keys gets them back once the machine's are all signed"):
    # A program the firmware would start, left unsigned on the ESP, which
    # would leave the machine unable to start once the keys enforce.
    unsigned = "/boot/EFI/Linux/unsigned.efi"
    machine.succeed(
        f"cp /run/current-system/systemd/lib/systemd/boot/efi/systemd-bootx64.efi {unsigned}"
    )
    # A firmware's setup clearing its keys leaves it as a fresh copy of
    # OVMF's variables is: in setup mode, with no keys and no boot
    # entries, so it starts systemd-boot from the disk's fallback path.
    machine.shutdown()
    machine.efi_vars_path.unlink()
    machine.start(allow_reboot=True)
    # Without Secure Boot the TPM's policy doesn't hold, so a YubiKey
    # opens the disk.
    unlock()
    machine.connect()
    # lanzaboote readies the keys again, from the machine's own, and
    # fails only at predicting the TPM's policy, which systemd-pcrlock
    # won't rewrite from a boot that policy doesn't cover. The restart
    # goes by what's on the ESP instead, and finds the unsigned program.
    machine.wait_until_succeeds(
        "systemctl show -P Result secure-boot-enrol-restart | grep -qx exit-code", timeout=300
    )
    t.assertEqual(
        machine.succeed("systemctl show -P Result prepare-sb-auto-enroll").strip(), "exit-code"
    )
    refused = machine.succeed("journalctl -b -o cat -u secure-boot-enrol-restart")
    t.assertIn("isn't signed, so the keys aren't enrolled", refused)
    t.assertIn(unsigned, refused)
    t.assertIn("Secure Boot: disabled", machine.succeed("bootctl status"))
    # Nor are they left for systemd-boot to enrol at whatever restart
    # comes next.
    machine.succeed("test ! -e /boot/loader/keys/auto")
    # With it gone, a restart readies them again, the check passes, and
    # the machine restarts itself for systemd-boot to enrol them before
    # any kernel starts.
    machine.succeed(f"rm {unsigned}")
    machine.reboot()
    unlock()
    machine.wait_for_console_text("Custom Secure Boot keys successfully enrolled")
    # Enforcing with the same keys, the TPM's policy, which the boot
    # without them couldn't rewrite, holds again, and its PIN alone opens
    # the disk.
    unlock(tpm_pin)
    machine.connect()
    log = machine.get_console_log()
    enrolled = log.rfind("Custom Secure Boot keys successfully enrolled")
    t.assertEqual(log[enrolled:].count("Linux version"), 1)
    # The guide finds the TPM still sealed, and goes on to the keys.
    on_screen("a PIN for alice's SSH keys", timeout=300)
    t.assertIn("Secure Boot: enabled (user)", machine.succeed("bootctl status"))
    t.assertIn(owner, machine.succeed("sbctl status"))
    # And the files it enrolled from go, so a later clearing can't have
    # them enrolled again behind the person's back.
    machine.wait_until_succeeds("test ! -e /boot/loader/keys/auto")

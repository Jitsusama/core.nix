"""The splash asking for the disk in the theme, its consoles, and a restart's screen to its end.

tests/plymouth.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at the
top level under its own name, and what sits outside them, the install onto the encrypted disk,
the boot, the helpers and the setting up of a boot whose initrd fails, runs whichever subtests
are chosen. What the script needs from Nix, the secrets, the systems to boot, the theme's
colours, face and padding, and the programs named by their paths in the store, comes in one
JSON file, whose path plymouth.nix writes in place of the marker at the top.
"""

import datetime
import filecmp
import json
import re
import shlex
import subprocess
import tempfile
import time
from pathlib import Path
from typing import TYPE_CHECKING

from test_driver.machine.ocr import perform_ocr_variants_on_screenshot

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest, t

values = json.loads(Path("@values@").read_text())
account: str = values["account"]
passphrase: str = values["passphrase"]
password: str = values["password"]
encrypted: str = values["encrypted"]
stops: str = values["stops"]
# The theme's alert colour, the terminal's background, and the face kmscon draws in.
alert: str = values["alert"]
background: str = values["background"]
mono_family: str = values["monoFamily"]
# The console's font, and the size and padding kmscon draws it at.
font: str = values["font"]
font_size: int = values["fontSize"]
padding_x: int = values["paddingX"]
padding_y: int = values["paddingY"]
# ImageMagick, which runs beside the driver, and what the machine runs by its path in the store.
magick: str = values["magick"]
fc_match: str = values["fcMatch"]
infocmp: str = values["infocmp"]
sleep: str = values["sleep"]
plymouth: str = values["plymouth"]

# The longest a wait may take to read words off the screen. A wait
# ends when they show, so this is only a ceiling: on a host busy
# with other machines and builds, one reading took 30 seconds, and a
# wait of 30 then fails before it can read twice.
reading = 120


def photographed(name: str) -> Path:
    """Photograph the screen, kept for a person to judge."""
    machine.screenshot(name)
    return machine.out_dir / f"{name}.png"


def photographs_until(words: str, mark: int, within: int) -> list[Path]:
    """Photograph the screen about once a second until the console, from mark on, says words.

    Reading a screen can take longer under load than a shutdown's screen stays up, so the
    photographs are read only afterwards, by showing.
    """
    shots = []
    began = time.monotonic()
    while words not in machine.get_console_log()[mark:]:
        t.assertLess(time.monotonic() - began, within, f"the console never said {words}")
        # As the driver photographs for its own reading: raw, since
        # its OCR can't read PNG.
        shot = burst / f"{len(list(burst.iterdir()))}.ppm"
        machine.send_monitor_command(f"screendump {shot}")
        shots.append(shot)
        time.sleep(1)
    return shots


def showing(shots: list[Path], words: str, name: str) -> Path:
    """Keep the newest of the photographs that shows words under name, for a person to judge."""
    for shot in reversed(shots):
        if any(re.search(words, text) for text in perform_ocr_variants_on_screenshot(shot)):
            kept = machine.out_dir / f"{name}.png"
            with kept.open("wb") as png:
                # pnmtopng is on the driver's PATH, which its own screenshots use it from, and the
                # photograph is the test's own.
                subprocess.run(["pnmtopng", shot], stdout=png, check=True)  # noqa: S603, S607
            return kept
    message = f"none of {len(shots)} photographs showed {words}"
    raise AssertionError(message)


# Where photographs_until keeps its photographs, which it numbers by how many are there already.
burst = Path(tempfile.mkdtemp())


def alerting(path: Path) -> int:
    """Count how many of a photograph's pixels are the theme's alert colour.

    Only the frame of a refused secret is drawn in it. The muted hint inside it is too faint
    on the dark panel for OCR to read.
    """
    # The program and every argument are the test's own, from Nix and this file.
    return int(
        subprocess.check_output(  # noqa: S603
            [
                magick,
                path,
                "-fuzz",
                "4%",
                "-fill",
                "black",
                "+opaque",
                alert,
                "-fill",
                "white",
                "-opaque",
                alert,
                "-format",
                "%[fx:round(mean*w*h)]",
                "info:",
            ],
            text=True,
        )
    )


# The frame is two panels' widths and two heights of hairline, so a
# few hundred pixels at the least.
framed = 400


def changed_from(before: Path, name: str, timeout: int = 30) -> None:
    """Wait for the screen to differ from a photograph.

    It does once Plymouth has drawn what a key changed.
    """

    def differs(_: bool) -> bool:
        """Say whether a new photograph differs from the one before."""
        return not filecmp.cmp(before, photographed(name), shallow=False)

    retry(differs, timeout)


def wears_the_face(console: str) -> None:
    """Check a console's glyphs are the theme's font, bitmap for bitmap.

    As many are compared as the font's header counts at its size.
    """
    machine.succeed(
        f"setfont -C {console} -O /tmp/held.psf",
        "word() { od -An -t u4 -j $(( $2 * 4 )) -N 4 $1 | tr -d ' '; }; "
        f"cmp -n $(( $(word {font} 4) * $(word {font} 5) )) "
        f"<(tail -c +33 /tmp/held.psf) <(tail -c +33 {font})",
    )


machine.wait_for_unit("multi-user.target")
machine.succeed(
    f"echo -n {passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
    f"echo -n {passphrase} | cryptsetup luksOpen -q /dev/vdb root",
    "mkfs.ext4 -q /dev/mapper/root",
    f"{encrypted}/bin/switch-to-configuration boot",
    "sync",
)
machine.crash()
machine.start()

with subtest("the theme asks for the disk's passphrase"):
    machine.wait_for_text("disk is locked", timeout=reading)
    asked = photographed("asks")
    t.assertLess(alerting(asked), framed // 10, "the first ask is framed in alert")

with subtest("each key shows as a dot"):
    machine.send_chars("48")
    changed_from(asked, "typing")
    for _ in range(2):
        machine.send_key("backspace")

with subtest("a wrong one is refused in the alert frame"):
    machine.send_chars("0000\n")
    retry(lambda _: alerting(photographed("wrong")) >= framed, 60)

with subtest("the right one opens the disk and the boot goes on"):
    machine.send_chars(f"{passphrase}\n")
    machine.wait_for_unit("multi-user.target")
    t.assertIn("/dev/mapper/root on / ", machine.succeed("mount"))

with subtest("a console signed in on draws the theme's face, with room round it"):
    # kmscon draws it, so the kernel's text of it is empty, and what
    # it drew is read from the screen and from what it says. The
    # switch is made directly: a chord sent as the compositor takes
    # the first console is lost.
    machine.succeed("chvt 3")
    machine.wait_for_unit("kmsconvt@tty3.service")
    machine.wait_for_text("login:", timeout=reading)
    said = machine.succeed("journalctl -o cat -u kmsconvt@tty3.service")
    t.assertRegex(said, r"font engine \[pango\]")
    used = re.search(r"Using \[(.*)\] size (\d+) -> cell size (\d+)x(\d+)", said)
    assert used, said
    # A face goes by more than one name, and Pango says any of them.
    names = machine.succeed(f"{fc_match} -f '%{{family}}' '{mono_family}'").split(",")
    t.assertIn(mono_family, names)
    t.assertTrue(any(used[1].startswith(f"{name} ") for name in names), used[1])
    t.assertEqual(int(used[2]), font_size)
    width, height = int(used[3]), int(used[4])
    grid = re.findall(r"Resized terminal to (\d+)x(\d+)", said)
    t.assertEqual(
        grid[-1],
        (str((1280 - 2 * padding_x) // width), str((800 - 2 * padding_y) // height)),
    )
    # The padding is clear on all four sides: nothing but the
    # background in it, though the prompt sits at the left and top.
    shot = photographed("console")
    for crop in [
        f"{padding_x}x800+0+0",
        f"{padding_x}x800+{1280 - padding_x}+0",
        f"1280x{padding_y}+0+0",
        f"1280x{padding_y}+0+{800 - padding_y}",
    ]:
        # The program and every argument are the test's own, from Nix and this file.
        inked = subprocess.check_output(  # noqa: S603
            [
                magick,
                shot,
                "-crop",
                crop,
                "+repage",
                "-fuzz",
                "4%",
                "-fill",
                "black",
                "-opaque",
                background,
                "-fill",
                "white",
                "+opaque",
                "black",
                "-format",
                "%[fx:round(mean*w*h)]",
                "info:",
            ],
            text=True,
        )
        t.assertEqual(int(inked), 0, f"the padding at {crop} has ink")

with subtest("a console greets by name, and signs in to a shell told it's kmscon"):
    # The encrypted root has no Nix database, so Home Manager can't
    # activate there, and the account has no shell files: an empty
    # one stands in, so zsh asks nothing of a new user.
    machine.succeed(f"install -o {account} -m 600 /dev/null /home/{account}/.zshrc")
    machine.wait_for_text("Hello", timeout=reading)
    machine.send_chars(f"{account}\n")
    machine.wait_for_text("Password", timeout=reading)
    machine.send_chars(f"{password}\n")
    machine.wait_until_succeeds(f"pgrep -u {account} -x zsh", timeout=30)
    # The prompt draws stand-ins only on a console that calls itself
    # linux, whose font can't hold its marks; kmscon calls itself by
    # its own name, which the machine's terminfo describes.
    told = machine.succeed(
        f"tr '\\0' '\\n' < /proc/$(pgrep -u {account} -x zsh | head -n 1)/environ"
    ).splitlines()
    t.assertIn("TERM=kmscon", told)
    machine.succeed(f"{infocmp} -A /run/current-system/sw/share/terminfo kmscon")
    # Signing out clears the console and greets again, ready for the
    # next, and the restarts below stop no more than they did before.
    machine.send_chars("exit\n")
    machine.wait_until_fails(f"pgrep -u {account} -x zsh", timeout=30)

    def greeted_afresh(_: bool) -> bool:
        """Say whether the console greets again, with no password asked for."""
        shown = machine.get_screen_text()
        return "Hello" in shown and "Password" not in shown

    retry(greeted_afresh, reading)

with subtest("the keys the greeting gives go back to the desktop"):
    machine.send_key("ctrl-alt-f1")
    machine.wait_until_succeeds("grep -qx tty1 /sys/class/tty/tty0/active", timeout=10)

with subtest("a console kmscon can't draw still asks, in the theme's face"):
    # A new console takes the font of the one it's switched from until
    # its login prompt sets its own, so it's switched to from the
    # first, which loses the face first. While the greeter starts or
    # stops, the first console can refuse to be opened for a moment.
    machine.succeed("chvt 1")
    machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=30)
    # Drop-ins apply in their names' order, so this one comes after
    # the one that sets kmscon's command.
    machine.succeed(
        "mkdir -p /run/systemd/system/kmsconvt@tty4.service.d",
        "printf '[Service]\\nExecStart=\\nExecStart=/bin/false\\n'"
        " > /run/systemd/system/kmsconvt@tty4.service.d/zz-fails.conf",
        "systemctl daemon-reload",
        "chvt 4",
    )
    machine.wait_for_unit("getty@tty4.service")
    machine.fail("systemctl is-active kmsconvt@tty4.service")
    machine.wait_until_tty_matches("4", "login: ")
    machine.wait_until_tty_matches("4", f"Hello, {account}")
    wears_the_face("/dev/tty4")
    machine.succeed("chvt 3")

with subtest("started on a running system with no splash, the watch ends at once"):
    # As a switch to a new configuration starts it.
    machine.succeed("systemctl start plymouth-waits.service")
    machine.wait_until_fails("systemctl is-active plymouth-waits.service", timeout=10)
    ended = machine.succeed(
        "systemctl show -p Result -p ExecMainStatus --value plymouth-waits.service"
    ).split()
    t.assertEqual(ended, ["success", "0"])

with subtest("a slow job is named while it runs, and goes when it ends"):
    # The splash back up over the first console, as at boot, with
    # nothing graphical holding it and logind kept from handing the
    # console on, and the watch started as at boot.
    machine.succeed(
        "systemctl mask --runtime --now systemd-logind.service",
        "systemctl stop greetd.service 'getty@*' 'autovt@*' 'kmsconvt@*'",
        "chvt 1",
        "systemctl restart plymouth-start.service",
        "systemctl start plymouth-waits.service",
        "systemd-run --no-block --collect --unit=slow -p Type=oneshot"
        " -p Description='Slow Hello' sleep infinity",
    )
    # Slow Hello lasts until it's stopped, so however long a busy host takes
    # to read its name off the screen, it's still running when that's done.
    machine.wait_for_text("waiting on Slow Hello", timeout=reading)
    photographed("waiting")
    machine.succeed("systemctl stop slow.service")
    retry(lambda _: "Slow" not in machine.get_screen_text(), reading)
    # logind stays masked and the greeter stopped, as the emergency
    # shell wants them next.
    machine.succeed(
        "systemctl stop plymouth-waits.service",
        "systemd-run --wait --collect plymouth quit",
    )

with subtest("the emergency shell wears the face, and says what failed"):
    # As after a failed boot: a unit has failed, nothing graphical
    # holds a console and no login prompt runs, logind masked so none
    # starts again and sets the face itself, and the splash is up over
    # consoles that have lost the face. The shell's own steps before it
    # asks, Plymouth's quitting and then this module's, run in order on
    # the console in front as the shell would run them. sulogin itself
    # isn't started: it asks on every console, the test's serial line
    # among them, and the test stops hearing the machine. The splash is
    # started by systemd, as at boot: started from the test's own
    # shell, it broke the driver's reading of that shell.
    # The account's activation fails on this new root, which has no
    # Nix profile for it, so failures from before are cleared, and the
    # summary lists only the one made here.
    machine.succeed("systemctl reset-failed")
    machine.execute(
        "systemd-run --unit=breaks --description='Breaks on purpose' "
        "sh -c 'echo the disk was not there; exit 1'"
    )
    machine.wait_until_succeeds("systemctl is-failed breaks.service", timeout=30)
    machine.succeed(
        "systemctl mask --runtime --now systemd-logind.service",
        "systemctl stop greetd.service 'getty@*' 'autovt@*' 'kmsconvt@*'",
    )
    # A boot has one console, the first, which the splash draws on.
    machine.succeed("chvt 1")
    machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=30)
    machine.succeed(
        "printf '\\033[H\\033[2J' > /dev/tty1",
        "systemctl restart plymouth-start.service",
    )
    steps = re.findall(
        r"argv\[\]=([^;]*) ;", machine.succeed("systemctl show -P ExecStartPre emergency.service")
    )
    machine.succeed(
        "systemd-run --wait --collect -p StandardInput=tty-force -p StandardOutput=tty"
        " -p TTYPath=/dev/tty0 /bin/sh -c " + shlex.quote("; ".join(steps))
    )
    front = machine.succeed("fgconsole").strip()
    wears_the_face(f"/dev/tty{front}")
    said = machine.get_tty_text(front)
    for words in [
        "breaks.service  Breaks on purpose",
        "the disk was not there",
        "Ctrl+Alt+Delete",
    ]:
        t.assertIn(words, said)
    photographed("emergency")

with subtest("a restart names a slow stop and keeps its screen to the end"):
    # A service that takes its time to stop, and a program left out of
    # every unit's stop that outlasts the last stage's kill and, as
    # whatever bounds a shutdown would, says so a second later, once
    # a splash that didn't survive the kill would have gone.
    machine.succeed(
        "systemd-run --unit=slowstop -p Description='Slow Goodbye'"
        f" -p ExecStop='{sleep} 12' sleep infinity",
        'systemd-run --unit=leftover -p KillMode=none sh -c "'
        f"trap '{sleep} 1;"
        f" {plymouth} update --status=leftovers;"
        f" exec {sleep} 25' TERM;"
        f' {sleep} 25 & wait"',
    )
    mark = len(machine.get_console_log())
    machine.reboot()
    # Every wait is bounded, so a restart that stalls fails here in
    # minutes rather than holding its VM until the hour is up.
    stopping = photographs_until("Sending SIGTERM to remaining processes", mark, 120)
    last = photographs_until("Sending SIGKILL to remaining processes", mark, 60)
    showing(stopping, "waiting on Slow Goodbye", "stopping-slowly")
    showing(last, "waiting on the last programs", "last-stage")
    # The test's machine ends at a restart rather than starting again.
    machine.wait_for_shutdown(timeout=datetime.timedelta(minutes=3))
    machine.start()
    machine.wait_for_text("disk is locked", timeout=180)
    machine.send_chars(f"{passphrase}\n")
    machine.wait_for_unit("multi-user.target")

# The next boot is one whose initrd fails, set up while the system is
# whole. The encrypted disk is a new root with no profile of its own,
# so it's given one, as nix-env would.
machine.succeed(
    "mkdir -p /nix/var/nix/profiles",
    f"ln -sfn {stops} /nix/var/nix/profiles/system-1-link",
    "ln -sfn system-1-link /nix/var/nix/profiles/system",
    f"{stops}/bin/switch-to-configuration boot",
    "sync",
)

with subtest("a failed initrd says what stopped it"):
    # Its shell can't be read through the test's own, which starts
    # later, so the screen is read.
    machine.crash()
    machine.start()
    for words in ["breaks.service", "the disk was not there", "Ctrl.Alt.Delete"]:
        machine.wait_for_text(words, timeout=reading)
    photographed("initrd-stops")

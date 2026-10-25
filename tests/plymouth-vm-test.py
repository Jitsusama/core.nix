"""The splash asking for the disk in the theme, its consoles, and a restart's screen to its end.

tests/plymouth.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: the machine installed onto its encrypted disk and booted to the splash's ask, then
the disk opened, then kmscon's console in front. A fixture's work is done once a run, however
many tests ask for it, so any test runs alone with `-k`. What the tests need from Nix, the
secrets, the systems to boot, the theme's colours, face and padding, and the programs named
by their paths in the store, is plymouth.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import filecmp
import re
import shlex
import subprocess
import tempfile
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import pytest
from test_driver.machine import retry
from test_driver.machine.ocr import perform_ocr_variants_on_screenshot

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# The longest a wait may take to read words off the screen. A wait
# ends when they show, so this is only a ceiling: on a host busy
# with other machines and builds, one reading took 30 seconds, and a
# wait of 30 then fails before it can read twice.
READING = timedelta(seconds=120)
# The entry the boot starts, named as lanzaboote names generation 7's.
CHOSEN = "nixos-generation-7-fixture"
# The frame is two panels' widths and two heights of hairline, so a
# few hundred pixels at the least.
FRAMED = 400
# The machine's screen, in pixels.
WIDTH = 1280
HEIGHT = 800


class Boot:
    """The machine booting from its encrypted disk, and what the tests need from Nix."""

    def __init__(self, machine: QemuMachine, values: dict[str, Any]) -> None:
        """Boot that machine, with plymouth.nix's values."""
        self.machine = machine
        self.account: str = values["account"]
        self.passphrase: str = values["passphrase"]
        self.password: str = values["password"]
        self.encrypted: str = values["encrypted"]
        self.stops: str = values["stops"]
        # The theme's alert colour, the terminal's background, and the face kmscon draws in.
        self.alert: str = values["alert"]
        self.background: str = values["background"]
        self.mono_family: str = values["monoFamily"]
        # The console's font, and the size and padding kmscon draws it at.
        self.font: str = values["font"]
        self.font_size: int = values["fontSize"]
        self.padding_x: int = values["paddingX"]
        self.padding_y: int = values["paddingY"]
        # ImageMagick, which runs beside the driver, and what the machine runs by its path in
        # the store.
        self.magick: str = values["magick"]
        self.fc_match: str = values["fcMatch"]
        self.infocmp: str = values["infocmp"]
        self.sleep: str = values["sleep"]
        self.plymouth: str = values["plymouth"]
        # Where photographs_until keeps its photographs, which it numbers by how many are
        # there already.
        self.burst = Path(tempfile.mkdtemp())

    def photographed(self, name: str) -> Path:
        """Photograph the screen, kept for a person to judge."""
        self.machine.screenshot(name)
        return self.machine.out_dir / f"{name}.png"

    def photographs_until(self, words: str, mark: int, within: timedelta) -> list[Path]:
        """Photograph the screen about once a second until the console, from mark on, says words.

        Reading a screen can take longer under load than a shutdown's screen stays up, so the
        photographs are read only afterwards, by showing.
        """
        shots = []
        began = time.monotonic()
        while words not in self.machine.get_console_log()[mark:]:
            assert time.monotonic() - began < within.total_seconds(), (
                f"the console never said {words}"
            )
            # As the driver photographs for its own reading: raw, since
            # its OCR can't read PNG.
            shot = self.burst / f"{len(list(self.burst.iterdir()))}.ppm"
            self.machine.send_monitor_command(f"screendump {shot}")
            shots.append(shot)
            time.sleep(1)
        return shots

    def showing(self, shots: list[Path], words: str, name: str) -> Path:
        """Keep the newest of the photographs that shows words under name, for a person to judge."""
        for shot in reversed(shots):
            if any(re.search(words, text) for text in perform_ocr_variants_on_screenshot(shot)):
                kept = self.machine.out_dir / f"{name}.png"
                with kept.open("wb") as png:
                    # pnmtopng is on the driver's PATH, which its own screenshots use it from,
                    # and the photograph is the test's own.
                    subprocess.run(["pnmtopng", shot], stdout=png, check=True)  # noqa: S603, S607
                return kept
        message = f"none of {len(shots)} photographs showed {words}"
        raise AssertionError(message)

    def alerting(self, path: Path) -> int:
        """Count how many of a photograph's pixels are the theme's alert colour.

        Only the frame of a refused secret is drawn in it. The muted hint inside it is too faint
        on the dark panel for OCR to read.
        """
        # The program and every argument are the test's own, from Nix and this file.
        return int(
            subprocess.check_output(  # noqa: S603
                [
                    self.magick,
                    path,
                    "-fuzz",
                    "4%",
                    "-fill",
                    "black",
                    "+opaque",
                    self.alert,
                    "-fill",
                    "white",
                    "-opaque",
                    self.alert,
                    "-format",
                    "%[fx:round(mean*w*h)]",
                    "info:",
                ],
                text=True,
            )
        )

    def inked(self, path: Path, crop: str) -> int:
        """Count how many of a photograph's pixels in crop are other than the background."""
        # The program and every argument are the test's own, from Nix and this file.
        return int(
            subprocess.check_output(  # noqa: S603
                [
                    self.magick,
                    path,
                    "-crop",
                    crop,
                    "+repage",
                    "-fuzz",
                    "4%",
                    "-fill",
                    "black",
                    "-opaque",
                    self.background,
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
        )

    def differing(self, one: Path, other: Path) -> int:
        """Count how many pixels differ between two photographs."""
        # The program and every argument are the test's own, from Nix and this file.
        compared = subprocess.run(  # noqa: S603
            [self.magick, "compare", "-metric", "AE", "-fuzz", "4%", one, other, "null:"],
            capture_output=True,
            text=True,
            check=False,
        )
        # compare says 1 when the two differ and 2 when it couldn't compare them.
        assert compared.returncode in {0, 1}, compared.stderr
        return int(float(compared.stderr.split()[0]))

    def changed_from(self, before: Path, name: str) -> None:
        """Wait for the screen to differ from a photograph.

        It does once Plymouth has drawn what a key changed.
        """

        def differs(_: bool) -> bool:
            """Say whether a new photograph differs from the one before."""
            return not filecmp.cmp(before, self.photographed(name), shallow=False)

        retry(differs, timeout=timedelta(seconds=30))

    def wears_the_face(self, console: str) -> None:
        """Check a console's glyphs are the theme's font, bitmap for bitmap.

        As many are compared as the font's header counts at its size.
        """
        self.machine.succeed(
            f"setfont -C {console} -O /tmp/held.psf",
            "word() { od -An -t u4 -j $(( $2 * 4 )) -N 4 $1 | tr -d ' '; }; "
            f"cmp -n $(( $(word {self.font} 4) * $(word {self.font} 5) )) "
            f"<(tail -c +33 /tmp/held.psf) <(tail -c +33 {self.font})",
        )

    def bare(self) -> None:
        """Leave the first console in front with nothing graphical holding it.

        logind is masked so no login prompt starts again and sets the face itself, and the
        first console loses the face, as a boot's has before its login prompt.
        """
        self.machine.succeed(
            "systemctl mask --runtime --now systemd-logind.service",
            "systemctl stop greetd.service 'getty@*' 'autovt@*' 'kmsconvt@*'",
            "chvt 1",
        )


@pytest.fixture(scope="module")
def asking(machine: BaseMachine, values: dict[str, Any]) -> Boot:
    """Give the machine installed onto its encrypted disk, booted to the splash's ask."""
    # The test's one machine is QEMU's, which takes keys and screenshots.
    boot = Boot(cast("QemuMachine", machine), values)
    boot.machine.wait_for_unit("multi-user.target")
    # The system under test isn't one of the first boot's own, so it's made
    # the profile's newest generation, as nix-env would, for the boot loader
    # to give it an entry.
    boot.machine.succeed(
        f"echo -n {boot.passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
        f"echo -n {boot.passphrase} | cryptsetup luksOpen -q /dev/vdb root",
        "mkfs.ext4 -q /dev/mapper/root",
        f"ln -sfn {boot.encrypted} /nix/var/nix/profiles/system-2-link",
        "ln -sfn system-2-link /nix/var/nix/profiles/system",
        f"{boot.encrypted}/bin/switch-to-configuration boot",
    )
    # NixOS names its own entries by a hash of what they hold, so the next
    # boot starts its entry under the name lanzaboote would give it, which
    # carries the generation.
    entry = boot.machine.succeed(
        f"grep -l 'init={boot.encrypted}/init' /boot/loader/entries/nixos-*.conf",
    ).strip()
    boot.machine.succeed(
        f"cp {entry} /boot/loader/entries/{CHOSEN}.conf",
        f"bootctl set-oneshot {CHOSEN}.conf",
        "sync",
    )
    boot.machine.crash()
    boot.machine.start()
    boot.machine.wait_for_text("disk is locked", timeout=READING)
    return boot


@pytest.fixture(scope="module")
def opened(asking: Boot) -> Boot:
    """Give the machine with its disk opened by the passphrase and its boot gone on."""
    asking.machine.send_chars(f"{asking.passphrase}\n")
    asking.machine.wait_for_unit("multi-user.target")
    return asking


@pytest.fixture(scope="module")
def on_kmscon(opened: Boot) -> Boot:
    """Give the machine with kmscon's console in front, asking who's there.

    The switch is made directly: a chord sent as the compositor takes the first console is
    lost.
    """
    opened.machine.succeed("chvt 3")
    opened.machine.wait_for_unit("kmsconvt@tty3.service")
    opened.machine.wait_for_text("login:", timeout=READING)
    return opened


def test_the_theme_asks_for_the_disks_passphrase(asking: Boot) -> None:
    """The theme asks for the disk's passphrase."""
    asked = asking.photographed("asks")
    assert asking.alerting(asked) < FRAMED // 10, "the first ask is framed in alert"


def test_the_splash_names_the_generation_the_boot_loader_started(asking: Boot) -> None:
    """The splash names the generation the boot loader started."""
    asking.machine.wait_for_text("generation 7", timeout=READING)
    asking.photographed("generation")


def test_each_key_shows_as_a_dot(asking: Boot) -> None:
    """Each key shows as a dot."""
    before = asking.photographed("before-typing")
    asking.machine.send_chars("48")
    asking.changed_from(before, "typing")
    for _ in range(2):
        asking.machine.send_key("backspace")


def test_a_wrong_one_is_refused_in_the_alert_frame(asking: Boot) -> None:
    """A wrong one is refused in the alert frame."""
    asking.machine.send_chars("0000\n")
    retry(
        lambda _: asking.alerting(asking.photographed("wrong")) >= FRAMED,
        timeout=timedelta(seconds=60),
    )


def test_the_right_one_opens_the_disk_and_the_boot_goes_on(opened: Boot) -> None:
    """The right one opens the disk and the boot goes on."""
    assert "/dev/mapper/root on / " in opened.machine.succeed("mount")


def test_a_console_draws_the_themes_face_with_room_round_it(on_kmscon: Boot) -> None:
    """A console signed in on draws the theme's face, with room round it.

    kmscon draws it, so the kernel's text of it is empty, and what it drew is read from the
    screen and from what it says.
    """
    boot = on_kmscon
    said = boot.machine.succeed("journalctl -o cat -u kmsconvt@tty3.service")
    assert re.search(r"font engine \[pango\]", said), said
    used = re.search(r"Using \[(.*)\] size (\d+) -> cell size (\d+)x(\d+)", said)
    assert used, said
    # A face goes by more than one name, and Pango says any of them.
    names = boot.machine.succeed(f"{boot.fc_match} -f '%{{family}}' '{boot.mono_family}'")
    assert boot.mono_family in names.split(","), names
    assert any(used[1].startswith(f"{name} ") for name in names.split(",")), used[1]
    assert int(used[2]) == boot.font_size
    width, height = int(used[3]), int(used[4])
    grid = re.findall(r"Resized terminal to (\d+)x(\d+)", said)
    assert grid[-1] == (
        str((WIDTH - 2 * boot.padding_x) // width),
        str((HEIGHT - 2 * boot.padding_y) // height),
    )
    # The padding is clear on all four sides: nothing but the
    # background in it, though the prompt sits at the left and top.
    shot = boot.photographed("console")
    for crop in [
        f"{boot.padding_x}x{HEIGHT}+0+0",
        f"{boot.padding_x}x{HEIGHT}+{WIDTH - boot.padding_x}+0",
        f"{WIDTH}x{boot.padding_y}+0+0",
        f"{WIDTH}x{boot.padding_y}+0+{HEIGHT - boot.padding_y}",
    ]:
        assert boot.inked(shot, crop) == 0, f"the padding at {crop} has ink"


def test_a_console_greets_by_name_and_signs_in_told_its_kmscon(on_kmscon: Boot) -> None:
    """A console greets by name, and signs in to a shell told it's kmscon."""
    boot, machine, account = on_kmscon, on_kmscon.machine, on_kmscon.account
    # The encrypted root has no Nix database, so Home Manager can't
    # activate there, and the account has no shell files: an empty
    # one stands in, so zsh asks nothing of a new user.
    machine.succeed(f"install -o {account} -m 600 /dev/null /home/{account}/.zshrc")
    machine.wait_for_text("Hello", timeout=READING)
    machine.send_chars(f"{account}\n")
    machine.wait_for_text("Password", timeout=READING)
    machine.send_chars(f"{boot.password}\n")
    machine.wait_until_succeeds(f"pgrep -u {account} -x zsh", timeout=timedelta(seconds=30))
    # The prompt draws stand-ins only on a console that calls itself
    # linux, whose font can't hold its marks; kmscon calls itself by
    # its own name, which the machine's terminfo describes.
    told = machine.succeed(
        f"tr '\\0' '\\n' < /proc/$(pgrep -u {account} -x zsh | head -n 1)/environ"
    ).splitlines()
    assert "TERM=kmscon" in told, told
    machine.succeed(f"{boot.infocmp} -A /run/current-system/sw/share/terminfo kmscon")
    # Signing out clears the console and greets again, ready for the
    # next, and the restarts below stop no more than they did before.
    machine.send_chars("exit\n")
    machine.wait_until_fails(f"pgrep -u {account} -x zsh", timeout=timedelta(seconds=30))

    def greeted_afresh(_: bool) -> bool:
        """Say whether the console greets again, with no password asked for."""
        shown = machine.get_screen_text()
        return "Hello" in shown and "Password" not in shown

    retry(greeted_afresh, timeout=READING)


def test_the_keys_the_greeting_gives_go_back_to_the_desktop(opened: Boot) -> None:
    """The keys the greeting gives go back to the desktop."""
    opened.machine.send_key("ctrl-alt-f1")
    opened.machine.wait_until_succeeds(
        "grep -qx tty1 /sys/class/tty/tty0/active", timeout=timedelta(seconds=10)
    )


def test_a_console_kmscon_cant_draw_still_asks_in_the_themes_face(opened: Boot) -> None:
    """A console kmscon can't draw still asks, in the theme's face."""
    machine = opened.machine
    # A new console takes the font of the one it's switched from until
    # its login prompt sets its own, so it's switched to from the
    # first, which loses the face first. While the greeter starts or
    # stops, the first console can refuse to be opened for a moment.
    machine.succeed("chvt 1")
    machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=timedelta(seconds=30))
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
    machine.wait_until_tty_matches("4", f"Hello, {opened.account}")
    opened.wears_the_face("/dev/tty4")
    machine.succeed("chvt 3")


def test_started_with_no_splash_the_watch_ends_at_once(opened: Boot) -> None:
    """Started on a running system with no splash, the watch ends at once."""
    machine = opened.machine
    # As a switch to a new configuration starts it.
    machine.succeed("systemctl start plymouth-waits.service")
    machine.wait_until_fails(
        "systemctl is-active plymouth-waits.service", timeout=timedelta(seconds=10)
    )
    ended = machine.succeed(
        "systemctl show -p Result -p ExecMainStatus --value plymouth-waits.service"
    ).split()
    assert ended == ["success", "0"]


def test_a_slow_job_is_named_while_it_runs_and_goes_when_it_ends(opened: Boot) -> None:
    """A slow job is named while it runs, and goes when it ends."""
    machine = opened.machine
    # The splash back up over the first console, as at boot, with
    # nothing graphical holding it and logind kept from handing the
    # console on, and the watch started as at boot.
    opened.bare()
    machine.succeed(
        "systemctl restart plymouth-start.service",
        "systemctl start plymouth-waits.service",
        "systemd-run --no-block --collect --unit=slow -p Type=oneshot"
        " -p Description='Slow Hello' sleep infinity",
    )
    # Slow Hello lasts until it's stopped, so however long a busy host takes
    # to read its name off the screen, it's still running when that's done.
    machine.wait_for_text("waiting on Slow Hello", timeout=READING)
    opened.photographed("waiting")
    machine.succeed("systemctl stop slow.service")
    retry(lambda _: "Slow" not in machine.get_screen_text(), timeout=READING)
    # logind stays masked and the greeter stopped, as the emergency
    # shell wants them next.
    machine.succeed(
        "systemctl stop plymouth-waits.service",
        "systemd-run --wait --collect plymouth quit",
    )


def test_a_security_key_waiting_for_a_touch_is_asked_for_until_it_is_touched(
    opened: Boot,
) -> None:
    """A security key waiting for a touch is asked for in the theme's words until it's touched."""
    machine = opened.machine
    opened.bare()
    machine.succeed("systemctl restart plymouth-start.service")
    machine.wait_for_text("starting", timeout=READING)
    # Once the splash has faded in, two photographs a moment apart are the same.
    shots = [opened.photographed("before-touch-0")]

    def settled(_: bool) -> bool:
        """Say whether a new photograph matches the one before it."""
        time.sleep(1)
        shots.append(opened.photographed(f"before-touch-{len(shots)}"))
        return opened.differing(shots[-2], shots[-1]) == 0

    retry(settled, timeout=timedelta(seconds=60))
    before = shots[-1]
    # What systemd sends Plymouth while a key waits for a touch, then the empty
    # message it ends that with once the key's been touched.
    machine.succeed(
        f"{opened.plymouth} display-message"
        " --text='Please confirm presence on security token to unlock.'"
    )
    machine.wait_for_text("touch the security key", timeout=READING)
    opened.photographed("touch")
    assert "confirm presence" not in machine.get_screen_text()
    machine.succeed(f"{opened.plymouth} display-message --text=''")
    retry(
        lambda _: opened.differing(before, opened.photographed("touched")) == 0,
        timeout=READING,
    )
    machine.succeed("systemd-run --wait --collect plymouth quit")


def test_the_emergency_shell_wears_the_face_and_says_what_failed(opened: Boot) -> None:
    """The emergency shell wears the face, and says what failed.

    As after a failed boot: a unit has failed, nothing graphical holds a console and no login
    prompt runs, and the splash is up over consoles that have lost the face. The shell's own
    steps before it asks, Plymouth's quitting and then this module's, run in order on the
    console in front as the shell would run them. sulogin itself isn't started: it asks on
    every console, the test's serial line among them, and the test stops hearing the machine.
    The splash is started by systemd, as at boot: started from the test's own shell, it broke
    the driver's reading of that shell.
    """
    machine = opened.machine
    # The account's activation fails on this new root, which has no
    # Nix profile for it, so failures from before are cleared, and the
    # summary lists only the one made here.
    machine.succeed("systemctl reset-failed")
    machine.execute(
        "systemd-run --unit=breaks --description='Breaks on purpose' "
        "sh -c 'echo the disk was not there; exit 1'"
    )
    machine.wait_until_succeeds("systemctl is-failed breaks.service", timeout=timedelta(seconds=30))
    # A boot has one console, the first, which the splash draws on.
    opened.bare()
    machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=timedelta(seconds=30))
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
    opened.wears_the_face(f"/dev/tty{front}")
    said = machine.get_tty_text(front)
    for words in [
        "breaks.service  Breaks on purpose",
        "the disk was not there",
        "Ctrl+Alt+Delete",
    ]:
        assert words in said, said
    opened.photographed("emergency")


def test_a_restart_names_a_slow_stop_and_keeps_its_screen_to_the_end(opened: Boot) -> None:
    """A restart names a slow stop and keeps its screen to the end."""
    machine, sleep = opened.machine, opened.sleep
    # A service that takes its time to stop, and a program left out of
    # every unit's stop that outlasts the last stage's kill and, as
    # whatever bounds a shutdown would, says so a second later, once
    # a splash that didn't survive the kill would have gone. Until the
    # kill it waits however long the shutdown takes to reach it.
    machine.succeed(
        "systemd-run --unit=slowstop -p Description='Slow Goodbye'"
        f" -p ExecStop='{sleep} 12' sleep infinity",
        'systemd-run --unit=leftover -p KillMode=none sh -c "'
        f"trap '{sleep} 1;"
        f" {opened.plymouth} update --status=leftovers;"
        f" exec {sleep} 25' TERM;"
        f' {sleep} infinity & wait"',
    )
    mark = len(machine.get_console_log())
    # Asked of systemd, as the launcher asks it: Ctrl+Alt+Delete is lost
    # when it lands as the desktop takes the keyboard. It starts a few
    # seconds on, once the test's shell has answered, since a shutdown
    # ends that shell, and the connection is made again after, as the
    # driver's own reboot leaves it.
    machine.succeed("systemd-run --collect --on-active=5 systemctl reboot")
    machine.connected = False
    # Every wait is bounded, so a restart that stalls fails here in
    # minutes rather than holding its VM until the hour is up.
    stopping = opened.photographs_until(
        "Sending SIGTERM to remaining processes", mark, timedelta(seconds=120)
    )
    last = opened.photographs_until(
        "Sending SIGKILL to remaining processes", mark, timedelta(seconds=60)
    )
    opened.showing(stopping, "waiting on Slow Goodbye", "stopping-slowly")
    opened.showing(last, "waiting on the last programs", "last-stage")
    # A shutdown's splash is a new one, told the generation again: the
    # boot's last frame says it too, so it's read beside what only the
    # shutdown says.
    opened.showing(stopping, r"(?s)(?=.*Slow Goodbye).*generation 7", "stopping-generation")
    # The test's machine ends at a restart rather than starting again.
    machine.wait_for_shutdown(timeout=timedelta(minutes=3))
    machine.start()
    machine.wait_for_text("disk is locked", timeout=timedelta(seconds=180))
    machine.send_chars(f"{opened.passphrase}\n")
    machine.wait_for_unit("multi-user.target")


def test_a_failed_initrd_says_what_stopped_it(opened: Boot) -> None:
    """A failed initrd says what stopped it."""
    machine = opened.machine
    # The next boot is one whose initrd fails, set up while the system is
    # whole. The encrypted disk is a new root with no profile of its own,
    # so it's given one, as nix-env would.
    machine.succeed(
        "mkdir -p /nix/var/nix/profiles",
        f"ln -sfn {opened.stops} /nix/var/nix/profiles/system-1-link",
        "ln -sfn system-1-link /nix/var/nix/profiles/system",
        f"{opened.stops}/bin/switch-to-configuration boot",
        "sync",
    )
    # Its shell can't be read through the test's own, which starts
    # later, so the screen is read.
    machine.crash()
    machine.start()
    for words in ["breaks.service", "the disk was not there", "Ctrl.Alt.Delete"]:
        machine.wait_for_text(words, timeout=READING)
    opened.photographed("initrd-stops")

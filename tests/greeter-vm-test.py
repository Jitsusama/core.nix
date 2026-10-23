"""The greeter signing an account in and out, in a virtual machine that doesn't sign in by itself.

tests/greeter.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the helpers and the values,
runs whichever subtests are chosen. The account's name and password are greeter.nix's, so
they come from the JSON file it writes, whose path it puts in place of the marker below.
"""

import json
import shlex
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

# The account the greeter signs in, and the password it takes, as greeter.nix declares them.
values = json.loads(Path("@values@").read_text())
account: str = values["account"]
secret: str = values["secret"]


def mark() -> str:
    """Give a point in the greeter's journal, so only what's said after it counts."""
    said = machine.succeed("journalctl --identifier=greeter --lines=1 --show-cursor --output=cat")
    return said.rsplit("-- cursor: ", 1)[1].strip()


def heard(what: str, since: str, timeout: int = 60) -> None:
    """Wait until the greeter says exactly this after the point.

    It follows the journal as it's written, and prints what the greeter did say if it never
    says this.
    """
    point = f"--after-cursor={shlex.quote(since)}" if since else "--boot"
    journal = f"journalctl --identifier=greeter --output=cat --no-pager {point}"
    wanted = shlex.quote(f"report[{what}]")
    follow = shlex.quote(f"{journal} --follow | grep -m1 -aqF {wanted}")
    status, _ = machine.execute(
        f"timeout {timeout} sh -c {follow}",
        timeout=timeout + 30,
    )
    if status != 0:
        print(machine.succeed(journal))
        machine.screenshot("unheard")
    assert status == 0, f"the greeter never said {what!r}"


with subtest("the greeter shows while the account is set up"):
    machine.wait_for_file("/run/home-hold/release", timeout=120)
    machine.wait_until_succeeds("systemctl is-active --quiet greetd.service", timeout=120)
    setup = machine.succeed(
        f"systemctl show --property=ActiveState --value home-manager-{account}.service"
    ).strip()
    assert setup == "activating", f"the account's setup was {setup!r}"

with subtest("the account's service manager waits for its setup"):
    manager = f"user@$(id -u {account}).service"
    state = machine.succeed(f"systemctl show --property=ActiveState --value {manager}").strip()
    assert state == "inactive", f"the manager was {state!r} before the setup finished"
    machine.succeed("echo go > /run/home-hold/release")
    machine.wait_until_succeeds(f"systemctl is-active --quiet {manager}", timeout=60)
    finished = int(
        machine.succeed(
            "systemctl show --property=ActiveEnterTimestampMonotonic --value"
            f" home-manager-{account}.service"
        )
    )
    started = int(
        machine.succeed(
            f"systemctl show --property=InactiveExitTimestampMonotonic --value {manager}"
        )
    )
    assert started >= finished, "the manager started before the setup finished"

with subtest("the greeter asks who's there"):
    heard("keys greeter", "", timeout=120)
    machine.screenshot("greeter")

with subtest("the greeter refuses a wrong password"):
    since = mark()
    machine.send_chars("wrong-horse\n")
    heard("refused greeter", since)
    machine.succeed("test ! -e /tmp/signed-in")
    # Refused once, saying "try again" rather than that it couldn't check.
    status, said = machine.execute(
        f"journalctl --identifier=greeter --output=cat --after-cursor={shlex.quote(since)}"
        " | grep -c 'report.refused greeter'"
    )
    assert said.strip() == "1", f"the greeter refused {said.strip()} times"
    machine.screenshot("greeter-refused")

with subtest("the greeter signs in with the right one"):
    since = mark()
    machine.send_chars(f"{secret}\n")
    heard(f"signs in {account}", since)
    # The file is the stand-in session's, on the machine, not a temporary file of the test's.
    machine.wait_for_file("/tmp/signed-in", timeout=60)  # noqa: S108
    signed_in = machine.succeed("cat /tmp/signed-in").strip()
    assert signed_in == f"{account} user", f"the session said {signed_in!r}"

with subtest("the greeter asks again after a logout"):
    since = mark()
    heard("keys greeter", since)
    machine.succeed("! pgrep -u greeter -x tuigreet")

with subtest("tuigreet asks when the greeter falls over"):
    # The brackets keep the test's own shell from matching.
    machine.succeed("pkill -u greeter -f '[q]uickshell[^ ]* -p '")
    machine.wait_until_succeeds("pgrep -u greeter -x tuigreet", timeout=60)
    machine.succeed("! pgrep -u greeter -f '[b]in/niri -c'")

# systemd starts no mount while it's over its limit on changes to the
# mount table, which held the mounts the greeter waits for at boot. Each
# systemd-mount waits for its mount, so each is a change of its own, and
# systemd says at debug level when the limit is reached. Debug goes only
# to the journal, since the serial console slows systemd enough to spread
# the burst out, and journald keeps it off the console too, so it keeps
# up and the last mount's line in the journal means all that came before
# it is there too.
with subtest("a burst of mounts doesn't wait on systemd's limit"):
    target = machine.succeed("systemd-analyze log-target").strip()
    machine.succeed("systemd-analyze log-target journal")
    machine.succeed("systemd-analyze log-level debug")
    machine.succeed(
        "for n in $(seq 12); do"
        " timeout 30 systemd-mount --quiet --type=tmpfs tmpfs /mnt/burst$n || exit 1;"
        " done"
    )
    machine.succeed("systemd-analyze log-level info")
    machine.succeed(f"systemd-analyze log-target {shlex.quote(target)}")
    machine.wait_until_succeeds(
        "journalctl --boot _PID=1 --output=cat | grep -qxF 'Mounted /mnt/burst12.'",
        timeout=30,
    )
    machine.fail(
        "journalctl --boot _PID=1 --output=cat"
        " | grep -F 'mount-monitor-dispatch) entered rate limit state'"
    )

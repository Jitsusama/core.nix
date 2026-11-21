"""A boot's last seconds kept where an administrator reads them after a reset, and no other account.

last-seconds.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. The tests need no
value from Nix. The power off, the sleep never woken from and the reset are last, since each
ends the boot the others read.
"""

from __future__ import annotations

import datetime as dt
import json
import time
from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# An administrator, in wheel, and an account that isn't one.
ADMINISTRATOR = "alice"
ACCOUNT = "bob"
# Where the rings are kept, a folder a boot.
STATE = "/var/lib/last-seconds"
# The recorder writes a record a second, so a few seconds give a few.
FEW = 3
# How many rest states a ring keeps, as src/record.rs's RESTS says.
RESTS = 8
# How much of a rest state's name a ring keeps, in bytes, as src/ring.rs's NAME says.
NAME = 8
# How soon after a wake the recorder writes again, in seconds: well before the minute after
# which it would go on unwoken, so only the wake can have started it.
WOKEN_TIMEOUT = 10
# What the sleep's hook says once it sees the recorder's record. It reaches the console
# through journald, which the kernel's freeze can stop before it does, so it's read from the
# journal once the machine is awake.
HOOK_SAW = "last-seconds-sleeps: second [0-9]* says the machine went to sleep"
# How long a few records may take to come, in seconds, on a loaded builder.
RECORDS_TIMEOUT = 60
# What last-seconds exits with when it may not read the rings, as docs/commands.md's table has.
REFUSED = 77


def this_boot(machine: BaseMachine) -> str:
    """Give this boot's id, as the folders name it: unhyphened."""
    return machine.succeed("tr -d '-' < /proc/sys/kernel/random/boot_id").strip()


def asked(machine: BaseMachine, words: str = "") -> dict[str, Any]:
    """Give what last-seconds answers the administrator, as JSON."""
    answer = machine.succeed(f"su {ADMINISTRATOR} -c 'last-seconds {words} --format json'")
    parsed: dict[str, Any] = json.loads(answer)
    return parsed


def seconds_of(machine: BaseMachine, boot: str) -> list[dict[str, Any]]:
    """Give every second kept of `boot`, oldest first."""
    seconds: list[dict[str, Any]] = asked(machine, f"{boot} --seconds 1000")["seconds"]
    return seconds


def kept_a_few(machine: BaseMachine) -> str:
    """Wait for a few of this boot's seconds to be kept, and give the boot."""
    machine.wait_for_unit("last-seconds.service")
    boot = this_boot(machine)
    deadline = time.monotonic() + RECORDS_TIMEOUT
    while asked(machine, boot)["seconds_kept"] < FEW:
        assert time.monotonic() < deadline, f"fewer than {FEW} seconds kept in time"
        # A record comes each second, so asking again each second misses none.
        time.sleep(1)
    return boot


def kept_past(machine: BaseMachine, boot: str, sequence: int) -> list[dict[str, Any]]:
    """Wait for a second past `sequence` to be kept, and give every second kept.

    The recorder writes a record only once the one before it is on the disk, so one past
    it says that one is there.
    """
    deadline = time.monotonic() + RECORDS_TIMEOUT
    while True:
        seconds = seconds_of(machine, boot)
        if seconds and seconds[-1]["sequence"] > sequence:
            return seconds
        assert time.monotonic() < deadline, f"nothing was kept past second {sequence}"
        # A record comes each second, so asking again each second misses none.
        time.sleep(1)


def test_the_recorder_runs_as_its_own_account_and_keeps_its_ring_for_wheel(
    machine: BaseMachine,
) -> None:
    boot = kept_a_few(machine)
    pid = machine.succeed("systemctl show -P MainPID last-seconds").strip()
    runs_as = machine.succeed(f"stat -c %U /proc/{pid}").strip()
    folder = machine.succeed(f"stat -c '%U %G %a' {STATE}/{boot}").strip()
    ring = machine.succeed(f"stat -c '%U %G %a' {STATE}/{boot}/ring").strip()
    assert runs_as == "last-seconds", f"the recorder runs as {runs_as}"
    assert folder == "last-seconds wheel 2750", f"the boot's folder is {folder}"
    assert ring == "last-seconds wheel 640", f"the ring is {ring}"


def test_an_administrator_reads_this_boot_as_still_being_recorded(machine: BaseMachine) -> None:
    boot = kept_a_few(machine)
    boots = asked(machine)["boots"]
    current = [kept for kept in boots if kept["boot"] == boot]
    assert len(current) == 1, f"this boot isn't listed once: {boots}"
    assert current[0]["current"], f"this boot isn't said to be the one running: {current}"
    assert current[0]["end"] == "recording", f"this boot's record has ended: {current}"


def test_what_the_kernel_gives_gets_through_the_recorders_sandbox(machine: BaseMachine) -> None:
    boot = kept_a_few(machine)
    newest = seconds_of(machine, boot)[-1]
    # QEMU gives no energy counters, fans or battery, but the kernel's own
    # load, memory and waits are there to be read, and a second record has
    # the waits' growth.
    assert newest["load_hundredths"] is not None, f"the load was hidden from it: {newest}"
    assert newest["available_kib"] is not None, f"memory was hidden from it: {newest}"
    assert newest["stalls_us"]["cpu_some"] is not None, f"the waits were hidden: {newest}"
    # The rest states are whatever the guest's idle driver gives, haltpoll's
    # here (last-seconds.nix), so the ring is held to the kernel's own list
    # rather than to a number, and each state it names has its time read. A
    # name may hold a space, as haltpoll's "haltpoll idle" does, and the ring
    # keeps its first NAME bytes.
    _, listed = machine.execute(
        "cd /sys/devices/system/cpu/cpu0/cpuidle 2> /dev/null &&"
        " for state in $(ls -dv state*); do cat $state/name; done"
    )
    given = [name[:NAME] for name in listed.splitlines()][:RESTS]
    named = asked(machine, boot)["rests"]
    read = newest["rests_us"][: len(given)]
    assert given, "the guest gives no rest states, so none could be shown hidden"
    assert named == given, f"the ring names rest states {named}, the kernel {given}"
    assert all(spent is not None for spent in read), f"the rest states were hidden: {newest}"


def test_an_account_that_isnt_an_administrator_is_refused(machine: BaseMachine) -> None:
    kept_a_few(machine)
    # The account has to be able to run anything at all, or a failing su
    # would pass alone.
    machine.succeed(f"su {ACCOUNT} -c 'last-seconds --help'")
    status, said = machine.execute(f"su {ACCOUNT} -c 'last-seconds' 2>&1")
    assert status == REFUSED, f"another account was answered with {status}: {said!r}"
    assert "id -nG" in said, f"the refusal doesn't say how to see the account's groups: {said!r}"


def test_a_stop_is_said_and_a_restart_goes_on_after_it(machine: BaseMachine) -> None:
    boot = kept_a_few(machine)
    machine.succeed("systemctl stop last-seconds")
    stopped = seconds_of(machine, boot)[-1]
    ended = [kept["end"] for kept in asked(machine)["boots"] if kept["boot"] == boot]
    assert stopped["stopped"], f"the newest record doesn't say it was stopped: {stopped}"
    assert ended == ["stopped"], f"a stopped boot is said to have ended {ended}"

    machine.succeed("systemctl start last-seconds")
    after = stopped["sequence"] + 1
    machine.wait_until_succeeds(
        f"su {ADMINISTRATOR} -c 'last-seconds {boot} --format json'"
        f" | grep -q '\"sequence\": *{after}\\b'",
        timeout=RECORDS_TIMEOUT,
    )
    kept = seconds_of(machine, boot)
    went_on = [second for second in kept if second["sequence"] == after]
    assert not went_on[0]["stopped"], f"the restart's first record says it stopped: {went_on}"
    # A ring made anew would also come to that second, in time, but without
    # the stop before it.
    assert any(second == stopped for second in kept), "the restart made the ring anew"


def test_a_recorder_killed_starts_again_by_itself_and_goes_on(machine: BaseMachine) -> None:
    boot = kept_a_few(machine)
    for _ in range(2):
        before = seconds_of(machine, boot)[-1]["sequence"]
        killed = machine.succeed("systemctl show -P MainPID last-seconds").strip()
        # A kill writes nothing on the way out, as a crash wouldn't.
        machine.succeed("systemctl kill --signal=SIGKILL last-seconds")
        # Once another recorder runs, the killed one can write nothing
        # more, so only a restart can write past the newest second then.
        machine.wait_until_succeeds(
            f"pid=$(systemctl show -P MainPID last-seconds); [ $pid != 0 ] && [ $pid != {killed} ]",
            timeout=RECORDS_TIMEOUT,
        )
        newest = seconds_of(machine, boot)[-1]["sequence"]
        went_on = [s for s in kept_past(machine, boot, newest) if s["sequence"] == newest + 1]
        assert newest >= before, f"the restart made the ring anew: {newest} after {before}"
        assert went_on, f"the restart didn't go on after second {newest}"
        assert not went_on[0]["stopped"], f"a kill was said as a stop: {went_on}"


def test_a_sleep_woken_from_goes_on_without_counting_the_sleep(machine: BaseMachine) -> None:
    boot = kept_a_few(machine)
    qemu = cast("QemuMachine", machine)
    # A key wakes a laptop, so the keyboard may wake the machine.
    machine.succeed("for w in /sys/bus/serio/devices/*/power/wakeup; do echo enabled > $w; done")
    recorder = machine.succeed("systemctl show -P MainPID,NRestarts last-seconds").split()
    machine.succeed("systemctl suspend")
    qemu.wait_for_console_text("PM: suspend entry", timeout=dt.timedelta(seconds=RECORDS_TIMEOUT))
    # A key while it's still going to sleep would only stop it going, and
    # the wait is the driver's, since the sleeping machine runs nothing.
    time.sleep(5)
    machine.send_key("shift")
    qemu.wait_for_console_text("PM: suspend exit", timeout=dt.timedelta(seconds=RECORDS_TIMEOUT))
    asleep = [second for second in seconds_of(machine, boot) if second["asleep"]]
    assert asleep, "no record said the machine went to sleep"
    slept = asleep[-1]["sequence"]
    deadline = time.monotonic() + WOKEN_TIMEOUT
    while True:
        after = [second for second in seconds_of(machine, boot) if second["sequence"] > slept]
        if after:
            break
        assert time.monotonic() < deadline, f"nothing was kept {WOKEN_TIMEOUT} s after the wake"
        # A record comes each second once woken, so asking again each second misses none.
        time.sleep(1)
    assert not after[0]["asleep"], f"the first record after the wake says asleep: {after[0]}"
    # A recorder started again would also write a record with no growth, so
    # it has to be the same one, never stopped.
    still = machine.succeed("systemctl show -P MainPID,NRestarts last-seconds").split()
    assert still == recorder, f"the recorder was started again over the sleep: {still}"
    assert not after[0]["stopped"], f"the wake was said as a stop: {after[0]}"
    machine.succeed(f"journalctl -b -u systemd-suspend.service | grep -q '{HOOK_SAW}'")
    # The waits grew only while it was awake, but over a sleep the energy
    # counters keep going, so the first record after says no growth at all.
    assert after[0]["stalls_us"]["cpu_some"] is None, f"it counted across the sleep: {after[0]}"


def test_a_power_off_stops_the_recorder_after_the_services(machine: BaseMachine) -> None:
    ended = kept_a_few(machine)
    machine.shutdown()
    machine.start()
    machine.wait_for_unit("last-seconds.service")
    boots = {kept["boot"]: kept for kept in asked(machine)["boots"]}
    assert ended in boots, f"the boot the power off ended isn't listed: {list(boots)}"
    assert boots[ended]["end"] == "stopped", f"a powered off boot ended {boots[ended]}"
    last = seconds_of(machine, ended)[-1]
    assert last["stopped"], "the power off's last record isn't a stop"
    # last-seconds.nix's service that stops late wrote when it stopped.
    late = int(machine.succeed("cat /var/lib/stops-late"))
    assert last["at_ms"] >= late, f"the recorder stopped at {last['at_ms']}, before {late}"


def test_a_sleep_never_woken_from_is_said_asleep(machine: BaseMachine) -> None:
    ended = kept_a_few(machine)
    qemu = cast("QemuMachine", machine)
    machine.succeed("systemctl suspend")
    # The hook waited for the recorder's record before the kernel's freeze,
    # which the boot read as asleep below shows.
    qemu.wait_for_console_text("PM: suspend entry", timeout=dt.timedelta(seconds=RECORDS_TIMEOUT))
    # Asleep, a reset ends it as a reset on the way out of a sleep would.
    qemu.crash()
    qemu.start()
    machine.wait_for_unit("last-seconds.service")
    boots = {kept["boot"]: kept for kept in asked(machine)["boots"]}
    assert ended in boots, f"the boot that slept isn't listed: {list(boots)}"
    assert boots[ended]["end"] == "asleep", f"a boot that never woke ended {boots[ended]}"
    said = machine.succeed(f"su {ADMINISTRATOR} -c 'last-seconds'")
    assert "never woke" in said, f"the list doesn't say the boot never woke: {said!r}"


def test_a_reset_leaves_the_boot_said_cut_short_with_its_last_seconds(
    machine: BaseMachine,
) -> None:
    ended = kept_a_few(machine)
    newest = kept_past(machine, ended, seconds_of(machine, ended)[-1]["sequence"])[-2]["sequence"]
    # A kill, as a reset or a loss of power is: nothing is stopped or
    # written on the way down. Only QEMU's machine can be killed.
    qemu = cast("QemuMachine", machine)
    qemu.crash()
    qemu.start()
    machine.wait_for_unit("last-seconds.service")
    boots = {kept["boot"]: kept for kept in asked(machine)["boots"]}
    assert ended in boots, f"the boot the reset ended isn't listed: {list(boots)}"
    assert boots[ended]["end"] == "cut", f"a reset boot is said to have ended {boots[ended]}"
    kept = [second["sequence"] for second in seconds_of(machine, ended)]
    assert newest in kept, f"the second last read before the reset was lost: {kept[-3:]}"
    said = machine.succeed(f"su {ADMINISTRATOR} -c 'last-seconds {ended[:8]}'")
    assert "cut short" in said, f"the boot's text doesn't say it was cut short: {said!r}"

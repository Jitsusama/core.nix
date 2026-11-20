"""A machine whose hang leaves a record: the watchdog armed, a lockup panics, the log sent.

hang.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The tests run in the file's order, since the last
one panics the machine. Where the listener keeps what it hears, and the name the machine's card
is given, are Nix values, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
import time
from typing import TYPE_CHECKING, Any, cast

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine, QemuMachine

WATCHDOG = "/sys/class/watchdog/watchdog0"
# The hook systemd-sleep runs before and after each sleep, as systemd-sleep runs it: with
# nothing in its environment but an empty PATH and the step it's taking.
SLEEPS = (
    "env -i PATH=/var/empty SYSTEMD_SLEEP_ACTION=hibernate"
    " /etc/systemd/system-sleep/watchdog-sleeps"
)
TARGETS = "/sys/kernel/config/netconsole"
# An address on the test's network that nothing holds, and the hardware address of a machine
# that held it once.
NOBODY = "192.168.1.250"
GONE = "52:54:00:12:01:aa"
# The hardware address of a machine that takes the listener's address while it's away.
STRANGER = "52:54:00:12:01:bb"
# Another address on a network numbered as the test's is, as another network's lease would be.
ELSEWHERE = "192.168.1.77/24"
# How netconsole says what it sends to isn't there (docs/commands.md).
UNAVAILABLE = 69
# How long the interface may take to come up and netconsole to start sending.
SENDING = dt.timedelta(seconds=90)
# How long a line may take to reach the listener.
HEARING = dt.timedelta(seconds=30)
# How long a card's going may take to stop its sending.
STOPPING = dt.timedelta(seconds=30)
# How long the machine may take to restart from when its panic is set off: the timer's 1 s and
# the kernel's 10 s, with room, but less than the 30 s after the panic that the watchdog takes
# at the earliest, so its reset can't pass for the kernel's.
RESTARTING = dt.timedelta(seconds=26)
# How long a hibernation takes in the test: past the 5 s in which systemd opens a watchdog
# once.
SLEEPING = 6


@pytest.fixture(scope="module")
def log(values: dict[str, Any]) -> str:
    """Give where the listener keeps what it hears."""
    path: str = values["log"]
    return path


@pytest.fixture(scope="module")
def card(values: dict[str, Any]) -> str:
    """Give the name the machine's card is given, as a dock's card is renamed."""
    name: str = values["card"]
    return name


def is_sending(machine: QemuMachine, card: str) -> None:
    """Wait until the machine sends its kernel's log through the card, and follows the listener."""
    try:
        machine.wait_until_succeeds(
            f"systemctl is-active netconsole-{card}.service", timeout=SENDING
        )
        machine.wait_until_succeeds(
            f"systemctl is-active netconsole-{card}-follows.timer", timeout=HEARING
        )
        # The sending stays up with nothing to send to, so the target is what says it sends.
        machine.wait_until_succeeds(f"grep -qx 1 {TARGETS}/{card}/enabled", timeout=SENDING)
    except Exception:
        # What the service said, since a unit that never sends says nothing on the console.
        _, said = machine.execute(f"systemctl status netconsole-{card}.service 2>&1")
        print(said)
        raise


@pytest.fixture(scope="module")
def sending(driver: Driver, machines: dict[str, BaseMachine], card: str) -> QemuMachine:
    """Give the machine once both are up and it sends its kernel's log to the listener."""
    driver.start_all()
    machines["listener"].wait_for_unit("netconsole.service")
    # The machine is QEMU's, whose console the panic is read from.
    machine = cast("QemuMachine", machines["machine"])
    machine.wait_for_unit("multi-user.target")
    is_sending(machine, card)
    return machine


def heard(listener: BaseMachine, log: str, line: str) -> None:
    """Wait until the listener has heard the line."""
    listener.wait_until_succeeds(f"grep -qF '{line}' {log}", timeout=HEARING)


def watchdog(machine: QemuMachine) -> str:
    """Say whether the watchdog counts."""
    return machine.succeed(f"cat {WATCHDOG}/state").strip()


def sent_to(machine: QemuMachine, card: str) -> str:
    """Give the hardware address the kernel sends its log to."""
    return machine.succeed(f"cat {TARGETS}/{card}/remote_mac").strip()


def test_the_watchdog_is_armed_for_a_minute(sending: QemuMachine) -> None:
    """Systemd arms the chipset's watchdog at a minute, and feeds it."""
    assert watchdog(sending) == "active", "systemd didn't arm the watchdog."
    assert sending.succeed(f"cat {WATCHDOG}/timeout").strip() == "60", (
        "The watchdog wasn't armed at 60 s."
    )


def test_the_watchdog_stops_for_a_hibernation_and_is_armed_again_after(
    sending: QemuMachine,
) -> None:
    """The sleep hook stops the watchdog for a hibernation, and systemd arms it again after."""
    sending.succeed(f"{SLEEPS} pre hibernate")
    assert watchdog(sending) == "inactive", "The watchdog kept counting into a hibernation."
    # systemd opens the watchdog at most once in 5 s, and otherwise only at its next feeding,
    # half a minute on. A hibernation is never that soon after the last open, so the test
    # waits it out, as a hibernation would, and the watchdog is to be back at once.
    time.sleep(SLEEPING)
    sending.succeed(f"{SLEEPS} post hibernate")
    assert watchdog(sending) == "active", "The watchdog wasn't armed again after a hibernation."
    assert sending.succeed(f"cat {WATCHDOG}/timeout").strip() == "60", (
        "The watchdog came back from a hibernation at another time than 60 s."
    )


def test_the_kernel_is_told_to_panic_at_a_hard_lockup_only(sending: QemuMachine) -> None:
    """The NMI watchdog is on beside perf, a hard lockup panics, and a soft one doesn't.

    QEMU gives the guest no performance counters, so the kernel has no NMI watchdog to run
    and reads it as on whatever it's told; what it's told is read instead.
    """
    assert "kernel.nmi_watchdog=1" in sending.succeed("cat /etc/sysctl.d/60-nixos.conf"), (
        "perf's NMI watchdog setting won, so no hard lockup would be seen."
    )
    # The test's machine names panic=1 on its command line after hang-panic's, as another
    # module might, so this is the sysctl holding once it's up.
    assert sending.succeed("sysctl -n kernel.panic").strip() == "10", (
        "A panic would restart the machine sooner than 10 s."
    )
    assert sending.succeed("sysctl -n kernel.hardlockup_panic").strip() == "1", (
        "A hard lockup would only warn, not panic."
    )
    assert sending.succeed("sysctl -n kernel.softlockup_panic").strip() == "0", (
        "A soft lockup would panic, so a busy machine's stalls would restart it."
    )


def test_the_kernels_log_reaches_the_listener_alone(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A line the kernel prints reaches the listener alone, and an outsider's isn't kept."""
    listener = machines["listener"]
    own = listener.succeed("cat /sys/class/net/eth1/address").strip()
    told = sent_to(sending, card)
    assert told == own, f"The kernel's log goes to {told}, not to the listener's {own}."
    # A line from outside the network named comes first, so it would be in the log by the
    # time the kernel's line is heard, were it kept.
    outsider = "hang test: a line from outside the network"
    listener.succeed(f"echo '{outsider}' | socat -u - UDP-SENDTO:127.0.0.1:6666")
    line = "hang test: a line the kernel prints"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(listener, log, line)
    listener.fail(f"grep -qF '{outsider}' {log}")
    # Each line carries the kernel's level, sequence number and time, so a gap or a line no
    # kernel sent shows.
    listener.succeed(f"grep -qE '^[0-9]+,[0-9]+,[0-9]+,[^;]*;{line}$' {log}")


def test_nothing_is_sent_where_nothing_answers(sending: QemuMachine, card: str) -> None:
    """A listener nothing answers for now is refused, though one did once; the target is kept.

    The kernel's table of neighbours keeps a machine that has gone for as long as nothing needs
    the room, as a listener's stand-in while it slept, so one is planted there.
    """
    told = sent_to(sending, card)
    sending.succeed(f"ip neigh replace {NOBODY} lladdr {GONE} nud stale dev {card}")
    command = sending.succeed(
        f"systemctl show -P ExecStart netconsole-{card}.service"
        " | sed -n 's/.*path=\\([^ ;]*\\).*/\\1/p'"
    ).strip()
    status, said = sending.execute(f"{command} start {card} {NOBODY} 6666 2>&1")
    assert status == UNAVAILABLE, (
        f"A listener nothing answers for wasn't refused as unavailable: {said}"
    )
    assert sent_to(sending, card) == told, "A refused listener changed where the log goes."


def test_the_log_follows_a_listener_that_moves_card(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A listener answering from another hardware address is sent to there at the next asking."""
    listener = machines["listener"]
    old = sent_to(sending, card)
    moved = "52:54:00:12:01:99"
    listener.succeed(f"ip link set dev eth1 address {moved}")
    # The machine still remembers the old card, as the laptop would for as long as nothing
    # needs the room.
    address = sending.succeed(f"cat {TARGETS}/{card}/remote_ip").strip()
    sending.succeed(f"ip neigh replace {address} lladdr {old} nud stale dev {card}")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    assert sent_to(sending, card) == moved, "The kernel's log still goes to the old card."
    line = "hang test: a line after the listener moved"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(listener, log, line)


def test_the_next_asking_sends_again_when_nothing_is_sent(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A target gone, as when the kernel refused it, or turned off, is made again."""
    command = sending.succeed(
        f"systemctl show -P ExecStart netconsole-{card}.service"
        " | sed -n 's/.*path=\\([^ ;]*\\).*/\\1/p'"
    ).strip()
    sending.succeed(f"{command} stop {card}")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    line = "hang test: a line after the target was made again"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(machines["listener"], log, line)
    # The kernel turns a target off itself when its interface goes, and leaves it there.
    sending.succeed(f"echo 0 > {TARGETS}/{card}/enabled")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    line = "hang test: a line after the target was turned on again"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(machines["listener"], log, line)


def test_a_follow_leaves_the_target_while_nothing_has_changed_or_answers(
    sending: QemuMachine, machines: dict[str, BaseMachine], card: str
) -> None:
    """A follow with the listener where it was, or asleep, keeps sending where it did."""
    listener = machines["listener"]
    told = sent_to(sending, card)
    # configfs gives each target made its own inode, so the same one is the same target.
    made = f"stat -c %i {TARGETS}/{card}"
    before = sending.succeed(made).strip()
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    assert sending.succeed(made).strip() == before, (
        "A follow made the target again with nothing changed, losing lines while it did."
    )
    listener.succeed("ip link set dev eth1 down")
    try:
        sending.succeed(f"systemctl start netconsole-{card}-follows.service")
        assert sent_to(sending, card) == told, "A follow let go of a listener that's asleep."
        sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
        # A .local listener asleep stops answering for its name too, as a name nothing ever
        # answers for doesn't; a name taken away would stay away in the name cache.
        command = sending.succeed(
            f"systemctl show -P ExecStart netconsole-{card}.service"
            " | sed -n 's/.*path=\\([^ ;]*\\).*/\\1/p'"
        ).strip()
        sending.fail("getent ahostsv4 nowhere.invalid")
        sending.succeed(f"{command} follow {card} nowhere.invalid 6666")
        assert sending.succeed(made).strip() == before, (
            "A follow let go of a listener asleep whose name went unanswered."
        )
        sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    finally:
        listener.succeed("ip link set dev eth1 up")


def test_a_follow_lets_the_target_go_where_the_listener_cant_be_found(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A card with no address on the listener's network stops sending, and sends once it's back.

    A dock's cable moved to another network leaves the card, so the target would otherwise keep
    sending to the listener's hardware address there, which a switch floods to every port.
    """
    listener = machines["listener"]
    own = sending.succeed(f"ip -4 -o address show dev {card} | awk '{{print $4}}'").strip()
    sending.succeed(f"ip address flush dev {card}")
    try:
        sending.succeed(f"systemctl start netconsole-{card}-follows.service")
        sending.fail(f"test -e {TARGETS}/{card}")
    finally:
        sending.succeed(f"ip address add {own} dev {card}")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    line = "hang test: a line once the card was back on the network"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(listener, log, line)
    # Another network numbered the same gives the card another address, and nothing there
    # answers for the listener's.
    listener.succeed("ip link set dev eth1 down")
    try:
        sending.succeed(f"ip address del {own} dev {card} && ip address add {ELSEWHERE} dev {card}")
        sending.succeed(f"systemctl start netconsole-{card}-follows.service")
        sending.fail(f"test -e {TARGETS}/{card}")
    finally:
        sending.succeed(f"ip address del {ELSEWHERE} dev {card} && ip address add {own} dev {card}")
        listener.succeed("ip link set dev eth1 up")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    command = sending.succeed(
        f"systemctl show -P ExecStart netconsole-{card}.service"
        " | sed -n 's/.*path=\\([^ ;]*\\).*/\\1/p'"
    ).strip()
    # A .local name goes unanswered on another network, where the address sent to may be
    # answered from anywhere, so it's asked for only from the address the card sent from.
    try:
        sending.succeed(f"ip address del {own} dev {card} && ip address add {ELSEWHERE} dev {card}")
        sending.succeed(f"{command} follow {card} nowhere.invalid 6666")
        sending.fail(f"test -e {TARGETS}/{card}")
    finally:
        sending.succeed(f"ip address del {ELSEWHERE} dev {card} && ip address add {own} dev {card}")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")
    # And another machine holding the address sent to, its name unanswered, isn't sent to.
    told = sent_to(sending, card)
    listener.succeed(f"ip link set dev eth1 address {STRANGER}")
    try:
        sending.succeed(f"{command} follow {card} nowhere.invalid 6666")
        sending.fail(f"test -e {TARGETS}/{card}")
    finally:
        listener.succeed(f"ip link set dev eth1 address {told}")
    sending.succeed(f"systemctl start netconsole-{card}-follows.service")
    sending.succeed(f"grep -qx 1 {TARGETS}/{card}/enabled")


def test_a_start_nothing_answers_is_tried_again_and_followed(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A start while the listener is away fails no switch, and sends once the listener is back.

    A switch counts a service waiting to try again after a failure as failed, so a start that
    found nothing to send to leaves the sending up, and its following tries again.
    """
    listener = machines["listener"]
    listener.succeed("ip link set dev eth1 down")
    try:
        sending.succeed(f"systemctl restart netconsole-{card}.service")
        sending.succeed(f"systemctl is-active netconsole-{card}.service")
        sending.fail(f"test -e {TARGETS}/{card}")
    finally:
        listener.succeed("ip link set dev eth1 up")
    # The following asks again 30 s after the start, which SENDING has room for.
    is_sending(sending, card)
    line = "hang test: a line after a retried start"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(listener, log, line)


def test_a_switch_that_adds_the_sending_with_the_card_in_starts_it(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """The sending starts with the system, not only as a card appears, which a switch isn't."""
    # Only the system pulls it in, so it's up because the system started it, as a switch does.
    state = sending.succeed(f"systemctl show -P ActiveState netconsole-{card}-starts.service")
    assert state.strip() == "active", "Nothing started the sending with the system."
    sending.succeed(f"systemctl stop netconsole-{card}.service")
    sending.fail(f"test -e {TARGETS}/{card}")
    # The switch that adds the sending loads its module beside starting it, so the sending
    # loads the module itself.
    sending.succeed("rmmod netconsole")
    sending.fail(f"test -e {TARGETS}")
    sending.succeed(f"systemctl restart netconsole-{card}-starts.service")
    is_sending(sending, card)
    line = "hang test: a line after the sending started with the system"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(machines["listener"], log, line)


def test_a_card_that_goes_and_comes_back_sends_again(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str, card: str
) -> None:
    """A card unplugged stops its sending, and plugged back in and renamed, sends again."""
    # The card's PCI device goes, as a dock's USB card does when it's unplugged.
    sending.succeed(f"echo 1 > /sys/class/net/{card}/device/../remove")
    sending.wait_until_fails(f"systemctl is-active netconsole-{card}.service", timeout=STOPPING)
    # A failed unit would fail every switch until the card came back.
    sending.fail(f"systemctl is-failed netconsole-{card}.service")
    sending.fail(f"test -e {TARGETS}/{card}")
    # It comes back under the kernel's name, and udev renames it, as the dock's card is.
    sending.succeed("echo 1 > /sys/bus/pci/rescan")
    is_sending(sending, card)
    line = "hang test: a line after the card came back"
    sending.succeed(f"echo '<3>{line}' > /dev/kmsg")
    heard(machines["listener"], log, line)


def test_a_panic_is_heard_before_the_restart(
    sending: QemuMachine, machines: dict[str, BaseMachine], log: str
) -> None:
    """A panic's words reach the listener, and the kernel restarts 10 s after."""
    # The kernel panics before a command that triggers it could say it's done, which the
    # driver would take for a broken machine, so a timer of the machine's triggers it instead.
    # Its accuracy is narrowed from systemd's minute, so it fires when it's set to.
    set_at = time.monotonic()
    sending.succeed(
        "systemd-run --on-active=1 --timer-property=AccuracySec=10ms"
        " sh -c 'echo c > /proc/sysrq-trigger'"
    )
    listener = machines["listener"]
    heard(listener, log, "Kernel panic - not syncing: sysrq triggered crash")
    # The kernel says when it'll restart, and the restart comes before the watchdog's could.
    heard(listener, log, "Rebooting in 10 seconds")
    # The panicked kernel restarts, which ends QEMU; waiting for that tells the driver the
    # machine is down, so it doesn't reach into it once the tests are over.
    left = RESTARTING.total_seconds() - (time.monotonic() - set_at)
    assert left > 0, "The panic took longer to be heard than the restart may take."
    sending.wait_for_shutdown(timeout=dt.timedelta(seconds=left))

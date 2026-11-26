"""The desktop test's locks and asks area.

Here the desktop locks and asks: the lock screen and what lets it go, polkit's prompt and the
askpass.

tests/desktop/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it: unlocked, the shell running, nothing of its
own still open, so any test runs alone.
"""

from __future__ import annotations

import shlex
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest
import vm_helpers
from desktop_session import PICTURE_COLOURS, SIGNING, STARTING, retry

if TYPE_CHECKING:
    from desktop_session import Desktop

# Where lock-screen and the shell say whether the screens are locked.
LOCKED = "$XDG_RUNTIME_DIR/quickshell-locked"
# The file that makes the lock's next password check stall (tests/desktop/desktop.nix).
STALLS = "/run/lock-stalls/once"
# Well past the 40 tries a new shell once asked polkit for its agent in before giving up:
# 250 ms apart, each after polkit's answer, so 10 s idle and more on a loaded host, where it's
# stretched as every wait is (vm_helpers.pace).
HELD_ASKED = timedelta(seconds=20)
# Stretched no further than this, short of the minute shell-hands-over.sh waits for the old
# launcher to close before it hands over anyway, so that launcher is still there to close.
HELD_AT_MOST = timedelta(seconds=45)
# How long each polkit ask is given to come up, a first ask after polkit restarts too, so a
# prompt drawn slowly on a busy builder is never read as refused (polkit_asked_again).
PROMPT_COMES = timedelta(seconds=30)
# Where an ask's systemctl leaves its exit status and what it said, so a refused ask is told
# from a slow or timed-out one: the account's own runtime folder, where nothing else writes.
ASK_ENDED = "/run/user/1000/polkit-ask-ended"


# Each test closes the launcher after it, so one failing with it open fails alone.
pytestmark = pytest.mark.usefixtures("launcher_closed_after")


def test_the_lock_screen_takes_only_the_right_password(desktop: Desktop) -> None:
    """The lock screen takes only the right password, and shows its picture meanwhile."""
    machine = desktop.machine
    with desktop.told("keys lock"):
        machine.send_key("meta_l-l")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")
    # PAM answers a wrong password after a delay, which the lock says.
    with desktop.told("refused lock"):
        machine.send_chars("wrong-password\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")

    # Its picture fades in, as the wallpaper's does.
    def pictured(last: bool) -> bool:  # As retry calls it.
        """Say whether the lock screen shows its picture rather than a flat colour."""
        _, colours, _ = desktop.colours_and_light("locked")
        assert not last or colours > PICTURE_COLOURS, "the lock screen shows a flat colour"
        return colours > PICTURE_COLOURS

    retry(pictured, timeout=timedelta(seconds=30))
    with desktop.told("released lock"):
        machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q false")


def test_a_hung_password_check_is_tried_again(desktop: Desktop) -> None:
    """A password check that hangs is killed and tried again, so the lock still takes it.

    Quickshell's check can hang for good (Lock.qml says why), which left the lock refusing to
    check any password again; here the machine stalls the first check in its place.
    """
    machine = desktop.machine
    with desktop.told("keys lock"):
        machine.send_key("meta_l-l")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")
    desktop.as_account(f"touch {STALLS}")
    since = desktop.mark()
    machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.reported("retried lock", lambda rest: rest == "", since)
    desktop.reported("released lock", lambda rest: rest == "", since)
    desktop.as_account("qs ipc call lock isLocked | grep -q false")
    desktop.as_account(f"! test -e {STALLS}")


def hinted(desktop: Desktop, locked: str) -> None:
    """Wait for logind to say the account's graphical session is locked or not, as polkit asks."""
    session = f"$(loginctl show-user {desktop.account} --property=Display --value)"
    desktop.machine.wait_until_succeeds(
        f'[ "$(loginctl show-session {session} --property=LockedHint --value)" = {locked} ]',
        timeout=10,
    )


def test_lock_screen_returns_once_the_lock_is_drawn(desktop: Desktop) -> None:
    """What locks before the machine sleeps returns only once niri has drawn the lock.

    So nothing else shows on waking. Logind is told each time, for polkit's rules.
    """
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    hinted(desktop, "no")
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    hinted(desktop, "yes")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    hinted(desktop, "no")


def test_the_lock_lets_go_without_its_password_when_released(desktop: Desktop) -> None:
    """The lock lets go without its password when released, and starts empty the next time.

    swayidle releases it when logind says to unlock, as after a resume from hibernation, where
    the disk's prompt has already asked for the password.
    """
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    # Half a password, left in the field.
    desktop.machine.send_chars("half-a-pass")
    with desktop.told("released lock"):
        desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    # Released again while unlocked, nothing happens.
    desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    # The next lock takes the password alone, so nothing was left behind.
    desktop.as_account("lock-screen")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_logind_saying_to_unlock_lets_the_lock_go(desktop: Desktop) -> None:
    """Logind saying to unlock lets the lock go.

    As after a resume from hibernation, where the disk's prompt has already asked for the
    password, so the lock doesn't ask again.
    """
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.succeed("loginctl unlock-sessions")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


# Where swayidle-away marks someone at the machine.
HERE = "$XDG_RUNTIME_DIR/someone-at-the-machine"

# swayidle-away's own seconds, made short: the mark, the lock, and the screens going off, late
# enough that the test never turns them off.
SOON = "2 4 3600"


def test_the_screen_locks_as_joel_steps_away_past_holds_that_keep_it_awake(
    desktop: Desktop,
) -> None:
    """The screen locks as Joel steps away though mycelium holds logind's idle off.

    A swayidle that hears logind drops every timeout while anything holds logind's lock
    against going idle, and mycelium holds it whenever it holds work, so the screen went
    unlocked for as long as Joel was away (2026-11-10). swayidle-away asks logind nothing.
    The test runs it with short times, since typing by wtype counts as no key here.
    """
    machine = desktop.machine
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account(
        "systemd-run --user --unit=holds-sessions systemd-inhibit --what=idle --mode=block"
        " --who=mycelium '--why=It holds sessions for the realm' sleep infinity"
    )
    try:
        desktop.as_account(f"touch {HERE}")
        with desktop.told("secured lock", timeout=timedelta(seconds=60)):
            desktop.as_account(f"systemd-run --user --unit=away-soon swayidle-away {SOON}")
        desktop.as_account(f"grep -qx locked {LOCKED}")
        desktop.as_account(f"test ! -e {HERE}")
        # Each lock says why it came and that it was drawn, so a lock that stops coming
        # shows as a gap in the journal.
        desktop.as_account("journalctl --user -u away-soon | grep -q 'locking: Joel stepped away'")
        machine.wait_until_succeeds(
            desktop.in_session("journalctl --user -u quickshell | grep -q 'lock: drawn'"),
            timeout=timedelta(seconds=30),
        )
        with desktop.told("released lock"):
            desktop.as_account("qs ipc call lock release")
        machine.wait_until_succeeds(
            desktop.in_session("journalctl --user -u quickshell | grep -q 'lock: lifted'"),
            timeout=timedelta(seconds=30),
        )
    finally:
        desktop.as_account("systemctl --user stop away-soon holds-sessions || true")
        desktop.as_account("qs ipc call lock release")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_a_restart_of_swayidle_away_marks_nobody_at_the_machine(desktop: Desktop) -> None:
    """Restarting swayidle-away marks nobody at the machine, and keeps a mark already there.

    A switch restarts swayidle-away, whose start used to touch the mark, so every switch read
    as Joel at the machine for two minutes and stopped what waits for him to be away
    (2026-11-11). Only a key or a touch marks him now, and typing by wtype counts as none here.
    """
    desktop.as_account(f"rm -f {HERE}")
    try:
        desktop.as_account("systemctl --user start swayidle-away")
        desktop.as_account("systemctl --user restart swayidle-away")
        desktop.as_account("systemctl --user is-active swayidle-away")
        desktop.as_account(f"test ! -e {HERE}")
        desktop.as_account(f"touch {HERE}")
        desktop.as_account("systemctl --user restart swayidle-away")
        desktop.as_account(f"test -e {HERE}")
    finally:
        desktop.as_account("systemctl --user stop swayidle-away || true")
        desktop.as_account(f"rm -f {HERE}")


def test_keep_awake_keeps_the_screen_unlocked_as_joel_steps_away(desktop: Desktop) -> None:
    """Keep Awake keeps the screen on and unlocked as Joel steps away, as a call's hold does."""
    machine = desktop.machine
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account("systemctl --user start keep-awake.service")
    try:
        desktop.as_account(f"systemd-run --user --unit=away-kept swayidle-away {SOON}")
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user -u away-kept | grep -q 'the screen stays: Keep Awake holds it'"
            ),
            timeout=timedelta(seconds=60),
        )
        desktop.as_account(f"grep -qx unlocked {LOCKED}")
    finally:
        desktop.as_account("systemctl --user stop away-kept keep-awake.service || true")
        desktop.as_account("qs ipc call lock release")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_polkits_prompt_lets_the_account_start_a_system_service(desktop: Desktop) -> None:
    """Polkit's prompt names the command asking, and lets the account start a system service.

    Its words come from the asking process, read by the pid polkit gives, where without them it
    could say only that systemd asked, as it did for every agent's run0.
    """
    machine = desktop.machine
    asked = "polkit asked by systemctl start polkit-allowed.service"
    with desktop.told("keys polkit"), desktop.told(asked):
        desktop.as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
    desktop.shoot("polkit")
    machine.send_chars(f"{desktop.values['secret']}\n")
    machine.wait_for_file("/run/polkit-allowed", timeout=timedelta(seconds=30))
    # The journal says how each ask ended, and whether niri gave the prompt the keys, so an ask
    # that fails on Joel's machine, where reports are off, is read rather than guessed. The
    # ask's end is heard through the real flow, so none here ends without saying how.
    # The keys go as the prompt is put away, which isn't a loss, so the line ends at when it had
    # them.
    ended = (
        "polkit: org.freedesktop.systemd1.manage-units answered after .*;"
        " it had the keys [0-9]* ms in$"
    )
    unsaid = "ended without saying how"
    journal = "journalctl --user -u quickshell -o cat"
    machine.wait_until_succeeds(
        desktop.in_session(f"{journal} | grep -q {shlex.quote(ended)}"),
        timeout=timedelta(seconds=30),
    )
    machine.succeed(desktop.in_session(f"! {journal} | grep -q {shlex.quote(unsaid)}"))


def test_polkits_prompt_comes_up_after_polkit_restarts(desktop: Desktop) -> None:
    """Polkit's prompt still comes up after polkit's service restarts beneath the shell.

    The shell registers its agent with polkit as it starts, and a switch can restart polkit
    under it without restarting the shell. polkit's agent library then registers the agent
    again by itself, so this guards that library, not the held shell's tries: a polkit or
    quickshell that stopped registering again would fail it. It doesn't replay 23 Nov, when
    polkit had restarted three times under one shell before its prompts stopped coming up,
    since that shell held the agent and drew nothing, a fault still unexplained.
    """
    for _ in range(3):
        polkit_restarted(desktop)
    polkit_asked_again(desktop, again=True)


def test_polkits_prompt_comes_up_after_a_hand_over_that_waited(desktop: Desktop) -> None:
    """Polkit's prompt still comes up after a hand-over that waited on the launcher closing.

    The new shell can take polkit's agent only once the old one has ended, and a hand-over
    puts that off while the launcher is open, for up to a minute (shell-hands-over.sh). A new
    shell that asked for the agent for 10 s from its start and then gave up held none after a
    longer wait, so every ask after found no agent, so it asks again once it's released.
    """
    machine = desktop.machine
    main = "systemctl --user show quickshell -p MainPID --value"
    old = desktop.as_account(main).strip()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    since = desktop.mark()
    journal = f"journalctl --user -u quickshell -o cat --after-cursor={shlex.quote(since)}"
    desktop.as_account("systemctl --user --no-block reload quickshell")
    said = f"{journal} | grep -qF"
    took = False
    try:
        machine.wait_until_succeeds(
            desktop.in_session(f"{said} 'Held until released'"), timeout=STARTING
        )
        held = min(HELD_ASKED * vm_helpers.pace(vm_helpers.busyness()), HELD_AT_MOST)
        # Past the tries a held shell once asked for the agent in, the old launcher still open.
        machine.sleep(duration=held)
        # Two shells run now, so the old one's launcher is closed by its own process.
        desktop.as_account(f"qs ipc --pid {old} call launcher toggle")
        # Handed over, the unit's main process is the new shell, never none (0).
        handed = f'pid="$({main})"; test "$pid" != {old} && test "$pid" != 0'
        machine.wait_until_succeeds(desktop.in_session(handed), timeout=STARTING)
        machine.wait_until_succeeds(
            desktop.in_session(f'{said} "took polkit\'s agent after"'), timeout=STARTING
        )
        took = True
    finally:
        # Two shells, or a new one holding no agent, would fail the tests after for this one's
        # reason, so the shell starts again and they run as they would alone.
        if not took:
            print(desktop.diagnosis("systemctl --user status quickshell"))
            shell_started_again(desktop)
    # The new shell has said it holds the agent, so a refused ask here is the fault itself.
    polkit_asked_again(desktop, again=False)


def polkit_restarted(desktop: Desktop) -> None:
    """Restart polkit's service, and wait for it to hold its name on the system bus again.

    The shell's agent is registered again by polkit's own agent library as the name comes back,
    saying nothing (polkit_asked_again).
    """
    machine = desktop.machine
    machine.succeed("systemctl restart polkit.service")
    machine.wait_for_unit("polkit.service")
    machine.wait_until_succeeds("busctl --system status org.freedesktop.PolicyKit1 > /dev/null")


def polkit_asked_again(desktop: Desktop, *, again: bool) -> None:
    """Asks polkit to start a system service, and answers its prompt, which must come up.

    With again, an ask refused before polkit's library registered the agent is asked once more.
    """
    machine = desktop.machine
    asked = "polkit asked by systemctl start polkit-allowed.service"
    # The service only ever makes the file, so one left by an earlier ask would pass this.
    machine.succeed(f"rm -f /run/polkit-allowed {ASK_ENDED}")
    ask = (
        "niri msg action spawn -- sh -c"
        ' \'said="$(systemctl start polkit-allowed.service 2>&1)";'
        f' echo "$? $said" > {ASK_ENDED}\''
    )
    answered = False
    try:
        with desktop.told("keys polkit"), desktop.told("drawn polkit"):
            try:
                with desktop.told(asked, timeout=PROMPT_COMES):
                    desktop.as_account(ask)
            except AssertionError:
                if not again:
                    raise
                # polkit's agent library registers the shell's agent again once polkit is back,
                # and polkit says nothing of it at its default log level, so there's nothing to
                # wait on. An ask just before finds no agent and is refused at once, so only an
                # ask that ended refused is asked once more, and that one's prompt must come up.
                # One still waiting, one an agent took and drew nothing for, and one systemctl
                # gave up on as timed out fail here, saying how the ask ended.
                refused = (
                    f"test -s {ASK_ENDED} && ! grep -q '^0 ' {ASK_ENDED}"
                    f" && ! grep -qi 'timed out' {ASK_ENDED}"
                )
                if machine.execute(refused)[0] != 0:
                    print(f"the first ask ended: {machine.execute(f'cat {ASK_ENDED}')[1]!r}")
                    raise
                machine.succeed(f"rm -f {ASK_ENDED}")
                with desktop.told(asked):
                    desktop.as_account(ask)
        machine.send_chars(f"{desktop.values['secret']}\n")
        machine.wait_for_file("/run/polkit-allowed", timeout=timedelta(seconds=30))
        answered = True
    finally:
        # What niri held and what polkit said are read before anything changes them, since
        # a prompt that never came up is what's still unexplained from 23 Nov. A shell that
        # drew none then drew none for later asks either, and one left asking would take the
        # keys from the tests after, so it starts again and they run as they would alone.
        if not answered:
            print(desktop.diagnosis("niri msg layers"))
            print(machine.execute("journalctl -b -u polkit.service -n 20 -o cat")[1])
            shell_started_again(desktop)


def shell_started_again(desktop: Desktop) -> None:
    """Start the shell again, and wait for it to serve the askpass the tests after ask."""
    desktop.as_account("systemctl --user restart quickshell")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
    )


def test_the_askpass_asks_for_a_yes_a_pin_and_a_touch(desktop: Desktop) -> None:
    """The askpass asks for a yes, a PIN and a touch, each answered as OpenSSH expects."""
    machine = desktop.machine
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.shoot("askpass")
    machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("gone askpass"):
        machine.send_key("esc")
    desktop.answered_with("confirm", 1)

    with desktop.told("keys askpass"):
        desktop.ask(
            "secret", f"Enter PIN for ED25519-SK key /home/{desktop.account}/.ssh/id_ed25519_sk: "
        )
    desktop.shoot("askpass-pin")
    machine.send_chars("2468\n")
    desktop.answered_with("secret", 0, "2468\n")

    # A touch takes no keys, and goes once ssh stops waiting on it.
    with desktop.told("drawn askpass"):
        desktop.ask("none", "Confirm user presence for key ED25519-SK SHA256:abc")
    with desktop.told("gone askpass"):
        desktop.as_account("pkill -TERM -f '^[^ ]*bash [^ ]*ssh-askpass Confirm'")
    desktop.answered_with("none", 1)
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")


def test_the_askpass_finds_the_shell_for_a_program_started_before_the_desktop(
    desktop: Desktop,
) -> None:
    """The askpass finds the shell for a program started before the desktop.

    linux-id's service starts with the account's systemd, before the desktop says which
    display it's on, and an askpass it runs knows no display.
    """
    with desktop.told("keys askpass"):
        desktop.as_account(
            "systemd-run --user --collect --quiet -- env -u WAYLAND_DISPLAY "
            f"{desktop.values['asking']} confirm '{SIGNING}'"
        )
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)


def test_the_askpass_asks_a_restarted_shell_again_and_gives_up_on_a_gone_one(
    desktop: Desktop,
) -> None:
    """The askpass asks a restarted shell again, and gives up on a gone one.

    The shell holds an ask only in memory, as a switch restarts it.
    """
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("keys askpass", timeout=timedelta(seconds=60)):
        desktop.as_account("systemctl --user restart quickshell")
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.as_account("systemctl --user stop quickshell")
    desktop.answered_with("confirm", 1, timeout=timedelta(seconds=60))
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")
    desktop.as_account("systemctl --user start quickshell")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
    )


def test_the_lock_holds_when_the_shell_restarts_and_only_for_its_niri(desktop: Desktop) -> None:
    """The lock holds when the shell restarts, and only for its niri.

    A switch restarts the shell, and can come while Joel is away and the screens are locked.
    niri holds a lock whose shell died, so the shell it starts again has to take that lock
    back, or nothing is left to type the password into.
    """
    holder = "$XDG_RUNTIME_DIR/quickshell-lock-holder"
    desktop.as_account("lock-screen")
    since = desktop.mark()
    desktop.as_account("systemctl --user restart quickshell")
    for word in ("secured lock", "keys lock"):
        desktop.reported(word, lambda rest: rest == "", since, timeout=timedelta(seconds=60))
    desktop.as_account("qs ipc call lock isLocked | grep -qx true")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")

    def started_unlocked() -> None:
        """Restart the shell, and check that it starts with the screens unlocked."""
        desktop.as_account("systemctl --user restart quickshell")
        desktop.machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
        )
        desktop.as_account("qs ipc call lock isLocked | grep -qx false")
        desktop.as_account(f"grep -qx unlocked {LOCKED}")

    # Let go, the lock isn't taken again.
    started_unlocked()
    # The runtime directory outlives a session while the account
    # lingers, so a lock a crashed niri held is left to that niri.
    desktop.as_account(f"echo $XDG_RUNTIME_DIR/niri.wayland-1.1.sock > {holder}")
    desktop.as_account(f"echo locked > {LOCKED}")
    started_unlocked()


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along.

    A binding that throws or assigns the wrong type only warns, and only once a surface draws
    it, which can be long after the start, so this comes last.
    """
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

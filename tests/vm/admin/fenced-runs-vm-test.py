"""An agent's run, started through its fence's launcher, reaches nothing the fence keeps out.

fenced-runs.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
Each run goes as the node starts an agent's `mycelium run`: the fence's `runs` from the account's
declared.json, then the login shell, then the command, in the run's folder. The account's user bus,
a display's socket, the node's socket, an SSH agent and an abstract X socket are all live beside it,
and each is first shown reachable from outside, so a refusal inside is the fence's. Its keys and
GitHub sign-in stay unread, and it writes only its folder, which it still can. A folder that is
home, or holds it, runs nothing.
"""

from __future__ import annotations

import re
import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HOME = "/home/someone"
RUNTIME = "/run/user/1000"
FOLDER = f"{HOME}/work"
AGENT = f"{RUNTIME}/ssh-agent.sock"
# What a session's environment points at, as the node's own would.
ENVIRONMENT = {
    "HOME": HOME,
    "XDG_RUNTIME_DIR": RUNTIME,
    "DBUS_SESSION_BUS_ADDRESS": f"unix:path={RUNTIME}/bus",
    "WAYLAND_DISPLAY": "wayland-1",
    "SSH_AUTH_SOCK": AGENT,
}
# Each socket a fenced command must not reach, by what it is.
SOCKETS = {
    "the user bus": f"UNIX-CONNECT:{RUNTIME}/bus",
    "the display": f"UNIX-CONNECT:{RUNTIME}/wayland-1",
    "the node": f"UNIX-CONNECT:{RUNTIME}/mycelium/home.sock",
    "the ssh agent": f"UNIX-CONNECT:{AGENT}",
    "an abstract X socket": "ABSTRACT-CONNECT:/tmp/.X11-unix/X0",
}
SECRETS = [f"{HOME}/.ssh/id_ed25519", f"{HOME}/.config/gh/hosts.yml"]
KINDS = ["fence", "strict"]
# How a fence says no: Landlock's refusals, or what its mounts hide or hold
# read-only. Any other failure, a missing program say, would pass a refusal
# test for nothing.
REFUSALS = [
    "Permission denied",
    "Operation not permitted",
    "No such file or directory",
    "Read-only file system",
]
# A login shell in the strict fence can't read the home's startup files and
# says so, which is no refusal of the command it then runs.
STARTUP = re.compile(r"^-?bash: .*/\.(bash_profile|bash_login|profile|bashrc)\b")


def refused(status: int, said: str) -> bool:
    """Say whether a command failed as a fence refuses, not for some other reason."""
    lines = [line for line in said.splitlines() if not STARTUP.match(line)]
    return status != 0 and any(refusal in line for line in lines for refusal in REFUSALS)


def as_someone(machine: BaseMachine, command: str, folder: str = FOLDER) -> tuple[int, str]:
    """Run a shell command as the account in `folder`, with its session's environment."""
    variables = " ".join(f"{name}={shlex.quote(value)}" for name, value in ENVIRONMENT.items())
    return machine.execute(
        f"cd {shlex.quote(folder)} && runuser -u someone -- env {variables} "
        f"bash -c {shlex.quote(command)} 2>&1"
    )


def fenced(machine: BaseMachine, kind: str, command: str, folder: str = FOLDER) -> tuple[int, str]:
    """Run `command` as an agent's run in fence `kind`: its runs, a login shell, then it."""
    runs = (
        f"mapfile -t runs < <(jq -r '.fence.{kind}.runs[]' {HOME}/.config/mycelium/declared.json)"
    )
    started = f'"${{runs[@]}}" "$SHELL" -lc {shlex.quote(command)}'
    return as_someone(machine, f"{runs} && {started}", folder)


def reaches(socket: str) -> str:
    """Give the command that connects to `socket`, saying nothing, and fails if it can't."""
    return f"socat -u OPEN:/dev/null {socket}"


@pytest.fixture(scope="module")
def up(machine: BaseMachine) -> BaseMachine:
    """Give the machine with the account's sockets listening and its secrets in place."""
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(f"test -S {RUNTIME}/bus")
    listening = [
        f"socat UNIX-LISTEN:{RUNTIME}/wayland-1,fork /dev/null",
        (
            f"mkdir -p {RUNTIME}/mycelium && "
            f"exec socat UNIX-LISTEN:{RUNTIME}/mycelium/home.sock,fork /dev/null"
        ),
        "socat ABSTRACT-LISTEN:/tmp/.X11-unix/X0,fork /dev/null",
        f"exec ssh-agent -D -a {AGENT}",
    ]
    for listener in listening:
        machine.succeed(
            f"runuser -u someone -- setsid -f bash -c {shlex.quote(listener)} >/dev/null 2>&1"
        )
    for socket in [f"{RUNTIME}/wayland-1", f"{RUNTIME}/mycelium/home.sock", AGENT]:
        machine.wait_until_succeeds(f"test -S {socket}")
    machine.wait_until_succeeds("grep -q '@/tmp/.X11-unix/X0' /proc/net/unix")
    for secret in SECRETS:
        machine.succeed(
            f"runuser -u someone -- sh -c 'mkdir -p $(dirname {secret}) && echo secret > {secret}'"
        )
    machine.succeed(f"runuser -u someone -- mkdir -p {FOLDER}")
    return machine


@pytest.mark.parametrize("kind", KINDS)
def test_a_run_works_in_its_folder(up: BaseMachine, kind: str) -> None:
    status, said = fenced(up, kind, "echo made > made && cat made")
    assert status == 0, said
    assert "made" in said


@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("what", SOCKETS)
def test_a_run_reaches_no_socket_of_the_account(up: BaseMachine, kind: str, what: str) -> None:
    status, said = as_someone(up, reaches(SOCKETS[what]))
    assert status == 0, f"{what} isn't reachable even outside the fence: {said}"
    status, said = fenced(up, kind, reaches(SOCKETS[what]))
    assert refused(status, said), f"a {kind} run reached {what}: {said}"


@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("secret", SECRETS)
def test_a_run_reads_no_secret(up: BaseMachine, kind: str, secret: str) -> None:
    status, said = as_someone(up, f"cat {secret}")
    assert status == 0, said
    status, said = fenced(up, kind, f"cat {secret}")
    assert refused(status, said), f"a {kind} run read {secret}: {said}"
    assert "secret" not in said.splitlines()


@pytest.mark.parametrize("kind", KINDS)
# The machine's own /tmp, which a fenced run must not write, not this test's.
@pytest.mark.parametrize("outside", [f"{HOME}/outside", "/tmp/outside"])  # noqa: S108
def test_a_run_writes_nothing_outside_its_folder(up: BaseMachine, kind: str, outside: str) -> None:
    status, said = fenced(up, kind, f"touch {outside}")
    assert refused(status, said), f"a {kind} run wrote {outside}: {said}"
    up.fail(f"test -e {outside}")


@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("folder", [HOME, "/home"])
def test_a_run_in_a_folder_holding_home_runs_nothing(
    up: BaseMachine, kind: str, folder: str
) -> None:
    # The shell works the sum out only if it runs, which even /home, that
    # the account can't write, shows.
    status, said = fenced(up, kind, "echo ran $((6 * 7))", folder)
    assert status != 0, f"a {kind} run in {folder} ran: {said}"
    assert "ran 42" not in said, f"a {kind} run in {folder} ran: {said}"

"""Root's Nix is given only the lender's lines that name a builder nix-offload's lines trust.

offload-lines.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what stands in for the cloud. The lender here is the test, writing
as the lender's account what the lender would: its lines, the instance it lent and a token.
"""

from __future__ import annotations

import base64
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

LENDER = "/run/nix-offload"
ROOTS = "/run/nix-offload-lines"
# How long the service may take to copy what the lender wrote: it reads every two seconds,
# and asks the cloud once for an instance it hasn't seen.
COPYING = timedelta(minutes=1)
# How long root's prover may take to drop a builder it never reaches: two proofs, a minute
# apart, each failing within 45 s.
PROVING = timedelta(minutes=3)
# How long the keeper may take to end once idle: a minute unasked.
KEEPER_IDLES = timedelta(minutes=2)


def carried(key: str) -> str:
    """Give a host key as a line carries it, base64 of its type and key."""
    return base64.b64encode(f"ssh-ed25519 {key}".encode()).decode()


def as_lender(machine: BaseMachine, command: str) -> None:
    """Run `command` as the lender's account, as the lender would."""
    machine.succeed(f"runuser -u someone -- sh -c {shlex.quote(command)}")


def write(machine: BaseMachine, path: str, text: str) -> None:
    """Write `text` to `path` as the lender's account."""
    as_lender(machine, f"printf %s {shlex.quote(text)} > {path}")


@pytest.fixture(scope="module")
def lent(values: dict[str, Any]) -> str:
    """Give the line naming the lent builder with the host key the cloud gives for it."""
    return f"{values['lentUri']} x86_64-linux /etc/nix/offload_ed25519 8 1 big-parallel - " + (
        carried(values["lentKey"])
    )


@pytest.fixture(scope="module")
def standing(values: dict[str, Any]) -> str:
    """Give the line naming the standing builder as the system's settings trust it."""
    key: str = values["standingKey"]
    return f"{values['standingUri']} x86_64-linux /etc/nix/builder_ed25519 4 1 - - {key}"


@pytest.fixture(scope="module")
def lending(
    driver: Driver, machines: dict[str, BaseMachine], lent: str, standing: str
) -> BaseMachine:
    """Give the machine once the lender has lent a builder and named one it didn't."""
    driver.start_all()
    machine = machines["machine"]
    machine.wait_for_unit("nix-offload-lines.service")
    token = f"{LENDER}/cloud-token"
    write(machine, token, "ya29.a-token")
    # As the lender leaves it: its own, and read by the service's account through an ACL, since
    # the lender's account is in none of the service's groups.
    as_lender(machine, f"chmod 0600 {token} && setfacl -m u:nix-offload-lines:r {token}")
    write(machine, f"{LENDER}/instance", "a-builder-1 a-zone\n")
    forged = lent.rsplit(" ", 1)[0] + " " + carried("AAAAforged")
    write(machine, f"{LENDER}/machines", f"{lent}\n{forged}\n")
    write(machine, f"{LENDER}/machines-with-standing", f"{standing}\n")
    return machine


def test_root_is_given_only_the_lines_it_trusts(
    lending: BaseMachine, lent: str, standing: str, values: dict[str, Any]
) -> None:
    """The lent builder's line and the standing one's reach root; a forged host key doesn't.

    The lent file names the lent builder by its lender and instance, for the fence's agent.
    """
    lending.wait_until_succeeds(f"grep -qxF {shlex.quote(lent)} {ROOTS}/machines", timeout=COPYING)
    assert lending.succeed(f"cat {ROOTS}/machines") == f"{lent}\n"
    vouched = f"nix-offload a-builder-1 ssh-ed25519 {values['lentKey']}\n"
    named = lending.succeed(f"cat {ROOTS}/lent")
    assert named == vouched, f"the lent file says {named!r}, not {vouched!r}"
    lending.wait_until_succeeds(
        f"grep -qxF {shlex.quote(standing)} {ROOTS}/machines-with-standing", timeout=COPYING
    )
    assert lending.succeed(f"cat {ROOTS}/machines-with-standing") == f"{standing}\n"


def test_the_lenders_account_is_in_none_of_the_services_groups(lending: BaseMachine) -> None:
    """The lender's account hands the token over by an ACL, so needs no group of the service's."""
    groups = lending.succeed("id -nG someone").split()
    assert "nix-offload-lines" not in groups, groups


def test_the_lenders_account_cannot_write_roots_lines(lending: BaseMachine) -> None:
    """Root's lines are the service's account's, which the lender's can't change or add to."""
    owners = lending.succeed(f"stat -c %U {ROOTS} {ROOTS}/machines {ROOTS}/lent").split()
    assert owners == ["nix-offload-lines"] * 3, owners
    lending.fail(f"runuser -u someone -- sh -c ': >> {ROOTS}/machines'")
    lending.fail(f"runuser -u someone -- sh -c ': >> {ROOTS}/lent'")
    lending.fail(f"runuser -u someone -- touch {ROOTS}/machines.draft")


def test_root_alone_says_how_the_standing_builder_stands(
    lending: BaseMachine, values: dict[str, Any]
) -> None:
    """Root proves the standing builder, here never reached, in a file the lender can't write."""
    said = "/run/nix-offload-standing/health.json"
    uri = values["standingUri"]
    # Two proofs a minute apart fail before it's dropped.
    lending.wait_until_succeeds(
        f'grep -qF \'"uri":"{uri}"\' {said} && grep -qF \'"state":"dropped"\' {said}',
        timeout=PROVING,
    )
    # Dropped because no builder took the build, nearby being unreachable,
    # not because root's Nix or the unit's fence refused the proof itself,
    # which would drop every builder.
    failed = lending.succeed(
        "journalctl -u nix-offload-prove-standing -o cat | grep 'failed its proof'"
    )
    assert "max-jobs" in failed, failed
    # A restart, as a switch makes, keeps what was last said.
    lending.succeed(f"systemctl restart nix-offload-prove-standing && test -s {said}")
    owners = lending.succeed(f"stat -c %U /run/nix-offload-standing {said}").split()
    assert owners == ["root", "root"], owners
    lending.fail(f"runuser -u someone -- sh -c ': >> {said}'")
    lending.fail("runuser -u someone -- touch /run/nix-offload-standing/health.writing")


def test_what_passed_is_kept_where_the_account_can_remove_none_of_it(
    lending: BaseMachine, values: dict[str, Any]
) -> None:
    """The account has the keeper keep what passed, reads it, and can remove only a commit's."""
    kept, socket = "/var/lib/nix-offload-passed", "/run/nix-offload-keep.sock"
    first, second = "a" * 40, "b" * 40
    lending.wait_for_unit("nix-offload-keep.socket")

    def ask(asked: str) -> str:
        command = f"printf %s {shlex.quote(asked)} | {values['socat']} -t5 - UNIX-CONNECT:{socket}"
        return lending.succeed(f"runuser -u someone -- sh -c {shlex.quote(command)}")

    for commit in (first, second):
        said = ask(f'{{"keep":"{commit}","lines":[{{"check":"checks.x.a"}}]}}')
        assert said == "ok\n", said
    said = ask('{"keep":"abc","lines":[{"check":"checks.x.a"}]}')
    assert said.startswith("refused"), said
    # Readers take the keeper's folder while its socket's unit is there.
    lending.succeed("test -e /etc/systemd/system/nix-offload-keep.socket")
    record = lending.succeed(f"runuser -u someone -- cat {kept}/{first}")
    assert record == '{"check":"checks.x.a"}\n', record
    owners = lending.succeed(f"stat -c %U {kept} {kept}/{first}").split()
    assert owners == ["nix-offload-passed", "nix-offload-passed"], owners

    lending.fail(f"runuser -u someone -- rm -rf {kept}/{first} {kept}/{second}")
    lending.fail(f"runuser -u someone -- sh -c ': > {kept}/{first}'")
    lending.succeed(f"test -s {kept}/{first} && test -s {kept}/{second}")

    lending.succeed(f"runuser -u someone -- {values['offload']} passed forget {first}")
    lending.succeed(f"test ! -e {kept}/{first} && test -s {kept}/{second}")
    # Asked nothing for a minute, it ends, and its socket starts it again when next asked.
    lending.wait_until_fails("systemctl is-active nix-offload-keep.service", timeout=KEEPER_IDLES)


DOOR = "/run/nix-offload-standing.sock"


def test_the_lenders_account_reaches_the_standing_builders_untrusted(
    lending: BaseMachine, standing: str
) -> None:
    """The account's build through the door is offered the standing builder; another isn't."""
    lending.wait_until_succeeds(
        f"grep -qxF {shlex.quote(standing)} {ROOTS}/machines-with-standing", timeout=COPYING
    )
    lending.wait_for_unit("nix-offload-standing.socket")
    # Nix says what it found on its standard error.
    asked = f"nix --extra-experimental-features nix-command store info --store unix://{DOOR} 2>&1"
    info = lending.succeed(f"runuser -u someone -- sh -c {shlex.quote(asked)}")
    assert "Trusted: 0" in info.splitlines(), info

    def built(name: str, store: str) -> str:
        """Build a derivation nothing has built, as the account, and give what Nix said."""
        expression = (
            f'derivation {{ name = "{name}-" + builtins.toString builtins.currentTime; '
            'system = builtins.currentSystem; builder = "/bin/sh"; args = [ "-c" "echo > $out" ]; }'
        )
        command = (
            f"nix --extra-experimental-features nix-command build --no-link --impure {store} "
            f"--expr {shlex.quote(expression)} 2>&1"
        )
        return lending.succeed(f"runuser -u someone -- sh -c {shlex.quote(command)}")

    # Unreachable here, the standing builder is tried and the build falls back to the machine.
    through = built("through", f"--store unix://{DOOR}")
    assert "nearby" in through, through
    around = built("around", "")
    assert "nearby" not in around, around


def test_no_other_account_opens_the_standing_door(
    lending: BaseMachine, values: dict[str, Any]
) -> None:
    """The door is in root's folder and the lender's account alone may open it."""
    lending.wait_for_unit("nix-offload-standing.socket")
    owner = lending.succeed(f"stat -c '%U %a' /run {DOOR}").split()
    assert owner == ["root", "755", "someone", "600"], owner
    lending.fail(f"runuser -u nobody -- {values['socat']} -u /dev/null UNIX-CONNECT:{DOOR}")


def test_nothing_lent_takes_the_lent_line_away(lending: BaseMachine, standing: str) -> None:
    """With no instance named, the lent builder's line no longer reaches root, the standing does.

    Nor does the lent file name it.
    """
    write(lending, f"{LENDER}/instance", "")
    lending.wait_until_succeeds(f"test ! -s {ROOTS}/machines", timeout=COPYING)
    lending.wait_until_succeeds(f"test ! -s {ROOTS}/lent", timeout=COPYING)
    assert lending.succeed(f"cat {ROOTS}/machines-with-standing") == f"{standing}\n"

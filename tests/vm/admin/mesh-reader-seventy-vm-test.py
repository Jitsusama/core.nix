"""The mesh reader reads each change once for seventy pi processes, and tells each within 100 ms.

mesh-reader-seventy.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), on a machine whose account lingers with the reader's socket and service
as their module renders them. The clients are mesh-reader-clients.py, put at /etc. While they
follow, inotifywait counts every open of the file they follow, by any process: only the reader
opens it, so the count is the reader's reads. The tests need no value from Nix.
"""

from __future__ import annotations

import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

SOCKET = "/run/user/1000/pi-mesh-reader/reader.sock"
SERVICE = "pi-mesh-reader.service"
FOLDER = "/home/alice/.local/state/pi/presence"
OPENS = "/home/alice/opens"
# As many as the pi processes that each listed the presence folder on 21 November.
CLIENTS = 70
# Publishes of the one file, apart, all within the reader's 30 s backstop listing.
CHANGES = 20
# The slowest a client may be told a change, from its publish.
TOLD_WITHIN_MS = 100
# The threads the reader keeps once its clients go: its listener and the folder's follower.
KEPT_THREADS = 2
# How long the clients' threads may take to go, well inside the 30 s backstop.
THREADS_GONE_S = 10


def as_alice(command: str) -> str:
    """Give a command that runs as alice, reaching her user manager."""
    return "su - alice -c " + shlex.quote(f"export XDG_RUNTIME_DIR=/run/user/1000; {command}")


@pytest.fixture(scope="module")
def alice(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with alice's manager running and her presence folder made."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(as_alice(f"test -S {SOCKET}"))
    machine.succeed(as_alice(f"mkdir -p {FOLDER}"))
    return machine


def test_the_first_to_ask_starts_it_and_seventy_are_each_told_each_change_once_read(
    alice: BaseMachine,
) -> None:
    """Seventy clients start one reader, which reads each change once and tells each in time."""
    state = alice.succeed(as_alice(f"systemctl --user show -p ActiveState --value {SERVICE}"))
    assert state.strip() == "inactive", f"the reader ran before anyone asked: {state}"
    alice.succeed(
        as_alice(
            f"inotifywait -m -q -e open --format '%e %f' {FOLDER} > {OPENS} 2>&1 & sleep 1",
        ),
    )
    slowest = alice.succeed(
        as_alice(f"python3 /etc/mesh-reader-clients.py {SOCKET} {FOLDER} {CLIENTS} {CHANGES}"),
    )
    alice.succeed(as_alice("pkill -x inotifywait || true"))
    opens = alice.succeed(as_alice(f"grep -c '^OPEN a.json$' {OPENS} || true")).strip()
    # The first listing reads the file once, and each change once more.
    assert int(opens) == CHANGES + 1, f"the file was opened {opens} times for {CHANGES} changes"
    assert int(slowest) <= TOLD_WITHIN_MS, f"the slowest client was told after {slowest} ms"
    # Its listener and the folder's follower, each client's two threads gone
    # within the second each looks whether its client went.
    alice.wait_until_succeeds(
        as_alice(
            f"test $(systemctl --user show -p TasksCurrent --value {SERVICE}) -le {KEPT_THREADS}",
        ),
        timeout=THREADS_GONE_S,
    )

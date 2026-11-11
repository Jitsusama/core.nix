"""A focused session window's memory in swap is read back, and nobody else's.

swap-in.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The sorter starts memory-swap-in@PID.service as the focus comes to a window, and the unit reads
back what swap holds of PID's scope only if PID belongs to the session window niri says has the
focus, at most once a window every 30 s. A stand-in niri, run as someone's own niri.service,
answers for someone, an administrator, from a file the tests write, and stand-in heaps play pi's,
each pushed out to swap on disk first.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# Each heap's size, in MiB: enough that its reads show over the noise.
SIZE = 64


@pytest.fixture(scope="module")
def up(machine: BaseMachine) -> BaseMachine:
    """Give the machine once it has booted, with a stand-in niri answering for someone."""
    machine.wait_for_unit("multi-user.target")
    runtime = runtime_of(machine)
    machine.wait_for_unit(f"user@{runtime.rsplit('/', 1)[1]}.service")
    machine.succeed(
        "systemd-run --machine=someone@.host --user --unit=niri "
        "/run/current-system/sw/bin/python3 /etc/swap-in/niri.py "
        f"{runtime}/niri.test.sock {runtime}/focus.json"
    )
    machine.wait_for_file(f"{runtime}/niri.test.sock")
    return machine


def runtime_of(machine: BaseMachine) -> str:
    """Give someone's runtime folder, where niri's socket is."""
    return "/run/user/" + machine.succeed("id -u someone").strip()


def heap(machine: BaseMachine, name: str, app_id: str) -> int:
    """Start a heap as someone, as the pi of window `app_id`, push it to swap, and give its pid."""
    said = f"{runtime_of(machine)}/heap-{name}.pid"
    machine.succeed(
        f"systemd-run --uid=someone --unit=heap-{name} -p MemoryZSwapMax=0 "
        f"--setenv=MYCELIUM_APP_ID={app_id} python3 /etc/swap-in/heap.py {SIZE} {said}"
    )
    machine.wait_for_file(said)
    # Reclaim can stop short of all it was asked, so what reached swap is checked instead.
    machine.execute(f"echo {SIZE}M > {scope(name)}/memory.reclaim")
    swapped = int(machine.succeed(f"cat {scope(name)}/memory.swap.current"))
    assert swapped >= (SIZE // 2) << 20, f"heap {name} has only {swapped} bytes in swap"
    return int(machine.succeed(f"cat {said}"))


def scope(name: str) -> str:
    """Give a heap's cgroup folder."""
    return f"/sys/fs/cgroup/system.slice/heap-{name}.service"


def resident(machine: BaseMachine, name: str) -> int:
    """Give the bytes of memory a heap's cgroup holds.

    The hint reads pages into the swap cache without mapping them, so memory.stat counts them as
    swapcached rather than anon; what the cgroup holds counts both.
    """
    return int(machine.succeed(f"cat {scope(name)}/memory.current"))


def focus(machine: BaseMachine, window: int, app_id: str) -> None:
    """Give the focus, as niri says it, to window `window` with `app_id`."""
    said = json.dumps({"id": window, "app_id": app_id, "pid": 0, "is_focused": True})
    machine.succeed(f"echo '{said}' > {runtime_of(machine)}/focus.json")


def swap_in(machine: BaseMachine, account: str, pid: int) -> tuple[int, str]:
    """Start the unit for `pid` as `account`, and give its status and what it said."""
    status, said = machine.execute(
        f"runuser -u {account} -- systemctl start memory-swap-in@{pid}.service 2>&1"
    )
    journal = machine.succeed(f"journalctl -u memory-swap-in@{pid}.service -o cat --no-pager")
    return status, said + journal


def test_the_focused_windows_memory_is_read_back(up: BaseMachine) -> None:
    pid = heap(up, "focused", "pi-session.1")
    before = resident(up, "focused")
    focus(up, 1, "pi-session.1")
    status, said = swap_in(up, "someone", pid)
    assert status == 0, said
    assert "Read back" in said, said
    # The hint queues the reads, which land as the kernel gets to them.
    up.wait_until_succeeds(
        f"test $(cat {scope('focused')}/memory.current) -ge {before + ((SIZE // 2) << 20)}",
        timeout=30,
    )


def test_a_window_is_read_back_once_in_30_s(up: BaseMachine) -> None:
    pid = heap(up, "twice", "pi-session.2")
    focus(up, 2, "pi-session.2")
    status, said = swap_in(up, "someone", pid)
    assert status == 0, said
    assert up.succeed("stat -c %U:%a /run/memory-swap-in").strip() == "root:700", (
        "the stamps aren't root's alone"
    )
    # Whatever the account clears of its own, the window's turn is kept.
    up.execute(f"runuser -u someone -- rm -rf /run/memory-swap-in {runtime_of(up)}/memory-swap-in")
    status, said = swap_in(up, "someone", pid)
    assert status == 0, said
    assert "was read back within 30s" in said, said


def test_a_process_outside_the_focused_window_is_left_alone(up: BaseMachine) -> None:
    pid = heap(up, "elsewhere", "pi-session.3")
    before = resident(up, "elsewhere")
    focus(up, 3, "pi-session.4")
    status, said = swap_in(up, "someone", pid)
    assert status != 0, said
    assert "isn't in the focused window" in said, said
    assert resident(up, "elsewhere") < before + (8 << 20), "the heap was read back anyway"
    # A call while the window lacks the focus doesn't use up its turn.
    focus(up, 3, "pi-session.3")
    status, said = swap_in(up, "someone", pid)
    assert status == 0, said
    assert "Read back" in said, said


def test_a_socket_that_isnt_niris_is_never_asked(up: BaseMachine) -> None:
    pid = heap(up, "planted", "pi-session.8")
    runtime = runtime_of(up)
    planted = json.dumps({"id": 8, "app_id": "pi-session.8", "pid": 0, "is_focused": True})
    up.succeed(f"echo '{planted}' > {runtime}/planted.json")
    # Named to sort ahead of niri's own, as any of the account's processes could.
    up.succeed(
        "systemd-run --uid=someone --unit=planted-niri python3 /etc/swap-in/niri.py "
        f"{runtime}/niri.0.sock {runtime}/planted.json"
    )
    up.wait_for_file(f"{runtime}/niri.0.sock")
    focus(up, 9, "pi-session.9")
    status, said = swap_in(up, "someone", pid)
    up.succeed("systemctl stop planted-niri.service")
    assert status != 0, said
    assert "isn't in the focused window 9" in said, said


def test_a_window_that_is_no_session_is_left_alone(up: BaseMachine) -> None:
    pid = heap(up, "browser", "firefox")
    focus(up, 5, "firefox")
    status, said = swap_in(up, "someone", pid)
    assert status != 0, said
    assert "isn't in the focused window" in said, said


def test_roots_processes_are_never_read_back(up: BaseMachine) -> None:
    pid = int(up.succeed("systemctl show -P MainPID systemd-journald").strip())
    status, said = swap_in(up, "someone", pid)
    assert status != 0, said
    assert "is root's" in said, said


def test_an_account_outside_wheel_may_not_start_it(up: BaseMachine) -> None:
    pid = heap(up, "refused", "pi-session.6")
    status, said = swap_in(up, "other", pid)
    assert status != 0, said
    assert "Read back" not in said, said

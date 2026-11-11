"""A focused session window's memory in swap is read back, and nobody else's.

swap-in.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The sorter starts memory-swap-in@PID.service as the focus comes to a window, and the unit reads
back what swap holds of PID's scope only if PID belongs to the session window niri says has the
focus, at most once a window every 30 s. A stand-in niri, run as someone's own niri.service,
answers for someone, an administrator, from a file the tests write, and stand-in heaps play pi's,
each pushed out to swap on disk first. A stand-in kitty plays one drawing many windows whose pis
have no app id, which only it can tell apart.
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


def focus(machine: BaseMachine, window: int, app_id: str, pid: int = 0, title: str = "") -> None:
    """Give the focus, as niri says it, to window `window` with `app_id`, drawn by `pid`."""
    said = json.dumps(
        {"id": window, "app_id": app_id, "pid": pid, "title": title, "is_focused": True}
    )
    machine.succeed(f"echo '{said}' > {runtime_of(machine)}/focus.json")


def kitty(machine: BaseMachine, name: str, heaps: int, listens_as: str = "") -> int:
    """Start a stand-in kitty as someone, with `heaps` heaps of its own, and give its pid."""
    runtime = runtime_of(machine)
    machine.succeed(
        f"systemd-run --machine=someone@.host --user --unit=kitty-{name} "
        "/run/current-system/sw/bin/python3 /etc/swap-in/kitty.py "
        f"{runtime} {name} {heaps} {SIZE} {listens_as}"
    )
    for each in [name] + [f"{name}-{number}" for number in range(1, heaps + 1)]:
        machine.wait_for_file(f"{runtime}/{each}.pid")
    return int(machine.succeed(f"cat {runtime}/{name}.pid"))


def shows(machine: BaseMachine, name: str, windows: list[tuple[bool, str, int]]) -> None:
    """Have kitty `name` list `windows`, each its focus, its title and the pid it runs."""
    listed = [
        {
            "id": number,
            "is_focused": focused,
            "tabs": [
                {"is_active": True, "windows": [{"is_active": True, "title": title, "pid": pid}]}
            ],
        }
        for number, (focused, title, pid) in enumerate(windows, 1)
    ]
    machine.succeed(f"echo '{json.dumps(listed)}' > {runtime_of(machine)}/{name}-shown.json")


def held_by(machine: BaseMachine, pid: int) -> str:
    """Give the cgroup folder `pid` runs in."""
    cgroup = machine.succeed(f"cat /proc/{pid}/cgroup").strip()
    return "/sys/fs/cgroup" + cgroup.split("::", 1)[1]


def pid_in(machine: BaseMachine, name: str) -> int:
    """Give the pid a stand-in wrote to `name`.pid."""
    return int(machine.succeed(f"cat {runtime_of(machine)}/{name}.pid"))


def stop_kittys(machine: BaseMachine) -> None:
    """Stop every stand-in kitty and its heaps."""
    machine.execute("systemctl --machine=someone@.host --user stop 'kitty-*'")


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
    # The focus having moved on is an answer, not a failure: the unit ends well.
    assert status == 0, said
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
    assert status == 0, said
    assert "isn't in the focused window 9" in said, said


def test_the_window_a_shared_kitty_names_is_read_back(up: BaseMachine) -> None:
    runtime = runtime_of(up)
    drawing = kitty(up, "shared", 2)
    left, focused = pid_in(up, "shared-1"), pid_in(up, "shared-2")
    shows(up, "shared", [(False, "π - left", left), (True, "π - focused", focused)])
    up.wait_for_file(f"{runtime}/shared.listening")
    cgroup = held_by(up, focused)
    up.execute(f"echo {SIZE}M > {cgroup}/memory.reclaim")
    swapped = int(up.succeed(f"cat {cgroup}/memory.swap.current"))
    assert swapped >= (SIZE // 2) << 20, f"the focused heap has only {swapped} bytes in swap"
    before = int(up.succeed(f"cat {cgroup}/memory.current"))
    focus(up, 20, "pi-spawned", pid=drawing, title="π - focused")
    status, said = swap_in(up, "someone", left)
    assert status == 0, said
    assert "isn't in the focused window 20" in said, said
    status, said = swap_in(up, "someone", focused)
    assert status == 0, said
    assert "Read back" in said, said
    up.wait_until_succeeds(
        f"test $(cat {cgroup}/memory.current) -ge {before + ((SIZE // 2) << 20)}", timeout=30
    )
    asked = up.succeed(f"cat {runtime}/shared-asked.txt").split()
    stop_kittys(up)
    assert asked, "kitty was never asked"
    assert set(asked) == {"ls"}, f"kitty was sent {asked}, not only ls"


def test_a_kitty_socket_planted_by_another_process_is_never_asked(up: BaseMachine) -> None:
    runtime = runtime_of(up)
    drawing = kitty(up, "unheard", 2, "none")
    named = pid_in(up, "unheard-1")
    shows(up, "planted", [(True, "π - planted", named)])
    # Under the name of the kitty drawing the window, as any of the account's processes could.
    kitty(up, "planted", 0, str(drawing))
    up.wait_for_file(f"{runtime}/planted.listening")
    focus(up, 21, "pi-spawned", pid=drawing, title="π - planted")
    status, said = swap_in(up, "someone", named)
    sent = up.execute(f"test -e {runtime}/planted-asked.txt")[0] == 0
    stop_kittys(up)
    assert status == 0, said
    assert "isn't in the focused window 21" in said, said
    assert not sent, "the planted socket was sent a command"


def test_a_window_that_is_no_session_is_left_alone(up: BaseMachine) -> None:
    pid = heap(up, "browser", "firefox")
    focus(up, 5, "firefox")
    status, said = swap_in(up, "someone", pid)
    assert status == 0, said
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

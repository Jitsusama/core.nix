"""The qemu VM tests run starts once the sockets it connects to are there, or fails naming one.

checks.nix's qemu-starts-after-its-sockets runs this with QEMU, a stand-in qemu put through
qemu-after-its-sockets.nix, whose qemu-system-x86_64 writes its arguments to the file QEMU_RAN
names.
"""

from __future__ import annotations

import os
import socket
import subprocess
import time
from pathlib import Path

import pytest

QEMU = Path(os.environ["QEMU"])


def chardev(path: Path, *extra: str) -> list[str]:
    """Give the arguments for a socket chardev at path."""
    return ["-chardev", ",".join(["socket", "id=share", f"path={path}", *extra])]


def started(ran: Path, *arguments: str) -> subprocess.Popen[str]:
    """Start the qemu with arguments, telling the stand-in where to say it ran."""
    return subprocess.Popen(  # noqa: S603
        [QEMU / "bin" / "qemu-system-x86_64", *arguments],
        env={**os.environ, "QEMU_RAN": str(ran)},
        stderr=subprocess.PIPE,
        text=True,
    )


def gone(pid: int) -> bool:
    """Say whether a process has ended, a zombie counting as ended."""
    try:
        state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
    except FileNotFoundError:
        return True
    return state == "Z"


def test_qemu_starts_only_once_its_socket_is_there(tmp_path: Path) -> None:
    """Qemu starts only once the socket it connects to is there."""
    ran = tmp_path / "ran"
    share = tmp_path / "share"
    qemu = started(ran, *chardev(share), "-m", "64")
    time.sleep(1)
    assert not ran.exists(), "qemu started before its socket was there"
    with socket.socket(socket.AF_UNIX) as listening:
        listening.bind(str(share))
        assert qemu.wait(timeout=30) == 0
    arguments = ran.read_text().splitlines()
    assert arguments == [*chardev(share), "-m", "64"]


def test_a_socket_qemu_serves_is_not_waited_for(tmp_path: Path) -> None:
    """A socket qemu serves itself isn't waited for."""
    ran = tmp_path / "ran"
    qemu = started(ran, *chardev(tmp_path / "monitor", "server=on", "wait=off"))
    assert qemu.wait(timeout=10) == 0
    assert ran.exists()


def test_a_socket_that_never_comes_fails_and_ends_what_the_script_started(
    tmp_path: Path,
) -> None:
    """A socket that never comes fails, naming it, and ends what the script started."""
    ran = tmp_path / "ran"
    missing = tmp_path / "missing"
    started_by_script = tmp_path / "pid"
    # As nixpkgs' script starts a virtiofsd in the background and then becomes qemu.
    script = (
        f"sleep 300 & echo $! > {started_by_script}; "
        f'exec {QEMU / "bin" / "qemu-system-x86_64"} "$@"'
    )
    qemu = subprocess.Popen(  # noqa: S603
        ["bash", "-c", script, "bash", *chardev(missing)],  # noqa: S607
        env={**os.environ, "QEMU_RAN": str(ran), "QEMU_SOCKETS_WAIT": "2"},
        stderr=subprocess.PIPE,
        text=True,
    )
    _, said = qemu.communicate(timeout=30)
    assert qemu.returncode == 1
    assert str(missing) in said
    assert not ran.exists()
    sleeper = int(started_by_script.read_text())
    deadline = time.monotonic() + 10
    while not gone(sleeper):
        if time.monotonic() > deadline:
            pytest.fail("what the script started was left running")
        time.sleep(0.1)


def test_the_rest_of_qemu_is_the_qemu_given() -> None:
    """The rest of qemu, qemu-img among it, is the qemu given."""
    assert (QEMU / "bin" / "qemu-img").resolve().read_text().startswith("#!")

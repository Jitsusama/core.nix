"""Opens password prompts on niri as Joel's and as an agent's, and checks only the agent's closes.

On 2026-11-17 an agent's test started its own keyring, whose prompt opened on Joel's screen, and
he typed his real password into it. The windows daemon knows an agent's window by its process's
cgroup (unasked.rs), so it closes one that asks for a password as it opens and tells Joel which
unit it came from. The build sandbox has no cgroups to put a process in, so the daemon reads
processes from a stand-in /proc (`WINDOWS_PROC`), where this process's cgroup is rewritten
before each window, and its notices go to a stand-in notify-send that keeps what it was told
(`WINDOWS_TOLD`). Then:

1. A prompt of Joel's own, gcr-prompter from app.slice, isn't closed.
2. The same prompt from an agent's job scope is asked to close within a second, the journal
   says so, and Joel is told it came from that scope.
3. A terminal from the same scope isn't closed: only what asks for a password is.

Every case is checked and reported before the stand-in fails.

usage: closes-agents-password-prompts.py <windows daemon>
"""

import json
import os
import socket
import subprocess
import sys
import time
from pathlib import Path

from stand_in import PATIENCE, Connection

APP = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gcr-prompter-1.scope\n"
JOB = (
    "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/"
    "jobs-work-keyring.slice/run-r1.scope\n"
)
# How long niri and the daemon have to close an agent's prompt, as the guard promises.
CLOSING = 1.0


def answering(daemon: Path) -> bool:
    """Whether the daemon has heard niri's desktop, so a window opened now is new to it."""
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as asking:
            asking.connect(str(daemon))
            asking.sendall(json.dumps({"id": 1, "ask": "hello", "version": 1}).encode() + b"\n")
            said = json.loads(asking.makefile("rb").readline())
    except OSError:
        return False
    # hello says each source by its state alone.
    return bool(said.get("sources", {}).get("niri") == "answering")


def main() -> None:
    """Run the three cases against the daemon, and fail naming each that didn't hold."""
    program = sys.argv[1]
    here = Path.cwd()
    processes = here / "proc"
    me = processes / str(os.getpid())
    me.mkdir(parents=True)
    (me / "cwd").symlink_to(here)
    cgroup = me / "cgroup"
    cgroup.write_text(APP)
    told = here / "told"
    told.touch()
    daemon = here / "windows.sock"
    journal = (here / "logs" / "windows.log").open("w")
    running = subprocess.Popen(  # The daemon under test, named by the check.
        [program, str(daemon)],
        env={**os.environ, "WINDOWS_PROC": str(processes), "WINDOWS_TOLD": str(told)},
        stderr=journal,
    )
    failures: list[str] = []
    try:
        until = time.monotonic() + PATIENCE
        while not answering(daemon):
            if time.monotonic() > until:
                sys.exit("the windows daemon never heard niri")
            time.sleep(0.1)
        joel = Connection(Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"])

        own = joel.window("gcr-prompter")
        if joel.asked_to_close(own, CLOSING):
            failures.append("1: Joel's own password prompt was closed")

        cgroup.write_text(JOB)
        agents = joel.window("gcr-prompter")
        if not joel.asked_to_close(agents, CLOSING):
            failures.append(f"2: an agent's password prompt wasn't closed within {CLOSING} s")
        said = (here / "logs" / "windows.log").read_text()
        if "closed a password prompt an agent opened: gcr-prompter" not in said:
            failures.append(f"2: the journal didn't say the prompt was closed: {said!r}")
        until = time.monotonic() + PATIENCE
        while "run-r1.scope" not in told.read_text() and time.monotonic() < until:
            time.sleep(0.05)
        if f"in {here}" not in told.read_text() or "run-r1.scope" not in told.read_text():
            failures.append(f"2: Joel wasn't told its scope and folder: {told.read_text()!r}")

        terminal = joel.window("foot")
        if joel.asked_to_close(terminal, CLOSING):
            failures.append("3: an agent's terminal was closed")
    finally:
        running.terminate()
        running.wait()
        journal.close()
    if failures:
        sys.exit("\n".join(failures))
    print("an agent's password prompt closes, and neither Joel's nor its terminal does")


if __name__ == "__main__":
    main()

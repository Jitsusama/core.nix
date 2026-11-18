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
3. A terminal from the same scope isn't closed, once the journal shows the daemon weighed it:
   only what asks for a password is.
4. A second prompt from the scope within a minute is closed, and Joel isn't told twice.
5. A prompt from the scope already open as the daemon starts again, as after a switch, is
   closed once it hears niri.

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


def said(journal: Path, words: str) -> bool:
    """Whether the daemon's journal says `words` within its patience, as it writes lines late."""
    until = time.monotonic() + PATIENCE
    while words not in journal.read_text():
        if time.monotonic() > until:
            return False
        time.sleep(0.05)
    return True


def heard_niri(daemon: Path) -> None:
    """Wait until the daemon has heard niri's desktop, or end the check."""
    until = time.monotonic() + PATIENCE
    while not answering(daemon):
        if time.monotonic() > until:
            sys.exit("the windows daemon never heard niri")
        time.sleep(0.1)


def written(path: Path, words: str, times: int) -> str:
    """Read `path` once it says `words` `times` times, or once patience runs out."""
    until = time.monotonic() + PATIENCE
    while path.read_text().count(words) < times and time.monotonic() < until:
        time.sleep(0.05)
    return path.read_text()


def prompts_close(joel: Connection, cgroup: Path, lines: Path, told: Path) -> list[str]:
    """Cases 1 and 2: Joel's own prompt stays, and the job's is closed, journalled and told."""
    failures: list[str] = []
    own = joel.window("gcr-prompter")
    if joel.asked_to_close(own, CLOSING):
        failures.append("1: Joel's own password prompt was closed")

    cgroup.write_text(JOB)
    agents = joel.window("gcr-prompter")
    if not joel.asked_to_close(agents, CLOSING):
        failures.append(f"2: an agent's password prompt wasn't closed within {CLOSING} s")
    if not said(lines, 'close a password prompt an agent opened: "gcr-prompter"'):
        failures.append(f"2: the journal didn't say it was closed: {lines.read_text()!r}")
    notices = written(told, "run-r1.scope", 1)
    if f"in {Path.cwd()}" not in notices or "run-r1.scope" not in notices:
        failures.append(f"2: Joel wasn't told its scope and folder: {notices!r}")
    return failures


def only_prompts_close(joel: Connection, lines: Path, told: Path) -> list[str]:
    """Cases 3 and 4: the job's terminal stays, and a second prompt is closed but not told."""
    failures: list[str] = []
    terminal = joel.window("foot")
    if not said(lines, "an agent opened a window over Joel's work: foot"):
        failures.append(f"3: the journal never said the terminal: {lines.read_text()!r}")
    if joel.asked_to_close(terminal, CLOSING):
        failures.append("3: an agent's terminal was closed")

    again = joel.window("gcr-prompter")
    if not joel.asked_to_close(again, CLOSING):
        failures.append(f"4: a second prompt wasn't closed within {CLOSING} s")
    written(lines, "close a password prompt", 2)
    notices = told.read_text()
    if notices.count("An agent's password window was closed") != 1:
        failures.append(f"4: Joel wasn't told once of the scope: {notices!r}")
    return failures


def main() -> None:
    """Run the five cases against the daemon, and fail naming each that didn't hold."""
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
    lines = here / "logs" / "windows.log"
    journal = lines.open("w")
    env = {**os.environ, "WINDOWS_PROC": str(processes), "WINDOWS_TOLD": str(told)}
    # The daemon under test, named by the check.
    running = subprocess.Popen([program, str(daemon)], env=env, stderr=journal)
    failures: list[str] = []
    try:
        heard_niri(daemon)
        joel = Connection(Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"])
        failures += prompts_close(joel, cgroup, lines, told)
        failures += only_prompts_close(joel, lines, told)

        # Case 5: the daemon goes, a prompt opens, and the daemon comes back, as at a switch.
        running.terminate()
        running.wait()
        daemon.unlink(missing_ok=True)
        waiting = joel.window("gcr-prompter")
        running = subprocess.Popen([program, str(daemon)], env=env, stderr=journal)
        heard_niri(daemon)
        if not joel.asked_to_close(waiting, CLOSING):
            failures.append("5: a prompt open as the daemon started again wasn't closed")
    finally:
        running.terminate()
        running.wait()
        journal.close()
    if failures:
        sys.exit("\n".join(failures))
    print("an agent's password prompts close, told once, and neither Joel's nor its terminal does")


if __name__ == "__main__":
    main()

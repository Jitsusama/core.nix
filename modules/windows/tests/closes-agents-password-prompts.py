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
5. An agent's prompt from another scope, already open as the daemon starts again, as after a
   switch, is closed once it hears niri, and Joel's own from case 1 still isn't. The agent's is
   opened by a process of its own, `--prompt`, so the two are weighed by their own cgroups.
6. Case 3's terminal, which opened focused, is put behind, on a workspace no screen shows, and
   the window focused before it is focused again.

Case 4's count is read once case 5's notice is written, so a second notice for case 4's scope,
which would go up as the second prompt closed, has had time to land.

The daemon's stand-in /proc gives every window of this process one cgroup, so case 5's
agent's prompt runs in a child, this file with `--prompt`: it opens the prompt, says so, and
says whether niri asked it to close within its patience.

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
from typing import TYPE_CHECKING

from stand_in import PATIENCE, Connection, ask, focused_window, settle

if TYPE_CHECKING:
    from collections.abc import Callable
    from typing import Any

APP = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gcr-prompter-1.scope\n"
JOB = (
    "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/"
    "jobs-work-keyring.slice/run-r1.scope\n"
)
# How long niri and the daemon have to close an agent's prompt, as the guard promises.
CLOSING = 1.0
# The prompts case 4 has the scope open: case 2's, and its own.
SCOPES_PROMPTS = 2


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


def prompts_close(joel: Connection, cgroup: Path, lines: Path, told: Path, own: int) -> list[str]:
    """Cases 1 and 2: Joel's own prompt stays, and the job's is closed, journalled and told."""
    failures: list[str] = []
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
    if "\n--\n" not in f"\n{notices}":
        failures.append(f"2: its words didn't follow a -- ending the flags: {notices!r}")
    return failures


def only_prompts_close(joel: Connection, lines: Path) -> list[str]:
    """Cases 3 and 4: the job's terminal stays, and a second prompt is closed but not told."""
    failures: list[str] = []
    # Joel's own prompt, case 1's and niri's first, has the focus as the terminal opens, so it's
    # his window the focus goes back to, not the agent's prompt case 2 only asked to close.
    joels = min(
        int(one["id"]) for one in ask("Windows")["Windows"] if one["app_id"] == "gcr-prompter"
    )
    ask({"Action": {"FocusWindow": {"id": joels}}})
    settle(joel)
    before = focused_window()
    if before != joels:
        failures.append(f"6: Joel's prompt {joels} didn't take the focus, {before} has it")
    terminal = joel.window("foot")
    if not said(lines, 'an agent opened a window over Joel\'s work: "foot"'):
        failures.append(f"3: the journal never said the terminal: {lines.read_text()!r}")
    if joel.asked_to_close(terminal, CLOSING):
        failures.append("3: an agent's terminal was closed")
    failures += goes_behind(joel, before)

    again = joel.window("gcr-prompter")
    if not joel.asked_to_close(again, CLOSING):
        failures.append(f"4: a second prompt wasn't closed within {CLOSING} s")
    closing = 'close a password prompt an agent opened: "gcr-prompter" "", from run-r1.scope'
    if written(lines, closing, SCOPES_PROMPTS).count(closing) < SCOPES_PROMPTS:
        failures.append(f"4: the journal didn't say the second was closed: {lines.read_text()!r}")
    return failures


def newest(app_id: str) -> dict[str, Any] | None:
    """Find the window niri opened last with this app id, as it lists it, if it lists one."""
    windows = [window for window in ask("Windows")["Windows"] if window["app_id"] == app_id]
    return max(windows, key=lambda window: int(window["id"]), default=None)


def in_view(window: dict[str, Any]) -> bool:
    """Whether a screen shows the workspace this window is on."""
    listed = ask("Workspaces")["Workspaces"]
    return any(one["is_active"] for one in listed if one["id"] == window["workspace_id"])


def goes_behind(joel: Connection, before: int | None) -> list[str]:
    """Case 6: the job's terminal goes behind, and the window focused before has it again."""
    if before is None:
        return ["6: no window had the focus before the terminal opened"]
    until = time.monotonic() + PATIENCE
    while (terminal := newest("foot")) is None or in_view(terminal):
        if time.monotonic() > until:
            return [f"6: an agent's terminal stayed where Joel sees it: {terminal}"]
        joel.pump(time.monotonic() + 0.05)
    settle(joel)
    after = focused_window()
    if after != before:
        return [f"6: the focus went from window {before} to {after}, not back"]
    return []


def prompt_elsewhere() -> None:
    """As `--prompt`: open an agent's prompt, say so, and say whether niri asked it to close."""
    agents = Connection(Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"])
    prompt = agents.window("gcr-prompter")
    print("open", flush=True)
    print("closed" if agents.asked_to_close(prompt, PATIENCE) else "stayed", flush=True)


def restarted(
    started: Callable[[], subprocess.Popen[bytes]], placed: Path
) -> tuple[list[str], subprocess.Popen[bytes] | None]:
    """Case 5: with the daemon gone, an agent's prompt opens, and closes once the daemon is back.

    `started` starts the daemon and `placed` is the stand-in /proc. Answers what failed, and the
    daemon started, if it was.
    """
    failures: list[str] = []
    agents = subprocess.Popen(
        [sys.executable, __file__, "--prompt"], stdout=subprocess.PIPE, text=True
    )
    if agents.stdout is None:
        sys.exit("the agent's prompt gave nothing to read")
    its = placed / str(agents.pid)
    its.mkdir()
    (its / "cwd").symlink_to(Path.cwd())
    (its / "cgroup").write_text(JOB.replace("run-r1", "run-r2"))
    if agents.stdout.readline().strip() != "open":
        agents.kill()
        return ["5: the agent's prompt never opened"], None
    running = started()
    begun = time.monotonic()
    try:
        if agents.stdout.readline().strip() != "closed":
            failures.append("5: a prompt open as the daemon started again wasn't closed")
        elif time.monotonic() - begun > CLOSING:
            failures.append(f"5: a prompt open as the daemon started again took over {CLOSING} s")
    finally:
        agents.wait()
    return failures, running


def told_once(told: Path, lines: Path) -> list[str]:
    """Case 4's count, read once case 5's prompt was told, so a second notice had its time."""
    written(told, "run-r2.scope", 1)
    notices = told.read_text()
    failures = []
    if "run-r2.scope" not in notices:
        failures.append(f"5: Joel wasn't told of the prompt: {notices!r}")
    if notices.count("run-r1.scope") != 1:
        failures.append(f"4: Joel wasn't told once of the scope: {notices!r}")
    if "from app-gcr-prompter" in lines.read_text():
        failures.append("1: Joel's own prompt was taken for an agent's")
    return failures


def main() -> None:
    """Run the six cases against the daemon, and fail naming each that didn't hold."""
    if sys.argv[1] == "--prompt":
        prompt_elsewhere()
        return
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

    def started() -> subprocess.Popen[bytes]:
        """Start the daemon under test, named by the check, and wait until it hears niri."""
        daemon.unlink(missing_ok=True)
        begun = subprocess.Popen([program, str(daemon)], env=env, stderr=journal)
        heard_niri(daemon)
        return begun

    running = started()
    failures: list[str] = []
    try:
        joel = Connection(Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"])
        own = joel.window("gcr-prompter")
        failures += prompts_close(joel, cgroup, lines, told, own)
        failures += only_prompts_close(joel, lines)

        # Case 5: the daemon goes, an agent's prompt opens, and the daemon comes back, as at a
        # switch. This process's windows are Joel's again, so his prompt from case 1 must stay.
        running.terminate()
        running.wait()
        cgroup.write_text(APP)
        found, again = restarted(started, processes)
        failures += found
        if again is None:
            sys.exit("\n".join(failures))
        running = again
        if joel.asked_to_close(own, CLOSING):
            failures.append("5: Joel's own prompt was closed as the daemon started again")
        failures += told_once(told, lines)
    finally:
        running.terminate()
        running.wait()
        journal.close()
    if failures:
        sys.exit("\n".join(failures))
    print(
        "an agent's password prompts close, told once, and neither Joel's nor its terminal does;"
        " its terminal goes behind"
    )


if __name__ == "__main__":
    main()

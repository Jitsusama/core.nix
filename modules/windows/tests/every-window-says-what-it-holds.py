"""Open each program given on niri and fail on a window titled only by a program.

A window titled only by its program, or by a program running in it, as a
shell's "zsh", reads like every other of its kind among the launcher's
windows and in a line Joel says about one. Each argument is a command,
split as a shell would, opened on the niri in NIRI_SOCKET; once every
command has a window, each window's title must say more than its app id or
the name of any process in it, by the time the deadline passes.
"""

import json
import os
import shlex
import subprocess
import sys
import time
from pathlib import Path

# How long, in seconds, the programs have to open their windows and title
# them: kitty and the reader draw in software here, and the shell starts
# direnv before its first prompt.
DEADLINE = 90
# How long, in seconds, between looks at niri's windows.
LOOK = 0.2


def windows() -> list[dict]:
    """Niri's windows, as its IPC says them."""
    said = subprocess.run(
        ["niri", "msg", "--json", "windows"], capture_output=True, text=True, check=True
    )
    return json.loads(said.stdout)


def processes(pid: int) -> list[int]:
    """A process and every process it started, however deep."""
    found = [pid]
    for task in Path(f"/proc/{pid}/task").glob("*"):
        try:
            children = (task / "children").read_text().split()
        except OSError:
            continue
        for child in children:
            found.extend(processes(int(child)))
    return found


def names(pid: int) -> set[str]:
    """What each process in a window is called: its comm and its command's file."""
    found = set()
    for each in processes(pid):
        try:
            found.add(Path(f"/proc/{each}/comm").read_text().strip().lower())
            command = Path(f"/proc/{each}/cmdline").read_bytes().split(b"\0")[0]
            found.add(Path(command.decode()).name.lower())
        except OSError:
            continue
    return found


def bare(window: dict) -> bool:
    """Whether a window's title says nothing a program's name doesn't."""
    title = (window.get("title") or "").strip().lower()
    programs = {(window.get("app_id") or "").lower()}
    if window.get("pid"):
        programs |= names(window["pid"])
    return title in programs | {""}


def main() -> None:
    """Open each command's window and judge every title niri gives."""
    commands = [shlex.split(command) for command in sys.argv[1:]]
    started = [
        subprocess.Popen(command, stdin=subprocess.DEVNULL, env=os.environ)
        for command in commands
    ]
    try:
        deadline = time.monotonic() + DEADLINE
        seen: list[dict] = []
        while time.monotonic() < deadline:
            ended = [" ".join(c) for c, p in zip(commands, started) if p.poll() is not None]
            if ended:
                sys.exit(f"these ended before their windows were judged: {ended}")
            seen = windows()
            if len(seen) >= len(commands) and not any(bare(window) for window in seen):
                return
            time.sleep(LOOK)
        said = [(window.get("app_id"), window.get("title")) for window in seen]
        sys.exit(
            f"after {DEADLINE} s, {len(seen)} of {len(commands)} windows were open, "
            f"and these were titled only by a program: "
            f"{[s for s, w in zip(said, seen) if bare(w)]}"
        )
    finally:
        for process in started:
            process.terminate()
        for process in started:
            process.wait()


if __name__ == "__main__":
    main()

"""Opens a sandbox's windows on niri, and checks none of them takes Joel's place or his focus.

A client connected through a wp-security-context-v1 socket is a sandbox's, as a fenced agent's
command is through pi-fence-display. Its windows have to open behind what is shown and never
take the focus, whatever they ask (sandboxed-windows-stay-behind.patch), since a fenced command
opening a window must not move Joel's keys. This runs pi-fence-display on the nested niri, then:

1. Joel's window opens on the display itself and takes the focus, as his own programs do.
2. A sandbox's window opens: the focus stays on Joel's, and the window lands on a workspace no
   screen shows.
3. Joel's window makes an activation token from his focus and the sandbox uses it on its own
   window: the focus stays on the same window and workspace, the sandbox's window stays behind,
   and it's marked urgent.
4. The sandbox makes a token and Joel's program uses it on a second window of his: the focus
   stays on his first, and the second is marked urgent.
5. Joel goes to the empty workspace at the end, and a second sandbox's window opens: the
   workspace he's on is still the focused one, and the window is elsewhere.
6. Joel goes to that window himself, by its id through niri's own action as his launcher does:
   it takes the focus, and its workspace is the focused one. Behind is never out of his reach.

Every case is checked and reported before the stand-in fails.

usage: opens-sandboxed-windows-behind.py <pi-fence-display>
"""

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from stand_in import PATIENCE, Connection, ask, focused_window, opened, settle


def focused_workspace() -> int:
    """Say which workspace has the focus, by its id."""
    return next(int(ws["id"]) for ws in ask("Workspaces")["Workspaces"] if ws["is_focused"])


def shown(workspace: int) -> bool:
    """Whether a screen shows this workspace."""
    return any(ws["is_active"] for ws in ask("Workspaces")["Workspaces"] if ws["id"] == workspace)


def behind(window: dict[str, Any], focused: int, failures: list[str], case: str) -> None:
    """Check a sandbox's window is on a workspace neither focused nor shown."""
    workspace = int(window["workspace_id"])
    if workspace == focused:
        failures.append(f"{case}: the sandbox's window opened on the focused workspace")
    elif shown(workspace):
        failures.append(f"{case}: the sandbox's window opened on a workspace a screen shows")


def kept(before: int | None, failures: list[str], case: str) -> None:
    """Check the focus is on the same window as before, saying where it went if not."""
    after = focused_window()
    if after != before:
        failures.append(f"{case}: the focus moved from window {before} to {after}")
    else:
        print(f"{case}: the focus stayed on window {before}")


def main() -> None:
    """Run the six cases against pi-fence-display's socket, and fail on any that didn't hold."""
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    filtered = runtime / "pi-fence-wayland"
    with subprocess.Popen([sys.argv[1], "--socket", str(filtered)]) as display:
        try:
            until = time.monotonic() + PATIENCE
            while not filtered.is_socket():
                if time.monotonic() > until or display.poll() is not None:
                    sys.exit("pi-fence-display never opened its socket")
                time.sleep(0.05)
            failures = run(Connection(runtime / os.environ["WAYLAND_DISPLAY"]), filtered)
        finally:
            display.terminate()
    for failure in failures:
        print(failure, file=sys.stderr)
    if failures:
        sys.exit(f"{len(failures)} of a sandbox's moves reached Joel's screen or his focus")
    print("every sandbox's window opened behind, and none took the focus")


def run(joel: Connection, filtered: Path) -> list[str]:
    """Run the six cases, Joel's window on `joel` and the sandbox's on the filtered socket."""
    failures: list[str] = []
    joels_surface = joel.window("joel.window")
    joels = int(opened("joel.window", joel)["id"])
    settle(joel)
    if focused_window() != joels:
        sys.exit("Joel's own window never took the focus")

    sandbox = Connection(filtered)
    sandboxes_surface = sandbox.window("pi.fence.window")
    window = opened("pi.fence.window", joel, sandbox)
    settle(joel, sandbox)
    kept(joels, failures, "a sandbox's window opening")
    behind(window, focused_workspace(), failures, "a sandbox's window opening")

    # Joel's window has the focus back, whatever the case before did, so the token comes from
    # a keyboard enter niri still honours.
    if focused_window() != joels:
        ask({"Action": {"FocusWindow": {"id": joels}}})
        settle(joel, sandbox)
    focused = focused_workspace()
    if joel.entered is None:
        sys.exit("Joel's window was never given the keyboard")
    token = joel.token(joels_surface)
    sandbox.activate(token, sandboxes_surface)
    settle(joel, sandbox)
    case = "a sandbox activating its window with Joel's token"
    kept(joels, failures, case)
    if focused_workspace() != focused:
        failures.append(f"{case}: the focus went to another workspace")
    activated = opened("pi.fence.window", joel, sandbox)
    behind(activated, focused, failures, case)
    # Urgency is all an activation may leave. niri always says whether a window is urgent, so a
    # missing field fails here rather than passing.
    if not activated["is_urgent"]:
        failures.append(f"{case}: the sandbox's window wasn't marked urgent")

    # A token the sandbox makes, used by Joel's own program on his own window, moves nothing
    # either: Joel's second window waits behind his first, marked urgent.
    case = "Joel's program activating its window with the sandbox's token"
    joels_other_surface = joel.window("joel.other")
    joels_other = int(opened("joel.other", joel, sandbox)["id"])
    ask({"Action": {"FocusWindow": {"id": joels}}})
    settle(joel, sandbox)
    joel.activate(sandbox.token(sandboxes_surface), joels_other_surface)
    settle(joel, sandbox)
    kept(joels, failures, case)
    if not opened("joel.other", joel, sandbox)["is_urgent"]:
        failures.append(f"{case}: Joel's window {joels_other} wasn't marked urgent")

    failures.extend(from_an_empty_workspace(joel, sandbox))
    return failures


def from_an_empty_workspace(joel: Connection, sandbox: Connection) -> list[str]:
    """Run cases 5 and 6: a sandbox's window opening from the empty workspace, then reached."""
    failures: list[str] = []
    ask({"Action": {"FocusWorkspace": {"reference": {"Id": empty_workspace()}}}})
    settle(joel, sandbox)
    focused = focused_workspace()
    sandbox.window("pi.fence.second")
    second = opened("pi.fence.second", joel, sandbox)
    settle(joel, sandbox)
    kept(None, failures, "a sandbox's window opening while Joel is on an empty workspace")
    if focused_workspace() != focused:
        failures.append("an empty workspace: the focus went to another workspace")
    behind(second, focused, failures, "a sandbox's window opening on an empty workspace")

    # Joel reaches a sandbox's window himself, the way his launcher's rows do: niri's action
    # focuses it by id, needing no token, so behind never means out of his reach.
    case = "Joel going to a sandbox's window"
    reached = int(second["id"])
    ask({"Action": {"FocusWindow": {"id": reached}}})
    settle(joel, sandbox)
    if focused_window() != reached:
        failures.append(f"{case}: the focus is on {focused_window()}, not its window {reached}")
    on = int(opened("pi.fence.second", joel, sandbox)["workspace_id"])
    if focused_workspace() != on:
        failures.append(f"{case}: the focused workspace isn't its workspace {on}")
    return failures


def empty_workspace() -> int:
    """Find the empty workspace the focused screen ends with, the one niri always keeps."""
    workspaces = ask("Workspaces")["Workspaces"]
    filled = {int(window["workspace_id"]) for window in ask("Windows")["Windows"]}
    output = next(ws["output"] for ws in workspaces if ws["is_focused"])
    last = max((ws for ws in workspaces if ws["output"] == output), key=lambda ws: ws["idx"])
    if int(last["id"]) in filled:
        sys.exit("niri's last workspace has a window on it")
    return int(last["id"])


if __name__ == "__main__":
    main()

"""Picture an ask's drawer and a standing yes's row in Asks, on niri, for a words review.

A lab, not a check: modules/quickshell/checks/asks.nix builds it, and no gate runs it. It runs the
experiment's daemon over stand-ins for op and loginctl, puts up an ask from a git tree on main,
pictures the ask's drawer, says yes, and pictures the standing yes's row, into the folder SHOTS
names, at SHOTS_SCALE.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import time
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri

if TYPE_CHECKING:
    from launcher_on_niri import Json, Shell

ITEM = "op://Private/Release Test/password"


def shot(name: str) -> None:
    """Picture niri's screen as it stands into SHOTS."""
    folder = Path(os.environ["SHOTS"])
    folder.mkdir(exist_ok=True)
    subprocess.run(["grim", str(folder / f"{name}.png")], check=True)


def readied(folder: Path) -> Path:
    """Make a git tree on main, and stand-ins for op and loginctl, and give the tree."""
    git = os.environ["GIT"]
    tree = folder / "core.nix"
    tree.mkdir(parents=True)
    subprocess.run([git, "-C", str(tree), "init", "-q", "--initial-branch=main"], check=True)
    (tree / "README").write_text("a tree\n")
    subprocess.run([git, "-C", str(tree), "add", "README"], check=True)
    subprocess.run(
        [git, "-C", str(tree), "-c", "user.name=t", "-c", "user.email=t@example.com"]
        + ["commit", "-q", "-m", "a commit"],
        check=True,
    )
    shell = os.environ["BASH_PROGRAM"]
    (folder / "op").write_text(f"#!{shell}\nprintf '%s' 'this-is-not-a-secret'\n")
    (folder / "loginctl").write_text(
        f"#!{shell}\ncase \"$1\" in show-user) echo 2 ;; show-session) echo no ;; esac\n"
    )
    (folder / "op").chmod(0o755)
    (folder / "loginctl").chmod(0o755)
    return tree


def served(path: Path) -> object:
    """Give what the daemon lists of its rows now, over its socket."""
    with socket.socket(socket.AF_UNIX) as client:
        client.connect(str(path))
        client.sendall(b'{"id": 1, "ask": "list", "kind": "row"}\n')
        told = b""
        while not told.endswith(b"\n"):
            told += client.recv(65536)
    return json.loads(told)


def drawer_scrolled(shell: Shell, name: str) -> None:
    """Step into the drawer, scroll it to its foot, picture it, and step back to the rows."""
    shell.normal()
    shell.as_account("wtype -M ctrl -k w -m ctrl")
    shell.send_chars("l")
    for _ in range(30):
        shell.send_chars("j")
        # Held a moment for the drawer to scroll.
        time.sleep(0.3)
    # Held still a moment, so the scroll is done.
    time.sleep(1)
    shot(name)
    shell.as_account("wtype -M ctrl -k w -m ctrl")
    shell.send_chars("h")
    shell.on_find()


def selected(shown: Json) -> Json:
    """Give the row the launcher has selected."""
    return cast("Json", shown["selected"] or {})


def test_an_asks_drawer_and_a_standing_yes_are_pictured() -> None:
    """Picture a waiting ask's drawer, then the standing yes its yes leaves."""
    folder = Path(os.environ["TMPDIR"]) / "asks"
    tree = readied(folder)
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    asks = runtime / "secret-asks"
    (runtime / "daemons").mkdir(mode=0o700, exist_ok=True)
    program = os.environ["SECRET_ASKS"]
    with launcher_on_niri.running() as shell:
        serving = subprocess.Popen(
            [program, "serve", "--socket", str(runtime / "daemons/secret-asks.sock")]
            + ["--asks", str(asks), "--vocabulary", os.environ["WORDS"], "--item", ITEM]
            + ["--op", str(folder / "op"), "--git", os.environ["GIT"]]
            + ["--loginctl", str(folder / "loginctl"), "--journal", str(folder / "none.sock")],
        )
        try:
            pictured(shell, program, tree, asks)
        finally:
            serving.terminate()


def pictured(shell: Shell, program: str, tree: Path, asks: Path) -> None:
    """Put up an ask, picture its drawer, say yes, and picture the standing yes."""
    scale = os.environ.get("SHOTS_SCALE", "")
    if scale:
        for name in json.loads(shell.as_account("niri msg --json outputs")):
            shell.as_account(f"niri msg output {name} scale {scale}")
    subprocess.run(
        [program, "ask", "--item", ITEM, "--tree", str(tree), "--why", "To publish the crate."]
        + ["--asks", str(asks), "--session", "Publish the Crate"],
        check=True,
    )
    shell.as_account(shell.ipc("collection", "asks"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: bool(shown["rows"]))
    print(json.dumps(shell.showing(), indent=1))
    # Held still a moment, so the drawer's fade is done and the floor on a yes has passed.
    time.sleep(2)
    shot("ask-drawer")
    drawer_scrolled(shell, "ask-drawer-foot")
    # Opened afresh, so the keys are the rows' again, and held past the floor on a yes.
    shell.close_launcher()
    shell.as_account(shell.ipc("collection", "asks"))
    shell.launcher_ready()
    time.sleep(2)
    shell.send_key("ret")
    # Held a moment between the arming Return and the one that releases.
    time.sleep(0.5)
    shell.send_key("ret")
    def granted(shown: Json) -> bool:
        return any(str(one.get("is", "")).startswith("grant:") for one in shown["things"])

    deadline = time.monotonic() + 10
    while not granted(shell.showing()) and time.monotonic() < deadline:
        # Held a moment before the rows are asked again.
        time.sleep(0.5)
    shot("after-yes")
    print("granted while open:", granted(shell.showing()))
    if not granted(shell.showing()):
        shell.close_launcher()
        shell.as_account(shell.ipc("collection", "asks"))
        shell.launcher_ready()
        deadline = time.monotonic() + 30
        while not granted(shell.showing()) and time.monotonic() < deadline:
            # Held a moment before the rows are asked again.
            time.sleep(0.5)
        print("granted once opened again:", granted(shell.showing()))
    print(json.dumps(served(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons/secret-asks.sock")))
    shown = shell.showing()
    print(json.dumps(shown, indent=1))
    for _ in range(len(shown["rows"])):
        if str(selected(shell.showing()).get("is", "")).startswith("grant:"):
            break
        shell.send_key("down")
        # Held a moment for the selection to move.
        time.sleep(0.3)
    print(json.dumps(shell.showing(), indent=1))
    # Held still a moment, so the drawer's fade is done.
    time.sleep(2)
    shot("grant-row")
    assert str(selected(shell.showing()).get("is", "")).startswith("grant:"), "no grant's row"
    drawer_scrolled(shell, "grant-row-foot")

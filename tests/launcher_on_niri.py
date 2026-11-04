"""The account's whole shell on a nested niri, for checks that type into the launcher.

modules/quickshell/checks.nix runs a file of tests through pytest inside
tests/nested-niri.sh, in the build sandbox: the shell's own QML on niri's real
surfaces, typed at through wtype, in seconds rather than a machine booted for
it. wtype's keys reach the surface holding the keyboard, the launcher's, but
never niri's binds, and nothing of a machine's is here: no battery, backlight,
system services or other programs. A case needing any of those stays in the
desktop's VM tests (tests/desktop/); one that only types and reads the
launcher comes here. The helpers keep desktop_session.Desktop's names, so a
case moves between the two with few words changed: keys are pressed on the
shell, since there's no machine.
"""

from __future__ import annotations

import contextlib
import json
import os
import signal
import subprocess
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

Json = dict[str, Any]

# How long the shell may take to start, and what a wait allows by default.
STARTING = timedelta(seconds=60)
WAITING = timedelta(seconds=30)
# How often what the launcher shows is asked again while a wait has heard nothing, and the
# pause between two keys typed, as the desktop test's keys are.
POLL = 0.05
KEY_PACE_MS = 50

# QEMU's names for the keys the desktop's tests press, as wtype's arguments.
KEYS = {
    "ret": ["-k", "Return"],
    "esc": ["-k", "Escape"],
    "tab": ["-k", "Tab"],
    "backspace": ["-k", "BackSpace"],
    "left": ["-k", "Left"],
    "right": ["-k", "Right"],
    "up": ["-k", "Up"],
    "down": ["-k", "Down"],
    "ctrl-left": ["-M", "ctrl", "-k", "Left", "-m", "ctrl"],
    "ctrl-ret": ["-M", "ctrl", "-k", "Return", "-m", "ctrl"],
}


class Shell:
    """The account's shell, started on the nested niri, and what a test does with it."""

    def __init__(self, folder: Path, logs: Path) -> None:
        """Hold the shell's folder and the folder its log goes in."""
        self.folder = folder
        self.log = logs / "shell.log"
        self.process: subprocess.Popen[bytes] | None = None

    def start(self) -> None:
        """Start the shell, and wait until its launcher answers, closed."""
        with self.log.open("wb") as log:
            self.process = subprocess.Popen(  # noqa: S603
                ["qs", "-p", str(self.folder)],  # noqa: S607
                stdout=log,
                stderr=subprocess.STDOUT,
            )
        deadline = time.monotonic() + STARTING.total_seconds()
        while time.monotonic() < deadline:
            status, said = self.execute(self.ipc("showing"))
            if status == 0 and not json.loads(said)["open"]:
                return
            time.sleep(POLL * 2)
        failed = f"the shell never answered:\n{self.tail()}"
        raise AssertionError(failed)

    def stop(self) -> None:
        """End the shell."""
        if self.process is not None:
            self.process.terminate()
            self.process.wait(timeout=10)

    def tail(self) -> str:
        """Give the end of the shell's log, to say why a wait failed."""
        return "\n".join(self.log.read_text(errors="replace").splitlines()[-60:])

    def ipc(self, *words: str) -> str:
        """Give the command that calls the launcher's IPC with these words."""
        return " ".join(["qs", "-p", str(self.folder), "ipc", "call", "launcher", *words])

    def execute(self, command: str) -> tuple[int, str]:
        """Run a command in the nested session, and give how it exited and what it printed."""
        done = subprocess.run(  # noqa: S603
            ["bash", "-c", command],  # noqa: S607
            capture_output=True,
            text=True,
            check=False,
        )
        return done.returncode, done.stdout

    def as_account(self, command: str) -> str:
        """Run a command in the nested session, failing as it fails, and give what it printed."""
        status, said = self.execute(command)
        assert status == 0, f"{command!r} exited {status}: {said}"
        return said

    def showing(self) -> Json:
        """Give what the launcher shows, from its showing call."""
        return cast("Json", json.loads(self.as_account(self.ipc("showing"))))

    def mark(self) -> int:
        """Mark how far the shell's log has reached."""
        return self.log.stat().st_size

    def said_since(self, mark: int) -> tuple[list[str], int]:
        """Give each report the shell has made since the mark, and the mark they end at.

        A report is a line ending report[what] (modules/quickshell/Report.qml); a line still
        being written is left for the next read.
        """
        with self.log.open("rb") as log:
            log.seek(mark)
            written = log.read()
        whole = written[: written.rfind(b"\n") + 1]
        reports = []
        for line in whole.decode(errors="replace").splitlines():
            start = line.find("report[")
            if start >= 0 and line.endswith("]"):
                reports.append(line[start + len("report[") : -1])
        return reports, mark + len(whole)

    def reported(
        self,
        prefix: str,
        test: Callable[[str], object],
        since: int,
        timeout: timedelta = WAITING,
    ) -> None:
        """Wait until a report made after the mark starts with the prefix and passes the test."""
        deadline = time.monotonic() + timeout.total_seconds()
        while time.monotonic() < deadline:
            reports, since = self.said_since(since)
            if any(said.startswith(prefix) and test(said[len(prefix) :]) for said in reports):
                return
            time.sleep(POLL)
        failed = f"the shell never reported {prefix!r} as wanted:\n{self.tail()}"
        raise AssertionError(failed)

    @contextlib.contextmanager
    def told(self, what: str, timeout: timedelta = WAITING) -> Iterator[None]:
        """Wait, after what's done inside it, for the shell to report exactly this."""
        since = self.mark()
        yield
        self.reported(what, lambda rest: rest == "", since, timeout)

    @contextlib.contextmanager
    def frozen(self) -> Iterator[None]:
        """Hold the shell stopped while what's done inside it runs, then let it go on.

        What niri sends it meanwhile waits in its socket, so it's all heard in one turn of its
        loop as it goes on, as the desktop test's frozen unit is.
        """
        assert self.process is not None, "the shell isn't started"
        self.process.send_signal(signal.SIGSTOP)
        try:
            yield
        finally:
            self.process.send_signal(signal.SIGCONT)

    def launcher_until(self, test: Callable[[Json], object], timeout: timedelta = WAITING) -> None:
        """Wait until what the launcher shows passes the test.

        Each change the launcher reports is tested as it comes, so a state it passes through is
        seen, and it's asked again now and then besides.
        """
        since = self.mark()
        deadline = time.monotonic() + timeout.total_seconds()
        shown = self.showing()
        while not test(shown):
            assert time.monotonic() < deadline, f"the launcher never showed what's wanted: {shown}"
            reports, since = self.said_since(since)
            for said in reports:
                if said.startswith("showing ") and test(json.loads(said[len("showing ") :])):
                    return
            time.sleep(POLL)
            shown = self.showing()

    def send_key(self, key: str) -> None:
        """Press a key by QEMU's name for it, as the desktop's tests do."""
        self.as_account(" ".join(["wtype", *KEYS[key]]))

    def send_chars(self, text: str) -> None:
        """Type the text, a key at a time, at the desktop test's pace."""
        subprocess.run(["wtype", "-d", str(KEY_PACE_MS), "--", text], check=True)  # noqa: S603, S607

    def launcher_ready(self) -> None:
        """Wait until the launcher is open and holds the keys."""
        self.launcher_until(lambda shown: shown["open"] and shown["keys"])

    def launcher_closed(self) -> None:
        """Wait until the launcher has closed."""
        self.launcher_until(lambda shown: not shown["open"], timedelta(seconds=10))

    def launcher_left_closed(self) -> None:
        """Close the launcher if a test left it open, so the next test starts as every test does."""
        if self.showing()["open"]:
            print("the launcher was left open, so it's closed for the next test")
            self.as_account(self.ipc("toggle"))
            self.launcher_closed()

    def type_in(self, text: str, selecting: str = "") -> None:
        """Type into the launcher, and wait until it holds every letter, Return's row settled."""
        self.send_chars(text)

        def holds(shown: Json) -> object:
            row = shown["selected"]
            name = row["name"].lower() if row else ""
            settled = row is None or row["settled"]
            return shown["typed"].endswith(text) and settled and selecting.lower() in name

        self.launcher_until(holds)

    def answer(self, typed: str) -> None:
        """Open the launcher, once it's closed, and ask it what's typed."""
        self.launcher_until(lambda shown: not shown["open"])
        self.as_account(self.ipc("toggle"))
        self.launcher_ready()
        self.type_in(typed)

    def step_in(self, key: str = "right") -> None:
        """Press a key that steps into the selected row, and wait until the launcher holds it."""
        before = len(self.showing()["held"])
        self.send_key(key)
        self.launcher_until(lambda shown: len(shown["held"]) > before)

    def down_to(self, name: str) -> None:
        """Move the selection to the row named, and wait until it's there."""
        shown = self.showing()
        rows = shown["rows"]
        selected = (shown["selected"] or {}).get("name")
        steps = rows.index(name) - (rows.index(selected) if selected in rows else 0)
        for _ in range(abs(steps)):
            self.send_key("down" if steps > 0 else "up")
        self.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == name)

    def windows(self) -> list[Json]:
        """Give every window niri lists."""
        return cast("list[Json]", json.loads(self.as_account("niri msg --json windows")))

    def opened(self, title: str) -> Json:
        """Open a window with this title, and give it once niri lists it.

        It's foot's, which draws without a GPU, where the desktop's machine opens a kitty.
        """
        self.as_account(f"niri msg action spawn -- foot --title '{title}' sleep 3600")
        deadline = time.monotonic() + STARTING.total_seconds()
        while time.monotonic() < deadline:
            titled = [window for window in self.windows() if window["title"] == title]
            if titled:
                return titled[0]
            time.sleep(POLL * 2)
        failed = f"no window titled {title!r} opened"
        raise AssertionError(failed)

    def close(self, window: Json) -> None:
        """Close a window niri listed."""
        self.as_account(f"niri msg action close-window --id {window['id']}")

    def copied(self) -> str:
        """Give what the clipboard holds."""
        return self.as_account("wl-paste --no-newline")

    def held_still(self, span: float = 0.5, timeout: timedelta = timedelta(seconds=10)) -> bool:
        """Say whether the launcher's layout holds still for half a second before the time is up.

        There's no screen to compare here, so what's compared is where it lays out each word.
        """
        deadline = time.monotonic() + timeout.total_seconds()
        last, since = "", time.monotonic()
        while time.monotonic() < deadline:
            now = self.as_account(self.ipc("laidOut"))
            if now != last:
                last, since = now, time.monotonic()
            elif time.monotonic() - since >= span:
                return True
            time.sleep(POLL)
        return False

    def frames(self) -> int:
        """Give how many frames the launcher has drawn since the shell started."""
        return int(self.as_account(self.ipc("frames")))

    def drawn_held_still(self) -> int:
        """Count the frames the open launcher draws in three seconds left still.

        What the last key moved draws first, its glide and a drawer it opened, so the count
        begins once a second passes with no frame, or after ten. A launcher that keeps drawing,
        as a blinking caret would, never settles, and is counted anyway. The foot's clock may
        turn its minute in the three seconds, once.
        """
        began = self.frames()
        settling = time.monotonic() + 10
        while time.monotonic() < settling:
            time.sleep(1)
            settled, began = began, self.frames()
            if began == settled:
                break
        time.sleep(3)
        return self.frames() - began


def started() -> Shell:
    """Start the shell in the folder the check gives, logging where it says."""
    shell = Shell(Path(os.environ["SHELL_FOLDER"]).resolve(), Path(os.environ["LOGS"]))
    shell.start()
    return shell

"""The account's whole shell on a nested niri, for checks that type into the launcher.

modules/quickshell/checks/helpers.nix's launcherOnNiri runs a file of tests
through pytest inside tests/nested-niri.sh, in the build sandbox: the shell's
own QML on niri's real surfaces, typed at through wtype, in seconds rather than
a machine booted for it. wtype's keys reach the surface holding the keyboard,
the launcher's or the held notices', but never niri's binds, so what a bind runs
is run instead, and nothing of a machine's is here: no battery, backlight,
system services or other programs, only the machine daemon over the sandbox's
own /proc when the check gives it as MACHINE, the programs daemon over the
entries here when it gives that as PROGRAMS, warp holding WARP's link when it
gives that as WARP, and the meaning daemon's stand-in when it gives that as
MEANING. A case needing any of those stays in the desktop's VM tests
(tests/desktop/); one that only types and reads the launcher comes here. The
helpers keep desktop_session.Desktop's names, so a case moves between the two
with few words changed: keys are pressed on the shell, since there's no machine.
"""

from __future__ import annotations

import contextlib
import functools
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
# The modes the launcher's line types in: the find, and insert while a row asks for words.
TYPING = ("find", "insert")

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
    "ctrl-r": ["-M", "ctrl", "-k", "r", "-m", "ctrl"],
    "ctrl-y": ["-M", "ctrl", "-k", "y", "-m", "ctrl"],
    "ctrl-bracketright": ["-M", "ctrl", "-k", "bracketright", "-m", "ctrl"],
    # The notices read a letter by what it types (NoticeKeys.js), so Shift and g come as the
    # G they type, typed as text: wtype finds a key's name without its case, so -k G is a g.
    "shift-g": ["--", "G"],
}


class Shell:
    """The account's shell, started on the nested niri, and what a test does with it."""

    def __init__(self, folder: Path, logs: Path) -> None:
        """Hold the shell's folder and the folder its log goes in."""
        self.folder = folder
        self.log = logs / "shell.log"
        self.process: subprocess.Popen[bytes] | None = None
        self.daemons: dict[str, subprocess.Popen[bytes]] = {}

    def start(self) -> None:
        """Start the shell, and wait until its launcher answers, closed."""
        # Machine's answers come from the machine daemon, over the sandbox's own /proc, on the
        # socket the account's systemd would hold for it, saying its rows in the words the check
        # gives as MACHINE_WORDS.
        if "MACHINE" in os.environ:
            self.machine()
        # The windows come from the windows daemon, hearing this niri as the account's does, and
        # the screens' rows with them, in the words the check gives as WINDOWS_WORDS.
        if "WINDOWS" in os.environ:
            words = os.environ.get("WINDOWS_WORDS")
            self.daemon("windows", os.environ["WINDOWS"], *(["--words", words] if words else []))
        # Programs' rows come from the programs daemon, over the desktop entries this niri's
        # environment names, given the lists the check gives as PROGRAMS_SOURCES.
        if "PROGRAMS" in os.environ:
            self.daemon("programs", os.environ["PROGRAMS"], os.environ["PROGRAMS_SOURCES"])
        # WARP's link comes from warp, asking the check's stand-in warp-cli.
        if "WARP" in os.environ:
            self.daemon("warp", os.environ["WARP"])
        # The home network comes from the eero daemon's stand-in, on the socket the daemon keeps,
        # holding the contract's three-node network (tests/desktop/stand-in-eero.py).
        if "EERO" in os.environ:
            socket = Path(os.environ["XDG_RUNTIME_DIR"]) / "eero" / "socket"
            self.daemon("eero", os.environ["EERO"], os.environ["EERO_NETWORK"], socket=socket)
        # Meanings come from the meaning daemon's stand-in, given what a test writes that each
        # text means in meaning-means.json, since no model loads here
        # (tests/desktop/stand-in-meaning.py).
        if "MEANING" in os.environ:
            means = Path(os.environ["XDG_RUNTIME_DIR"]) / "meaning-means.json"
            self.daemon("meaning", os.environ["MEANING"], str(means))
        # The clipboard's history, the store's own when the check gives it as CLIPBOARD, run as
        # its unit runs it, so the launcher's copies and pastes go through it, its rows said in
        # the launcher's words the check gives as CLIPBOARD_WORDS.
        if "CLIPBOARD" in os.environ:
            socket = Path(os.environ["XDG_RUNTIME_DIR"]) / "clipboard-history.sock"
            with (self.log.parent / "clipboard-history.log").open("wb") as log:
                history = subprocess.Popen(  # noqa: S603, the program the check gives
                    [
                        os.environ["CLIPBOARD"],
                        "--socket",
                        str(socket),
                        "--vocabulary",
                        os.environ["CLIPBOARD_WORDS"],
                    ],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                self.daemons["clipboard-history"] = history
        self.launch("wb")

    def restart(self) -> None:
        """End the shell alone and start it again, as a switch restarts its unit.

        Its daemons go on running, as their own units do through a switch that leaves them be,
        and its log goes on where it was, so a mark made before still reads what came after.
        """
        assert self.process is not None, "the shell was never started"
        self.process.terminate()
        self.process.wait(timeout=10)
        self.launch("ab")
        # smithay keeps one keymap active for the seat and sends a virtual keyboard's only when
        # it differs from that one, while a keyboard made since is given the seat's own: the
        # restarted shell would read wtype's next word, if it's the last one typed, by the
        # seat's keymap, "agents" as "123456". A key of another keymap first makes the word's
        # keymap new again, as a key Joel presses does on his keyboard. F24 does nothing here.
        subprocess.run(["wtype", "-k", "F24"], check=True)  # noqa: S607

    def launch(self, mode: str) -> None:
        """Start the shell alone, its log opened in this mode, and wait until it answers, closed."""
        # What a lab gives the shell besides, as QS_GIVEN: `--debug 3768` opens it to Qt's
        # profiler (tests/launcher-keys-lab-on-niri.py).
        given = os.environ.get("QS_GIVEN", "").split()
        with self.log.open(mode) as log:
            self.process = subprocess.Popen(  # noqa: S603
                ["qs", "-p", str(self.folder), *given],  # noqa: S607
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

    def daemon(self, name: str, program: str, *given: str, socket: Path | None = None) -> None:
        """Start a daemon on the socket the account's systemd would hold for it, its log beside.

        One that keeps a socket of its own, as eero's does, is given it.
        """
        daemons = Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons"
        daemons.mkdir(mode=0o700, exist_ok=True)
        socket = socket or daemons / f"{name}.sock"
        # Appended to, so one started again after a case stood in keeps what it said before.
        with (self.log.parent / f"{name}.log").open("ab") as log:
            self.daemons[name] = subprocess.Popen(  # noqa: S603
                [program, str(socket), *given],
                stdout=log,
                stderr=subprocess.STDOUT,
            )

    def machine(self) -> None:
        """Start the machine daemon the check gives as MACHINE, in MACHINE_WORDS' words."""
        words = os.environ.get("MACHINE_WORDS")
        self.daemon("machine", os.environ["MACHINE"], *(["--words", words] if words else []))

    def daemon_stopped(self, name: str) -> None:
        """Stop the daemon started as `name`, so a case may stand in on its socket."""
        process = self.daemons.pop(name, None)
        if process is not None:
            process.terminate()
            process.wait(timeout=10)
        (Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / f"{name}.sock").unlink(missing_ok=True)

    def stop(self) -> None:
        """End the shell."""
        for process in (self.process, *self.daemons.values()):
            if process is not None:
                process.terminate()
                process.wait(timeout=10)

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
        """Press a key by QEMU's name for it, as the desktop's tests do; a letter is itself."""
        self.as_account(" ".join(["wtype", *KEYS.get(key, ["-k", key])]))

    def send_chars(self, text: str) -> None:
        """Type the text, a key at a time, at the desktop test's pace."""
        subprocess.run(["wtype", "-d", str(KEY_PACE_MS), "--", text], check=True)  # noqa: S603, S607

    def take_notices(self) -> None:
        """Take the notices by the keys, by what Super+N runs, since wtype never reaches it."""
        self.as_account(os.environ["TAKE_NOTICES"])

    def notices_gone(self, timeout: timedelta = WAITING) -> None:
        """Wait until niri lists no notices' layer."""
        deadline = time.monotonic() + timeout.total_seconds()
        while time.monotonic() < deadline:
            status, _ = self.execute("niri msg layers | grep -q notifications")
            if status != 0:
                return
            time.sleep(POLL)
        failed = f"the notices never went:\n{self.tail()}"
        raise AssertionError(failed)

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

    def normal(self) -> None:
        """Put the launcher's line in normal mode, by Escape from the find, and wait for it."""
        if self.showing()["mode"] == "normal":
            return
        self.send_key("esc")
        self.launcher_until(lambda shown: shown["mode"] == "normal")

    def on_find(self) -> None:
        """Put the launcher's line back to typing, by A, at the end of what's typed.

        It types on the find, or in insert while a row asks for words, as a password.
        """
        if self.showing()["mode"] in TYPING:
            return
        self.send_chars("A")
        self.launcher_until(lambda shown: shown["mode"] in TYPING)

    def close_launcher(self) -> None:
        """Press Escape until normal is at rest, then close the launcher by :q, as Joel does.

        Escape gives normal from the find, lets the key list go if it's shown, and cancels a
        count or a key begun, but never closes (Joel, 2026-11-05: "fuck escape quitting"), so
        :q does, from any depth.
        """

        def where(shown: Json) -> tuple[object, ...]:
            return (
                shown["open"],
                shown["mode"],
                len(shown["held"]),
                shown.get("listing"),
                shown.get("command") is not None,
                shown["operator"] or shown["counted"] or shown["begun"],
            )

        def moved(was: tuple[object, ...], shown: Json) -> bool:
            return where(shown) != was

        for _ in range(12):
            was = where(self.showing())
            if not was[0]:
                return
            _, mode, _, listing, commanding, pending = was
            if mode == "normal" and not (listing or commanding or pending):
                self.send_chars(":q")
                self.send_key("ret")
            else:
                self.send_key("esc")
            self.launcher_until(functools.partial(moved, was))
        still = self.showing()
        message = f"the launcher was still open after 12 Escapes: {still}"
        raise AssertionError(message)

    def type_in(self, text: str, selecting: str = "") -> None:
        """Type into the launcher, and wait until it holds every letter, Return's row settled.

        In normal mode it goes back to typing first, at the end of what's typed.
        """
        self.on_find()
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

    def step_in(self, key: str = "ret") -> None:
        """Press a key that steps into the selected row, and wait until the launcher holds it.

        Return goes in, from the find or normal alike.
        """
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

    def opened(self, title: str, app: str = "", running: str = "sleep 3600") -> Json:
        """Open a window with this title, as this app's if one's named, running this.

        It's given once niri lists it. It's foot's, which draws without a GPU, where the
        desktop's machine opens a kitty.
        """
        carrying = f"--app-id '{app}' " if app else ""
        self.as_account(f"niri msg action spawn -- foot {carrying}--title '{title}' {running}")
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
        """Give what the clipboard holds, and nothing while it holds nothing."""
        # wl-paste says "Nothing is copied" and exits 1 the moment before a
        # copy lands, so a wait for the copy reads that as not yet rather
        # than failing on it (2026-11-09, Passwords' copy on a busy builder).
        # Any other failure is the clipboard's own, and fails.
        done = subprocess.run(
            ["wl-paste", "--no-newline"],  # noqa: S607
            capture_output=True,
            text=True,
            check=False,
        )
        if done.returncode == 0:
            return done.stdout
        assert "Nothing is copied" in done.stderr, f"wl-paste failed: {done.stderr}"
        return ""

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
        as a blinking caret would, never settles, and is counted anyway: Joel's caret blinks
        for 15 s after a key, as his kitty's does, so the check holds it steady
        (jitsusama.quickshell.caret.fade). The foot's clock may
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


@contextlib.contextmanager
def niri_held() -> Iterator[None]:
    """Hold the nested niri stopped while what's done inside it runs, so it answers no frame."""
    held = [
        int(process.name)
        for process in Path("/proc").iterdir()
        if process.name.isdigit()
        and (process / "cmdline").read_bytes().split(b"\0")[1:2] == [b"-c"]
        and (process / "comm").read_text().strip() == "niri"
    ]
    assert len(held) == 1, f"there should be one niri to hold, not {held}"
    os.kill(held[0], signal.SIGSTOP)
    try:
        yield
    finally:
        os.kill(held[0], signal.SIGCONT)


def started() -> Shell:
    """Start the shell in the folder the check gives, logging where it says."""
    shell = Shell(Path(os.environ["SHELL_FOLDER"]).resolve(), Path(os.environ["LOGS"]))
    shell.start()
    return shell


@contextlib.contextmanager
def running() -> Iterator[Shell]:
    """Give the shell, started, and stop it once done with, as a test file's fixture does."""
    shell = started()
    try:
        yield shell
    finally:
        shell.stop()

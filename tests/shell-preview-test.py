"""Test that shell-preview lays the shell out to follow the working copy, and ends what it starts.

A preview is only worth its while if an edit to a tracked file shows without building
again, so each built file the working copy holds must be linked to it, and only what the
build made copied. And however it's ended, the nested desktop it started must end with it.
The tool is the file SHELL_PREVIEW names, since it's a program rather than a module; BASH
names the shell the stand-in desktops run in.
"""

import importlib.machinery
import importlib.util
import os
import signal
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, cast

if TYPE_CHECKING:
    from collections.abc import Mapping


class Preview(Protocol):
    """What the tests use of shell-preview."""

    def digest(self, path: Path) -> str:
        """Give a file's SHA-256."""

    def lay_out(self, built: Path, known: Mapping[str, Path], into: Path) -> tuple[int, int]:
        """Lay the built folder out, a tracked file as a link to it and any other copied."""

    def niri_config(self, built: Path, into: Path, quickshell: Path, shell: Path) -> Path:
        """Copy niri's config, starting the shell laid out at shell, and give its path."""

    def stops_as_exit(self) -> None:
        """End the preview as an exit at each of its stop signals."""

    def present(self, group: int) -> bool:
        """Say whether any process is left in a process group."""

    def end(self, session: subprocess.Popen[bytes], grace: float = ...) -> None:
        """End every process in the session's group: asked, then killed once grace runs out."""


def load(path: str) -> Preview:
    """Load the tool from its path, as a module of no package."""
    loader = importlib.machinery.SourceFileLoader("shell_preview", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        message = f"{path} can't be loaded"
        raise ImportError(message)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("Preview", module)


class Folders(unittest.TestCase):
    """A built folder, a working copy and a preview's own, made afresh for each test."""

    tool: Preview

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["SHELL_PREVIEW"])

    def setUp(self) -> None:
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.root = Path(folder.name)
        self.built = self.root / "built"
        self.work = self.root / "work"
        self.into = self.root / "into"
        for path in [self.built / "launcher", self.work / "modules/quickshell/launcher"]:
            path.mkdir(parents=True)

    def write(self, path: Path, text: str) -> Path:
        """Write a file, and give its path."""
        path.write_text(text)
        return path


class LayOut(Folders):
    """Laying the built Quickshell folder out."""

    def test_a_tracked_file_is_linked_to_the_working_copy(self) -> None:
        source = self.write(self.work / "modules/quickshell/Launcher.qml", "Item {}\n")
        self.write(self.built / "Launcher.qml", "Item {}\n")
        linked, copied = self.tool.lay_out(
            self.built, {self.tool.digest(source): source}, self.into
        )
        self.assertEqual((linked, copied), (1, 0))
        self.assertEqual((self.into / "Launcher.qml").readlink(), source)

    def test_a_file_the_build_made_is_copied(self) -> None:
        self.write(self.built / "Theme.qml", "QtObject { property color accent }\n")
        linked, copied = self.tool.lay_out(self.built, {}, self.into)
        self.assertEqual((linked, copied), (0, 1))
        self.assertFalse((self.into / "Theme.qml").is_symlink())
        self.assertIn("accent", (self.into / "Theme.qml").read_text())

    def test_a_file_in_a_folder_keeps_its_place(self) -> None:
        (self.work / "modules/tuple").mkdir(parents=True)
        source = self.write(self.work / "modules/tuple/Tuple.qml", "Item { id: tuple }\n")
        self.write(self.built / "launcher/Tuple.qml", "Item { id: tuple }\n")
        self.tool.lay_out(self.built, {self.tool.digest(source): source}, self.into)
        self.assertEqual((self.into / "launcher/Tuple.qml").readlink(), source)

    def test_an_edited_file_is_copied_as_built(self) -> None:
        source = self.write(self.work / "modules/quickshell/Launcher.qml", "Item { edited }\n")
        self.write(self.built / "Launcher.qml", "Item {}\n")
        linked, copied = self.tool.lay_out(
            self.built, {self.tool.digest(source): source}, self.into
        )
        self.assertEqual((linked, copied), (0, 1))


class NiriConfig(Folders):
    """niri's config, copied to start the preview's shell."""

    def test_the_config_starts_the_laid_out_shell_and_keeps_its_includes(self) -> None:
        niri = self.root / "niri"
        niri.mkdir()
        self.write(niri / "config.kdl", 'include "binds.kdl"\n')
        self.write(niri / "binds.kdl", "binds {}\n")
        config = self.tool.niri_config(
            niri, self.into, Path("/store/quickshell/bin/quickshell"), Path("/home/someone/a shell")
        )
        self.assertEqual((self.into / "binds.kdl").read_text(), "binds {}\n")
        self.assertTrue(
            config.read_text().endswith(
                'spawn-at-startup "/store/quickshell/bin/quickshell" "-p" "/home/someone/a shell"\n'
            )
        )
        self.assertIn('include "binds.kdl"', config.read_text())


class Ending(unittest.TestCase):
    """Ending the nested desktop, here a stand-in: a shell and the sleep it started."""

    tool: Preview

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["SHELL_PREVIEW"])

    def desktop(self, script: str) -> subprocess.Popen[bytes]:
        """Start a stand-in desktop in a group of its own, as the preview starts niri.

        It's given back once it has started the sleep, so ending it has a child to end too.
        """
        session = subprocess.Popen(  # noqa: S603
            [os.environ["BASH"], "-c", f"{script}; sleep 60 & echo started; wait"],
            stdout=subprocess.PIPE,
            start_new_session=True,
        )
        self.addCleanup(self.take_away, session)
        started = session.stdout.readline() if session.stdout else b""
        self.assertEqual(started, b"started\n")
        return session

    def take_away(self, session: subprocess.Popen[bytes]) -> None:
        """Kill what a test left of its stand-in, so a failing one leaves nothing running."""
        if self.tool.present(session.pid):
            os.killpg(session.pid, signal.SIGKILL)
        session.wait()
        if session.stdout:
            session.stdout.close()

    def test_ending_asks_the_desktop_and_all_it_started_to_end(self) -> None:
        session = self.desktop("true")
        self.tool.end(session)
        self.assertFalse(self.tool.present(session.pid), "something it started is left")
        self.assertEqual(session.returncode, -signal.SIGTERM)

    def test_a_desktop_that_ignores_being_asked_is_killed(self) -> None:
        session = self.desktop("trap '' TERM")
        self.tool.end(session, grace=0.2)
        self.assertFalse(self.tool.present(session.pid), "something it started is left")
        self.assertEqual(session.returncode, -signal.SIGKILL)

    def test_each_stop_signal_ends_the_preview_as_an_exit(self) -> None:
        stops = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)
        for stop in stops:
            kept = signal.getsignal(stop)
            if kept is not None:
                self.addCleanup(signal.signal, stop, kept)
        self.tool.stops_as_exit()
        for stop in stops:
            with self.subTest(stop=stop.name), self.assertRaises(SystemExit) as ended:
                signal.raise_signal(stop)
            self.assertEqual(ended.exception.code, 128 + stop)


if __name__ == "__main__":
    unittest.main()

"""Test that shell-preview lays the shell out to follow the working copy.

A preview is only worth its while if an edit to a tracked file shows without building
again, so each built file the working copy holds must be linked to it, and only what the
build made copied. The tool is the file SHELL_PREVIEW names, since it's a program rather than
a module.
"""

import importlib.machinery
import importlib.util
import os
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


if __name__ == "__main__":
    unittest.main()

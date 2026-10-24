"""Test that changed-checks names each check by how its derivation moved.

A check is worth building again only when its derivation differs from the
base's, so the comparison has to tell a changed check from one that stopped
evaluating, and one never evaluated from one fixed. Each side's checks are
evaluated a few dozen at a time, in a process each, and the base's are kept
by its commit, so a second run evaluates only the working tree. The tool is
the file CHANGED_CHECKS names, since it's a program rather than a module.
"""

import importlib.machinery
import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, cast

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping, Sequence


class Comparison(Protocol):
    """What the tests read of a comparison."""

    added: list[str]
    changed: list[str]
    removed: list[str]
    broken: list[str]

    def to_build(self, *, vm: bool) -> list[str]:
        """Give the checks worth building, with or without the VM tests."""


class Tool(Protocol):
    """What the tests use of changed-checks."""

    def compare(
        self, before: Mapping[str, str | None], after: Mapping[str, str | None]
    ) -> Comparison:
        """Compare each check's derivation at the base with the working tree's."""

    def chunks(self, names: Sequence[str], size: int) -> list[list[str]]:
        """Split the names, in order, into lists of at most size."""

    def selecting(self, names: Sequence[str]) -> str:
        """Give the Nix function taking the checks to these ones' derivations."""

    def kept(
        self, path: Path, evaluate: Callable[[], dict[str, str | None]]
    ) -> dict[str, str | None]:
        """Give the derivations kept at the path, or evaluate and keep them."""


def load(path: str) -> Tool:
    """Load the tool from its path, as a module of no package."""
    loader = importlib.machinery.SourceFileLoader("changed_checks", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        message = f"{path} can't be loaded"
        raise ImportError(message)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("Tool", module)


BEFORE = {
    "same": "a.drv",
    "moved": "b.drv",
    "vm-moved": "c.drv",
    "gone": "d.drv",
    "breaks": "e.drv",
    "mends": None,
    "never": None,
}
AFTER = {
    "same": "a.drv",
    "moved": "b2.drv",
    "vm-moved": "c2.drv",
    "new": "f.drv",
    "breaks": None,
    "mends": "g.drv",
    "never": None,
}


class Compare(unittest.TestCase):
    """The comparison of a base's checks with a working tree's."""

    tool: Tool

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["CHANGED_CHECKS"])

    def test_a_check_whose_derivation_differs_has_changed(self) -> None:
        comparison = self.tool.compare(BEFORE, AFTER)
        self.assertEqual(comparison.changed, ["mends", "moved", "vm-moved"])

    def test_a_check_only_in_the_working_tree_is_added(self) -> None:
        self.assertEqual(self.tool.compare(BEFORE, AFTER).added, ["new"])

    def test_a_check_only_at_the_base_is_removed(self) -> None:
        self.assertEqual(self.tool.compare(BEFORE, AFTER).removed, ["gone"])

    def test_a_check_that_stops_evaluating_is_broken_not_changed(self) -> None:
        comparison = self.tool.compare(BEFORE, AFTER)
        self.assertEqual(comparison.broken, ["breaks"])
        self.assertNotIn("breaks", comparison.changed)

    def test_a_check_that_never_evaluated_is_named_nowhere(self) -> None:
        comparison = self.tool.compare(BEFORE, AFTER)
        named = comparison.added + comparison.changed + comparison.removed + comparison.broken
        self.assertNotIn("never", named)
        self.assertNotIn("same", named)

    def test_the_vm_tests_are_left_out_when_asked(self) -> None:
        comparison = self.tool.compare(BEFORE, AFTER)
        self.assertEqual(comparison.to_build(vm=True), ["new", "mends", "moved", "vm-moved"])
        self.assertEqual(comparison.to_build(vm=False), ["new", "mends", "moved"])


class Chunks(unittest.TestCase):
    """Evaluating the checks a few at a time, so no process holds them all."""

    tool: Tool

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["CHANGED_CHECKS"])

    def test_every_name_is_in_one_chunk_in_its_order(self) -> None:
        names = ["a", "b", "c", "d", "e"]
        self.assertEqual(self.tool.chunks(names, 2), [["a", "b"], ["c", "d"], ["e"]])

    def test_no_checks_need_no_evaluation(self) -> None:
        self.assertEqual(self.tool.chunks([], 40), [])

    def test_a_chunk_names_each_of_its_checks(self) -> None:
        chosen = self.tool.selecting(["formatting", "vm-desktop-works"])
        self.assertIn('"formatting"', chosen)
        self.assertIn('"vm-desktop-works"', chosen)

    def test_a_name_nix_would_read_as_code_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            self.tool.selecting(["ok", "x''; builtins.abort \"\"; ''"])


class Kept(unittest.TestCase):
    """Keeping a commit's derivations, which never change, so it's evaluated once."""

    tool: Tool

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["CHANGED_CHECKS"])

    def setUp(self) -> None:
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.path = Path(folder.name) / "kept" / "abc-x86_64-linux.json"
        self.evaluated = 0

    def evaluate(self) -> dict[str, str | None]:
        """Stand in for an evaluation, counting how often it's asked."""
        self.evaluated += 1
        return {"a": "a.drv", "broken": None}

    def test_the_first_run_evaluates_and_keeps_what_it_found(self) -> None:
        found = self.tool.kept(self.path, self.evaluate)
        self.assertEqual(found, {"a": "a.drv", "broken": None})
        self.assertEqual(self.evaluated, 1)
        self.assertTrue(self.path.exists())

    def test_a_later_run_reads_what_was_kept_without_evaluating(self) -> None:
        self.tool.kept(self.path, self.evaluate)
        found = self.tool.kept(self.path, self.evaluate)
        self.assertEqual(found, {"a": "a.drv", "broken": None})
        self.assertEqual(self.evaluated, 1)


if __name__ == "__main__":
    unittest.main()

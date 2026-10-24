"""Test that changed-checks names each check by how its derivation moved.

A check is worth building again only when its derivation differs from the
base's, so the comparison has to tell a changed check from one that stopped
evaluating, and one never evaluated from one fixed. The tool is the file
CHANGED_CHECKS names, since it's a program rather than a module.
"""

import importlib.machinery
import importlib.util
import os
import unittest
from typing import TYPE_CHECKING, Protocol, cast

if TYPE_CHECKING:
    from collections.abc import Mapping


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


if __name__ == "__main__":
    unittest.main()

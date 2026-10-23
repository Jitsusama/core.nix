"""Test that run-subtests keeps what is named, and only that.

run-subtests keeps a test script's boot and helpers and only the subtests
named, so what it runs is what was asked for. The script here has a helper
between its subtests, a decorator, comments, and a string at the margin
inside a subtest, which a cut by indentation would split. The tool is the
file RUN_SUBTESTS names, since it's a program rather than a module.
"""

import ast
import importlib.machinery
import importlib.util
import os
import unittest
from typing import TYPE_CHECKING, Protocol, cast

if TYPE_CHECKING:
    from collections.abc import Sequence

SCRIPT = "".join(
    line + "\n"
    for line in [
        "start_all()",
        "# The first one.",
        'with subtest("the bar shows"):',
        '    machine.succeed("""',
        "text at the margin",
        '""")',
        "",
        "@contextlib.contextmanager",
        "def held():",
        "    yield",
        "",
        'with subtest("the launcher opens"):',
        "    held()",
        'photograph("desktop")',
    ]
)


class Tool(Protocol):
    """What the tests use of run-subtests."""

    def chunks(self, script: str) -> list[tuple[str | None, list[str]]]:
        """Split a test script into its top-level statements."""

    def names(self, script: str) -> list[str]:
        """List the script's subtests, in the order they run."""

    def cut(self, script: str, words: Sequence[str]) -> str:
        """Cut the script down to the subtests the words name."""


def load(path: str) -> Tool:
    """Load the tool from its path, as a module of no package."""
    loader = importlib.machinery.SourceFileLoader("run_subtests", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        message = f"{path} can't be loaded"
        raise ImportError(message)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("Tool", module)


class Cut(unittest.TestCase):
    """The cut of a script with two subtests, the second named."""

    tool: Tool

    @classmethod
    def setUpClass(cls) -> None:
        cls.tool = load(os.environ["RUN_SUBTESTS"])

    def test_the_subtests_are_named_in_order(self) -> None:
        self.assertEqual(self.tool.names(SCRIPT), ["the bar shows", "the launcher opens"])

    def test_the_boot_and_helpers_stay_with_the_subtest_named(self) -> None:
        cut = self.tool.cut(SCRIPT, ["launcher"])
        for kept in ("start_all()", "@contextlib", "def held", "    held()", "photograph"):
            self.assertIn(kept, cut)

    def test_a_subtest_not_named_goes_with_its_comment_and_strings(self) -> None:
        cut = self.tool.cut(SCRIPT, ["launcher"])
        for dropped in ("the bar shows", "text at the margin", "The first one"):
            self.assertNotIn(dropped, cut)

    def test_the_cut_still_parses(self) -> None:
        ast.parse(self.tool.cut(SCRIPT, ["launcher"]))

    def test_the_pieces_make_the_script_whole(self) -> None:
        pieces = "".join(line for _, lines in self.tool.chunks(SCRIPT) for line in lines)
        self.assertEqual(pieces, SCRIPT)


if __name__ == "__main__":
    unittest.main()

"""Test Claude Code's status line: what it shows of a session, and in which colours.

The status line is the program CLAUDE_STATUSLINE names, as home.nix builds it with the theme's
colours, since it's a program rather than a module.
"""

import importlib.machinery
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any, Protocol, cast


class StatusLine(Protocol):
    """What the tests use of the status line."""

    COLOURS: dict[str, str]

    def paint(self, text: str, colour: str, *, bold: bool = False) -> str:
        """Give the text in a #rrggbb colour, as a terminal draws it, bold if asked."""

    def thousands(self, tokens: int) -> str:
        """Give a count of tokens in thousands, to a tenth below ten thousand."""

    def context_tokens(self, transcript: Path) -> int | None:
        """Give how many tokens the last answer read, cached ones too, or None."""

    def status(self, session: dict[str, Any]) -> str:
        """Give the status line for Claude Code's session."""


def load(path: str) -> StatusLine:
    """Load the status line from its path, as a module of no package."""
    loader = importlib.machinery.SourceFileLoader("statusline", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        message = f"{path} can't be loaded"
        raise ImportError(message)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("StatusLine", module)


def answer(usage: dict[str, int]) -> str:
    """Give a transcript line for an answer that read and wrote these tokens."""
    return json.dumps({"type": "assistant", "message": {"usage": usage}})


class Counts(unittest.TestCase):
    """How a count of tokens reads."""

    line: StatusLine

    @classmethod
    def setUpClass(cls) -> None:
        cls.line = load(os.environ["CLAUDE_STATUSLINE"])

    def test_under_ten_thousand_reads_to_a_tenth(self) -> None:
        self.assertEqual(self.line.thousands(1234), "1.2k")

    def test_ten_thousand_and_more_reads_whole(self) -> None:
        self.assertEqual(self.line.thousands(45678), "46k")

    def test_nothing_reads_as_nought(self) -> None:
        self.assertEqual(self.line.thousands(0), "0")


class Context(unittest.TestCase):
    """How full the context is, from the session's transcript."""

    line: StatusLine

    @classmethod
    def setUpClass(cls) -> None:
        cls.line = load(os.environ["CLAUDE_STATUSLINE"])

    def setUp(self) -> None:
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.transcript = Path(folder.name) / "transcript.jsonl"

    def test_the_last_answer_counts_every_token_it_read(self) -> None:
        self.transcript.write_text(
            "\n".join(
                [
                    answer({"input_tokens": 9, "output_tokens": 500}),
                    json.dumps({"type": "user", "message": {"content": "hi"}}),
                    answer(
                        {
                            "input_tokens": 10,
                            "cache_creation_input_tokens": 200,
                            "cache_read_input_tokens": 3000,
                            "output_tokens": 40,
                        }
                    ),
                ]
            )
        )
        self.assertEqual(self.line.context_tokens(self.transcript), 3210)

    def test_a_line_that_isnt_an_answer_is_passed_over(self) -> None:
        self.transcript.write_text("\n".join([answer({"input_tokens": 7}), "{not json", "42"]))
        self.assertEqual(self.line.context_tokens(self.transcript), 7)

    def test_no_transcript_or_no_answer_says_nothing(self) -> None:
        self.assertIsNone(self.line.context_tokens(self.transcript))
        self.transcript.write_text(json.dumps({"type": "user"}))
        self.assertIsNone(self.line.context_tokens(self.transcript))


class Status(unittest.TestCase):
    """The line Claude Code shows for a session."""

    line: StatusLine

    @classmethod
    def setUpClass(cls) -> None:
        cls.line = load(os.environ["CLAUDE_STATUSLINE"])

    def test_it_shows_the_folder_and_the_model_in_the_themes_colours(self) -> None:
        shown = self.line.status(
            {
                "workspace": {"current_dir": "/home/someone/src/core.nix"},
                "model": {"id": "claude-opus-4-6", "display_name": "Opus 4.6"},
            }
        )
        folder = self.line.paint("\uf07b core.nix", self.line.COLOURS["folder"], bold=True)
        self.assertTrue(shown.startswith(folder))
        self.assertIn(self.line.paint("\uf2db Opus 4.6", self.line.COLOURS["model"]), shown)

    def test_it_shows_the_context_when_the_transcript_has_an_answer(self) -> None:
        with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False) as transcript:
            transcript.write(answer({"input_tokens": 12500}))
        self.addCleanup(Path(transcript.name).unlink)
        shown = self.line.status({"transcript_path": transcript.name, "model": {"id": "x"}})
        self.assertIn(self.line.paint("\uf019 12k", self.line.COLOURS["context"]), shown)

    def test_a_colour_is_drawn_as_its_red_green_and_blue(self) -> None:
        colour = "#" + "".join(f"{channel:02x}" for channel in (255, 128, 0))
        self.assertEqual(self.line.paint("a", colour), "\033[38;2;255;128;0ma\033[0m")


if __name__ == "__main__":
    unittest.main()

"""Test that run-subtests runs what is named, and only that.

run-subtests keeps a test script's boot and helpers and only the subtests
named, so what it runs is what was asked for. The script here has a helper
between its subtests, a decorator, comments, and a string at the margin
inside a subtest, which a cut by indentation would split. A test written for
pytest is run whole, with pytest told which tests to pick. The tool is the
file RUN_SUBTESTS names, since it's a program rather than a module; the
whole program runs against stand-ins for nix and a check's driver.
"""

import ast
import importlib.machinery
import importlib.util
import json
import os
import shlex
import shutil
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, cast

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator, Sequence

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

# A check's script as a pytest-driven VM test's testScript writes it.
PYTEST_SCRIPT = 'import vm_pytest\nvm_pytest.run(globals(), "/nix/store/x-signing-vm-test.py")\n'

# A stand-in for nix: this machine's system, and the driver it "built".
NIX = """
import os, sys
if sys.argv[1] == "eval":
    print("x86_64-linux")
else:
    print(os.environ["STAND_IN_DRIVER"])
"""

# A stand-in for a check's driver: it notes the script it was handed and the
# options pytest would read, which go with the run's folder, and fails as told.
DRIVER = """
import json, os, sys
script = open(sys.argv[sys.argv.index("--test-script") + 1]).read()
with open(os.environ["STAND_IN_RAN"], "w") as ran:
    json.dump({"script": script, "pytest": os.environ.get("PYTEST_ADDOPTS")}, ran)
sys.exit(int(os.environ.get("STAND_IN_STATUS", "0")))
"""


# A stand-in for nix-offload: it notes what it was asked, puts a picture in
# the folder it was given, and answers as told.
NIX_OFFLOAD = """
import json, os, sys
with open(os.environ["STAND_IN_ASKED"], "w") as asked:
    json.dump(sys.argv[1:], asked)
out = sys.argv[sys.argv.index("--out") + 1]
open(os.path.join(out, "desktop.png"), "w").close()
sys.exit(int(os.environ.get("STAND_IN_OFFLOAD_STATUS", "0")))
"""


class Tool(Protocol):
    """What the tests use of run-subtests."""

    def chunks(self, script: str) -> list[tuple[str | None, list[str]]]:
        """Split a test script into its top-level statements."""

    def names(self, script: str) -> list[str]:
        """List the script's subtests, in the order they run."""

    def cut(self, script: str, words: Sequence[str]) -> str:
        """Cut the script down to the subtests the words name."""

    def pytest_driven(self, script: str) -> bool:
        """Say whether the script hands its tests to pytest."""

    def selecting(self, words: Sequence[str]) -> str:
        """Give the pytest options that pick the tests the words name."""

    def main(self, argv: Sequence[str]) -> int:
        """List a check's subtests, or run those the words name."""

    def nix_string(self, text: str) -> str:
        """Write text as a Nix string."""

    @property
    def USAGE(self) -> int:  # noqa: N802 the module's constant, read through the protocol
        """What a run asked for wrongly is refused with."""


@pytest.fixture(scope="session", name="tool")
def loaded_tool() -> Tool:
    """Load the tool from its path, as a module of no package."""
    path = os.environ["RUN_SUBTESTS"]
    loader = importlib.machinery.SourceFileLoader("run_subtests", path)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        pytest.fail(f"{path} can't be loaded")
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return cast("Tool", module)


def program(path: Path, source: str) -> None:
    """Write a Python program at path, run by this Python wherever it's started."""
    path.write_text(f"#!{sys.executable}\n{source}")
    path.chmod(0o755)


class StandIn:
    """A check's driver and the nix that builds it, first on the path."""

    def __init__(self, folder: Path, runtime: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        """Lay the stand-ins out in folder, and put them first on the path.

        The runtime folder is short, as the tool asks of one it runs a driver under.
        """
        self.folder = folder
        driver = folder / "driver"
        (driver / "bin").mkdir(parents=True)
        (folder / "bin").mkdir()
        program(folder / "bin" / "nix", NIX)
        program(folder / "bin" / "nix-offload", NIX_OFFLOAD)
        program(driver / "bin" / "nixos-test-driver", DRIVER)
        self.driver = driver
        self.script = driver / "test-script"
        monkeypatch.setenv("STAND_IN_ASKED", str(folder / "asked.json"))
        monkeypatch.setenv("PATH", f"{folder / 'bin'}{os.pathsep}{os.environ['PATH']}")
        monkeypatch.setenv("STAND_IN_DRIVER", str(driver))
        monkeypatch.setenv("STAND_IN_RAN", str(folder / "ran.json"))
        monkeypatch.setenv("XDG_RUNTIME_DIR", str(runtime))
        monkeypatch.delenv("PYTEST_ADDOPTS", raising=False)

    def ran(self) -> dict[str, object] | None:
        """Give how the driver was run, or None if it wasn't."""
        ran = self.folder / "ran.json"
        return cast("dict[str, object]", json.loads(ran.read_text())) if ran.exists() else None

    def asked(self) -> list[str] | None:
        """Give what nix-offload was asked, or None if it wasn't."""
        asked = self.folder / "asked.json"
        return cast("list[str]", json.loads(asked.read_text())) if asked.exists() else None

    def ran_script(self) -> str:
        """Give the script the driver was handed."""
        ran = self.ran()
        assert ran is not None, "the driver never ran"
        return cast("str", ran["script"])


@pytest.fixture(name="stand_in")
def a_stand_in(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[StandIn]:
    """Give a check's driver and nix, as stand-ins, for one test."""
    runtime = Path(tempfile.mkdtemp(prefix="rt."))
    yield StandIn(tmp_path, runtime, monkeypatch)
    shutil.rmtree(runtime)


def test_the_subtests_are_named_in_order(tool: Tool) -> None:
    assert tool.names(SCRIPT) == ["the bar shows", "the launcher opens"]


def test_the_boot_and_helpers_stay_with_the_subtest_named(tool: Tool) -> None:
    cut = tool.cut(SCRIPT, ["launcher"])
    for kept in ("start_all()", "@contextlib", "def held", "    held()", "photograph"):
        assert kept in cut


def test_a_subtest_not_named_goes_with_its_comment_and_strings(tool: Tool) -> None:
    cut = tool.cut(SCRIPT, ["launcher"])
    for dropped in ("the bar shows", "text at the margin", "The first one"):
        assert dropped not in cut


def test_the_cut_still_parses(tool: Tool) -> None:
    ast.parse(tool.cut(SCRIPT, ["launcher"]))


def test_the_pieces_make_the_script_whole(tool: Tool) -> None:
    pieces = "".join(line for _, lines in tool.chunks(SCRIPT) for line in lines)
    assert pieces == SCRIPT


def test_a_script_handing_its_tests_to_pytest_is_known_by_its_call(tool: Tool) -> None:
    assert tool.pytest_driven(PYTEST_SCRIPT)
    assert not tool.pytest_driven(SCRIPT)
    assert not tool.pytest_driven('# vm_pytest.run(globals(), "x")\nprint("vm_pytest.run")\n')


def test_words_pick_the_tests_whose_names_hold_them(tool: Tool) -> None:
    chosen = tool.selecting(["terminal", "a refused  confirmation"])
    assert shlex.split(chosen) == ["-k", "terminal or a_refused_confirmation"]


@pytest.mark.parametrize("word", ["a'b", "(x)", "not", "OR", "   ", "a or b; rm"])
def test_a_word_pytest_would_read_as_more_than_a_name_is_refused(tool: Tool, word: str) -> None:
    with pytest.raises(ValueError, match="can't be matched against a test's name"):
        tool.selecting([word])


def test_a_scripts_subtests_named_are_all_it_runs(tool: Tool, stand_in: StandIn) -> None:
    stand_in.script.write_text(SCRIPT)

    assert tool.main(["vm-desktop-x", "launcher"]) == 0

    assert stand_in.ran_script() == tool.cut(SCRIPT, ["launcher"])
    ran = stand_in.ran()
    assert ran is not None
    assert ran["pytest"] is None


def test_a_pytest_check_runs_whole_with_its_tests_picked(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("PYTEST_ADDOPTS", "-x")

    assert tool.main(["vm-commits-are-signed", "terminal"]) == 0

    assert stand_in.ran_script() == PYTEST_SCRIPT
    ran = stand_in.ran()
    assert ran is not None
    assert ran["pytest"] == "-x -k terminal"


def test_a_pytest_checks_tests_are_listed_without_words(tool: Tool, stand_in: StandIn) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)

    assert tool.main(["vm-commits-are-signed"]) == 0

    ran = stand_in.ran()
    assert ran is not None
    assert ran["pytest"] == "--collect-only --quiet"


def test_a_pytest_check_that_fails_fails_the_run(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_STATUS", "1")

    assert tool.main(["vm-commits-are-signed", "terminal"]) == 1


def test_a_word_pytest_cant_take_is_refused_before_anything_runs(
    tool: Tool, stand_in: StandIn
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)

    assert tool.main(["vm-commits-are-signed", "a'b"]) == tool.USAGE
    assert stand_in.ran() is None


def test_there_has_the_subtests_built_elsewhere_and_runs_nothing_here(
    tool: Tool, stand_in: StandIn, tmp_path: Path
) -> None:
    out = tmp_path / "pictures"

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x", "the launcher"]) == 0

    assert stand_in.asked() == [
        "there",
        "-L",
        "--out",
        str(out),
        "--apply",
        'subtests: subtests { check = "vm-desktop-x"; words = [ "the launcher" ]; }',
        str(Path.cwd()),
        "legacyPackages.x86_64-linux.subtests",
    ]
    assert (out / "desktop.png").exists()
    assert stand_in.ran() is None


def test_there_gives_the_builds_status(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 1
    assert stand_in.ran() is None


def test_an_offload_that_cant_take_it_runs_it_here(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    stand_in.script.write_text(SCRIPT)
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", str(tool.USAGE))

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0

    assert stand_in.ran_script() == tool.cut(SCRIPT, ["launcher"])


def test_inside_a_build_the_driver_given_is_run_and_none_is_built(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    stand_in.script.write_text(SCRIPT)
    monkeypatch.setenv("STAND_IN_DRIVER", "/nowhere")
    out = tmp_path / "out"

    assert tool.main(["--driver", str(stand_in.driver), "--out", str(out), "vm-desktop-x"]) == 0

    assert (out / "subtests").read_text() == "the bar shows\nthe launcher opens\n"


def test_a_word_is_written_for_nix_as_it_reads(tool: Tool) -> None:
    assert tool.nix_string('a "b" ${c}') == '"a \\"b\\" \\${c}"'

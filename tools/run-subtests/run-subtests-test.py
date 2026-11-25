"""Test that run-subtests runs what is named, and only that.

run-subtests runs a check's script whole, with pytest told which tests to
pick, and refuses a script that doesn't hand its tests to pytest, so what it
runs is what was asked for. The tool is the file RUN_SUBTESTS names, since
it's a program rather than a module; the whole program runs against
stand-ins for nix and a check's driver.
"""

import importlib.machinery
import importlib.util
import json
import os
import re
import shlex
import shutil
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, cast

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator, Sequence

# A script the driver runs top to bottom, as VM tests were written before
# pytest, which has no tests to pick.
SCRIPT = 'start_all()\nwith subtest("the bar shows"):\n    machine.succeed("true")\n'

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

# A stand-in for a check's driver: it notes the script it was handed, the
# options pytest would read, which go with the run's folder, and how it was
# run. Asked for a listing, it lists two tests as pytest's quiet listing
# does, past a warning and a byte no text reads, or as -v lists them when
# told; and it fails as told.
DRIVER = """
import json, os, sys
script = open(sys.argv[sys.argv.index("--test-script") + 1]).read()
with open(os.environ["STAND_IN_RAN"], "w") as ran:
    json.dump({
        "script": script,
        "pytest": os.environ.get("PYTEST_ADDOPTS"),
        "unbuffered": os.environ.get("PYTHONUNBUFFERED"),
        "interactive": "--interactive" in sys.argv,
    }, ran)
if "--collect-only" in os.environ.get("PYTEST_ADDOPTS", ""):
    if os.environ.get("STAND_IN_LISTS_VERBOSE"):
        print("<Module signing-vm-test.py>")
        print("  <Function test_terminal>")
    else:
        print("vm_pytest.py:12: UserWarning: a warning before the names", flush=True)
        sys.stdout.buffer.write(b"a byte no text reads: \\xff\\n")
        sys.stdout.buffer.flush()
        print("signing-vm-test.py::test_terminal")
        print("signing-vm-test.py::test_refused[the password]")
    print()
    print("2 tests collected in 0.01s")
sys.exit(int(os.environ.get("STAND_IN_STATUS", "0")))
"""


# A stand-in for nix-offload: it notes what it was asked, puts a picture in
# the folder it was given unless told the run took none, and answers as told.
NIX_OFFLOAD = """
import json, os, sys
with open(os.environ["STAND_IN_ASKED"], "w") as asked:
    json.dump(sys.argv[1:], asked)
out = sys.argv[sys.argv.index("--out") + 1]
if os.environ.get("STAND_IN_OFFLOAD_SAYS"):
    print(os.environ["STAND_IN_OFFLOAD_SAYS"], file=sys.stderr)
if os.environ.get("STAND_IN_OFFLOAD_UNREADABLE"):
    sys.stderr.buffer.write(b"a byte no text reads: \\xff\\n")
    sys.stderr.buffer.flush()
if not os.environ.get("STAND_IN_NO_PICTURES"):
    open(os.path.join(out, "desktop.png"), "w").close()
if os.environ.get("STAND_IN_OFFLOAD_LISTS"):
    with open(os.path.join(out, "subtests"), "w") as listed:
        listed.write(os.environ["STAND_IN_OFFLOAD_LISTS"])
sys.exit(int(os.environ.get("STAND_IN_OFFLOAD_STATUS", "0")))
"""


# A stand-in for git: origin main's quarantine list is the file
# STAND_IN_QUARANTINE names, and without one main has no list.
GIT = """
import os, sys
listed = os.environ.get("STAND_IN_QUARANTINE")
if sys.argv[3:] != ["show", "refs/remotes/origin/main:tools/merge-net/quarantine.json"]:
    sys.exit(2)
if not listed:
    sys.exit(128)
print(open(listed).read())
"""


class Tool(Protocol):
    """What the tests use of run-subtests."""

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
        program(folder / "bin" / "git", GIT)
        program(driver / "bin" / "nixos-test-driver", DRIVER)
        self.driver = driver
        self.script = driver / "test-script"
        monkeypatch.setenv("STAND_IN_ASKED", str(folder / "asked.json"))
        monkeypatch.setenv("PATH", f"{folder / 'bin'}{os.pathsep}{os.environ['PATH']}")
        monkeypatch.setenv("STAND_IN_DRIVER", str(driver))
        monkeypatch.setenv("STAND_IN_RAN", str(folder / "ran.json"))
        monkeypatch.setenv("XDG_RUNTIME_DIR", str(runtime))
        monkeypatch.delenv("PYTEST_ADDOPTS", raising=False)
        monkeypatch.delenv("STAND_IN_QUARANTINE", raising=False)
        monkeypatch.delenv("DIRENV_DIR", raising=False)

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


def test_a_script_handing_its_tests_to_pytest_is_known_by_its_call(tool: Tool) -> None:
    assert tool.pytest_driven(PYTEST_SCRIPT), "a script calling vm_pytest.run wasn't known"
    assert not tool.pytest_driven(SCRIPT), "a script of its own subtests was taken for pytest's"
    assert not tool.pytest_driven('# vm_pytest.run(globals(), "x")\nprint("vm_pytest.run")\n'), (
        "a call only in a comment or a string was taken for a real one"
    )


def test_words_pick_the_tests_whose_names_hold_them(tool: Tool) -> None:
    chosen = tool.selecting(["terminal", "a refused  confirmation", "left-hand"])
    assert shlex.split(chosen) == ["-k", "terminal or a_refused_confirmation or left-hand"]


@pytest.mark.parametrize(
    "word", ["a'b", "(x)", "not", "OR", "   ", "a or b; rm", "-x", "--keep-machine-state", " -I"]
)
def test_a_word_pytest_would_read_as_more_than_a_name_is_refused(tool: Tool, word: str) -> None:
    with pytest.raises(ValueError, match="can't be matched against a test's name"):
        tool.selecting([word])


def test_a_word_starting_with_a_dash_is_refused_after_others_too(tool: Tool) -> None:
    with pytest.raises(ValueError, match="'-x' can't be matched against a test's name"):
        tool.selecting(["terminal", "-x"])


def test_a_script_not_handed_to_pytest_is_refused_before_anything_runs(
    tool: Tool, stand_in: StandIn, capsys: pytest.CaptureFixture[str]
) -> None:
    stand_in.script.write_text(SCRIPT)

    assert tool.main(["vm-desktop-x", "bar"]) == tool.USAGE

    said = capsys.readouterr().err
    assert "doesn't hand its tests to pytest" in said, f"the refusal said: {said!r}"
    assert re.search(r"nix-offload there \. checks\.\S+\.vm-desktop-x", said), (
        f"the refusal didn't say how to run the whole test: {said!r}"
    )
    assert stand_in.ran() is None, "a script with no tests to pick ran the driver"


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


def test_a_run_without_words_lists_and_says_it_ran_none(
    tool: Tool, stand_in: StandIn, capsys: pytest.CaptureFixture[str]
) -> None:
    # On 15 November merge-net needs told a session to run fifteen tests'
    # subtests with no words, and each listed, ran nothing and passed.
    stand_in.script.write_text(PYTEST_SCRIPT)

    assert tool.main(["vm-desktop-x"]) == 0

    said = capsys.readouterr()
    assert "ran none" in said.err, f"a listing didn't say it ran nothing: {said.err!r}"
    whole = re.search(r"nix-offload there \. checks\.\S+\.vm-desktop-x", said.err)
    assert whole, f"a listing didn't say how to run the whole test: {said.err!r}"
    ran = stand_in.ran()
    assert ran is not None, "a listing never asked pytest for the tests"
    assert ran["pytest"] == "--collect-only --quiet", f"a listing ran {ran['pytest']!r}"


def test_a_listing_built_elsewhere_gives_the_names_the_build_kept(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # Kept as the build's output, so a build already made lists them too.
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_LISTS", "test_terminal\n")

    assert tool.main(["--there", "vm-desktop-x"]) == 0

    asked = stand_in.asked()
    assert asked is not None, "a listing elsewhere never asked nix-offload"
    assert asked[-1] == "legacyPackages.x86_64-linux.subtests", f"it asked for {asked!r}"
    apply = asked[asked.index("--apply") + 1]
    assert "words = [  ]" in apply, f"a listing elsewhere asked for {apply!r}"
    said = capsys.readouterr()
    assert said.out == "test_terminal\n", f"a listing elsewhere printed {said.out!r}"
    assert "pictures" not in said.err, f"a listing spoke of pictures: {said.err!r}"


def test_a_listing_built_elsewhere_that_brings_back_no_names_fails(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # The build keeps a listing's names or fails, so a pass with none is
    # never one the person can read from.
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")

    assert tool.main(["--there", "vm-desktop-x"]) == 1

    assert stand_in.asked() is not None, "a listing elsewhere never asked nix-offload"
    said = capsys.readouterr()
    assert said.out == "", f"a listing bringing back none printed {said.out!r}"
    assert "brought back no names" in said.err, f"it said {said.err!r}"
    assert "build the whole test with nix-offload there" in said.err, f"it said {said.err!r}"
    assert "ran none" not in said.err, f"a listing with no names read as one: {said.err!r}"


def test_a_listing_built_elsewhere_that_brings_back_an_empty_list_fails(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_LISTS", "\n")

    assert tool.main(["--there", "vm-desktop-x"]) == 1

    assert stand_in.asked() is not None, "a listing elsewhere never asked nix-offload"
    said = capsys.readouterr().err
    assert "brought back no names" in said, f"it said {said!r}"


def test_a_listing_elsewhere_failing_with_no_names_is_never_excused_by_a_quarantine(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")

    assert tool.main(["--there", "vm-desktop-x"]) == 1
    assert stand_in.asked() is not None, "a listing elsewhere never asked nix-offload"


def served(kept: Path) -> str:
    """Say what nix-offload says of subtests it kept in `kept` from a pass before."""
    return (
        "nix-offload: these subtests passed at 0123456789ab already, so they aren't run "
        f"again; their pictures are copied from {kept}, and `nix-offload passed forget "
        "0123456789abcdef` has them run again"
    )


def test_a_listing_served_from_the_folder_it_was_kept_in_keeps_its_names(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # nix-offload copies nothing when the pass it kept is in the folder
    # asked for, so the list there is taken as that pass's own.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_kept\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", served(out))

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == 0

    assert capsys.readouterr().out == "test_kept\n", "a kept pass's names weren't shown"
    assert (out / "subtests").read_text() == "test_kept\n", "the kept pass lost its names"


def test_a_listing_served_into_a_folder_named_from_here_keeps_its_names(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # nix-offload names the folder it kept whole, as asked from here.
    monkeypatch.chdir(stand_in.folder)
    (stand_in.folder / "out").mkdir()
    (stand_in.folder / "out" / "subtests").write_text("test_kept\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", served(stand_in.folder / "out"))

    assert tool.main(["--there", "--out", "out", "vm-desktop-x"]) == 0

    assert capsys.readouterr().out == "test_kept\n", "a kept pass's names weren't shown"


def test_a_listing_served_from_a_kept_folder_that_lost_its_names_says_how_to_list_again(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    out = stand_in.folder / "out"
    out.mkdir()
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", served(out))

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == 1

    said = capsys.readouterr().err
    assert (
        f"vm-desktop-x's listing was served from a pass kept in {out}, which holds no names; "
        "`nix-offload passed forget 0123456789abcdef` has it listed again"
    ) in said, said


def test_a_build_logs_word_of_a_kept_pass_is_never_taken_for_nix_offloads(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # A line of the build's log opens with the build's name, so it can't
    # serve the folder's old list as a kept pass's.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", f"vm-test-run-vm-desktop-x> {served(out)}")

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == 1

    assert "test_stale" not in capsys.readouterr().out, "a build's log served the old list"


def test_a_listing_served_from_another_folder_never_takes_the_old_list_for_its_own(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # Kept elsewhere, the kept folder's list comes with the copy, so one
    # left in the folder asked for is from some other run.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", served(stand_in.folder / "kept"))

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == 1

    assert "test_stale" not in capsys.readouterr().out, "an old list was shown as the kept one"
    assert not (out / "subtests").exists(), "an old list was put back"


# Each is nix-offload's status and the listing's own.
@pytest.mark.parametrize("statuses", [(0, 1), (1, 1), (75, 75)])
def test_a_listing_built_elsewhere_never_prints_a_list_left_from_before(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    statuses: tuple[int, int],
) -> None:
    status, code = statuses
    # A build that brings back no list, passing, failing or taken by no
    # builder, would leave the folder's earlier list where the run reads it;
    # nor does it lose that list.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", str(status))

    # A build bringing back no list fails the listing, with nix-offload's
    # status when it failed.
    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == code

    assert stand_in.asked() is not None, "a listing elsewhere never asked nix-offload"
    said = capsys.readouterr().out
    assert "test_stale" not in said, f"a build bringing back none printed an old list: {said!r}"
    assert (out / "subtests").read_text() == "test_stale\n", "a build writing none lost the list"


@pytest.mark.parametrize("unreadable", ["", "1"])
def test_a_listing_built_elsewhere_keeps_its_new_names_over_the_old(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    unreadable: str,
) -> None:
    # A VM's log can carry any bytes, and they mustn't end the run or put
    # an old list over the new one.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_UNREADABLE", unreadable)
    monkeypatch.setenv("STAND_IN_OFFLOAD_LISTS", "test_new\n")

    assert tool.main(["--there", "--out", str(out), "vm-desktop-x"]) == 0

    assert capsys.readouterr().out == "test_new\n", "the new list's names weren't shown"
    assert (out / "subtests").read_text() == "test_new\n", "an old list went over the new one"


def test_a_listing_stopped_partway_keeps_the_list_from_before(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Ctrl-C while nix-offload runs ends the run before it says anything.
    out = stand_in.folder / "out"
    out.mkdir()
    (out / "subtests").write_text("test_kept\n")

    def stopped(_command: list[str]) -> None:
        raise KeyboardInterrupt

    monkeypatch.setattr(tool, "offloaded", stopped)

    with pytest.raises(KeyboardInterrupt):
        tool.main(["--there", "--out", str(out), "vm-desktop-x"])

    assert (out / "subtests").read_text() == "test_kept\n", "a stopped run lost the list"


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


def test_a_word_pytest_cant_take_is_refused_before_anything_is_built_elsewhere(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # The build would refuse it and fail, and a quarantine would excuse the
    # failure of a run that ran no test.
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "100")

    assert tool.main(["--there", "vm-desktop-x", "and"]) == tool.USAGE
    assert stand_in.asked() is None, "a word pytest can't take was sent to be built"


@pytest.mark.parametrize("word", ["-x", "--keep-machine-state", "-I"])
def test_a_word_read_as_an_option_is_refused_before_anything_is_built_elsewhere(
    tool: Tool, stand_in: StandIn, capsys: pytest.CaptureFixture[str], word: str
) -> None:
    # pytest would read one starting with a dash as an option of its own, so
    # it's refused before a build is asked for, as any word pytest can't take.
    assert tool.main(["--there", "vm-desktop-x", "--", word]) == tool.USAGE

    assert stand_in.asked() is None, f"{word!r} was sent to be built"
    assert "can't be matched against a test's name" in capsys.readouterr().err


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
    assert (out / "desktop.png").exists(), "the run's picture never reached the folder given"
    assert stand_in.ran() is None


@pytest.mark.usefixtures("stand_in")
def test_a_run_that_brought_back_no_pictures_says_so_and_where_a_gallery_takes_them(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0

    said = capsys.readouterr().err
    assert not any(line.startswith("pictures:") for line in said.splitlines()), (
        f"a run with no pictures said where its pictures are: {said}"
    )
    assert "no pictures: the subtests run took none" in said
    assert "vm-desktop-x.gallery" in said


@pytest.mark.usefixtures("stand_in")
def test_a_gallerys_run_with_no_pictures_names_no_other_gallery(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")

    assert tool.main(["--there", "vm-desktop-x.gallery", "launcher"]) == 0

    said = capsys.readouterr().err
    assert "no pictures: the subtests run took none" in said
    assert ".gallery.gallery" not in said


def test_there_gives_the_builds_status(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 1
    assert stand_in.ran() is None


def test_a_check_main_quarantines_fails_saying_so_and_blocking_nothing(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    listed = tmp_path / "quarantine.json"
    listed.write_text(
        json.dumps([{"test": "vm-desktop-x", "reason": "failed then passed", "owner_pr": 7}])
    )
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0
    assert tool.main(["--there", "vm-desktop-x.gallery", "launcher"]) == 0
    assert tool.main(["--there", "vm-desktop-y", "launcher"]) == 1

    said = capsys.readouterr().err
    assert (
        "vm-desktop-x failed, but main quarantines it (failed then passed; #7 owns it), "
        "so it blocks nothing"
    ) in said
    assert "vm-desktop-y failed" not in said
    assert stand_in.ran() is None


@pytest.mark.usefixtures("stand_in")
@pytest.mark.parametrize(
    "case",
    [
        ("vm-test-run-x> FAILED vm-pytest-x/x-vm-test.py::test_a[one] - assert", 0),
        (
            (
                "x> FAILED vm-pytest-x/x-vm-test.py::test_a - assert\n"
                "x> FAILED vm-pytest-x/x-vm-test.py::test_b - assert"
            ),
            1,
        ),
        ("x> action timed out after 300 seconds", 1),
        ("       > ERROR vm-pytest-x/x-vm-test.py::test_a - Timeout", 0),
        ("x> machine # FAILED x-vm-test.py::test_a - echoed", 1),
    ],
)
def test_a_check_main_quarantines_by_its_subtests_is_excused_only_for_those(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    case: tuple[str, int],
) -> None:
    says, status = case
    # As the net's quarantine since 12 November: an entry naming subtests
    # excuses a run whose every failed subtest it names, and no other, so a
    # new break in a quarantined test, or a run naming none, still blocks.
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x", "subtests": ["test_a"]}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")
    monkeypatch.setenv("STAND_IN_OFFLOAD_SAYS", says)

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == status, (
        f"a run saying {says!r} wasn't judged by the subtests main lists"
    )
    said = capsys.readouterr().err
    assert ("blocks nothing" in said) == (status == 0), said


@pytest.mark.usefixtures("stand_in")
def test_a_checks_entries_are_read_together_each_failure_said_with_its_owner(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # As the net lists a test since 13 November: each owner's subtests in
    # an entry of their own, so a run failing in both is excused by both.
    listed = tmp_path / "quarantine.json"
    listed.write_text(
        json.dumps(
            [
                {"test": "vm-desktop-x", "reason": "r1", "owner_pr": 1, "subtests": ["test_a"]},
                {"test": "vm-desktop-x", "reason": "r2", "owner_pr": 2, "subtests": ["test_b"]},
            ]
        )
    )
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")
    monkeypatch.setenv(
        "STAND_IN_OFFLOAD_SAYS",
        "x> FAILED vm-pytest-x/x-vm-test.py::test_a - assert\n"
        "x> FAILED vm-pytest-x/x-vm-test.py::test_b - assert",
    )

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0, (
        "a failure in subtests two owners hold blocked"
    )
    said = capsys.readouterr().err
    assert (
        "vm-desktop-x failed, but main quarantines test_a (r1; #1 owns it) and "
        "test_b (r2; #2 owns it), so it blocks nothing"
    ) in said, said


@pytest.mark.usefixtures("stand_in")
def test_a_check_that_isnt_a_vm_test_is_refused_and_never_excused(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # As 2RCBOJ's run of a nested-niri check on 11 November: it has no
    # subtests, so its evaluation failed, and the quarantine called it a pass.
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "the-launcher-answers-on-niri"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")
    monkeypatch.setenv(
        "STAND_IN_OFFLOAD_SAYS",
        "error: run-subtests: the-launcher-answers-on-niri isn't a VM test, so it has no "
        "subtests; build it whole",
    )

    assert tool.main(["--there", "the-launcher-answers-on-niri", "x"]) == tool.USAGE
    said = capsys.readouterr().err
    # The build's own line is echoed too, so only the tool's words show it
    # read that line as a refusal.
    assert "the-launcher-answers-on-niri has no subtests to pick" in said
    assert "quarantines it" not in said


@pytest.mark.usefixtures("stand_in")
def test_a_script_refused_inside_the_build_elsewhere_is_refused_and_never_excused(
    tool: Tool,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # The build runs this program on the check's driver, which refuses a
    # script not handed to pytest; the build fails naming no test, which a
    # whole entry would otherwise excuse as a pass.
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", "1")
    monkeypatch.setenv(
        "STAND_IN_OFFLOAD_SAYS",
        "vm-test-run-vm-desktop-x> run-subtests: vm-desktop-x's script doesn't hand its "
        "tests to pytest (tests/lib/vm_pytest.py), so it has none to pick",
    )

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == tool.USAGE, (
        "a refusal inside the build was taken for the check failing"
    )
    said = capsys.readouterr().err
    assert "vm-desktop-x has no subtests to pick" in said, said
    assert "quarantines it" not in said, said


@pytest.mark.usefixtures("stand_in")
@pytest.mark.parametrize("status", [75, 69, 255])
def test_a_run_that_never_ran_is_never_excused_by_a_quarantine(
    tool: Tool, monkeypatch: pytest.MonkeyPatch, tmp_path: Path, status: int
) -> None:
    # As when no builder was lent, between 01:30 and 04:15 on 8 November:
    # nothing ran, so nothing failed that a quarantine could excuse.
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", str(status))

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == status


def test_a_shell_loaded_for_another_tree_is_refused_rather_than_this_folder_tested(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # As `direnv exec TREE run-subtests` from the main checkout, which loads
    # the tree's shell and stays where it is: on 8 November runs meant for
    # branches tested main and kept their passes at main's commit.
    main, branch = tmp_path / "main", tmp_path / "branch"
    main.mkdir()
    (branch / "modules").mkdir(parents=True)
    monkeypatch.setenv("DIRENV_DIR", f"-{branch}")
    monkeypatch.chdir(main)

    with pytest.raises(SystemExit):
        tool.main(["--there", "vm-desktop-x", "launcher"])
    assert stand_in.asked() is None
    assert f"cd to {branch}" in capsys.readouterr().err

    assert tool.main(["--there", "--flake", str(main), "vm-desktop-x", "launcher"]) == 0
    monkeypatch.chdir(branch / "modules")
    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0


def test_without_a_list_on_main_a_failure_fails_the_run(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_STATUS", "1")

    assert tool.main(["vm-desktop-x", "launcher"]) == 1


def test_inside_a_build_a_quarantined_failure_is_the_builds_own(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    listed = tmp_path / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_STATUS", "1")
    driven = ["--driver", str(stand_in.driver), "--out", str(tmp_path / "out")]

    assert tool.main([*driven, "vm-desktop-x", "launcher"]) == 1


def test_an_offload_that_cant_take_it_runs_it_here(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_OFFLOAD_STATUS", str(tool.USAGE))

    assert tool.main(["--there", "vm-desktop-x", "launcher"]) == 0

    assert stand_in.ran_script() == PYTEST_SCRIPT
    ran = stand_in.ran()
    assert ran is not None
    assert ran["pytest"] == "-k launcher", f"the run here picked {ran['pytest']!r}"


def test_inside_a_build_the_driver_given_is_run_and_none_is_built(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    tmp_path: Path,
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_DRIVER", "/nowhere")
    out = tmp_path / "out"

    assert tool.main(["--driver", str(stand_in.driver), "--out", str(out), "vm-desktop-x"]) == 0

    ran = stand_in.ran()
    assert ran is not None, "the driver given never ran"
    assert ran["pytest"] == "--collect-only --quiet", f"a listing ran {ran['pytest']!r}"
    listed = (out / "subtests").read_text()
    wanted = "test_terminal\ntest_refused[the password]\n"
    assert listed == wanted, f"the build kept {listed!r}"
    said = capsys.readouterr().out
    assert "signing-vm-test.py::test_terminal" in said, f"the log lost the listing: {said!r}"
    assert "a byte no text reads: \ufffd" in said, f"a byte no text reads was lost: {said!r}"
    assert ran["unbuffered"] == "1", "the listing's driver held its lines back"


def test_inside_a_build_the_check_and_words_past_a_double_dash_are_read(
    tool: Tool, stand_in: StandIn, tmp_path: Path
) -> None:
    # subtests.nix hands the check and its words over past `--`.
    stand_in.script.write_text(PYTEST_SCRIPT)
    driven = ["--driver", str(stand_in.driver), "--out", str(tmp_path / "out")]

    assert tool.main([*driven, "--", "vm-desktop-x", "launcher"]) == 0

    ran = stand_in.ran()
    assert ran is not None, "the driver given never ran"
    assert ran["pytest"] == "-k launcher", f"the build picked {ran['pytest']!r}"


def test_inside_a_build_a_check_past_a_double_dash_with_no_words_is_listed(
    tool: Tool, stand_in: StandIn, tmp_path: Path
) -> None:
    # A listing asked elsewhere reaches the build as the check alone past `--`.
    stand_in.script.write_text(PYTEST_SCRIPT)
    out = tmp_path / "out"
    driven = ["--driver", str(stand_in.driver), "--out", str(out)]

    assert tool.main([*driven, "--", "vm-desktop-x"]) == 0

    ran = stand_in.ran()
    assert ran is not None, "the driver given never ran"
    assert ran["pytest"] == "--collect-only --quiet", f"a listing ran {ran['pytest']!r}"
    listed = (out / "subtests").read_text()
    assert listed == "test_terminal\ntest_refused[the password]\n", f"the build kept {listed!r}"


def test_inside_a_build_a_listing_naming_none_fails_and_keeps_none(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    tmp_path: Path,
) -> None:
    # pytest fails a listing that collects nothing, so one that passes
    # naming none is in another shape, which an empty list would hide.
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_LISTS_VERBOSE", "1")
    out = tmp_path / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")

    assert tool.main(["--driver", str(stand_in.driver), "--out", str(out), "vm-desktop-x"]) == 1

    assert not (out / "subtests").exists(), "a listing naming none kept a list"
    said = capsys.readouterr().err
    assert "named no test" in said, f"a listing naming none passed unsaid: {said!r}"
    assert "build the whole test with nix-offload there" in said, said


def test_an_interactive_listing_keeps_its_output_the_persons(
    tool: Tool,
    stand_in: StandIn,
    capfd: pytest.CaptureFixture[str],
    tmp_path: Path,
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    out = tmp_path / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")

    assert tool.main(["-I", "--out", str(out), "vm-desktop-x"]) == 0

    ran = stand_in.ran()
    assert ran is not None, "an interactive listing never ran the driver"
    assert ran["interactive"], "an interactive listing ran the driver as no terminal"
    shown = capfd.readouterr().out
    assert "signing-vm-test.py::test_terminal" in shown, f"the person lost it: {shown!r}"
    assert not (out / "subtests").exists(), "an interactive listing kept names, or old ones"


@pytest.mark.parametrize(
    "case",
    [
        (["--out", "out", "vm-desktop-x"], {"STAND_IN_LISTS_VERBOSE": "1"}, 1),
        (["--out", "out", "vm-desktop-x"], {"STAND_IN_STATUS": "1"}, 1),
        (["--there", "vm-desktop-x"], {"STAND_IN_OFFLOAD_STATUS": "100"}, 100),
    ],
)
def test_a_failed_listing_is_never_excused_by_a_quarantine(
    tool: Tool,
    stand_in: StandIn,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
    case: tuple[list[str], dict[str, str], int],
) -> None:
    # A quarantine excuses tests that ran and failed, and a listing runs
    # none, so one failing, here or in the build elsewhere, fails.
    argv, given, status = case
    stand_in.script.write_text(PYTEST_SCRIPT)
    listed = stand_in.folder / "quarantine.json"
    listed.write_text(json.dumps([{"test": "vm-desktop-x"}]))
    monkeypatch.setenv("STAND_IN_QUARANTINE", str(listed))
    monkeypatch.setenv("STAND_IN_NO_PICTURES", "1")
    for name, value in given.items():
        monkeypatch.setenv(name, value)
    monkeypatch.chdir(stand_in.folder)

    assert tool.main(argv) == status

    said = capsys.readouterr().err
    assert "blocks nothing" not in said, f"a quarantine excused a listing: {said!r}"
    assert "ran none" not in said, f"a failed listing said it listed: {said!r}"


def test_the_whole_tests_command_is_said_ready_to_paste(
    tool: Tool, stand_in: StandIn, capsys: pytest.CaptureFixture[str]
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)

    assert tool.main(["--flake", "/a flake", "vm-desktop-x"]) == 0

    said = capsys.readouterr().err
    whole = re.search(r"nix-offload there '/a flake' checks\.\S+\.vm-desktop-x", said)
    assert whole, f"the whole test's command can't be pasted as said: {said!r}"


def test_inside_a_build_a_failed_listing_keeps_no_names(
    tool: Tool, stand_in: StandIn, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    stand_in.script.write_text(PYTEST_SCRIPT)
    monkeypatch.setenv("STAND_IN_STATUS", "1")
    out = tmp_path / "out"
    out.mkdir()
    (out / "subtests").write_text("test_stale\n")

    assert tool.main(["--driver", str(stand_in.driver), "--out", str(out), "vm-desktop-x"]) == 1

    assert stand_in.ran() is not None, "the driver given never ran"
    assert not (out / "subtests").exists(), "a failed listing kept names"


def test_a_word_is_written_for_nix_as_it_reads(tool: Tool) -> None:
    assert tool.nix_string('a "b" ${c}') == '"a \\"b\\" \\${c}"'

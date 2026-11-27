"""Test that a failure main's quarantine lists blocks nothing, and no other does.

run-subtests reads origin main's tools/merge-net/quarantine.json after a
run fails: a failure every entry it matches excuses blocks nothing, saying
whose it is, and a run that never ran, a refusal or a failure no entry names
still fails. A listing and a refused word, which it never excuses either,
keep their cases in run-subtests-test.py. The whole program runs against
stand_ins.py's stand-ins, which conftest.py gives each test as its fixtures.
"""

import json
from typing import TYPE_CHECKING

import pytest
from stand_ins import PYTEST_SCRIPT, StandIn, Tool

if TYPE_CHECKING:
    from pathlib import Path


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

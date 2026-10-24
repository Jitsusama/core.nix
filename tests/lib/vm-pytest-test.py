"""Test running a pytest file inside the NixOS test driver (vm_pytest.py), against a stand-in.

Each test runs a small pytest file in this process through pytester, with the plugin given a
driver that only records what the plugin asks of its log, so the whole file takes a second.
"""

from __future__ import annotations

import contextlib
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
import vm_pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

pytest_plugins = ["pytester"]


@dataclass
class Machine:
    """A machine, known only by its name."""

    name: str


@dataclass
class Log:
    """The driver's log, recording what's grouped in it and which subtests failed."""

    said: list[str] = field(default_factory=list)
    failed: list[str] = field(default_factory=list)

    @contextlib.contextmanager
    def nested(self, message: str) -> Iterator[None]:
        """Record a group of lines."""
        self.said.append(message)
        yield

    @contextlib.contextmanager
    def subtest(self, name: str) -> Iterator[None]:
        """Record a subtest, and whether it failed, as the driver's does."""
        self.said.append(f"subtest: {name}")
        try:
            yield
        except Exception:
            self.failed.append(name)
            raise


@dataclass
class Driver:
    """The driver, with its machines and its log."""

    machines: list[Machine]
    logger: Log = field(default_factory=Log)


@pytest.fixture
def driver() -> Driver:
    """Give a driver with two machines, one named machine."""
    return Driver([Machine("machine"), Machine("server")])


@pytest.fixture
def symbols(driver: Driver) -> dict[str, Any]:
    """Give what the driver hands a script as its globals."""
    return {"driver": driver, "machine": driver.machines[0], "subtest": driver.logger.subtest}


def run_tests(
    pytester: pytest.Pytester,
    symbols: dict[str, Any],
    source: str,
    values: Path | None = None,
) -> pytest.RunResult:
    """Run a pytest file holding that source with the plugin, as the driver would."""
    tests = pytester.makepyfile(source)
    return pytester.runpytest_inprocess(
        *vm_pytest.arguments(tests), plugins=[vm_pytest.Driven(symbols, values)]
    )


def test_each_test_is_a_subtest_named_by_its_docstring(
    pytester: pytest.Pytester, symbols: dict[str, Any], driver: Driver
) -> None:
    run_tests(
        pytester,
        symbols,
        '''
        def test_one():
            """The first thing holds.

            And more about it.
            """
        ''',
    ).assert_outcomes(passed=1)
    assert "subtest: The first thing holds." in driver.logger.said


def test_a_test_without_a_docstring_is_named_as_pytest_names_it(
    pytester: pytest.Pytester, symbols: dict[str, Any], driver: Driver
) -> None:
    run_tests(pytester, symbols, "def test_unsaid(): pass").assert_outcomes(passed=1)
    assert "subtest: test_unsaid" in driver.logger.said


def test_a_failing_test_is_a_failed_subtest_with_pytest_saying_why(
    pytester: pytest.Pytester, symbols: dict[str, Any], driver: Driver
) -> None:
    result = run_tests(
        pytester,
        symbols,
        '''
        def test_sum():
            """Two and two make five."""
            total = 2 + 2
            assert total == 5
        ''',
    )
    result.assert_outcomes(failed=1)
    assert driver.logger.failed == ["Two and two make five."]
    result.stdout.fnmatch_lines(["*assert 4 == 5*"])


def test_the_fixtures_give_the_drivers_machines(
    pytester: pytest.Pytester, symbols: dict[str, Any]
) -> None:
    run_tests(
        pytester,
        symbols,
        """
        def test_machines(driver, machine, machines):
            assert machine is driver.machines[0]
            assert sorted(machines) == ["machine", "server"]
            assert machines["server"] is driver.machines[1]
        """,
    ).assert_outcomes(passed=1)


def test_what_a_tests_fixtures_do_is_grouped_under_its_name(
    pytester: pytest.Pytester, symbols: dict[str, Any], driver: Driver
) -> None:
    run_tests(
        pytester,
        symbols,
        '''
        import pytest

        @pytest.fixture
        def ready():
            pass

        def test_ready(ready):
            """It's ready."""
        ''',
    ).assert_outcomes(passed=1)
    assert driver.logger.said == ["getting ready to prove It's ready.", "subtest: It's ready."]


def test_the_values_fixture_gives_what_the_tests_nix_wrote(
    pytester: pytest.Pytester, symbols: dict[str, Any], tmp_path: Path
) -> None:
    written = tmp_path / "values.json"
    written.write_text('{"pin": "2468", "runtime": "/run/user/1000"}')
    run_tests(
        pytester,
        symbols,
        """
        def test_values(values):
            assert values == {"pin": "2468", "runtime": "/run/user/1000"}
        """,
        values=written,
    ).assert_outcomes(passed=1)


def test_a_test_asking_for_values_none_were_given_fails_saying_so(
    pytester: pytest.Pytester, symbols: dict[str, Any]
) -> None:
    result = run_tests(pytester, symbols, "def test_values(values): pass")
    result.assert_outcomes(errors=1)
    result.stdout.fnmatch_lines(["*no values were given to vm_pytest.run*"])


def test_tests_from_the_store_are_staged_under_the_name_they_were_written_with(
    tmp_path: Path,
) -> None:
    store = tmp_path / "store"
    store.mkdir()
    tests = store / "0s90b738xh329yayd1wr56i0jyjb5iid-network-revive-vm-test.py"
    tests.write_text("def test_it(): pass\n")
    folder = tmp_path / "folder"
    folder.mkdir()
    copy = vm_pytest.staged(tests, folder)
    assert copy == folder / "network-revive-vm-test.py"
    assert copy.read_text() == "def test_it(): pass\n"


def test_tests_named_without_a_hash_keep_their_name(tmp_path: Path) -> None:
    tests = tmp_path / "local-vm-test.py"
    tests.write_text("")
    folder = tmp_path / "folder"
    folder.mkdir()
    assert vm_pytest.staged(tests, folder) == folder / "local-vm-test.py"


def test_the_driver_ends_as_a_failed_subtest_ends_it_when_a_test_fails(
    monkeypatch: pytest.MonkeyPatch, symbols: dict[str, Any], tmp_path: Path
) -> None:
    tests = tmp_path / "tests.py"
    tests.write_text("")
    monkeypatch.setattr(pytest, "main", lambda *_, **__: pytest.ExitCode.TESTS_FAILED)
    with pytest.raises(SystemExit) as ended:
        vm_pytest.run(symbols, str(tests))
    assert ended.value.code == 1


def test_the_driver_carries_on_when_every_test_passes_given_a_copy_of_its_own(
    monkeypatch: pytest.MonkeyPatch, symbols: dict[str, Any], tmp_path: Path
) -> None:
    tests = tmp_path / "tests.py"
    tests.write_text("")
    asked: list[list[str]] = []

    def main(arguments: list[str], **_: object) -> pytest.ExitCode:
        asked.append(arguments)
        return pytest.ExitCode.OK

    monkeypatch.setattr(pytest, "main", main)
    vm_pytest.run(symbols, str(tests))
    ((given, *options),) = asked
    copy = Path(given)
    assert copy.name == "tests.py"
    assert copy.parent != tmp_path
    assert options == [f"--rootdir={copy.parent}", *vm_pytest.OPTIONS]


def test_the_values_file_given_to_run_is_the_one_the_tests_read(
    monkeypatch: pytest.MonkeyPatch, symbols: dict[str, Any], tmp_path: Path
) -> None:
    tests = tmp_path / "tests.py"
    tests.write_text("")
    written = tmp_path / "values.json"
    given: list[object] = []

    def main(_: list[str], plugins: list[object]) -> pytest.ExitCode:
        given.extend(plugins)
        return pytest.ExitCode.OK

    monkeypatch.setattr(pytest, "main", main)
    vm_pytest.run(symbols, str(tests), values=str(written))
    (plugin,) = given
    assert isinstance(plugin, vm_pytest.Driven)
    assert plugin.values == written

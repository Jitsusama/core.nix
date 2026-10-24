"""Run a VM test written for pytest inside the NixOS test driver.

The driver runs a test's script with the machines among the script's globals. A script that
calls `run(globals(), path)` hands a pytest file to pytest, in the driver's own process, with
the machines as fixtures. Each test is one of the driver's subtests in its log, named by the
first line of its docstring, and a failed test fails the driver as a failed subtest does.

PYTEST_ADDOPTS picks tests and adds options as it does for any pytest run, so
`PYTEST_ADDOPTS='-k reload'` before the driver's command runs only the tests it names, after
whatever fixtures they ask for.
"""

from __future__ import annotations

import inspect
import re
import shutil
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Generator, Mapping
    from contextlib import AbstractContextManager

    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

# What a run in the driver needs beyond pytest's defaults.
OPTIONS = (
    # The machines' consoles stream into the log as the tests run, never held back for a
    # failure.
    "--capture=no",
    # A test's file is named as it was written, hyphens and all, which isn't a module's name.
    "--import-mode=importlib",
    # The store is read-only, and every run starts from nothing.
    "-p",
    "no:cacheprovider",
    "--no-header",
    # The tests that took a second or more, slowest first, so what to make quicker is plain.
    "--durations=0",
    "--durations-min=1",
)

# A store path's name: its hash, then the name it was given.
STORE_NAME = re.compile(r"[0-9a-z]{32}-(?P<name>.+)")


def described(item: pytest.Item) -> str:
    """Give the sentence a test proves: its docstring's first line, or its name without one."""
    doc = inspect.getdoc(item.function) if isinstance(item, pytest.Function) else None
    return doc.splitlines()[0] if doc else item.name


class Driven:
    """The driver's machines as fixtures, and each test as one of its subtests."""

    def __init__(self, symbols: Mapping[str, Any]) -> None:
        """Take what the driver gave the script as its globals."""
        self.symbols = symbols

    @pytest.fixture(scope="session")
    def driver(self) -> Driver:
        """Give the driver itself."""
        driver: Driver = self.symbols["driver"]
        return driver

    @pytest.fixture(scope="session")
    def machine(self) -> BaseMachine:
        """Give the test's only machine, or the one named machine."""
        machine: BaseMachine = self.symbols["machine"]
        return machine

    @pytest.fixture(scope="session")
    def machines(self, driver: Driver) -> dict[str, BaseMachine]:
        """Give every machine, by its name in the test's nodes."""
        return {machine.name: machine for machine in driver.machines}

    @pytest.hookimpl(wrapper=True)
    def pytest_runtest_setup(self, item: pytest.Item) -> Generator[None]:
        """Group what the test's fixtures do under its name in the log."""
        driver: Driver = self.symbols["driver"]
        with driver.logger.nested(f"getting ready to prove {described(item)}"):
            return (yield)

    @pytest.hookimpl(wrapper=True)
    def pytest_runtest_call(self, item: pytest.Item) -> Generator[None]:
        """Run the test as a subtest, so the log names it and says if it failed.

        It's the driver's own subtest, the one a script's `with subtest(...)` uses, so the log
        reads as it always has.
        """
        subtest: Callable[[str], AbstractContextManager[None]] = self.symbols["subtest"]
        with subtest(described(item)):
            return (yield)


def staged(tests: Path, folder: Path) -> Path:
    """Copy the tests into a folder of their own, under the name they were written with.

    The driver is given them at the top of the store, and pytest lists each folder from the
    file's own up as it collects: at the top of a host's store, which holds some 180,000
    paths, that took 17 s, where a folder of its own takes none. Without the store's hash,
    the tests' names in what pytest says are the file's own.
    """
    found = STORE_NAME.fullmatch(tests.name)
    copy = folder / (found["name"] if found else tests.name)
    shutil.copyfile(tests, copy)
    return copy


def arguments(tests: Path) -> list[str]:
    """Give pytest's arguments for running the tests in that file within the driver.

    Its folder is pytest's root, so a test is named in what pytest says by its file's name
    alone, rather than by the path to it from wherever the driver runs.
    """
    return [str(tests), f"--rootdir={tests.parent}", *OPTIONS]


def run(symbols: Mapping[str, Any], tests: str) -> None:
    """Run the pytest file given with the driver's machines, ending the driver if one fails.

    It exits as the driver does on a failed subtest, so the driver stops its machines, says
    the test failed, and the build fails with it.
    """
    with tempfile.TemporaryDirectory(prefix="vm-pytest-") as folder:
        copy = staged(Path(tests), Path(folder))
        code = pytest.main(arguments(copy), plugins=[Driven(symbols)])
    if code != pytest.ExitCode.OK:
        sys.exit(1)

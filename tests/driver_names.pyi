# What the NixOS test driver gives a VM test's script, for type checkers only.
#
# The driver runs a script with its machines and helpers among the script's
# globals, so a `*-vm-test.py` file imports them from here inside
# `if TYPE_CHECKING:`, and mypy reads them as the driver's own types. A test
# whose only machine isn't named `machine` declares its own here.

from contextlib import AbstractContextManager

from test_driver.driver import Driver
from test_driver.machine import QemuMachine

driver: Driver
machine: QemuMachine

# Starts every machine at once.
def start_all() -> None: ...

# Groups what's inside under a name, which run-subtests selects by.
def subtest(name: str) -> AbstractContextManager[None]: ...

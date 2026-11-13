"""Run one part of a file's cases, so a long file's parts can run side by side.

modules/quickshell/checks/helpers.nix's launcherOnNiri gives each part the case it starts at as
CASES_FROM, the case the next part starts at as CASES_UNTIL (none for the last), and the cases
every part runs last as CASES_EVERY, such as one judging the whole run's log. A part keeps the
file's order, so a case leaning on the shell state the one before left stays with it unless a
part starts between them, and a case added joins the part it sits in. A case named that isn't
there stops the run, rather than leaving cases out.
"""

from __future__ import annotations

import os

import pytest


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    """Keep this part's run of cases, in the file's order, then those every part runs."""
    first = os.environ.get("CASES_FROM", "")
    until = os.environ.get("CASES_UNTIL", "")
    every = os.environ.get("CASES_EVERY", "").split()
    names = [item.name for item in items]
    missing = [name for name in [first, until, *every] if name and name not in names]
    if missing:
        message = f"no case is named {', '.join(missing)}"
        raise pytest.UsageError(message)
    start = names.index(first) if first else 0
    end = names.index(until) if until else len(items)
    if start >= end:
        message = f"{first} starts after {until}, so its part holds nothing"
        raise pytest.UsageError(message)
    kept = [item for item in items[start:end] if item.name not in every]
    kept += [item for item in items if item.name in every]
    config.hook.pytest_deselected(items=[item for item in items if item not in kept])
    items[:] = kept
